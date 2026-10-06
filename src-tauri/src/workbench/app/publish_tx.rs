//! **发布事务**（第三刀下半 · 收口）—— 把「审计 → 生成 → 定稿 → 本地 git → 平台 PR/MR」
//! 串成**一次手势**，并给前端一个**平台无关**的状态模型。
//!
//! # 为什么要有这一层（而不是让前端串四个按钮）
//!
//! 作者 2026-10-04 定死的产品边界：**前端只有一个「发布」入口**。
//! `wb_generate` / `wb_publish` / 「创建 PR」都不是用户动作，是这次事务的**内部步骤**。
//! 出现四个按钮的那一刻，"发布"就退化成"给开发者包了一层 CLI" —— 那不是桌面产品。
//!
//! # 锁的边界（★ 这一层存在的技术理由）
//!
//! `with_ctx` 的锁**不可重入**：事务跑在锁里，若它回头调 `wb_generate` / `wb_publish`
//! 这些命令壳，就是**自己把自己锁死**（挂死，不是报错）。
//! 所以这一层的每一个函数**只收 `&Ctx`**，复用 [`super::build::generate_with`] /
//! [`super::audit::publish_audit`] / [`super::delivery::publish_into`] 这些自由函数。
//! 判据 `the_transaction_chain_never_calls_a_command_shell` 用源码扫描钉住这条。
//!
//! # 两条链在这里汇合，但不合并
//!
//! 客户端那两条链是「软件更新」与「数据兼容」（见 `docs/PUBLISH-ARCHITECTURE.md` §5.3.1）。
//! 这里是**发布侧**：一次发布事务里，"审计"管"这次发布合不合规"，"平台状态"管
//! "PR/MR 走到哪了"。两者**不混成一句话** —— 审计红是"还不能发"，CI 红是"发了但没过"。

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::platform::{Hosting, RemoteReview, ReviewSpec, ReviewState};
use super::{state, Ctx};

/// **发布目标** —— 一次发布"发到哪个仓库、以谁的身份"，**来自发布账户配置**
/// （[`super::account`]），不是从 `git remote` 推断（作者 2026-10-04 定死）。
///
/// 命令壳从 `<appDataDir>/publish-account.json` + 凭据文件解析出它，再传给锁无关内核
/// [`run`]。内核因此不碰 `AppHandle`（= 不破锁纪律）。
#[derive(Debug, Clone)]
pub struct PublishTarget {
    /// 平台 id（`"github"` / `"gitee"`）
    pub platform: String,
    /// 配置里的仓库地址（发布目标的**权威来源**）
    pub repository_url: String,
    /// 用户名（认证 + PR/MR 归属）
    pub username: String,
    /// 凭据文件里的 Token（**只在内存里过一手**，不落任何返回值/日志）
    pub token: String,
    /// owner / repo（已从 [`repository_url`] 解析出来）
    pub owner: String,
    pub repo: String,
}

impl PublishTarget {
    /// owner / repo 之外的同一份目标（用于建 [`ReviewSpec`]）。
    fn review_spec(
        &self,
        head: String,
        base: String,
        files: usize,
        generated: usize,
    ) -> ReviewSpec {
        ReviewSpec {
            owner: self.owner.clone(),
            repo: self.repo.clone(),
            head,
            base,
            title: format!("发布：{files} 份产物"),
            body: format!(
                "由 SupportEase 工作台发布。\n\n产物 {files} 份；本轮新生成 {generated} 份。",
            ),
        }
    }
}

/// 发布事务的阶段。**这就是工作台自己的发布状态模型** ——
/// 前端只认这一档枚举，不认识 GitHub 的 "check run" / Gitee 的 MR status 这些方言。
///
/// 顺序即流程：`Audit → Generate → Commit → Push → OpenReview → Status`。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PublishStage {
    /// 停在审计（有 Blocker 红）—— 零写入
    BlockedAudit,
    /// 审计过了、正在生成 + 定稿
    Generated,
    /// 已提交（本地）
    Committed,
    /// 已推送（本地分支上了远端）
    Pushed,
    /// 已建 PR / MR
    ReviewOpened,
    /// 建了 PR、也回了状态，这一轮结束了
    StatusRead,
}

impl PublishStage {
    /// 线上 / 历史文件里的名字（与 serde 的 camelCase 形状**逐字一致**；判据钉它）。
    pub fn wire_name(self) -> &'static str {
        match self {
            PublishStage::BlockedAudit => "blockedAudit",
            PublishStage::Generated => "generated",
            PublishStage::Committed => "committed",
            PublishStage::Pushed => "pushed",
            PublishStage::ReviewOpened => "reviewOpened",
            PublishStage::StatusRead => "statusRead",
        }
    }
}

/// 一轮发布事务的结果。
///
/// ★ 它是**阶段的快照**，不是"最终结论"：事务可能停在任何一步（审计红 / 建 PR 失败），
/// 前端据 `stage` 显示"停在哪、为什么"。没有 `success: bool` 那种把过程压扁的字段 ——
/// 那正是"用户不知道卡在哪"的来源。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishTxReport {
    /// 走到哪一步了
    pub stage: PublishStage,
    /// 审计十五项的摘要（Pass / Fail 计数）—— 便于前端在事务结果里也能看到闸的结论
    pub audit_passed: usize,
    pub audit_failed: usize,
    /// 生成的产物：新写 / 未变
    pub generated: usize,
    pub unchanged: usize,
    /// 这次提交的路径（相对仓库根）；空 = 没有可提交的
    pub committed_paths: Vec<String>,
    /// 本次发布对应的 PR / MR（建了才有）
    pub review: Option<RemoteReview>,
    /// 本机 git 分支（给前端显示"发到哪个分支"）
    pub branch: Option<String>,
    /// 这次发布提交的短 sha（回执里显示"提交了哪一笔"；没提交 = `None`，如实说）
    pub commit: Option<String>,
    /// 交付根的产物份数
    pub files: usize,
    /// 这一步的一句话说明（给状态条）
    pub summary: String,
    /// **本次发布时刻**（= 写进目录 `publishedAt` 的那一个，也与 manifest 的 `updated` 同戳）。
    /// 审计没过（一个字节都没写）时是 `None` —— 如实说"这次没有发布时刻"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_at: Option<String>,
    /// **本次发布的目录指纹**（交付面 `catalog.json` 的 `revision`）。没走到生成 = `None`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
    /// 本次变化的总数（新增 / 修改 / 删除 / 未变化）。没走到生成 = `None`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub changes: Option<ChangeSummary>,
    /// 逐份明细（只含新增 / 修改 / 删除那几份；未变化的不列）
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub changed_files: Vec<FileChange>,
}

