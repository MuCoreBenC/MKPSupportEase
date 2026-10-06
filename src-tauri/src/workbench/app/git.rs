//! **本地 git**（发布事务的「本地那一半」）—— 子进程封装。
//!
//! # 为什么是子进程 `git`，而不是「远程平台的凭据系统」
//!
//! Git 是**本地版本控制工具**：它不需要任何"平台账号"，也不需要网络。
//! 把本地提交/推送交给系统上的 `git`，是桌面产品的正常做法（仓库本来就是这么建的）。
//!
//! ★ 它与**远程平台认证**（GitHub / Gitee 的 Token）是**两件不同的事**：
//! 平台认证与 PR/MR 全部由我们自己的 HTTP 模块处理（见 [`super::platform`]），
//! **不借用户的 `gh` / `git` 登录态**。这条边界是产品决定，不是实现细节。
//!
//! # 两条硬规矩
//!
//! 1. **参数一律显式数组**，绝不拼 shell 字符串（`Command::new("git").args([...])`）——
//!    拼字符串会带进注入面，而这里的参数里有文件名。
//! 2. ★ **只 stage 白名单路径**（[`STAGE_ALLOWLIST`]）。永不 `git add -A` ——
//!    "这次发布"要提交的是**交付产物那一批**，把工作区里别的改动一起卷进去是事故。
//!    这条口径与发布闸 ⑮ `git/clean`（排除 `presets/delivery/` 之外不许有脏）互为反面：
//!    ⑮ 保证"除产物外没有别的改动"，本模块保证"只提交那一批"。
//!
//! # 取不到 git 怎么办
//!
//! 与 ⑮ 同一取向：**不假装成功**。每次调用返回 `Result`，取不到就如实报
//! （`AppError`，运输类）。发布闸可以在没有 git 的机器上"跳过"⑮，但**发布事务**
//! 要真推东西，拿不到 git 就是硬失败 —— 那正是它该有的样子。

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::error::AppError;

/// 允许被 stage 的路径白名单（**相对仓库根**，前缀匹配）。
///
/// ★ 这就是"这次发布提交什么"的**唯一答案**。想提交新东西？加到这份清单里来，
/// 别在这里写 `-A`。交付产物 `presets/delivery/` 是主体；结构规则表与资产台账跟着
/// 结构 / 资产变化走，一起提交。
pub const STAGE_ALLOWLIST: [&str; 3] = [
    "presets/delivery/",
    "presets/structure-signatures.toml",
    "presets/assets.toml",
];

/// **「发布软件版本」那条链**的 stage 白名单（第四刀）。
///
/// ★ 两份清单**刻意分开**（两条链不混）：这里是"版本号那一笔 + `release.json`"，
/// 上面那份是预设交付产物。**发布预设绝不会提交 `presets/delivery/release.json`**，
/// 反过来也不会 —— 各自只认识自己那一批，混入另一条链的产物进不去索引。
///
/// 清单里每一条都是**派生或发布物**：`src-tauri/Cargo.toml` 是版本真值那一格，
/// 其余三个是它的派生结果（见 [`super::version`]），`presets/delivery/release.json` 是
/// 上传成功后才写的发布信息（2026-10-05 起住交付根，总纲 §1②）。
/// 源码（`src/**` / `src-tauri/src/**`）一个都不在里头。
pub const RELEASE_STAGE_ALLOWLIST: [&str; 5] = [
    "src-tauri/Cargo.toml",
    "src-tauri/tauri.conf.json",
    "package.json",
    "Cargo.lock",
    "presets/delivery/release.json",
];

/// 一次 ref 推送的结果（**从 git 自己的推送摘要里读出来**，不另加网络往返）。
///
/// ★ 为什么要把这件事读回来（2026-10-06，加"合并后同步镜像源"那一步时）：
/// 把主线同步到第二个官方源是**幂等**动作，而「推上去了」与「那边本来就是这一版」
/// 是**两件不同的事实** —— 回执里说"已经推过去"和说"那边早就有了"对用户的意义不一样
/// （前者意味着镜像刚跟上，后者意味着这一步是空操作）。git 在推送成功时已经把这件事
/// 打在 stderr 上了（`= [up to date]` 对 `abc1234..def5678`），直接读它比再问一次远端
/// （`ls-remote`）便宜，也少一条会挂住的网络路径。
///
/// ★ 它**不改变**失败语义：失败仍然是 `Err(AppError)`（那才是"发布事务不能糊过去"的那一半）。
#[derive(Debug, Clone)]
pub struct PushOutcome {
    /// 远端本来就是这一版（空操作，引用没动）
    pub up_to_date: bool,
    /// 人看的推送摘要（如 `abc1234..def5678  main -> main`、`* [new branch]`）；
    /// **已 redact token、已剔掉带凭据的 URL**；读不出时是空串（调用方自己措辞）
    pub detail: String,
}