/// 事务的开关：控制"走到哪一步停"。
///
/// **默认全开**（一次点击走完全程）；`dry_run` 给"只想看看会提交什么"用
/// （`--dry-run` 那种），`open_review = false` 给"只提交推送、PR 我自己去平台建"用。
/// 它不是给前端摆开关的 —— 是给"发布"这一条链内部与判据用的。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TxOptions {
    /// 只审计 + 生成 + 定稿 + 报"会提交什么"，不 commit / 不 push / 不建 PR
    #[serde(default)]
    pub dry_run: bool,
    /// 提交推送之后要不要建 PR/MR
    #[serde(default = "default_open_review")]
    pub open_review: bool,
    /// 目标分支（PR 的 base）。空 = `main`
    #[serde(default)]
    pub base: String,
    /// 发到哪个平台（`"github"` / `"gitee"`）。`None` = 自动挑（唯一配好的那个，
    /// 或多个配好时取"与当前 remote 一致"的那个）
    #[serde(default)]
    pub platform: Option<String>,
}

fn default_open_review() -> bool {
    true
}

impl Default for TxOptions {
    fn default() -> Self {
        Self {
            dry_run: false,
            open_review: true,
            base: "main".to_owned(),
            platform: None,
        }
    }
}

impl TxOptions {
    /// 目标分支：空串回落 `main`（下游直接拿它当 PR 的 base）。
    pub fn base_or_main(&self) -> &str {
        if self.base.trim().is_empty() {
            "main"
        } else {
            self.base.trim()
        }
    }
}

/// 事务的**锁无关内核**：给定一份会话，按 [`TxOptions`] 跑完"审计 → 生成 → 定稿"，
/// 并按开关续接本地 git 与平台 PR/MR。
///
/// 步骤：
/// 1. [`super::audit::publish_audit`] —— 任一 Blocker 红 → **立即返回 [`PublishStage::BlockedAudit`]，零写入**
/// 2. [`super::build::generate_with`] —— 把带 scope 的产物写进 `delivery/mkp/presets/` + 重算目录
/// 3. [`super::delivery::publish_into`] —— 定稿 catalog / manifest / source
///    - **3½ 最终一致性核对**（[`super::audit::finalize_consistency`]）：定稿刚写下的
///      三本账与交付根真字节逐条对上、无残留。红 = 内部错误或并发改动 →
///      **Err 短路，绝不产生半截发布提交**
/// 4. （非 dry_run）本地 git：白名单 stage → commit → push
/// 5. （open_review 且给了平台）平台：create_review
///
/// `target` = 发布目标（来自发布账户配置）；`hosting` = 平台客户端（`None` = 不建 PR）。
/// **两者都是参数而不是内部构造** —— 好让判据注入假的，不真发网络请求。
/// `repo_root` = 前一个参数 `hosting` 时用的仓库根（测试可指定临时仓库；生产传 `None`
/// 走 [`super::git::Git::at_repo_root`]）。
///
/// ★ 步骤 1 的 `publish_audit` 是**纯读**（不写盘）—— 它红了这里一个字节都不落。
pub fn run(
    ctx: &Ctx,
    opts: &TxOptions,
    target: Option<&PublishTarget>,
    hosting: Option<&dyn Hosting>,
    repo_root: Option<PathBuf>,
) -> Result<PublishTxReport, AppError> {
    // ① 审计：唯一入口判定器（与工作台界面的闸、cargo test 判据同一个函数）。
    //    ★ 调**锁无关内核** `audit_with`，不是 `publish_audit` —— 事务跑在 `with_ctx` 里，
    //    那个入口会再取一次（不可重入的）锁 = 自锁挂死（2026-10-04 真机卡死的根因）。
    let audit = super::audit::audit_with(ctx)?;
    let passed = audit
        .items
        .iter()
        .filter(|i| i.status == super::audit::AuditStatus::Pass)
        .count();
    let failed = audit
        .items
        .iter()
        .filter(|i| i.status == super::audit::AuditStatus::Fail)
        .count();
    if !audit.can_publish {
        return Ok(PublishTxReport {
            stage: PublishStage::BlockedAudit,
            audit_passed: passed,
            audit_failed: failed,
            generated: 0,
            unchanged: 0,
            committed_paths: Vec::new(),
            review: None,
            branch: None,
            commit: None,
            files: 0,
            summary: "发布闸没全绿 —— 一个字节都没写。先照「去修」把红项处理掉".to_owned(),
            published_at: None,
            revision: None,
            changes: None,
            changed_files: Vec::new(),
        });
    }

    // ② 生成 + ③ 定稿。生成用 `Scope::Stale`（只补该补的）—— 与界面「生成」同一个口子。
    //    生成会重算一份 catalog；定稿再把 manifest / source 落上，两头对上。
    //
    // ★ 生成**之前**先留一份"目录里登记的指纹"（2026-10-06，回执的「本次变化」用它）：
    //    生成会把这份 catalog 覆盖掉，事后就问不出"发布前是什么"了。
    //    读的**是目录登记的值**（`sha256`），不是文件系统 mtime —— 回执里每个数都要能对上目录。
    let root = super::super::paths::delivery_root()?;
    let before = catalog_fingerprints(&root.join(super::delivery::NEW_CATALOG_FILE));
    let gen = super::build::generate_with(ctx, &super::build::Scope::Stale)?;

    // 定稿要一份 `&Book`（与生成算的是同一份内存状态）。
    let (c, d, _) = state(ctx)?;
    let book = super::super::domain::derive::Book::new(&ctx.presets, &c, &d);
    let asset_root = super::super::paths::assets_root()?;
    let meta = super::delivery::PublishMeta {
        stamp: crate::workbench::clock::now_iso8601(),
        channel: "stable".to_owned(),
        version: String::new(),
    };
    let out = super::delivery::publish_into(&root, &asset_root, &book, &meta)?;

    // 定稿之后的目录：取它的指纹（回执里显示「这一版是哪一版」）并与生成前逐份比。
    let catalog_path = root.join(super::delivery::NEW_CATALOG_FILE);
    let delivered = crate::runtime::catalog::Catalog::parse(&std::fs::read(&catalog_path)?)?;
    let after: std::collections::BTreeMap<String, String> = delivered
        .files
        .iter()
        .filter_map(|f| f.sha256.clone().map(|s| (f.path.clone(), s)))
        .collect();
    let (changes, changed_files) = diff_fingerprints(&before, &after);

    // ③½ **最终一致性核对**（发布事务的最后一道安全检查，不是第二套闸）：
    //    定稿刚按真实字节写下的 manifest / catalog 与交付根必须逐条对上、无残留。
    //    核对跑在 stage / commit 之前 —— 红了就 Err 短路，一个字节都不提交。
    //    （清单刻意不含 ⑮ git/clean：定稿后的工作区理应带着 presets/delivery 的改动，
    //    那正是这次要提交的东西。见 `audit::finalize_consistency` 的文档。）
    let final_items = super::audit::finalize_consistency(&book, &root);
    let broken: Vec<String> = final_items
        .iter()
        .filter(|i| {
            i.severity == super::audit::AuditSeverity::Blocker
                && i.status == super::audit::AuditStatus::Fail
        })
        .map(|i| format!("{}：{}", i.name, i.details))
        .collect();
    if !broken.is_empty() {
        return Err(AppError::internal(format!(
            "定稿后的最终一致性核对没过（{} 项红）—— 已停止，没有提交任何东西",
            broken.len()
        ))
        .with_detail(broken.join("；")));
    }

    // 状态条那一句里就把「改了什么」说清（"7 步全绿"答不了"我改了哪几份"）
    let summary = format!(
        "已生成 {} 份、定稿 {} 份产物。本次变化：新增 {} · 修改 {} · 删除 {} · 未变化 {}。",
        gen.written.len(),
        out.files,
        changes.added,
        changes.changed,
        changes.removed,
        changes.unchanged
    );
    let mut report = PublishTxReport {
        stage: PublishStage::Generated,
        audit_passed: passed,
        audit_failed: failed,
        generated: gen.written.len(),
        unchanged: gen.unchanged.len(),
        committed_paths: Vec::new(),
        review: None,
        branch: None,
        commit: None,
        files: out.files,
        summary,
        published_at: Some(meta.stamp.clone()),
        revision: Some(delivered.revision.clone()),
        changes: Some(changes),
        changed_files,
    };

    // dry_run：到这里就停（"只想看看会提交什么"）。
    if opts.dry_run {
        report.summary.push_str("（演练：未提交）");
        return Ok(report);
    }

    // ④ 本地 git：白名单 stage → commit → push。
    let git = match repo_root {
        Some(root) => super::git::Git::open(root),
        None => super::git::Git::at_repo_root(),
    };
    let branch = git.branch()?;
    report.branch = Some(branch.clone());

    // stage 的候选 = 交付产物根 + 台账（与生成 / 定稿写的同一批）。
    let candidates = vec![
        "presets/delivery/".to_owned(),
        "presets/structure-signatures.toml".to_owned(),
        "presets/assets.toml".to_owned(),
    ];
    let staged = git.stage_allowed(&candidates)?;
    report.committed_paths = staged.clone();

    if !git.has_staged()? {
        // 没有可提交的（产物逐字节没变）—— 不算失败，如实说。
        report.stage = PublishStage::Committed;
        report.summary.push_str("产物无变化，没有可提交的内容。");
        return Ok(report);
    }

    let message = commit_message(out.files, gen.written.len());
    git.commit(&message)?;
    // 回执要显示"提交了哪一笔"。取不到 sha 不影响发布本身 —— 那是展示，不是闸
    report.commit = git.head_short().ok();
    report.stage = PublishStage::Committed;

    /*
     * 推送：**从发布账户配置取凭据**（SupportEase 自持），Token 经 `http.extraHeader`
     * 临时喂给 git —— **不进 remote URL、不读用户已有凭据**（详见 `git::push_authenticated`）。
     *
     * ★ 推送前先**校验 remote 与配置一致**：当前工作目录必须就是配置的那个仓库。
     *   不一致 = 用户在错的目录里点了发布 —— 如实拒绝，不推错地方。
     */
    match target {
        Some(t) => {
            if !git.remote_matches(&t.repository_url)? {
                return Err(AppError::invalid_argument(
                    "当前仓库的远端地址和「发布账户」里配置的不一致，先确认一下发布目标",
                )
                .with_detail(format!("配置：{}", t.repository_url)));
            }
            git.push_authenticated(&branch, &t.username, &t.token)?;
        }
        None => git.push(&branch)?,
    }
    report.stage = PublishStage::Pushed;
    report
        .summary
        .push_str(&format!("已提交并推送到 `{branch}`。"));

    // ⑤ 平台：建 PR / MR（owner/repo 来自**配置**，不是 remote 推断）。
    if opts.open_review {
        let (Some(hosting), Some(t)) = (hosting, target) else {
            report
                .summary
                .push_str("未配置发布账户 —— 已推送，但没建 PR。");
            return Ok(report);
        };
        let spec = t.review_spec(
            branch.clone(),
            opts.base_or_main().to_owned(),
            out.files,
            gen.written.len(),
        );
        // ★ 平台不许同一个 head→base 开两份 PR：在合并之前再点一次「发布」，`create_review`
        //   会落 422（"这一支已经有一份"）。那不是失败 —— 产物已经推上去了，人只是又发了一次；
        //   **回读那一份开着的 PR** 继续（发布要可重跑，别把人堵在死路上）。
        //   找不到开着的才把原错误抛出来（那时它才是真错误）。
        let mut reused_why: Option<String> = None;
        let review = match hosting.create_review(&spec) {
            Ok(r) => r,
            Err(e) => {
                match hosting.find_open_review(&t.owner, &t.repo, &branch, opts.base_or_main())? {
                    Some(found) => {
                        reused_why = Some(e.message.clone());
                        found
                    }
                    None => return Err(e),
                }
            }
        };
        report.stage = PublishStage::ReviewOpened;
        match &reused_why {
            None => report
                .summary
                .push_str(&format!("已建 PR !{}。", review.number)),
            Some(why) => report.summary.push_str(&format!(
                "PR !{} 这一支上已经开着 —— 回读那一份，没有重复建（平台原话：{why}）。",
                review.number
            )),
        }
        report.review = Some(review);
    }

    Ok(report)
}