/// 一条 git 命令的结果：成功 = stdout（已 trim 掉尾换行），失败 = 带 stderr 的 `AppError`。
///
/// **与 ⑮ 里那个只返回 `Option<String>` 的 `git()` 不同**：那个是"探测"（没有 git 就当
/// 没有），这里是"执行"（没成功就是错）。发布事务不能把"没提交上"当"没有 git"糊过去。
pub struct Git {
    repo: PathBuf,
}

impl Git {
    /// 打开仓库根这一处的 git。**不检查它是不是仓库** —— 检查落在第一次命令上
    /// （`rev-parse`），那时才拿得到真实原因。
    pub fn open(repo: impl Into<PathBuf>) -> Self {
        Self { repo: repo.into() }
    }

    /// 真仓库。
    pub fn at_repo_root() -> Self {
        Self::open(super::super::paths::repo_root())
    }

    /// 仓库根
    pub fn repo(&self) -> &Path {
        &self.repo
    }

    /// 当前分支名（`git symbolic-ref --short HEAD`）。游离 HEAD 上会失败 —— 如实报。
    pub fn branch(&self) -> Result<String, AppError> {
        Ok(self
            .run(&["symbolic-ref", "--short", "HEAD"])?
            .trim()
            .to_owned())
    }

    /// 工作区状态（`git status --porcelain=v1 -z --untracked-files=all`），原样返回。
    ///
    /// 用 `-z`（NUL 分隔）而不是默认换行：文件名里可以有换行 / 引号，`-z` 是不需要转义
    /// 解析的那一种（与闸 ⑮ 同一取向）。
    pub fn status_raw(&self) -> Result<String, AppError> {
        self.run(&["status", "--porcelain=v1", "-z", "--untracked-files=all"])
    }

    /// 差异统计（`git diff --stat`），给人看的一段文本。
    pub fn diff_stat(&self) -> Result<String, AppError> {
        self.run(&["diff", "--stat"])
    }

    /// 把白名单里**确实有变化**的路径加进索引。
    ///
    /// ★ 传进来的是候选路径清单，这里**再按 [`STAGE_ALLOWLIST`] 过滤一遍** ——
    /// 第二道闸。就算调用方一时手滑传了 `presets/assets/foo.png`，也进不去索引。
    /// 返回真正 stage 了的路径（相对仓库根），空 = 没有可提交的。
    pub fn stage_allowed(&self, candidates: &[String]) -> Result<Vec<String>, AppError> {
        self.stage_allowed_in(candidates, &STAGE_ALLOWLIST)
    }

    /// 按**指定的**白名单 stage（「发布软件版本」用 [`RELEASE_STAGE_ALLOWLIST`]）。
    ///
    /// 清单作参数而不是再写一个函数：过滤规则只有[`is_allowed_in`]一处实现，
    /// 两条链共用同一道闸，只是喂进去的清单不同。
    pub fn stage_allowed_in(
        &self,
        candidates: &[String],
        allowlist: &[&str],
    ) -> Result<Vec<String>, AppError> {
        let allowed: Vec<&String> = candidates
            .iter()
            .filter(|p| is_allowed_in(p, allowlist))
            .collect();
        if allowed.is_empty() {
            return Ok(Vec::new());
        }
        let mut args: Vec<&str> = vec!["add", "--"];
        args.extend(allowed.iter().map(|s| s.as_str()));
        self.run(&args)?;
        Ok(allowed.into_iter().cloned().collect())
    }

    /// 提交已 stage 的内容。**只提交索引里那批**（`git commit` 不带 `-a`）——
    /// 绝不把未 stage 的工作区改动卷进来。
    pub fn commit(&self, message: &str) -> Result<(), AppError> {
        self.run(&["commit", "-m", message])?;
        Ok(())
    }

    /// 当前 HEAD 的短 sha（`git rev-parse --short HEAD`）—— 发布回执里显示"提交了哪一笔"。
    pub fn head_short(&self) -> Result<String, AppError> {
        Ok(self
            .run(&["rev-parse", "--short", "HEAD"])?
            .trim()
            .to_owned())
    }

    /// 推分支（`git push -u origin <branch>`）。`branch` 为空 = 用当前分支。
    ///
    /// **不带认证** —— 只在凭据已由别处（如 CI 的 credential helper）提供时用。
    /// 发布事务走 [`push_authenticated`]（自持 Token）。
    pub fn push(&self, branch: &str) -> Result<(), AppError> {
        let target = self.resolve_branch(branch)?;
        self.run(&["push", "-u", "origin", &target])?;
        Ok(())
    }