/// 提交信息：**系统生成**（版本 + 变更摘要），不让前端拼。
fn commit_message(files: usize, generated: usize) -> String {
    format!("发布：交付产物 {files} 份（本轮生成 {generated} 份）")
}

/* ---------- 发布账户（命令面） ---------- */

/// 一个平台在设置页里的**完整视图**：仓库地址 / 用户名（配置）+ 有没有 Token（秘密）。
///
/// ★ **没有 token 原值** —— 只有 `has_token` 与一个尾号提示（`tokenHint`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformAccountView {
    pub platform: String,
    /// 配置里的仓库地址（空 = 还没配）
    pub repository_url: String,
    /// 配置里的用户名（空 = 还没配）
    pub username: String,
    /// 有没有存过 Token（**只有真假，没有原值**）
    pub has_token: bool,
    /// Token 尾号提示（如 `…a1b2`）—— 确认"是不是这一把"，**不是原值**
    pub token_hint: Option<String>,
}

/// 给设置页看的**发布账户**总览（GitHub / Gitee 对称）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishAccount {
    /// 每个平台一格
    pub platforms: Vec<PlatformAccountView>,
    /// 当前工作目录的远端 URL（给用户对照"是不是这个仓库"）
    pub remote_url: Option<String>,
    /// 当前远端是否与**已配置的某个平台**一致（发布前的一致性提示）
    pub remote_matches_config: bool,
    /// 当前分支
    pub branch: Option<String>,
}

/// 组装一个平台的视图（配置 + 凭据状态）。
fn view_for(
    cfg: &super::account::PublishAccountConfig,
    creds: &super::credentials::Session,
    platform: &str,
) -> Result<PlatformAccountView, AppError> {
    let acct = cfg.get(platform);
    let status = creds.status(platform)?;
    Ok(PlatformAccountView {
        platform: platform.to_owned(),
        repository_url: acct.map(|a| a.repository_url.clone()).unwrap_or_default(),
        username: acct.map(|a| a.username.clone()).unwrap_or_default(),
        has_token: status.configured,
        token_hint: status.hint,
    })
}

/// 读发布账户现状（只读、`async`）。设置页开场调它。
#[tauri::command(async)]
pub fn wb_publish_account(app: tauri::AppHandle) -> Result<PublishAccount, AppError> {
    crate::ipc::traced("wb_publish_account", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let cfg = super::account::load(&root)?;
        let creds = super::credentials::session();
        let platforms = super::credentials::PLATFORMS
            .iter()
            .map(|p| view_for(&cfg, creds, p))
            .collect::<Result<Vec<_>, _>>()?;

        let git = super::git::Git::at_repo_root();
        let remote_url = git.remote_url().ok();
        let remote_matches_config = match remote_url.as_deref() {
            Some(remote) => platforms
                .iter()
                .filter(|p| !p.repository_url.is_empty())
                .any(|p| {
                    super::git::normalize_repo_url_for_compare(&p.repository_url)
                        == super::git::normalize_repo_url_for_compare(remote)
                }),
            None => false,
        };
        Ok(PublishAccount {
            platforms,
            remote_url,
            remote_matches_config,
            branch: git.branch().ok(),
        })
    })
}

/// 存一个平台的**发布目标**（仓库地址 + 用户名，进 `publish-account.json`）。
///
/// Token **不在这里** —— 它走 [`wb_set_publish_token`] 进凭据文件（配置与秘密分离）。
///
/// ★ `(async)`：它要读盘（发布账户配置）—— 碰盘的命令不占主线程。
#[tauri::command(async)]
pub fn wb_set_publish_account(
    app: tauri::AppHandle,
    platform: String,
    repository_url: String,
    username: String,
) -> Result<PlatformAccountView, AppError> {
    crate::ipc::traced("wb_set_publish_account", |_| {
        super::credentials::ensure_known(&platform)?;
        let account = super::account::PlatformAccount {
            repository_url: repository_url.trim().to_owned(),
            username: username.trim().to_owned(),
        };
        if !account.is_complete() {
            return Err(AppError::invalid_argument(
                "仓库地址与用户名都要填 —— 它们是发布目标的组成部分",
            ));
        }
        // 平台归属校验：GitHub 格里填 gitee 地址 = 明显填错
        super::account::check_platform_matches(&platform, &account)?;

        let root = crate::fsx::paths::internal_root(&app)?;
        let mut cfg = super::account::load(&root)?;
        cfg.set(&platform, Some(account));
        super::account::save(&root, &cfg)?;

        let creds = super::credentials::session();
        view_for(&cfg, creds, &platform)
    })
}

/// 存一个发布 Token（**只进不出**：写凭据文件，返回的状态里没有原值）。
///
/// ★ `(async)`：碰盘的命令不占主线程（与 IO 单子的规矩一致）。
#[tauri::command(async)]
pub fn wb_set_publish_token(
    platform: String,
    token: String,
) -> Result<PlatformAccountView, AppError> {
    crate::ipc::traced("wb_set_publish_token", |_| {
        super::credentials::ensure_known(&platform)?;
        let creds = super::credentials::session();
        creds.set_token(&platform, &token)?;
        let status = creds.status(&platform)?;
        Ok(PlatformAccountView {
            platform: platform.clone(),
            repository_url: String::new(),
            username: String::new(),
            has_token: status.configured,
            token_hint: status.hint,
        })
    })
}

/// 清一个平台的**发布账户**（配置 + 凭据文件里的 Token，都清；幂等）。
///
/// ★ `(async)`：碰盘的命令不占主线程。
#[tauri::command(async)]
pub fn wb_clear_publish_account(
    app: tauri::AppHandle,
    platform: String,
) -> Result<PlatformAccountView, AppError> {
    crate::ipc::traced("wb_clear_publish_account", |_| {
        super::credentials::ensure_known(&platform)?;
        let root = crate::fsx::paths::internal_root(&app)?;
        let mut cfg = super::account::load(&root)?;
        cfg.set(&platform, None);
        super::account::save(&root, &cfg)?;

        let creds = super::credentials::session();
        creds.clear(&platform)?;
        view_for(&cfg, creds, &platform)
    })
}

/// **手动回读**一份 PR/MR 的当前状态（快照 + 手动刷新；**不做后台轮询**）。
///
/// 作者定死：状态是"看一看"，不是常驻任务。前端给一颗「刷新」按钮调它。
/// 平台 / owner / repo 都来自**发布账户配置**（不是 remote 推断）。
#[tauri::command(async)]
pub fn wb_publish_status(app: tauri::AppHandle, number: u64) -> Result<RemoteReview, AppError> {
    crate::ipc::traced("wb_publish_status", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let target = resolve_target(&root, None)?;
        let id = super::platform::ReviewId {
            owner: target.owner,
            repo: target.repo,
            number,
        };
        match target.platform.as_str() {
            "github" => super::platform::github::GitHub::new(target.token).get_review(&id),
            "gitee" => super::platform::gitee::Gitee::new(target.token).get_review(&id),
            other => Err(AppError::invalid_argument(format!("不认识的平台：{other}"))),
        }
    })
}

/// 一次发布"改了什么"：按**目录里登记的文件**逐份比指纹（不是文件系统 mtime）。
///
/// ★ 口径（2026-10-06）：只有真正在交付目录里登记了 `sha256` 的那种文件才算一份
/// （`CatalogFile.sha256`）。登记不了期望值的（随包 bootstrap 那种）不参与计数 ——
/// 数出来的每一个数都要能被 `catalog.json` 自己解释。
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangeSummary {
    pub added: usize,
    pub changed: usize,
    pub removed: usize,
    pub unchanged: usize,
}

/// 单份文件的指纹变化。`before` 为空 = 新增；`after` 为空 = 删除。
///
/// 只给**指纹**（`sha256` 前 8 位由前端截），不给内容 diff —— 预设是数据文件，
/// 逐行 diff 要另做一套"TS 侧解析 TOML"的活，那是另一件事。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    /// 相对交付根的落点（`mkp/presets/A1-fast.toml`）
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

/// 读一份目录里登记的 `路径 → sha256`（读不出 / 解析不了 = 空，**不编**）。
fn catalog_fingerprints(path: &std::path::Path) -> std::collections::BTreeMap<String, String> {
    let Ok(bytes) = std::fs::read(path) else {
        return Default::default();
    };
    let Ok(catalog) = crate::runtime::catalog::Catalog::parse(&bytes) else {
        return Default::default();
    };
    catalog
        .files
        .iter()
        .filter_map(|f| f.sha256.clone().map(|s| (f.path.clone(), s)))
        .collect()
}

/// 两份指纹表比出「新增 / 修改 / 删除 / 未变化」与逐份明细。
fn diff_fingerprints(
    before: &std::collections::BTreeMap<String, String>,
    after: &std::collections::BTreeMap<String, String>,
) -> (ChangeSummary, Vec<FileChange>) {
    let mut summary = ChangeSummary::default();
    let mut files = Vec::new();
    for (path, new) in after {
        match before.get(path) {
            None => {
                summary.added += 1;
                files.push(FileChange {
                    path: path.clone(),
                    before: None,
                    after: Some(new.clone()),
                });
            }
            Some(old) if old == new => summary.unchanged += 1,
            Some(old) => {
                summary.changed += 1;
                files.push(FileChange {
                    path: path.clone(),
                    before: Some(old.clone()),
                    after: Some(new.clone()),
                });
            }
        }
    }
    for (path, old) in before {
        if !after.contains_key(path) {
            summary.removed += 1;
            files.push(FileChange {
                path: path.clone(),
                before: Some(old.clone()),
                after: None,
            });
        }
    }
    (summary, files)
}

/// 镜像同步的结论。**不是布尔** —— 「推上去了」和「那边本来就是这一版」要对用户分开说。
///
/// `Deserialize` 是给**发布历史**读的（`PublishRecord` 要反序列化回来）；
/// `PartialEq` 是给判据比的（同一份历史读回来该与原样相等）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum MirrorStatus {
    /// 推上去了（镜像的引用跟着动了）
    Pushed,
    /// 镜像上本来就是这一版（幂等空操作）
    UpToDate,
    /// 没做这一步，原因在 `detail` 里（没配那个源 / 这一笔是反向）
    Skipped,
    /// 试过、失败了。★ **不回滚"已合并"这个既成事实**，只如实报
    Failed,
}

/// 合并之后「把主线同步到第二个官方源」这一步的结果。
///
/// 作者 2026-10-06 定：一次发布**只开一条 PR**（代码主线仍只走 GitHub 的 PR），
/// PR 合并之后把主线数据同步一份到第二个官方源上 —— **幂等、不额外开 PR**。
///
/// ★ 为什么这是一件必须单独报出来的事：客户端的数据源读的就是**仓库里的
/// `presets/delivery/`**。所以"合进 GitHub 的 main"与"客户端读的那个源上的 main
/// 也有这一版"是**两件事** —— 2026-10-06 的那次事故正是前者成了、后者没成，
/// 而回执七步全绿（详见 `docs/PUBLISH-ARCHITECTURE.md` 与当日台账）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MirrorSync {
    /// 目标平台 id（`"gitee"`）；没做这一步时是**本该去**的那个
    pub platform: String,
    /// 目标仓库地址（没配时是空串）
    pub repository_url: String,
    /// 推的是哪条分支
    pub branch: String,
    pub status: MirrorStatus,
    /// 一句话：成功 = 推了什么；跳过 / 失败 = 为什么
    pub detail: String,
}