    /// **带认证**推分支（发布事务用）：Token 经 `http.extraHeader` 临时喂给 git。
    ///
    /// 命令行形状（作者 2026-10-04 定死）：
    ///
    /// ```text
    /// git -c credential.helper= \
    ///     -c http.extraHeader="Authorization: Basic <base64(username:token)>" \
    ///     push -u origin <branch>
    /// ```
    ///
    /// ★ **Token 不进 remote URL**：URL 里没有凭据，`git remote -v`、`.git/config`、
    ///   push 回显里都不会出现 Token。
    /// ★ **`credential.helper=`（空值）清掉全局 helper**：Git 因此**不会**去读用户系统里
    ///   已存的 Git 凭据 —— 正是作者要避免的"不读用户已有凭据"。
    /// ★ 子进程环境加 `GIT_TERMINAL_PROMPT=0`：不许 git 弹交互提问（会把后台命令挂死）。
    /// ★ 失败时**不把带 header 的参数写进错误 detail**（否则 Token 进日志）。
    pub fn push_authenticated(
        &self,
        branch: &str,
        username: &str,
        token: &str,
    ) -> Result<PushOutcome, AppError> {
        let target = self.resolve_branch(branch)?;
        self.push_ref_authenticated(&target, username, token)
    }

    /// **带认证**推**任意 ref 到 `origin`**（第四刀：推 tag 用）。
    ///
    /// 与 [`Self::push_authenticated`] 同一条命令行形状、同一套凭据纪律
    /// （Token 不进 URL / 清掉全局 helper / `GIT_TERMINAL_PROMPT=0`）——
    /// 差别只是不再把参数解释成"分支"，`refs/tags/v0.0.2` 这类也推得动。
    pub fn push_ref_authenticated(
        &self,
        refspec: &str,
        username: &str,
        token: &str,
    ) -> Result<PushOutcome, AppError> {
        self.push_ref_to_authenticated("origin", refspec, username, token)
    }

    /// **带认证**推**任意 ref 到指定远端**（远端给 **URL**，不要求本机配过这个 remote）。
    ///
    /// ★ 为什么需要"按 URL 推"（2026-10-05，作者定"后续走 Gitee"）：软件版本要发到**几个**
    /// 仓库去 —— GitHub 放 Release 与 tag，**Gitee 放数据源**（国内直连的那一份）。
    /// 本机不一定配了 `gitee` 这个 remote（也不该要求开发者配），而发布目标已经写在
    /// 发布账户配置里了（`repository_url`）—— **按那个 URL 推**，配置是唯一真值。
    /// ★ 两家的 git-http **吃不同的认证形状**（2026-10-06 真机实测，同一份 token）：
    ///   GitHub 认**先发式 Basic 头**（`http.extraHeader`）—— "Token 不进 URL"的纪律在它那边成立；
    ///   **Gitee 无视那个头**（401 落到凭据提示，`GIT_TERMINAL_PROMPT=0` 下直接死），
    ///   只认 **URL 内嵌凭据** —— 所以 Gitee 走 URL（命令行参数里短暂可见，本机单人场景），
    ///   错误信息里 **redact**（见下）。
    pub fn push_ref_to_authenticated(
        &self,
        remote: &str,
        refspec: &str,
        username: &str,
        token: &str,
    ) -> Result<PushOutcome, AppError> {
        // Gitee → URL 内嵌凭据；其余（GitHub / 自建）→ 先发式 Basic 头
        let with_url_creds = gitee_url_with(remote, username, token);
        let target = with_url_creds.clone().unwrap_or_else(|| remote.to_owned());
        let mut args: Vec<&str> = vec!["-c", "credential.helper="];
        let header;
        if with_url_creds.is_none() {
            let basic = base64_encode(format!("{username}:{token}").as_bytes());
            header = format!("http.extraHeader=Authorization: Basic {basic}");
            // `-c credential.helper=` 清掉全局 helper；`-c http.extraHeader=` 临时带认证。
            // 两个 `-c` 必须在子命令（push）**之前**。
            args.extend(["-c", &header]);
        }
        args.extend(["push", "-u", &target, refspec]);
        // ★ 推**发布仓库（Gitee 镜像）的 main**（⑥½"数据源主线跟随发布仓库"）是发布事务
        //   **设计内的裸推** —— 本地推送闸②（pre-push hook，PR-only）分不清远端，见到
        //   main 就拦。对发往 Gitee 发布仓库的主线推，由这里显式放行（hook 会留痕）；
        //   GitHub 的 main 不受影响：推进仍只有 PR 一条路（服务端 ruleset 也在兜底）。
        let allow_main = with_url_creds.is_some() && refspec_targets_main(refspec);
        let mut cmd = Command::new("git");
        cmd.args(&args)
            .current_dir(&self.repo)
            // 禁交互：没有 TTY 时 git 会尝试提示，这里直接关掉（防挂死）
            .env("GIT_TERMINAL_PROMPT", "0");
        if allow_main {
            cmd.env("ALLOW_PUSH_MAIN", "1");
        }
        let out = cmd.output().map_err(|e| spawn_failed(&self.repo, e))?;
        if !out.status.success() {
            // ★ detail 只用 stderr，且 **redact 掉 token** —— URL 内嵌凭据会随 git 报错
            //   原文出现（如 `unable to access 'https://user:token@…'`），不能原样带出去
            let err = String::from_utf8_lossy(&out.stderr).replace(token, "***");
            return Err(AppError::io("git push 失败了").with_detail(format!(
                "{}（退出码 {:?}）",
                err.trim(),
                out.status.code()
            )));
        }
        // 成功：把 git 自己那句推送摘要读回来（**不加网络往返**——它已经在 stderr 上了）。
        // 只取含 ` -> ` 的那一行，带凭据的 `To https://user:***@…` 那行不带出去。
        let stderr = String::from_utf8_lossy(&out.stderr).replace(token, "***");
        let update = stderr
            .lines()
            .map(str::trim)
            .find(|l| l.contains(" -> "))
            .unwrap_or("")
            .to_owned();
        Ok(PushOutcome {
            up_to_date: update.contains("[up to date]"),
            detail: update,
        })
    }