/// 把主线同步到"另一个官方源"。**只做 GitHub → Gitee 这一个方向**。
///
/// 三条纪律：
/// 1. **不给主流程判生死**：这一步失败不回滚"已合并"（那已经是事实），回执里如实报；
/// 2. **只推主线、不开 PR**：镜像源是**数据面**，不是协作面；
/// 3. **反向不做**：在 Gitee 上合完再裸推 GitHub 的 `main` 会被服务端 ruleset 拒
///    （代码主线在 GitHub 上只有 PR 一条路）—— 所以那种情形如实标「跳过」，
///    而不是去撞一次必然失败的推送。
fn sync_mirror_after_merge(
    root: &std::path::Path,
    merged_on: &PublishTarget,
    base: &str,
) -> MirrorSync {
    let platform = if merged_on.platform == "github" {
        "gitee"
    } else {
        "github"
    };
    let mut out = MirrorSync {
        platform: platform.to_owned(),
        repository_url: String::new(),
        branch: base.to_owned(),
        status: MirrorStatus::Skipped,
        detail: String::new(),
    };

    if merged_on.platform != "github" {
        out.detail = format!(
            "这一笔合在 {} 上 —— 代码主线在 GitHub 上只有 PR 一条路，这里不反向裸推。\
             要让另一个源也跟上，把发布目标切到 GitHub 走一次发布。",
            merged_on.platform
        );
        return out;
    }

    // 镜像目标从**同一份发布账户配置**里取（与发布目标是同一个真值来源）。
    // 没配就不是"失败"，是"没这一步" —— 如实标跳过。
    let mirror = match resolve_target(root, Some(platform)) {
        Ok(t) => t,
        Err(e) => {
            out.detail = format!("没有可用的 {platform} 发布账户，这一步跳过（{e}）");
            return out;
        }
    };
    out.repository_url = mirror.repository_url.clone();
    if super::git::normalize_repo_url_for_compare(&mirror.repository_url)
        == super::git::normalize_repo_url_for_compare(&merged_on.repository_url)
    {
        out.detail = "镜像目标与发布目标是同一个仓库，这一步没有意义，跳过".to_owned();
        return out;
    }

    let git = super::git::Git::at_repo_root();

    /* ★ 同步的**源**是 `origin/<base>`，不是本地那个 `<base>`（2026-10-06 实测踩到的）：
     *   合并发生在**平台上**，本地分支往往是旧的 —— 本机 `main` 停在 PR #46，而
     *   `origin/main` 才是刚合出来的那一笔。拿本地 main 当源会把镜像**推回旧版**。
     *   所以先 fetch 一次把它拉到最新；**拉不到就不推**（宁可不推，也不推一份旧的）。 */
    if let Err(e) = git.fetch("origin") {
        out.status = MirrorStatus::Failed;
        out.detail = format!("拉取远端的 {base} 失败，读不到合并结果，没敢同步：{e}");
        return out;
    }
    let source = format!("refs/remotes/origin/{base}");
    if git.rev_parse_short(&source).is_err() {
        out.status = MirrorStatus::Failed;
        out.detail = format!("本机没有 {source}（读不到合并结果），没敢同步");
        return out;
    }

    match git.push_ref_to_authenticated(
        &mirror.repository_url,
        &format!("{source}:refs/heads/{base}"),
        &mirror.username,
        &mirror.token,
    ) {
        Ok(p) if p.up_to_date => {
            out.status = MirrorStatus::UpToDate;
            out.detail = format!("{platform} 上已经是这一版（幂等空操作）");
        }
        Ok(p) => {
            out.status = MirrorStatus::Pushed;
            out.detail = match mirror_gap(&git, &source, &p.detail) {
                /* 首次补齐要给个数：落后几个提交就是几个（那是"补了多少"，不是"改了什么"） */
                Some(n) => format!("已把 {base} 推到 {platform}（补了 {n} 个提交）"),
                None if p.detail.is_empty() => format!("已把 {base} 推到 {platform}"),
                None => format!("已把 {base} 推到 {platform}：{}", p.detail),
            };
        }
        Err(e) => {
            out.status = MirrorStatus::Failed;
            out.detail = format!("{platform} 同步失败 —— 已合并这件事不受影响：{e}");
        }
    }
    out
}

/// 从 git 那句推送摘要里读"这一次把镜像补了多少个提交"（读不出 = `None`，**不编**）。
///
/// 摘要在快进时长这样：`abc1234..def5678  refs/remotes/origin/main -> main`
/// —— 有了旧 sha 就能本地数差多少。旧 sha 在本机不认识（浅克隆等）就如实不数，
/// 只报静默版的"已推过去"。
fn mirror_gap(git: &super::git::Git, source_ref: &str, detail: &str) -> Option<usize> {
    let old = detail.split_once("..")?.1.split_whitespace().next()?;
    let old = old.trim_matches(|c: char| !c.is_ascii_hexdigit());
    if old.len() < 7 {
        return None;
    }
    let (ahead, _) = git.ahead_behind(source_ref, old).ok()?;
    Some(ahead)
}

/// **合并**一份 PR/MR（squash）—— 用户在回执屏上**显式点过「合并」**才调。
///
/// 口径（作者 2026-10-04 拍）：
/// - **一律 squash**（不摆方式选择）；
/// - **不强制等 CI** —— "CI 没跑完 / 已经红了"的二次确认在界面做，这里不重复设闸；
/// - 合完**回读**一份真状态返回（`state` 应落到 `merged`；平台拒合时 4xx 原样报错）。
///
/// ★ `(async)`：碰网络（IO 单子）。
#[tauri::command(async)]
pub fn wb_merge_review(
    app: tauri::AppHandle,
    number: u64,
    platform: Option<String>,
) -> Result<MergeOutcome, AppError> {
    crate::ipc::traced("wb_merge_review", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let target = resolve_target(&root, platform.as_deref())?;
        let id = super::platform::ReviewId {
            owner: target.owner.clone(),
            repo: target.repo.clone(),
            number,
        };
        let method = super::platform::MergeMethod::Squash;
        let merged =
            match target.platform.as_str() {
                "github" => super::platform::github::GitHub::new(target.token.clone())
                    .merge_review(&id, method),
                "gitee" => super::platform::gitee::Gitee::new(target.token.clone())
                    .merge_review(&id, method),
                other => Err(AppError::invalid_argument(format!("不认识的平台：{other}"))),
            }?;

        /* ★ 合并成功之后**把主线同步到第二个官方源**（2026-10-06 加）。
         *
         * 为什么必须在这里：客户端的数据源读的是**仓库里的 `presets/delivery/`**，
         * 而"合进 GitHub 的 main"与"客户端读的那个源上的 main 也有这一版"是两件事。
         * 2026-10-06 那次事故正是前者成了、后者没成，而回执七步全绿 —— 用户看不出区别。
         *
         * ★ 只有**真合上了**才做：平台回读不是 `merged`（比如只关了 PR）时主线没动，
         *   推镜像就是无意义甚至有害的动作。
         */
        let mirror = (merged.state == ReviewState::Merged)
            .then(|| sync_mirror_after_merge(&root, &target, &merged.base));

        // 合并是**我们亲手造成的状态变化** —— 顺手把它记回发布历史（记账不必问网络）。
        // 记不上不影响结论：已经合了就是合了；下次「刷新」还能看到真状态。
        // 镜像那一条一起记进同一批记录（刷新不会把它抹掉，见 `history::update_review`）。
        let _ = super::history::update_review(&root, &merged, mirror.as_ref());
        Ok(MergeOutcome {
            review: merged,
            mirror,
        })
    })
}

/// **合并**这一步的完整结论：评审的真状态 + **镜像同步**（2026-10-06 加）。
///
/// ★ 从"只返回 [`RemoteReview`]"改成这个形状，是为了让回执能说清"合完了，
/// 但**客户端读的那个源**跟上了没有" —— 那件事与平台上的 PR 状态是两回事
/// （见 [`MirrorSync`]）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeOutcome {
    pub review: RemoteReview,
    /// 只有**确实合并成功**之后才有这一步的结论；没合上 / 没配第二个源时如实带原因
    pub mirror: Option<MirrorSync>,
}

/* ---------- 发布目标的解析（命令壳用；内核只收解析好的 PublishTarget） ---------- */

/// 从发布账户配置 + 凭据文件解析出**发布目标**。
///
/// `platform` 指定用哪个平台；`None` = 自动挑（配置里唯一配好的那个；两个都配好时优先
/// 取"当前 remote 一致"的那个，还不行就报错让用户显式说）。
///
/// ★ 这一步在**命令壳**里做（要 `AppHandle` 解析配置根）。内核
/// [`run`] 只收解析好的 [`PublishTarget`] —— **不碰 AppHandle，不破锁纪律**。
pub fn resolve_target(
    root: &std::path::Path,
    platform: Option<&str>,
) -> Result<PublishTarget, AppError> {
    let cfg = super::account::load(root)?;
    let creds = super::credentials::session();

    // 候选：配完整的账户（仓库地址 + 用户名都有）
    let mut candidates: Vec<(String, super::account::PlatformAccount)> = Vec::new();
    for p in super::credentials::PLATFORMS {
        if let Some(a) = cfg.get(p) {
            if a.is_complete() {
                candidates.push((p.to_owned(), a.clone()));
            }
        }
    }
    if candidates.is_empty() {
        return Err(AppError::not_found(
            "还没配置发布账户 —— 去设置里填「仓库地址 / 用户名 / Token」",
        ));
    }

    // 选定平台
    let chosen = match platform {
        Some(p) => candidates
            .into_iter()
            .find(|(c, _)| c == p)
            .ok_or_else(|| AppError::not_found(format!("{p} 的发布账户还没配完整")))?,
        None if candidates.len() == 1 => candidates.pop().expect("刚好一个"),
        None => {
            // 两个都配好：优先取"与当前 remote 一致"的那个
            let remote = super::git::Git::at_repo_root()
                .remote_url()
                .unwrap_or_default();
            let matched = candidates.iter().find(|(_, a)| {
                super::git::normalize_repo_url_for_compare(&a.repository_url)
                    == super::git::normalize_repo_url_for_compare(&remote)
            });
            match matched {
                Some((p, a)) => (p.clone(), a.clone()),
                None => return Err(AppError::invalid_argument(
                    "配了多个发布账户，当前仓库又不与其中任何一个一致 —— 请明确指定发到哪个平台",
                )),
            }
        }
    };
    let (platform, account) = chosen;

    let token = creds.token(&platform)?.ok_or_else(|| {
        AppError::not_found(format!("{platform} 的 Token 还没填 —— 去设置里补上"))
    })?;
    let (owner, repo) =
        super::platform::parse_owner_repo(&account.repository_url).ok_or_else(|| {
            AppError::invalid_argument(format!(
                "从配置的仓库地址里拆不出 owner/repo：{}",
                account.repository_url
            ))
        })?;

    Ok(PublishTarget {
        platform,
        repository_url: account.repository_url,
        username: account.username,
        token,
        owner,
        repo,
    })
}