    /// 当前工作目录的 remote（`origin`）是不是**配置的那个仓库**（发布前的一致性校验）。
    ///
    /// 规范化后比较：https / ssh 两种形状等价、`…/o/r.git` 与 `…/o/r` 等价、
    /// host 大小写不敏感。认不出形状（自建源等）→ `false`（不硬猜）。
    ///
    /// ★ 作者定死：**remote 只用来校验"是不是发到配置的那个仓库"**，不是用来决定发布目标。
    /// 不一致 = 当前目录不是配置的那个仓库 —— 如实拒绝，不发。
    pub fn remote_matches(&self, repository_url: &str) -> Result<bool, AppError> {
        let actual = self.remote_url()?;
        Ok(normalize_repo_url(&actual) == normalize_repo_url(repository_url))
    }

    /* ---------- 「发布软件版本」那一半（第四刀） ----------
     *
     * 发布预设那条链只 commit / push 当前分支；发软件版本还要**切分支、打 tag、推 tag**，
     * 以及"主线到底齐不齐"的判定。全部走同一个 `run`（显式参数数组），不另开一条路。
     */

    /// 拉远端的最新引用（`git fetch <remote> --tags --prune`）。
    ///
    /// ★ **判定"主线齐不齐"之前必须先 fetch**：不 fetch 的 `origin/main` 是上次拉到的样子，
    /// 拿它当"远端现状"会得出一个过期的、偏乐观的结论。
    pub fn fetch(&self, remote: &str) -> Result<(), AppError> {
        self.run(&["fetch", remote, "--tags", "--prune"])?;
        Ok(())
    }

    /// 两个引用谁领先多少：`(本地独有的提交数, 远端独有的提交数)`。
    ///
    /// `git rev-list --left-right --count <a>...<b>` —— 输出形如 `2\t0`。
    /// 「确认主线」用它回答"要发的代码都在 main 上了吗"。
    pub fn ahead_behind(&self, left: &str, right: &str) -> Result<(usize, usize), AppError> {
        let out = self.run(&[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{left}...{right}"),
        ])?;
        let mut parts = out.split_whitespace();
        let a = parts.next().and_then(|s| s.parse().ok());
        let b = parts.next().and_then(|s| s.parse().ok());
        match (a, b) {
            (Some(a), Some(b)) => Ok((a, b)),
            _ => Err(AppError::io("读不出 ahead/behind 计数").with_detail(out)),
        }
    }

    /// 切到已有分支（`git switch <branch>`）。
    ///
    /// ★ 它会**改变开发者工作区的当前分支** —— 调用方（事务内核 / 界面）必须
    /// **事先把这件事说清楚**，不许悄悄切（作者 2026-10-04 的"破坏性命令要讲明白"）。
    pub fn switch(&self, branch: &str) -> Result<(), AppError> {
        self.run(&["switch", branch])?;
        Ok(())
    }

    /// 新建并切到一条分支（`git switch -c <branch>`）。
    pub fn switch_new(&self, branch: &str) -> Result<(), AppError> {
        self.run(&["switch", "-c", branch])?;
        Ok(())
    }

    /// 快进拉取（`git pull --ff-only <remote> <branch>`）。
    ///
    /// ★ **只许 ff-only**：非快进的 pull 会在本地造一个合并提交，而"tag 打在 main 的 tip 上"
    /// 要求 main 的 tip 就是远端那一笔 —— 有合并提交的话，打出来的 tag 指的就不是远端那一笔。
    pub fn pull_ff(&self, remote: &str, branch: &str) -> Result<(), AppError> {
        self.run(&["pull", "--ff-only", remote, branch])?;
        Ok(())
    }

    /// 打一个**带注释**的 tag（`git tag -a <name> -m <message>`）。
    ///
    /// ★ 纪律（作者定死，**与 `scripts/release.mjs` 同一条**）：tag 必须打在 **main 的 tip 上**。
    /// 分支上打的 tag 指向一个不在 main 历史里的提交（squash 会重写提交）——
    /// 那是"看起来发了，其实 main 上没有"的那一类错。
    pub fn tag(&self, name: &str, message: &str) -> Result<(), AppError> {
        self.run(&["tag", "-a", name, "-m", message])?;
        Ok(())
    }