/// 平台方言 → 统一评审状态。**纯函数**，判据钉它。
pub fn collapse_review_state(raw: &str) -> ReviewState {
    super::platform::collapse_state(raw)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 阶段枚举的线上形状（降成 camelCase 字符串）—— 前端按它画状态条，
    /// 发布历史按它落盘。**`wire_name()` 必须与 serde 逐字一致**（两处写法不许漂）。
    #[test]
    fn publish_stages_serialize_as_camel_case() {
        for stage in [
            PublishStage::BlockedAudit,
            PublishStage::Generated,
            PublishStage::Committed,
            PublishStage::Pushed,
            PublishStage::ReviewOpened,
            PublishStage::StatusRead,
        ] {
            let wire = serde_json::to_value(stage).unwrap();
            assert_eq!(
                wire.as_str().unwrap(),
                stage.wire_name(),
                "{stage:?} 的两种写法漂了"
            );
        }
        let s = serde_json::to_value(PublishStage::ReviewOpened).unwrap();
        assert_eq!(s, "reviewOpened");
        let s = serde_json::to_value(PublishStage::BlockedAudit).unwrap();
        assert_eq!(s, "blockedAudit");
    }

    /// 方言收敛：两个平台的原始状态都落到同一档里。**前端不认识任何一个平台。**
    #[test]
    fn remote_state_collapses_platform_dialects() {
        // open / opened / active 都归 Open
        for raw in ["open", "opened", "active"] {
            assert_eq!(collapse_review_state(raw), ReviewState::Open, "{raw}");
        }
        // merged 归 Merged
        assert_eq!(collapse_review_state("merged"), ReviewState::Merged);
        // closed / declined 归 Closed
        for raw in ["closed", "declined"] {
            assert_eq!(collapse_review_state(raw), ReviewState::Closed, "{raw}");
        }
        // 认不出 = Unknown（不猜）
        assert_eq!(collapse_review_state("weird"), ReviewState::Unknown);
    }

    /// ★★ **事务链内部绝不调命令壳**（`with_ctx` 锁不可重入 —— 回头调命令壳 = 自锁挂死）。
    ///
    /// 这条与第二刀那条发布闸的坑是同一条：闸跑在 `with_ctx` 里，复用能力只能调收 `&Ctx`
    /// 的自由函数。事务把 audit → generate → 定稿 → git 串成一条，任何一个环节的手滑
    /// （例如图省事调 `wb_generate()`）都会把工作台锁死，而且**表现是挂死不是报错** ——
    /// 那种 bug 在真机上极难定位，所以用源码扫描拦住。
    ///
    /// 判法：本文件（事务内核所在）里**不许出现**以下命令壳的调用：
    /// `wb_generate(` / `wb_publish(` / `wb_publish_audit(` / `wb_generate_preview(`。
    /// 允许的是它们对应的自由函数（`generate_with` / `publish_audit` / `publish_into`）。
    #[test]
    fn the_transaction_chain_never_calls_a_command_shell() {
        let src = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src")
                .join("workbench")
                .join("app")
                .join("publish_tx.rs"),
        )
        .expect("读得到 publish_tx.rs");

        // 只扫**生产代码**：`#[cfg(test)]` 之后是本测试自己（它的断言字符串里就写着这些名字）。
        let prod = src
            .split("#[cfg(test)]")
            .next()
            .expect("文件里一定有生产段");

        // 只看**调用**（`名字(`），跳过定义行（`pub fn 名字(`）与注释行。命令壳在别处定义，
        // 这里出现 `名字(` 基本就是调用。
        //
        // ★ 最后一味不是命令壳，是**会自己取锁的入口函数**：`audit::publish_audit` 是
        //   `with_ctx(audit_with)` 的薄壳 —— 事务里回头调它 = 再取一次不可重入的锁，
        //   同样是自锁挂死。**这一类名字（"取锁入口"）和命令壳同罪**，一起拦。
        let shells = [
            "wb_generate(",
            "wb_publish(",
            "wb_publish_audit(",
            "wb_generate_preview(",
            "publish_audit(",
        ];
        for shell in shells {
            let mut hits = Vec::new();
            for (i, line) in prod.lines().enumerate() {
                // 跳过本模块自己的命令定义（`pub fn wb_publish_account(` 这种以 wb_publish 开头）
                let trimmed = line.trim_start();
                if trimmed.starts_with("pub fn ") || trimmed.starts_with("fn ") {
                    continue;
                }
                // 注释行也跳过（文档里会提到这些名字）
                if trimmed.starts_with("//")
                    || trimmed.starts_with("///")
                    || trimmed.starts_with("*")
                {
                    continue;
                }
                if line.contains(shell) {
                    hits.push(format!("第 {} 行：{}", i + 1, trimmed));
                }
            }
            assert!(
                hits.is_empty(),
                "事务内核里调了命令壳 `{shell}` —— with_ctx 锁不可重入，这会自锁挂死：\n{}",
                hits.join("\n")
            );
        }
    }

    /// **镜像同步只走"合上 GitHub → 推 Gitee"一个方向**（2026-10-06）。
    ///
    /// 反向（在 Gitee 上合完再裸推 GitHub 的 `main`）**刻意不做**：代码主线在 GitHub 上
    /// 只有 PR 一条路（服务端 ruleset 也在兜底），裸推必然被拒 —— 与其去撞一次必然失败的
    /// 推送、把回执弄成红的，不如如实标"跳过 + 为什么"。这条判据钉住那个"不去撞"。
    #[test]
    fn the_mirror_step_never_pushes_back_to_github() {
        let on_gitee = PublishTarget {
            platform: "gitee".to_owned(),
            repository_url: "https://gitee.com/o/r".to_owned(),
            username: "u".to_owned(),
            token: "t".to_owned(),
            owner: "o".to_owned(),
            repo: "r".to_owned(),
        };
        // root 随便给：这条路径**在碰 root 之前就该返回**（否则下面这几句断言就没意义了）
        let out = sync_mirror_after_merge(std::path::Path::new("/nonexistent"), &on_gitee, "main");
        assert_eq!(out.platform, "github", "本该去的是另一个方向");
        assert_eq!(out.status, MirrorStatus::Skipped);
        assert!(
            out.detail.contains("只有 PR 一条路"),
            "跳过要说明为什么，不是静默：{}",
            out.detail
        );
        assert!(out.repository_url.is_empty(), "没做这一步就不该报目标地址");
    }

    /// **没配第二个源 = 跳过，不是失败**（2026-10-06）。
    ///
    /// 这条边界很重要：镜像那一步**不给主流程判生死**（合上了就是合上了），
    /// 所以"没配 Gitee"绝不能长成 `Failed` —— 那会让一次正常发布在回执里看着像出了事。
    #[test]
    fn a_missing_mirror_account_is_skipped_not_failed() {
        let d = tempfile::tempdir().unwrap();
        let on_github = PublishTarget {
            platform: "github".to_owned(),
            repository_url: "https://github.com/o/r".to_owned(),
            username: "u".to_owned(),
            token: "t".to_owned(),
            owner: "o".to_owned(),
            repo: "r".to_owned(),
        };
        let out = sync_mirror_after_merge(d.path(), &on_github, "main");
        assert_eq!(out.platform, "gitee");
        assert_eq!(out.status, MirrorStatus::Skipped, "没配 ≠ 失败");
        assert!(
            out.detail.contains("跳过"),
            "要说清是跳过、为什么：{}",
            out.detail
        );
    }

    /// **"本次变化"按目录登记的指纹算**（2026-10-06）—— 新增 / 修改 / 删除 / 未变化四档，
    /// 明细里只列动过的那几份（未变化的不列，免得清单变成一整页）。
    #[test]
    fn the_change_summary_counts_fingerprints_not_mtime() {
        let map = |pairs: &[(&str, &str)]| -> std::collections::BTreeMap<String, String> {
            pairs
                .iter()
                .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                .collect()
        };
        let before = map(&[
            ("a.toml", "1111"),
            ("b.toml", "2222"),
            ("gone.toml", "3333"),
        ]);
        let after = map(&[("a.toml", "1111"), ("b.toml", "2222"), ("new.toml", "4444")]);

        let (sum, files) = diff_fingerprints(&before, &after);
        assert_eq!(
            (sum.added, sum.changed, sum.removed, sum.unchanged),
            (1, 0, 1, 2),
            "新增 1、删除 1、其余未变"
        );
        assert_eq!(files.len(), 2, "明细只列动过的那两份");
        let new = files.iter().find(|f| f.path == "new.toml").unwrap();
        assert!(new.before.is_none(), "新增那份的 before 该是空的");
        assert_eq!(new.after.as_deref(), Some("4444"));
        let gone = files.iter().find(|f| f.path == "gone.toml").unwrap();
        assert_eq!(gone.before.as_deref(), Some("3333"));
        assert!(gone.after.is_none(), "删掉那份的 after 该是空的");

        // 指纹变了就是"修改"（不按 mtime、不按大小）
        let touched = map(&[("a.toml", "9999")]);
        let (sum2, _) = diff_fingerprints(&before, &touched);
        assert_eq!((sum2.changed, sum2.unchanged), (1, 0));
    }
}