    /// 这个 tag 存不存在（`git rev-parse -q --verify refs/tags/<name>`）。
    pub fn tag_exists(&self, name: &str) -> Result<bool, AppError> {
        Ok(self
            .try_run(&["rev-parse", "-q", "--verify", &format!("refs/tags/{name}")])?
            .is_some())
    }

    /// 推一个 tag（`git push origin refs/tags/<name>`，**不带认证**）。
    ///
    /// 只在凭据已由别处提供时用；发布事务走 [`Self::push_ref_authenticated`]。
    pub fn push_tag(&self, name: &str) -> Result<(), AppError> {
        self.run(&["push", "origin", &format!("refs/tags/{name}")])?;
        Ok(())
    }

    /// 解析一个引用的短 sha（`git rev-parse --short <what>`）。
    ///
    /// 「这个 tag 指的到底是哪一笔」用它回答 —— 版本号撞车时，只有它分得清
    /// "这一版发出去过"与"上一次事务的续跑"。
    pub fn rev_parse_short(&self, what: &str) -> Result<String, AppError> {
        Ok(self.run(&["rev-parse", "--short", what])?.trim().to_owned())
    }

    /// 工作区干不干净（`git status --porcelain` 为空）。
    ///
    /// 发布软件版本要提交**版本号那一批**，工作区里混着别的东西就说不清了 ——
    /// 与发布闸 ⑮ `git/clean` 同一条道理，只是这里的清单换成[`RELEASE_STAGE_ALLOWLIST`]。
    pub fn is_clean(&self) -> Result<bool, AppError> {
        Ok(self.run(&["status", "--porcelain"])?.trim().is_empty())
    }

    fn resolve_branch(&self, branch: &str) -> Result<String, AppError> {
        if branch.trim().is_empty() {
            self.branch()
        } else {
            Ok(branch.trim().to_owned())
        }
    }

    /// 索引里有没有已 stage 的改动（`git diff --cached --quiet` 退出码反过来）。
    ///
    /// 用它决定"要不要 commit"：没有任何 staged 改动时提交会以 `nothing to commit` 失败，
    /// 那不是错误，是"这次没东西可提"。
    pub fn has_staged(&self) -> Result<bool, AppError> {
        let out = Command::new("git")
            .args(["diff", "--cached", "--quiet"])
            .current_dir(&self.repo)
            .output()
            .map_err(|e| spawn_failed(&self.repo, e))?;
        // 退出码 0 = 没有差异；1 = 有差异；其余 = 出错
        match out.status.code() {
            Some(0) => Ok(false),
            Some(1) => Ok(true),
            _ => Err(AppError::io("git diff --cached 没跑通")
                .with_detail(String::from_utf8_lossy(&out.stderr).trim().to_owned())),
        }
    }

    /// 远端 URL（`origin`）。**平台推断**（[`super::platform::detect_platform`]）用它。
    pub fn remote_url(&self) -> Result<String, AppError> {
        Ok(self
            .run(&["remote", "get-url", "origin"])?
            .trim()
            .to_owned())
    }

    /// 探测：跑一条 git 命令，**跑不通 = `None`**（不报错）。
    ///
    /// 给"存不存在"这类问题用（tag / 引用在不在）—— 那不是失败，是一个答案。
    fn try_run(&self, args: &[&str]) -> Result<Option<String>, AppError> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.repo)
            .output()
            .map_err(|e| spawn_failed(&self.repo, e))?;
        if !out.status.success() {
            return Ok(None);
        }
        Ok(Some(String::from_utf8_lossy(&out.stdout).into_owned()))
    }

    /// 跑一条 git 命令。参数显式数组；工作目录固定仓库根。
    fn run(&self, args: &[&str]) -> Result<String, AppError> {
        let out = Command::new("git")
            .args(args)
            .current_dir(&self.repo)
            .output()
            .map_err(|e| spawn_failed(&self.repo, e))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::io(format!(
                "git {} 失败了",
                args.first().copied().unwrap_or("")
            ))
            .with_detail(format!("{}（退出码 {:?}）", err.trim(), out.status.code())));
        }
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }
}

/// 这条 refspec 的**目的地**是不是 `refs/heads/main` —— 放行裸推 main 的判据。
///
/// ★ 看**目的地**那一侧，不看源（2026-10-06 放宽）：同步镜像时推的是
/// `refs/remotes/origin/main:refs/heads/main`，源是**远端跟踪引用** —— 把源写死成
/// `refs/heads/main` 会让这种形状的推送被本地钩子拦下（或漏掉放行）。
///
/// ★ 为什么源必须是 `origin/<base>` 而不是本地 `<base>`：合并发生在**平台上**，
/// 本地那个 main 往往是旧的（2026-10-06 实测：本机 main 停在 PR #46，比 `origin/main`
/// 落后三个 PR）—— 拿它当源会把镜像**推回旧版**。
fn refspec_targets_main(refspec: &str) -> bool {
    refspec.rsplit_once(':').map_or(refspec, |(_, dst)| dst) == "refs/heads/main"
}

/// 路径在不在 stage 白名单里。**前缀匹配**（`presets/delivery/` 收下它下头的一切）。
pub fn is_allowed(path: &str) -> bool {
    is_allowed_in(path, &STAGE_ALLOWLIST)
}

/// 同一道闸，喂进去的清单由调用方定（两条链各一份，见 [`RELEASE_STAGE_ALLOWLIST`]）。
pub fn is_allowed_in(path: &str, allowlist: &[&str]) -> bool {
    let p = path.trim().trim_matches('"');
    allowlist
        .iter()
        .any(|a| p == a.trim_end_matches('/') || p.starts_with(a))
}

fn spawn_failed(repo: &Path, e: std::io::Error) -> AppError {
    AppError::io("起不了 git —— 它可能不在 PATH 上").with_detail(format!("{}：{e}", repo.display()))
}

/// 把仓库地址规范化成可比形状：`host/path`（小写、去协议、去 `.git`、去尾部斜杠）。
///
/// 让 `https://GitHub.com/o/r.git`、`git@github.com:o/r.git`、`https://github.com/o/r`
/// 三者等价 —— "当前 remote 是不是配置的那个仓库"才算得准。
/// 认不出形状 → 返回原串（小写、去尾斜杠）—— 那会让它跟配置对不上，**保守判"不匹配"**。
///
/// `pub`：发布账户命令（`publish_tx`）做"配置的仓库 vs 当前 remote"比对时也用它 ——
/// **规范化只有这一处实现**，两处调用方不许各写一遍。
pub fn normalize_repo_url_for_compare(url: &str) -> String {
    normalize_repo_url(url)
}

fn normalize_repo_url(url: &str) -> String {
    let u = url.trim().trim_end_matches('/');
    let u = u.strip_suffix(".git").unwrap_or(u);
    // ssh 形状 git@host:path → host/path
    if let Some(rest) = u.strip_prefix("git@") {
        return rest.replacen(':', "/", 1).to_ascii_lowercase();
    }
    // https://host/path 或 http://host/path → host/path
    for scheme in ["https://", "http://", "ssh://"] {
        if let Some(rest) = u.strip_prefix(scheme) {
            // ssh://git@host/path 还要再去一层 user@
            let rest = rest.rsplit('@').next().unwrap_or(rest);
            return rest.to_ascii_lowercase();
        }
    }
    u.to_ascii_lowercase()
}

/// 最小 base64 编码（标准字母表 + `=` 补齐）。
///
/// **手写而不引 crate**：只用在这一处（构造 Basic 认证头），
/// 引入一个 `base64` 依赖不值当（作者定的"能不加依赖就不加"）。
fn base64_encode(input: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18 & 0x3f) as usize] as char);
        out.push(TABLE[(n >> 12 & 0x3f) as usize] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6 & 0x3f) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[(n & 0x3f) as usize] as char
        } else {
            '='
        });
    }
    out
}

/// Gitee 的 **URL 内嵌凭据**形状：只对 gitee.com 的 http(s) URL 生效
/// （remote 名（`origin`）/ 别的域 → `None`，调用方回落到 Basic 头）。
///
/// 为什么存在（2026-10-06 真机实测）：Gitee 的 git-http **无视先发式
/// `Authorization: Basic` 头**，同一份 token 以 `https://user:token@…` 内嵌则通。
fn gitee_url_with(remote: &str, username: &str, token: &str) -> Option<String> {
    let rest = remote
        .strip_prefix("https://gitee.com/")
        .or_else(|| remote.strip_prefix("http://gitee.com/"))?;
    Some(format!("https://{username}:{token}@gitee.com/{rest}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **裸推 main 的放行判据看目的地、不看源**（2026-10-06）。
    ///
    /// 这条不是纸面功夫：同步镜像推的是 `refs/remotes/origin/main:refs/heads/main`，
    /// 若判据只认字符串等于 `refs/heads/main`，这种推送就**拿不到放行**
    /// （本地钩子 PR-only 那道闸会拦下），镜像永远同步不过去 —— 而那正是
    /// "回执全绿、客户端没变化"那类事故的形状。
    #[test]
    fn the_main_push_allowance_looks_at_the_destination() {
        assert!(refspec_targets_main("refs/heads/main"), "老形状照旧放行");
        assert!(
            refspec_targets_main("refs/remotes/origin/main:refs/heads/main"),
            "同步镜像那种形状也要放行（源是远端跟踪引用）"
        );
        assert!(
            refspec_targets_main("abc1234:refs/heads/main"),
            "任意源，只要目的地是 main"
        );
        assert!(!refspec_targets_main("refs/heads/feature"), "推进分支不算");
        assert!(
            !refspec_targets_main("refs/heads/main:refs/heads/other"),
            "往外推 main 的内容不算裸推 main"
        );
        assert!(!refspec_targets_main("refs/tags/v0.0.6"), "tag 不走这条");
        assert!(!refspec_targets_main("refs/heads/mainish"), "前缀像不算");
    }

    /// ★ Gitee 的 git-http 不吃先发式 Basic 头（2026-10-06 真机实测）—— 只认 URL 内嵌凭据。
    /// 这条钉**形状判定**：gitee.com 的 http(s) URL → 内嵌；remote 名 / 别的域 → `None` 回落。
    #[test]
    fn gitee_push_target_gets_url_credentials() {
        assert_eq!(
            gitee_url_with("https://gitee.com/o/r.git", "u", "t"),
            Some("https://u:t@gitee.com/o/r.git".to_owned())
        );
        assert_eq!(
            gitee_url_with("http://gitee.com/o/r", "u", "t"),
            Some("https://u:t@gitee.com/o/r".to_owned())
        );
        assert_eq!(gitee_url_with("origin", "u", "t"), None);
        assert_eq!(gitee_url_with("https://github.com/o/r", "u", "t"), None);
    }

    /// ★ **`git add -A` 的替代是"白名单前缀匹配"** —— 这条钉住"什么进得去、什么进不去"。
    /// 交付产物、结构规则表、资产台账进得去；源码、配置、别的东西进不去。
    #[test]
    fn stage_paths_are_an_explicit_allowlist() {
        for ok in [
            "presets/delivery/catalog.json",
            "presets/delivery/mkp/presets/A1-standard.toml",
            "presets/structure-signatures.toml",
            "presets/assets.toml",
        ] {
            assert!(is_allowed(ok), "{ok} 该在允许清单里");
        }
        for no in [
            "src-tauri/src/lib.rs",
            "package.json",
            "presets/machines/A1.toml",
            "presets/assets/bbs/Process/0.2mm/A1.json",
            "handoff.md",
        ] {
            assert!(
                !is_allowed(no),
                "{no} 不该被 stage —— 那会把无关改动卷进发布提交"
            );
        }
    }

    /// 白名单路径**是相对仓库根的前缀**，不是绝对路径（提交时 git 也按仓库根解释）。
    #[test]
    fn the_allowlist_is_relative_to_the_repo_root() {
        for a in STAGE_ALLOWLIST {
            assert!(!a.starts_with('/'), "{a} 不该是绝对路径");
            assert!(a.starts_with("presets/"), "{a} 该在 presets/ 下");
        }
    }

    /// ★★ **push 带认证但 Token 不进 URL** —— 这条钉住这一刀的认证方式。
    ///
    /// 只验"参数构造"（不真推）：`--dry-run` 那种真推需要网络，判据里不做。
    /// 断言：Basic 头是 `base64(username:token)`，且**没有**把 token 塞进 remote URL 的形状。
    #[test]
    fn git_push_authenticates_without_putting_the_token_in_the_url() {
        // 构造与 push_authenticated 内一致的那条 header
        let token = "ghp_secrettoken";
        let basic = base64_encode(format!("ben:{token}").as_bytes());
        let header = format!("http.extraHeader=Authorization: Basic {basic}");
        assert!(header.contains("Basic "), "该是一条 Basic 认证头");
        // base64 解出来的原文 = user:token
        assert_eq!(base64_decode_for_test(&basic), format!("ben:{token}"));
        // 头里不出现裸 token（base64 之后本就看不到），且**没有任何地方把它拼进 URL**
        let args = [
            "-c".to_owned(),
            "credential.helper=".to_owned(),
            "-c".to_owned(),
            header.clone(),
            "push".to_owned(),
            "-u".to_owned(),
            "origin".to_owned(),
            "publish/x".to_owned(),
        ];
        let joined = args.join(" ");
        // 关键：URL/参数里没有 `user:token@` 那种形状
        assert!(
            !joined.contains("ben:ghp"),
            "Token 不该以明文进参数：{joined}"
        );
        assert!(
            !joined.contains("ghp_secrettoken@"),
            "Token 不该进 URL：{joined}"
        );
        assert!(
            joined.contains("credential.helper="),
            "要清掉全局 credential helper"
        );
    }

    /// base64 编码正确性（对照几个已知向量）。
    #[test]
    fn base64_encodes_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
        // Basic 认证头的典型向量
        assert_eq!(base64_encode(b"ben:pass"), "YmVuOnBhc3M=");
    }

    /// 仓库地址规范化：https / ssh / `.git` / 大小写 / 尾斜杠都归一到同一形状。
    #[test]
    fn repo_url_normalization_makes_equivalent_addresses_match() {
        let a = normalize_repo_url("https://github.com/o/r.git");
        assert_eq!(a, "github.com/o/r");
        assert_eq!(normalize_repo_url("git@github.com:o/r.git"), a);
        assert_eq!(normalize_repo_url("https://GitHub.com/o/r"), a);
        assert_eq!(normalize_repo_url("https://github.com/o/r/"), a);
        assert_eq!(normalize_repo_url("ssh://git@github.com/o/r.git"), a);
        // 不同仓库不该相等
        assert_ne!(normalize_repo_url("https://github.com/o/r2.git"), a);
    }

    /// remote 与配置一致 / 不一致（临时真仓库验一遍）。
    #[test]
    fn remote_matches_compares_the_configured_repository() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let git = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("起 git");
            assert!(out.status.success(), "git {args:?} 失败");
        };
        git(&["init", "-q"]);
        git(&[
            "remote",
            "add",
            "origin",
            "git@github.com:MuCoreBenC/MKPSupportEase.git",
        ]);

        let g = Git::open(root);
        // 同一仓库的不同写法都算匹配
        assert!(g
            .remote_matches("https://github.com/MuCoreBenC/MKPSupportEase.git")
            .unwrap());
        assert!(g
            .remote_matches("git@github.com:MuCoreBenC/MKPSupportEase.git")
            .unwrap());
        // 别的仓库 = 不匹配
        assert!(!g
            .remote_matches("https://github.com/other/repo.git")
            .unwrap());
        assert!(!g
            .remote_matches("https://gitee.com/MuCoreBenC/MKPSupportEase.git")
            .unwrap());
    }

    /// 测试用：把 base64 解回原文（只覆盖 ASCII + 补齐，够验这几个向量）。
    fn base64_decode_for_test(s: &str) -> String {
        const TABLE: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let clean: Vec<u8> = s.bytes().filter(|b| *b != b'=').collect();
        let val = |b: u8| TABLE.iter().position(|t| *t == b).unwrap() as u32;
        let mut out = Vec::new();
        for chunk in clean.chunks(4) {
            let mut n = 0u32;
            for (i, b) in chunk.iter().enumerate() {
                n |= val(*b) << (18 - 6 * i);
            }
            out.push((n >> 16 & 0xff) as u8);
            if chunk.len() > 2 {
                out.push((n >> 8 & 0xff) as u8);
            }
            if chunk.len() > 3 {
                out.push((n & 0xff) as u8);
            }
        }
        String::from_utf8(out).unwrap()
    }

    /// 一个真实的 git 仓库走一遍：stage 白名单 + 提交，**别的东西一个都不许进去**。
    ///
    /// 用临时目录造一个真仓库（`git init`）—— 这是少数值得真跑 git 的地方：
    /// "白名单真的拦住了 `git add`"只有真调 git 才验得出来（假脚本验不了 git 自己的行为）。
    #[test]
    fn a_real_repo_only_commits_the_allowlisted_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let ok = |args: &[&str]| {
            let out = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .expect("起 git");
            assert!(
                out.status.success(),
                "git {args:?} 失败：{}",
                String::from_utf8_lossy(&out.stderr)
            );
        };
        ok(&["init", "-q"]);
        ok(&["config", "user.email", "t@example.com"]);
        ok(&["config", "user.name", "t"]);
        // 切到任务分支再提交：这套仓库的开发机装了「禁止直接在 main 上提交」的全局钩子，
        // 临时仓库默认落在 main 上会被拦（真实发布流程也从任务分支起）。这也顺带验了
        // `symbolic-ref --short HEAD` 拿得到非 main 分支名。
        ok(&["checkout", "-q", "-b", "publish/demo"]);

        // 造两类文件：白名单里的（交付产物）与白名单外的（源码）。
        // 走 `atomic_write`（clippy 对测试也生效，且与产品代码同一条写盘纪律）。
        std::fs::create_dir_all(root.join("presets/delivery")).unwrap();
        crate::fsx::atomic::atomic_write(&root.join("presets/delivery/catalog.json"), b"{}")
            .unwrap();
        std::fs::create_dir_all(root.join("src")).unwrap();
        crate::fsx::atomic::atomic_write(&root.join("src/lib.rs"), b"// not ours").unwrap();

        let git = Git::open(root);
        let staged = git
            .stage_allowed(&[
                "presets/delivery/catalog.json".to_owned(),
                "src/lib.rs".to_owned(), // 意图混进来 —— 必须被第二道闸挡掉
            ])
            .expect("stage");
        assert_eq!(staged, vec!["presets/delivery/catalog.json".to_owned()]);
        assert!(git.has_staged().unwrap(), "该有一份 staged");

        // 索引里只有白名单那一份
        let out = Command::new("git")
            .args(["diff", "--cached", "--name-only"])
            .current_dir(root)
            .output()
            .unwrap();
        let names = String::from_utf8_lossy(&out.stdout);
        assert!(names.contains("presets/delivery/catalog.json"));
        assert!(
            !names.contains("src/lib.rs"),
            "白名单外的文件被 stage 进去了：{names}"
        );

        git.commit("发布：演示").expect("commit");
        // 提交之后 src/lib.rs 仍是未跟踪 —— 它没被卷进去
        assert!(git.status_raw().unwrap().contains("src/lib.rs"));
    }
}
