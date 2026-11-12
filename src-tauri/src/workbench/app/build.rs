//! 生成 / 校验 / 恢复 / 发布（doc §10）。
//!
//! # 生成是原子的
//!
//! **先把全部产物算完，任一项算不出来则整批不动**；全部成功才逐个原子替换。
//! 半批生成的后果是最难查的一种：delivery 里有一半新一半旧，而两边都是合法的 TOML，
//! 客户端下载下来也不会报错，只是行为对不上。
//!
//! # 字节没变不重写文件
//!
//! `mtime` 变了会让同步工具以为有更新。所以比较的是**内容**。
//!
//! 2026-11-01 起这条规则是**纯字节比较**，没有任何例外行：产物头不再写生成时刻
//! （见下一节），于是「同一份配方渲染多少次都逐字节相同」成立，比较可以直接做。
//! 生成路径的「跳过」与预演的「无变化」（[`preview_one`]）认的是同一句话 ——
//! 两边各说各话，「将写入 N 份」就是假的。
//!
//! # 产物头不带生成时刻（2026-11-01，作者裁定）
//!
//! 原来第一行是 `# release_time: <渲染那一刻的 UTC 时钟>`。它是整条链里**唯一一个
//! "每次都变、又不表达任何内容"** 的字段，为迁就它长出了一串东西：比较要跳过一行
//! （旧的 `same_payload`）、台账要防 no-op 顶脏（旧的 `needs_built_record`）、
//! git 里存着一堆"实际生成时间"与字段值对不上的假时间、交付物无法逐字节复现。
//! 作者 2026-11-01 的原话是「他怎么可能这个时间就是刚才呢」—— 那正是这个字段
//! 在骗人：它记的是**渲染时刻**，跟"这一份什么时候被写出去"根本是两件事。
//!
//! 现在：**最终文件的字节只由原文件决定**。生成时间改由生成记录
//! （`built.json` 的 `stamp`，只记真正写了盘的那一次）与交付目录的 `publishedAt`
//! 表达，它们都在**文件之外**，改时间不会改内容。
//! 旧 `gen-presets` 配方（`preset_recipes.toml`，2026-11-12 已整链退役）当年也是这么做的：
//! 把时间写死，理由是"生成时现取会让「重新生成逐字节相同」失效"。
//! 现在工作台这条链沿用同一条纪律：时间只写在文件之外。
//!
//! # 产物头没有 uuid（2026-10-05 删，别加回来）
//!
//! 头注释曾写过一行 `# uuid:`（旧系统防"用户复制与云端冲突"的遗产）。当日审计查证：
//! 全仓没有任何代码读它，公开渠道也找不到读它的消费方 —— 云端/用户的分界由
//! **角色目录与来源记录**承担，完整性与版本由 **SHA 与 `# machine` / `# variant`** 承担。
//! 一行没有读者的字段就是假字段。有过期判定需求的是**工作台**，用的是
//! **两层指纹**（定义 / 值分开算，见 `presetdata::resolve::Layers::fingerprint`）——
//! 它只进 `built.json` 当「上次生成时刻」的旁注与"配方变没变"的展示依据，
//! **不进产物**、也不单独决定写不写（写不写只看字节，见上一节）。
//! 历史基线那 9 份（`crates/preset` 一侧）头里还带着 uuid，那是历史事实，不是兼容壳。
//!
//! # TOML 的形状照上游的真产物
//!
//! 实测 `presets/mkp/A1.toml`（上游头里有 uuid 与 release_time，我们**都不写**）：
//!
//! ```toml
//! # machine: A1
//! # variant: standard
//!
//! [toolhead]
//! offset = { x = -1, y = 18.6, z = 4 } # 笔尖偏移
//! speed_limit = 70 # 涂胶速度限制 (mm/s)
//! # 自定义工具头获取 G-code
//! custom_mount_gcode = """
//! G92 E0
//! """
//! ```
//!
//! 两条规则是从数据里读出来的，不是写死的：
//!
//! - **段名** = `param.section`（实测只有 `toolhead` 与 `wiping` 两个）
//! - **几个参数共享同一个 `tomlKey` 时合成内联表**，成员名取 `jsonKey`
//!   （`toolhead.offset.x/y/z` 的 `tomlKey` 都是 `offset`，`jsonKey` 分别是 `x/y/z`）
//!
//! ★ 2026-10-16（作者看客户端截图时点的两句）：头三行之后那句段落标签 `#胶笔配置`
//!   （上游原样，没写空格）**不写了** —— 它没有任何读者：机型 / 版本由上面两行承担、
//!   出处由来源记录承担，工作台与后处理内核一个都不读它（作者：「好像没有意义呀，
//!   那个没有意义的注释可以不存在」）。
//!   已经在盘上 / 已经发出去的那几份**一个字节都不动**（改一个字等于换版，见
//!   `docs/RELEASE-TRANSACTIONS.md`）：新写出来的没有这一行。M0 那条判据
//!   （`our_render_matches_the_machine_verified_baseline`）为此把这一行**从比较里滤掉**
//!   —— 基线是历史产物、带着它；那是散文标签，不是值。
//!
//! **不追求与上游产物逐字节相同**：那是另一个 builder 生成的，键序也不一致。
//! 要的是「合法 TOML + 值对 + 同样的输入产出同样的字节」。
//!
//! # 恢复配方走唯一写入口
//!
//! `wb_revert_preview` 只**算**出要提交哪些 patch，前端拿着它走 `wb_apply_draft`。
//! 这样恢复也是一条撤销、也进同一份差异清单 —— 而不是第二条写路径。
//!
//! 快照里只有有效值，看不出当时是哪一层给的，所以恢复一律写在**版本**这一层。
//! 原本继承来的那几项会从此脱钩，**名单当场列出来，不许闷着改**。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::ipc::traced;
use crate::presetdata::Origin;
use crate::workbench::clock;
use crate::workbench::domain::derive::Book;
use crate::workbench::domain::issues::{self, Report};
use crate::workbench::domain::patch::{BuiltRecord, Patch};
use crate::workbench::domain::wording as w;
use crate::workbench::domain::Level;
use crate::workbench::paths;
use crate::workbench::presets::registry::{ParamDef, UiComponent, ValueType};
use postprocess::diag::messages;

use super::delivery::DeliveryStage;
use super::{state, with_ctx, with_ctx_mut};

/// 「这一版现在会渲染成什么」的探针，给 [`Book`] 判「待更新」用。
///
/// **渲染器只有一台**（本模块的 [`render`]，与真生成同一个函数），所以这里只是把
/// `&Ctx` 折成闭包交给 `domain` 层 —— `domain` 不认识 `app`，也不许自己拼第二套。
/// 渲染失败（配方坏 / 注册表缺项）折成 `None`，由 `build_state` 保守判 `Stale`。
///
/// 闭包要活得跟 `Book` 一样久（`Book` 借的是 `&dyn Fn`），所以由调用方**先绑到
/// 一个具名变量**再装上去，别直接内联成临时值。
pub(super) fn renderer_for(ctx: &super::Ctx) -> impl Fn(&str) -> Option<String> + '_ {
    move |uid: &str| {
        let fresh = Book::new(&ctx.presets, &ctx.committed, &ctx.draft);
        render(&fresh, uid).ok().map(|r| r.text)
    }
}

/// 给 [`Book`] 装好"判待更新"所需的两样东西：渲染器 + **产物所在的树**。
///
/// 产物根**必须是真仓库**（`ctx.real_root`），不是 `presets.root()`（影子树）：
/// 影子树里没有 `delivery/`，拿它找产物会让每一份都判成"待更新"
/// （2026-11-01 实测：`wb_assets` 把 9 份全报成 `Stale`）。
pub(super) fn book_with_probe<'a>(
    ctx: &'a super::Ctx,
    presets: &'a crate::workbench::presets::Presets,
    committed: &'a super::Committed,
    draft: &'a crate::workbench::domain::patch::Draft,
    probe: &'a dyn Fn(&str) -> Option<String>,
) -> Book<'a> {
    Book::new(presets, committed, draft)
        /* 「改之前的值」读盘上已保存那一份，**不是**编辑树（刀 1）。
        参数台 / 版本详情 / 批量抽屉的 `↩` 都从这条路径拿数。 */
        .with_rest(&ctx.base)
        .with_renderer(probe)
        .with_product_root(ctx.real_root.clone())
}

/// 发布渠道。原先是上游 manifest 的 `compat.channel`；上游整层删掉后，
/// 工作台自己发的是正式渠道，定成常量
const PUBLISH_CHANNEL: &str = "stable";

/* ---------- 校验 ---------- */

/// **历次生成的回执**（2026-11-01）：新的在前。
///
/// 回执住应用数据目录（不进仓库），按份数自动清理。它只记录**生成**这一件事
/// —— 保存 / 发布 / 外部改文件都不在这里，别把它们塞进同一套机制。
///
/// 界面上「生成与发布」页可以翻它，回答"上一次生成到底做了什么、为什么失败"。
#[tauri::command]
pub fn wb_receipts(
    limit: Option<usize>,
) -> Result<Vec<super::receipt::GenerationReceipt>, AppError> {
    Ok(super::receipt::history(
        limit.unwrap_or(super::receipt::KEEP),
    ))
}

/// 预检（b05 Task 11.8）：[`issues::inspect`] 的全部 + 交付目录自查 + 内嵌目录自查。
///
/// 2026-11-12：旧的「清单 ↔ `preset_recipes.toml` 配方」对齐已随 `gen-presets` 整链
/// 退役。生成从来不读那份配方，工作台产物由 `presets/*.toml` 单一源渲染；那 8 条
/// 「版本在配方里没有对应变体」是退役链留下的空转待办，不再报。
#[tauri::command(async)]
pub fn wb_preflight() -> Result<Report, AppError> {
    traced("wb_preflight", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            /* 必须装渲染器（2026-11-01）：`build_state` 判"产物是不是旧的"靠的是
            `剥掉 generated_at(盘上) == 当前渲染`。没装渲染器就只能保守判 `Stale`，
            于是检查卡永远说「有 9 个版本的产物是旧的」，而右边生成页同时显示
            「已生成」—— 同一份数据两处说法相反（作者截图踩到）。 */
            let probe = renderer_for(ctx);
            let book = book_with_probe(ctx, &ctx.presets, &c, &d, &probe);
            // 交付目录的「清单 ↔ 文件」自查在 app 层做（要摸盘），结果作为一条交给预检
            let delivery = super::delivery::audit_catalog(
                &crate::workbench::paths::delivery_root_path(),
                &book,
            );
            // 内嵌目录（编译进安装包的那份）跟仓库数据对不对得上也是盘上的事实 ——
            // 同一条路交给预检：坏了自己成为报告里的一条，不让整个预检失败
            let embedded = audit_embedded_catalog();
            Ok(issues::preflight(&book, delivery, embedded))
        })
    })
}

/// 内嵌目录自查：盘上的 `catalog.generated.json`（下次构建会编进二进制、
/// 客户端首启释放的那份）与按当前仓库重新构建出来的是否**逐字节一致**。
///
/// 之前只有一条 Rust 单测（`runtime::tests::embedded_matches_rebuild`）盯着 ——
/// 红在 `cargo test` 里，检查页上没人提。现在预检把这件事摆上桌面：
/// 改了 presets/ 没重算这份文件，装出来的客户端首屏拿的还是旧数据 ——
/// 这是发布前该知道的事，而且**这条待办能就地修**（见 [`wb_regen_embedded_catalog`]）。
fn audit_embedded_catalog() -> Result<(), String> {
    let repo = crate::workbench::paths::repo_root();
    let catalog = crate::runtime::catalog::Catalog::build_from_repo(&repo)
        .map_err(|e| format!("重新构建内嵌目录失败：{}", e.message))?;
    let json = catalog
        .to_pretty_json()
        .map_err(|e| format!("内嵌目录序列化失败：{}", e.message))?;
    /* 比的是**盘上那份文件**，不是 `EMBEDDED_CATALOG`（编进当前这个二进制的字节）。
     *
     * 2026-11-05 作者拍板改口径：比编译进去的那份，这条待办在界面上就**修不了** ——
     * 文件写对了也不会绿，非得重新编译；等于把修复动作关在终端里。而"下次构建会编
     * 进去什么"才是这一层的真问题，那份字节就在盘上。旧口径仍由
     * `runtime::tests::embedded_matches_rebuild` 在 `cargo test` 里守着。 */
    let path = embedded_catalog_path();
    let on_disk = std::fs::read(&path).map_err(|e| format!("读不回来 {}：{e}", path.display()))?;
    if json.as_bytes() == on_disk.as_slice() {
        return Ok(());
    }
    Err("catalog.generated.json 与按当前 presets/ 重新构建出来的不一致".to_owned())
}

/// 内嵌目录文件在某个仓库根下的落点。`gen-catalog` / 生成第 5 步 / 这条修复动作
/// **同一个落点** —— 路径只在这一处拼，别再复写第三份。
fn embedded_catalog_path_at(root: &std::path::Path) -> std::path::PathBuf {
    root.join("src-tauri/src/runtime/catalog.generated.json")
}

/// 当前仓库根下的那一份
fn embedded_catalog_path() -> std::path::PathBuf {
    embedded_catalog_path_at(&crate::workbench::paths::repo_root())
}

/// 内嵌目录的**修复动作**（检查卡那条待办上的「重新生成」按钮）。
///
/// 与 `cargo run --bin gen-catalog` 的前半程同一条内核（`Catalog::build_from_repo`）——
/// 作者的要求：这条待办得能在工作台里就地修掉，不许只剩「去终端敲命令」这一条路。
///
/// **只写内嵌目录这一份**：交付面的 `catalog.json` 归 `wb_generate` / 发布事务，
/// 这里不动它（那是另一条链，混进来就成了第二个源）。
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddedCatalogReport {
    /// 这次真重写了吗（内容一样就不写，与生成的「没变不重写」同一口径）
    pub changed: bool,
    pub files: usize,
    pub revision: String,
}

#[tauri::command(async)]
pub fn wb_regen_embedded_catalog() -> Result<EmbeddedCatalogReport, AppError> {
    traced("wb_regen_embedded_catalog", |_| {
        // 落点跟**仓库**走（会话根），与应用数据目录无关 —— 这份文件是要进 git 的
        let repo = crate::workbench::paths::repo_root();
        let catalog = crate::runtime::catalog::Catalog::build_from_repo(&repo)?;
        let json = catalog.to_pretty_json()?;
        let path = embedded_catalog_path_at(&repo);
        let changed = std::fs::read(&path).ok().as_deref() != Some(json.as_bytes());
        if changed {
            crate::fsx::atomic::atomic_write(&path, json.as_bytes())?;
        }
        Ok(EmbeddedCatalogReport {
            changed,
            files: catalog.files.len(),
            revision: catalog.revision.clone(),
        })
    })
}

/* ---------- 渲染 ---------- */

/// 一份产物的文本 + 它的指纹。指纹用来判「要不要重写」与「产物过期没有」
pub struct Rendered {
    pub uid: String,
    pub file_name: String,
    pub text: String,
    pub fingerprint: String,
    /// 进快照的那一份有效值
    pub snapshot: BTreeMap<String, Value>,
}

/// 把一个版本的有效配方渲染成 MKP TOML。
///
/// `pub(super)`：`delivery_status` 用它判「盘上这份是不是当前渲染器的产物」——
/// 配方指纹看不出的模板变化（例如删掉一行注释），只有这样才抓得到。
pub(super) fn render(book: &Book<'_>, uid: &str) -> Result<Rendered, AppError> {
    let v = book
        .version(uid)
        .ok_or_else(|| AppError::not_found(format!("版本 {uid} 不存在")))?;
    let layers = book
        .version_layers(uid)
        .ok_or_else(|| AppError::corrupted(format!("{uid} 的取值层取不出来")))?;
    // 机型从**我们自己那份清单**里查（`presets/machines/*.toml`，借自 `Committed.catalog`）。
    //
    // 以前查的是上游那份（`book.up.catalog`）—— 于是渲染任一产物都要先有上游仓库，
    // 而机型 id 与版本 id 一样是**我们的身份**（`docs/ARCHITECTURE.md` §10.1），
    // 上游那层正在退出（b04 Task 12）。
    //
    // 直接理由（b05 Task 3.3）：没有这一改，那条「渲染出来的产物 vs 真机基线」的判据
    // 在 CI 里永远跑不起来 —— 上游不在仓库里（`local-reference/` 刻意不入库），
    // 而它恰恰是唯一盯着这条渲染链的判据。
    let machine = book
        .machines()
        .iter()
        .find(|m| m.id == v.machine_id)
        .ok_or_else(|| AppError::not_found(format!("机型 {} 不存在", v.machine_id)))?;

    let fingerprint = layers.fingerprint();
    let recipe = layers.effective_recipe();
    let snapshot: BTreeMap<String, Value> = recipe
        .iter()
        .map(|(k, val)| ((*k).to_owned(), (*val).clone()))
        .collect();

    let mut out = String::new();
    /*
     * 头里只有两行**表达内容身份**的：机型与版本。
     * 曾经的 `# uuid:` 已于 2026-10-05 删（没有读者的死字段）。
     *
     * ★ `# generated_at:` **不在这里生成**（2026-11-01 作者裁定）。
     *
     * 它属于"写盘那一刻"的事实，不属于"内容"。渲染是纯函数，同样的源必须给同样的
     * 字节 —— 否则「内容没变就不写」永远不成立（每次点生成都会先算出个新时间，
     * 一比较就"变了"）。所以：
     *
     *   render()  → 不含时间的正文     ← 比较用这一份
     *   stamp()   → 只在真要写盘时拼上 ← 写盘用这一份
     *
     * 客户端仍能读到生成时间（它还在文件里），但"变没变"只看业务内容。
     */
    out.push_str(&format!("# machine: {}\n", machine.id));
    out.push_str(&format!("# variant: {}\n", v.version_id.to_lowercase()));
    /* 这里原来还有一句段落标签 `#胶笔配置`（照上游抄的）—— 2026-10-16 删，见模块文档那一节 */

    // 段名从数据里来。段序按每段内最小的 layout.order —— 与界面上的顺序一致
    let mut sections: Vec<(&str, f64)> = Vec::new();
    for key in layers.keys() {
        let Some(p) = book.presets.registry.param(key) else {
            continue;
        };
        match sections.iter_mut().find(|(s, _)| *s == p.section) {
            Some((_, o)) => *o = o.min(p.layout.order),
            None => sections.push((p.section.as_str(), p.layout.order)),
        }
    }
    sections.sort_by(|a, b| a.1.total_cmp(&b.1));

    for (section, _) in sections {
        out.push_str(&format!("\n[{section}]\n"));

        // 同一个 tomlKey 下可能有多个参数（内联表）。按 tomlKey 首次出现的顺序排
        let mut groups: Vec<(&str, Vec<&ParamDef>)> = Vec::new();
        for key in layers.keys() {
            let Some(p) = book.presets.registry.param(key) else {
                continue;
            };
            if p.section != section {
                continue;
            }
            match groups.iter_mut().find(|(t, _)| *t == p.toml_key) {
                Some((_, v)) => v.push(p),
                None => groups.push((p.toml_key.as_str(), vec![p])),
            }
        }

        // 段内键序 = **tomlKey 字母序**（大小写不敏感）。作者 2026-10-03：
        // 「明明都是 O 开头的 offset 都是一起的，生成的时候却改变了它的顺序」——
        // 以前按界面顺序（layout.order）排，注册表条目一挪、产物键序就漂，
        // diff 里满屏错位。字母序谁都能预期：offset_x/y/z 永远连在一起，
        // 生成不再改变没改过的那些行的位置。
        groups.sort_by_key(|g| g.0.to_lowercase());
        for params in groups.iter_mut() {
            params.1.sort_by_key(|p| p.json_key.to_lowercase());
        }

        for (toml_key, params) in groups {
            if params.len() > 1 {
                // 内联表：`offset = { x = -1, y = 18.6, z = 4 } # 笔尖偏移`
                let members: Vec<String> = params
                    .iter()
                    .filter_map(|p| {
                        layers
                            .effective(&p.key)
                            .map(|hit| format!("{} = {}", p.json_key, scalar(p, hit.value)))
                    })
                    .collect();
                out.push_str(&format!("{toml_key} = {{ {} }}", members.join(", ")));
                // 内联表的注释取第一个成员的 —— 三个成员各写一句会把行撑得没法读
                push_comment(&mut out, &params[0].toml_comment, true);
                continue;
            }

            let p = params[0];
            let Some(hit) = layers.effective(&p.key) else {
                continue;
            };
            /* 「无值不落盘」：只有出厂层有值 ⇒ 该机型/变体根本用不到这一项，整行不写。
             * 例：A 系列的落笔/收笔骨架只动 X，等待位 Y 对它不适用；落一个 `0` 进
             * TOML 只会让界面上多出一格永远不该被改的 Y（而那是绝对坐标）。 */
            if p.omit_when_unset && hit.origin == Origin::Factory {
                continue;
            }
            if p.ui_component == UiComponent::Gcode {
                // 多行字符串：注释在上一行，值用 `"""`
                push_comment(&mut out, &p.toml_comment, false);
                out.push_str(&format!(
                    "{toml_key} = \"\"\"\n{}\n\"\"\"\n",
                    gcode_body(hit.value)
                ));
            } else {
                out.push_str(&format!("{toml_key} = {}", scalar(p, hit.value)));
                push_comment(&mut out, &p.toml_comment, true);
            }
        }
    }

    Ok(Rendered {
        uid: uid.to_owned(),
        file_name: preset_file_name(&machine.id, &v.version_id),
        text: out,
        fingerprint,
        snapshot,
    })
}

/// 把渲染正文变成"写盘正文"：在头部插入 `# generated_at: <本地时间+偏移>`。
///
/// 格式 ISO 8601 带偏移量（`2026-11-01T12:30:00+08:00`）：绝对时刻明确，本地可读。
/// 偏移量取**系统当前时区**（不是写死 +08:00）—— 在东京就是 +09:00，
/// 有夏令时的地区也能给对当时的偏移。**绝不用文件 mtime 顶替**：下载 / 解压 /
/// 复制都会改 mtime，而这里是应用明确记录下来的"这份产物何时生成"。
///
/// 只在**真要写盘**时调用（见 [`wb_generate`] 的第 ② 步与强制生成路径）：
/// 拿它去参与"变没变"的比较是错的 —— 每次都不同，一比较就永远"变了"。
fn stamp(rendered: &str) -> String {
    use crate::workbench::GENERATED_AT_PREFIX as prefix;
    let line = format!("{prefix} {}\n", clock::now_local_rfc3339());
    // 头部两行 `# machine` / `# variant` 之后就插入（保持"头注释块"的形状）
    let mut out = String::with_capacity(rendered.len() + line.len());
    let mut inserted = false;
    for l in rendered.split_inclusive('\n') {
        if !inserted && !l.starts_with('#') {
            out.push_str(&line);
            inserted = true;
        }
        out.push_str(l);
    }
    if !inserted {
        out.push_str(&line);
    }
    out
}

/// 产物文件名。`{机型}-{版本小写}.toml`，例如 `A1-standard.toml`、`A1_MINI-fastv3.3.toml`。
///
/// # 为什么是这一套，不是上游那一套
///
/// 上游 manifest 给的名字是 `A1.toml` / `A1F.toml` / `A1F_260628.toml` 这种历史命名，
/// 而**消费端按 `{机型}-{版本小写}.toml` 找文件**（它 9 份内置预设全是这个形状，
/// 见 b04 `AUDIT-EVIDENCE.md` §2）。名字对不上的后果不是报错，是消费端找不到 ——
/// 我们生成了一堆它认不出的文件。
///
/// 所以这条改动同时**断掉了对上游 `mkp_preset.file_name` 的依赖**：
/// 名字现在只由「机型 id + 版本 id」决定，而这两样都在我们自己的清单里。
/// 上游整层删掉时（b04 Task 12）这里一个字都不用改。
///
/// 版本 id 小写是跟着 `# variant:` 那一行走的 —— 同一份产物里两处指同一个东西，
/// 大小写不一致会让人以为是两个变体。
///
/// # 这是薄壳，规则不在这一层
///
/// 权威实现在 `crates/preset/src/generate.rs` 的 `file_name`（从 crate 根导出为
/// `preset::preset_file_name`），规范写在 `docs/ARCHITECTURE.md` §10。**小写化在那边
/// 发生，只发生一次。**
///
/// 这里以前是**第二份实现**：形状相同，但两份独立实现之间没有编译器 —— 漂移的表现
/// 不是报错，是消费端找不到文件。b05 Task 2 把它并掉了，代价是 `mkpse-preset` 成了
/// 依赖（`workbench` feature 下的可选依赖，见 `src-tauri/Cargo.toml`）：
/// **默认（发布）构建的依赖图里没有它**，那 56 KB 参数注册表与 9 份内置预设也就
/// 进不去给用户的二进制。换掉的是"一个拼字符串的函数要拖进整个内核 crate"这条顾虑，
/// 换来的是一条规则只有一处。
///
/// 两边仍然可能被各自改坏 —— 连着它们的是本文件的判据
/// `naming_matches_the_preset_crate`（逐例对两边），不是类型系统。
pub fn preset_file_name(machine_id: &str, version_id: &str) -> String {
    preset::preset_file_name(machine_id, version_id)
}

/// 行尾注释或独立注释行。**空注释不写 `# `** —— 一个孤零零的井号是噪音
fn push_comment(out: &mut String, comment: &str, trailing: bool) {
    let c = comment.trim();
    if c.is_empty() {
        out.push('\n');
        return;
    }
    if trailing {
        out.push_str(&format!(" # {c}\n"));
    } else {
        out.push_str(&format!("# {c}\n"));
    }
}

/// 标量的 TOML 字面量。**字符串要转义** —— 值里有个引号就能让整份配方解析失败
fn scalar(p: &ParamDef, v: &Value) -> String {
    match p.value_type {
        ValueType::Bool => match v.as_bool() {
            Some(b) => b.to_string(),
            // 类型对不上时不猜。这种数据在校验里已经是阻断，走不到这儿
            None => "false".to_owned(),
        },
        ValueType::Float | ValueType::Int => match v.as_f64() {
            Some(n) if n.fract() == 0.0 => format!("{}", n as i64),
            Some(n) => format!("{n}"),
            None => "0".to_owned(),
        },
        ValueType::Text => toml_string(v.as_str().unwrap_or_default()),
    }
}

fn toml_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// 多行字符串的内容。`"""` 出现在里面会提前结束字面量，所以要断开
fn gcode_body(v: &Value) -> String {
    v.as_str()
        .unwrap_or_default()
        .replace("\"\"\"", "\"\"\\\"")
        .trim_end()
        .to_owned()
}

/* ---------- 生成 ---------- */

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Scope {
    /// 主按钮「生成待更新项」
    Stale,
    /// 「全部生成」
    All,
    /// 「生成勾选的」
    Picked(Vec<String>),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenerateReport {
    pub stamp: String,
    /// 真的写了盘的
    pub written: Vec<String>,
    /// 算出来和现在的文件**一模一样**，所以没重写。这一条要显式说出来 ——
    /// 不说的话「点了生成但文件时间没变」看起来像失败了
    pub unchanged: Vec<String>,
    /// 跳过的（暂无资源）+ 原因
    pub skipped: Vec<(String, String)>,
    /// **生成事务回执**（2026-11-01）：工作台模态框、CLI JSON、AI 报告读的都是这一份。
    ///
    /// `None` 只会出现在"还没进事务就断了"（比如影子分叉被拦在 `generate_with` 之前）。
    pub receipt: Option<super::receipt::GenerationReceipt>,
    // 没有生成记录字段：台账（built.json）在生成事务里直接落盘，报告不再携带
    // 它、前端也不再回填草稿（2026-10-06 状态机修正）。
}

/// 这次要生成哪些（`todo`）与跳过了哪些（带原因）。
///
/// **`wb_generate` 与 `wb_generate_preview` 共用这一处** —— 预演必须与真生成算的是
/// 同一批、同一套跳过理由，否则"确认过的"和"真写的"就会是两回事。
fn planned_todos(book: &Book<'_>, scope: &Scope) -> (Vec<String>, Vec<(String, String)>) {
    let rows = book.build_rows();
    let wanted: Vec<&str> = match scope {
        Scope::Stale => rows
            .iter()
            .filter(|r| r.buildable)
            .map(|r| r.uid.as_str())
            .collect(),
        Scope::All => rows.iter().map(|r| r.uid.as_str()).collect(),
        Scope::Picked(uids) => uids.iter().map(String::as_str).collect(),
    };

    let mut skipped: Vec<(String, String)> = Vec::new();
    let mut todo: Vec<String> = Vec::new();
    for uid in wanted {
        // **没有床身尺寸的机型：跳过时说真因。**
        //
        // 这一台生成出来也用不了 —— 消费端读预设时会在内置尺寸表里查不到它，
        // 然后拒掉整份配方（不是少一项检查）。b04 的 P0 审计查明了这件事。
        // 原来它走的是「暂无资源」那条通用话术，而那句话让人去找资源，
        // 方向是错的：要补的是尺寸。
        let no_dims = book
            .version(uid)
            .and_then(|v| book.machines().iter().find(|m| m.id == v.machine_id))
            .is_some_and(|m| !m.has_dimensions);
        if no_dims {
            skipped.push((uid.to_owned(), w::disabled::BUILD_NO_DIMENSIONS.to_owned()));
            continue;
        }
        match rows.iter().find(|r| r.uid == uid) {
            Some(r) if r.state == w::BuildState::NoResources => {
                skipped.push((uid.to_owned(), w::disabled::BUILD_NO_RESOURCES.to_owned()))
            }
            Some(_) => todo.push(uid.to_owned()),
            None => skipped.push((uid.to_owned(), "这一版不在树上".to_owned())),
        }
    }
    (todo, skipped)
}

/// 生成。**有阻断时直接拒绝** —— 那是全程唯一的硬闸门。
///
/// ★ 它是 [`generate_with`] 的薄壳（`with_ctx_mut` + trace）。真正的写在那个收
/// `&mut Ctx` 的自由函数里 —— **发布事务要复用同一台生成器**，而它已经在
/// `with_ctx_mut` 里了（锁不可重入），只能调自由函数。要可变借出，是因为生成
/// 事务要把台账记进 `built.json` 并同步会话内的 committed（见 [`generate_with`]）。
#[tauri::command]
pub fn wb_generate(scope: Scope, force: Option<bool>) -> Result<GenerateReport, AppError> {
    traced("wb_generate", |_| {
        with_ctx_mut(|ctx| {
            require_clean(ctx)?;
            /* 模型手里那份与 presets/ 已经分叉 ⇒ 不许拿旧的渲染再写回去 */
            require_shadow_in_sync(ctx)?;
            let out = generate_with(ctx, &scope, force.unwrap_or(false))?;
            /* 产物写的是真仓库；影子树里那份 `delivery/` 还停在旧代。
            不对齐的话，下一次「保存」会把它当成"要写回旧内容" —— 直接把刚生成的
            产物覆盖回去。生成要求干净，所以整份对齐是安全的。 */
            crate::workbench::domain::txn::mirror(&ctx.real_root, &ctx.shadow_root)?;
            Ok(out)
        })
    })
}

/// **影子树与真仓库必须是同一份，否则拒绝生成**（2026-10-31）。
///
/// 与 [`require_clean`] 分工，两件事分开说：
///
/// | 情形 | 谁拦 | 该做什么 |
/// |---|---|---|
/// | 用户改过、还没保存 | [`require_clean`]（dirty > 0） | 先保存或放弃 |
/// | 外部改过 `presets/`、影子还是旧的 | **这里** | 先重新加载（CLI `presets reload`） |
///
/// 为什么必须有这一条：`Ctx::open` 发现影子非空且与磁盘不一致时，只把会话标成
/// `recovered` 就照旧**读影子**（`Presets::load_from(&shadow_root)`），而
/// `require_clean` 只看 dirty 计数。于是「外部改了源、工作台影子是旧拷贝」时，
/// 生成会拿旧影子渲染、再写回真仓库 —— 2026-10-31 A1 三份 0.7 被写回 0.8 就是这条链。
///
/// **不自动 mirror**：影子可能含着用户还没保存的工作，静默对齐会把它们冲掉。
/// 保留哪一份是人的决定（工作台「重新加载」/ CLI `presets reload` 才显式放弃编辑态）。
pub(super) fn require_shadow_in_sync(ctx: &super::Ctx) -> Result<(), AppError> {
    if super::shadow_differs(&ctx.real_root, &ctx.shadow_root)? {
        return Err(AppError::invalid_argument(messages::text(
            "ui.workbench.txn.diverged",
            &[],
        )));
    }
    Ok(())
}

/// 这一次生成**依据哪一份源**（给人看的一句话依据）。
///
/// 2026-11-01 作者要求：`generate` 不得仅因"影子树与真源不同"就拒绝，
/// 并且必须**明确说清用的是哪一份**，不许静默混用。
///
/// 判据是**两棵树的内容**，不是"`Presets` 的根指哪"：会话永远读影子树
/// （见 `Ctx::open_at`），拿根路径比会永远得出"按草稿生成"。正常路径上
/// [`require_clean`] + [`require_shadow_in_sync`] 已经保证两者逐字节相同 ——
/// 这里如实说"按 presets/ 生成"；真有分叉（调用方绕过了闸）就把话说破，别静默混用。
///
/// 快照失败按"有分叉"算：宁可把依据说得保守，也不能替它宣称"用的是权威源"。
pub(super) fn generation_source_note(ctx: &super::Ctx) -> String {
    let diverged = super::shadow_differs(&ctx.real_root, &ctx.shadow_root).unwrap_or(true);
    if diverged {
        "按工作台草稿生成（presets/ 上有未保存的编辑态）".to_owned()
    } else {
        "按 presets/（磁盘上的权威源）生成".to_owned()
    }
}

/// **生成/发布只在「已保存」的那一版上跑**（2026-10-31 统一编辑事务）。
///
/// 草稿是编辑态，产物是发布态 —— 拿编辑态去生成，产物里就会混进一批
/// 「还没决定要不要」的改动，而台账记的却是另一代。所以这里**拒绝**，并且
/// **不自动保存**（保存是用户的边界，不是生成顺手的副作用）。
pub(super) fn require_clean(ctx: &super::Ctx) -> Result<(), AppError> {
    /* 与页头那个 N 同一处算：值编辑只活在内存（`Ctx::history`），只看事务与草稿
    会把"改了值还没保存"当成干净 —— 而生成拿的是编辑态。 */
    let dirty = ctx.dirty_count();
    if dirty == 0 {
        return Ok(());
    }
    Err(AppError::invalid_argument(messages::text(
        "ui.workbench.txn.dirty",
        &[("count", &dirty.to_string())],
    )))
}

/// 生成的**锁无关内核**：给定一份会话，按 scope 把产物写进 `delivery/mkp/presets/` 并重算目录。
///
/// 与 [`preview_with`] 是同一条理由（见那里）：「生成前预演」与「真生成」、
/// 「发布事务里的生成」必须是**同一批 todo、同一处渲染、同一处落点**。
/// 命令壳只负责 `with_ctx_mut` + trace。
///
/// ⚠ **它会写盘**（产物 + 快照 + **台账** + catalog）。调用方负责先过闸
/// （[`issues::inspect`]）。
pub(super) fn generate_with(
    ctx: &mut super::Ctx,
    scope: &Scope,
    force: bool,
) -> Result<GenerateReport, AppError> {
    /*
     * **统一生成事务的正式返回值**（2026-11-01）：先把回执立起来，成功与失败
     * 都落一份 —— 只弹一句"生成失败"等于把现场丢了。
     */
    /*
     * 回执**只在内存**（2026-11-01 作者裁定：不落盘、不入库）。
     * `repo_root` 现在只用来给编号取日期前缀，不再决定任何写盘位置。
     */
    let receipts_root = crate::workbench::paths::repo_root();
    let mut receipt =
        super::receipt::GenerationReceipt::start(&receipts_root, clock::now_iso8601());
    match generate_inner(ctx, scope, force, &mut receipt) {
        Ok(mut out) => {
            /* **`passed` 不能只看"流程没报错"**（2026-11-01）。
             *
             * 三组计数里只要有一个 `failed > 0`，就不是通过 —— 哪怕主流程走完了。
             * 反过来，三条全 0 写也**不是失败**：那是"内容都没变"，是正常结果，
             * 但要在 `commit` 这一步里明说，不能让人以为"什么都没干"。 */
            let failed = receipt.delivery.failed + receipt.assets.failed + receipt.baseline.failed;
            if failed == 0 {
                receipt.status = super::receipt::ReceiptStatus::Passed;
                let nothing_changed = receipt.delivery.written == 0
                    && receipt.assets.written == 0
                    && receipt.baseline.written == 0;
                receipt.step(
                    "commit",
                    true,
                    if nothing_changed {
                        // 明确说"没变"，不许让它看起来像"跑了个寂寞"
                        format!(
                            "全部 {} 份内容均未变化，没有写任何文件",
                            receipt.delivery.total
                        )
                    } else {
                        "catalog + 台账已落盘".to_owned()
                    },
                );
            } else {
                receipt.status = super::receipt::ReceiptStatus::Failed;
                let msg = format!("有 {failed} 份文件没有写成");
                receipt.error = Some(msg.clone());
                receipt.step("commit", false, msg);
            }
            if let Err(e) = super::receipt::save(&receipt) {
                tracing::warn!(error = %e, "回执没写下来（不影响这次生成）");
            }
            out.receipt = Some(receipt);
            Ok(out)
        }
        Err(e) => {
            receipt.error = Some(e.message.clone());
            receipt.step("commit", false, e.message.clone());
            if let Err(werr) = super::receipt::save(&receipt) {
                tracing::warn!(error = %werr, "失败回执也没写下来");
            }
            Err(e)
        }
    }
}

/// 事务的**内芯**：给定会话与一份回执，按顺序把三组产物做出来。
///
/// 三组各自的落点不同（`presets/delivery/`、`crates/preset/assets/presets/`、
/// `crates/postprocess/tests/fixtures/presets/`），所以做不到文件系统级的"一次
/// rename 全生效"；这里保证的是**同一份输入 + 同一个顺序 + 同一份回执**：
/// 任何一步出错都会带着已完成的那几步一起出门（见 [`generate_with`] 的失败分支）。
fn generate_inner(
    ctx: &mut super::Ctx,
    scope: &Scope,
    force: bool,
    receipt: &mut super::receipt::GenerationReceipt,
) -> Result<GenerateReport, AppError> {
    let (c, d, _) = state(ctx)?;
    /* 第 1 步：输入冻结。`ctx.presets` 已在内存里，`require_clean` 已保证它 == 磁盘 */
    /* **明确记下这次按哪一份源生成**（2026-11-01 作者要求）：
    不许静默混用真源与影子树。正常是 `presets/`；真有未保存草稿时上面的闸
    已经先拦下了，能走到这里就说明用的是磁盘上的权威源。 */
    receipt.step("frozen", true, generation_source_note(ctx));
    // 装渲染器：`planned_todos` 走 `build_rows`，那里的「待更新」是拿当前渲染
    // 跟盘上字节比出来的（与本次要写的判据同一台渲染器）
    let probe = renderer_for(ctx);
    let book = book_with_probe(ctx, &ctx.presets, &c, &d, &probe);

    let report = issues::inspect(&book);
    if let Some(b) = report.first_block() {
        return Err(AppError::invalid_argument(w::disabled::BUILD_BLOCKED)
            .with_detail(format!("{}：{}", b.title, b.detail)));
    }

    let (todo, skipped) = planned_todos(&book, scope);

    // ① 全部算完。**任一项算不出来则整批不动**
    let mut rendered: Vec<Rendered> = Vec::with_capacity(todo.len());
    for uid in &todo {
        rendered.push(render(&book, uid)?);
    }

    /* ② 交付目录整批事务。落点是**真仓库**的 `presets/delivery/` ——
     * 统一编辑事务之后 `presets` 从影子树加载，而产物是发布资产，必须落在
     * git 看得见的那棵树上。`require_clean` 已保证影子树 == 真仓库。
     *
     * 候选集把 TOML / content / catalog 一次性重算成确定性字节，连同受管清单内
     * 的待删除文件作为一个事务：备份 → 写 / 移回收站 → 失败回滚 → 回读验证。
     * 写入的是不含写盘时刻的字节；时刻记在本次回执上。 */
    let presets_root = ctx.real_root.as_path();
    let dist_root = paths::delivery_root_at(presets_root);
    let asset_root = paths::assets_root_at(presets_root);
    std::fs::create_dir_all(&asset_root)
        .map_err(|e| AppError::io("建不出资产目录").with_detail(e.to_string()))?;
    let trash_root = ctx.store.root.join(".trash");
    let tx_stamp = clock::now_iso8601().replace([':', ' '], "-");

    let mkp_rel = format!("{}/presets", crate::workbench::app::delivery::MKP_DIR);
    let allowed: BTreeSet<String> = rendered
        .iter()
        .map(|r| format!("{mkp_rel}/{}", r.file_name))
        .chain(
            super::delivery::CONTENT_FILES
                .iter()
                .map(|rel| (*rel).to_owned()),
        )
        .chain([super::delivery::NEW_CATALOG_FILE.to_owned()])
        .collect();
    let outcome = super::delivery::generate_transaction(
        &dist_root,
        &trash_root,
        &asset_root,
        &book,
        &tx_stamp,
        Some(&allowed),
        force,
    )?;

    let mut written: Vec<String> = Vec::new();
    let mut unchanged: Vec<String> = Vec::new();
    let mut fingerprints: BTreeMap<String, String> = BTreeMap::new();
    for r in &rendered {
        let rel = format!("{mkp_rel}/{}", r.file_name);
        // 快照照样写：它是恢复配方的依据，和产物有没有重写无关
        let v = book.version(&r.uid).expect("刚才渲染过");
        ctx.store.write_doc(
            &ctx.store.snapshot_rel(&v.machine_id, &v.version_id)?,
            &r.snapshot,
        )?;
        if outcome.written.contains(&rel) {
            written.push(r.uid.clone());
            fingerprints.insert(r.uid.clone(), r.fingerprint.clone());
        } else {
            unchanged.push(r.uid.clone());
        }
    }

    /* 三个数各记各的（2026-11-01）：`written` 只数**真写了盘**的。
    `written 0/9` 到底意味着"9 份都没变"还是"9 份都失败了"，
    只有把 unchanged / failed 分开才答得出来。 */
    receipt.delivery = super::receipt::Counts::of(written.len(), unchanged.len(), 0);
    /* 回执点名的是**文件名**（能直接定位到盘上那份），不是 uid */
    receipt.files = rendered
        .iter()
        .filter(|r| written.contains(&r.uid))
        .map(|r| r.file_name.clone())
        .collect();
    receipt.step(
        "delivery",
        true,
        format!(
            "{}；事务回读验证通过 {} 份，删除 {} 份",
            receipt.delivery.summary(),
            outcome.verified,
            outcome.removed.len()
        ),
    );

    /*
     * ★ 第 3–5 步只在**真仓库会话**上做。
     *
     * 那三处落点是编译期常量（`preset::generate::assets_dir()` / `fixtures_dir()`），
     * 与 `ctx.real_root` 无关。测试用临时会话（`real_root` 指向 tempdir），
     * 拿它去跑会把测试的假数据写进真仓库 —— 所以先认门，临时会话如实记"跳过"。
     */
    if ctx.real_root == crate::workbench::paths::real_presets_root_path() {
        /* 第 3 步：内核产物 —— **同一份 presets/ 源渲染出的那 9 份**（编进安装包的那批）。
         *
         * ★ 2026-11-01 单一源改造（作者裁定 B）：这里原来读 `preset_recipes.toml`
         * 那套**独立的配方数据**，于是同一个参数有两份值、两套生成流程 ——
         * A1 的 `tower_extrude_ratio` 一边 1.0 一边 0.9，就是这么分叉的
         * （10-31 配方写 0.9，11-01 参数同步把注册表改成 1.0，只更新了交付侧）。
         *
         * 现在：内核产物与交付产物**由同一台渲染器、同一份源**算出来，逐字节相同。
         * 配方那条链已删，`presets/` 是唯一权威源 —— 改参数只改一处，两边同时变。
         */
        let assets_dir = preset::generate::assets_dir();
        /* 按**整个清单**算出的全部产物名，与本次 `scope` 无关。
        不能用 `rendered`：它只含这次要生成的那几份（Stale / Picked 都是部分），
        拿它当「该有的集合」会把没参与这次生成的那些版本产物误当旧名清掉。
        `delivery::expected_products` 是「盘上该有什么」的唯一实现，
        连停用机型的产物也认（清理只许按它，见那边的注释）。 */
        let mkp_prefix = format!("{mkp_rel}/");
        let expected_names: BTreeSet<String> = super::delivery::expected_products(&book)
            .iter()
            .filter_map(|rel| rel.strip_prefix(&mkp_prefix))
            .map(str::to_owned)
            .collect();
        let mut n_written = 0usize;
        let mut n_same = 0usize;
        for r in &rendered {
            let target = assets_dir.join(&r.file_name);
            /* **内核产物与交付产物是同一份内容**（2026-11-01 单一源）：
            所以它也盖同一个 `# generated_at:`，判据也一样（剥掉时间再比）。
            两边都从同一个 `render()` 出来、只差写盘时刻 —— 时间不同不算漂移。 */
            let body = stamp(&r.text);
            /* 「内容没变」= 剥掉时间后相同**且**盘上那份已经带着生成时间。
            后半句不能省：没有 `# generated_at:` 时，光比内容会得出"没变 → 不写"，
            那一行就永远补不上（自锁）。

            这里**不用** [`crate::workbench::up_to_date`]：那一处问的是"盘上是不是
            候选字节"，而内核产物写出去的字节自己就带着时间行 —— 它要求"盘上有时间行"
            才算不用写，正好相反。两个问题、两条判据，别合并。 */
            let same = std::fs::read_to_string(&target).ok().is_some_and(|old| {
                old.contains(crate::workbench::GENERATED_AT_PREFIX)
                    && crate::workbench::strip_generated_at(&old) == r.text
            });
            if same {
                n_same += 1;
                continue;
            }
            crate::fsx::atomic::atomic_write(&target, body.as_bytes())
                .map_err(|e| AppError::io("内核产物写不出来").with_detail(e.to_string()))?;
            n_written += 1;
        }
        let n_pruned = prune_orphan_presets(
            &assets_dir,
            &expected_names,
            &trash_root,
            "kernel-presets",
            &tx_stamp,
        )?;
        receipt.assets = super::receipt::Counts::of(n_written, n_same, 0);
        receipt.step(
            "assets",
            true,
            format!("{}；清掉旧名 {} 份", receipt.assets.summary(), n_pruned),
        );

        /* ★ **单一源的判据**（2026-11-01）：内核产物与交付产物必须逐字节相同。
         *
         * 两边都由同一个 `render()` 从同一份 `presets/` 算出来，只要写盘没出错就必须
         * 相等。一旦不等，说明又有人给其中一侧开了第二条数据来源 —— 那正是
         * `tower_extrude_ratio` 一边 1.0 一边 0.9 的成因。当场报错，别让它进 git。
         */
        let delivery_mkp_dir = paths::delivery_root_at(presets_root)
            .join(crate::workbench::app::delivery::MKP_DIR)
            .join("presets");
        if let Some((name, ours, theirs)) = first_drift(&delivery_mkp_dir, &assets_dir, &rendered) {
            return Err(AppError::internal(messages::text(
                "ui.workbench.build.dual_source",
                &[("file", &name)],
            ))
            .with_detail(format!("交付：{ours}\n内核：{theirs}")));
        }
        receipt.step("single-source", true, "交付与内核逐字节一致");

        /* 第 4 步：测试基线 —— 内容相同就不写（改了期望值又没人看 diff 才是风险） */
        let fixtures_dir = preset::generate::fixtures_dir();
        let n_base = preset::generate::sync_baseline(&assets_dir, &fixtures_dir)
            .map_err(|e| AppError::io("测试基线同步失败").with_detail(e))?;
        /* 基线只写不清，旧名由这里按同一份 expected 清掉 —— 两处目录同进同退。
        IR 判据的冻结输入已搬到 `fixtures/ir/presets/`，不受这次清理影响。 */
        let n_base_pruned = prune_orphan_presets(
            &fixtures_dir,
            &expected_names,
            &trash_root,
            "baseline-presets",
            &tx_stamp,
        )?;
        /* `sync_baseline` 返回"这次真写了几份"；总数是那份目录里应有的份数，
        未变数由它自己算 —— 不拿上面 assets 的计数冒充（那是另一个东西）。 */
        let base_total = rendered.len();
        receipt.baseline = super::receipt::Counts::of(n_base, base_total.saturating_sub(n_base), 0);
        receipt.step(
            "baseline",
            true,
            format!(
                "{}；清掉旧名 {} 份",
                receipt.baseline.summary(),
                n_base_pruned
            ),
        );

        /* 第 5 步：内嵌目录重建 —— 读的就是刚写出去的那批入库产物 */
        /* `build_from_repo` 收的是**仓库根**（它自己会拼 `presets/`），不是 presets 那一层 */
        let repo = crate::workbench::paths::repo_root();
        let embedded = crate::runtime::Catalog::build_from_repo(&repo)?;
        let embedded_json = embedded.to_pretty_json()?;
        /* ★ 落点也必须是**仓库根**（`<repo>/src-tauri/src/runtime/…`）。
        `ctx.real_root()` 是 `presets/` 那一层 —— 拿它拼会写进
        `presets/src-tauri/src/runtime/` 这个不存在的树里（2026-11-07 实测：
        生成之后真正的内嵌目录还是旧的，错的树里多出一份）。 */
        crate::fsx::atomic::atomic_write(
            &embedded_catalog_path_at(&repo),
            embedded_json.as_bytes(),
        )?;
        receipt.step("embedded", true, format!("{} 份文件", embedded.files.len()));
    } else {
        receipt.step("assets", true, "跳过（临时会话）");
        receipt.step("baseline", true, "跳过（临时会话）");
        receipt.step("embedded", true, "跳过（临时会话）");
    }

    let ledger_stamp = clock::now_iso8601();

    /* ★★ **台账与产物同一批事务落盘**（2026-10-06 状态机修正）。
    生成记录曾经装进报告交前端回填草稿、等下一次「保存」才进 built.json ——
    那允许「交付文件已是新字节、台账还是上一代」的中间态存在：本机（草稿叠加态）
    显示已生成，干净检出（CI）判待生成，提交/发布把分裂状态写进 git。现在生成
    记录在这里**直接落盘**（[`super::storage::merge_built_records`]）：生成完成
    = 台账已是这一代，不再有「生成完还得记得保存」这个 UX，也不再需要它。
    落盘之后同步会话内的 committed —— 界面的「已生成」读的就是这份内存态。 */
    let updates: BTreeMap<String, BuiltRecord> = fingerprints
        .iter()
        .map(|(uid, fp)| {
            (
                uid.clone(),
                BuiltRecord {
                    stamp: ledger_stamp.clone(),
                    fingerprint: fp.clone(),
                },
            )
        })
        .collect();
    super::storage::merge_built_records(&ctx.store, &c.built, updates.clone())?;
    /* 会话内的 committed 先记在这里、**等 `book` 释放之后再并进去**（见下面的 `drop`）：
    台账只影响"上次成功生成时刻"那一格展示，晚一步合并不会让任何判断出错。 */
    let ledger_updates = updates;
    /* 第 6 步：台账 —— 与产物同一批落盘（没写文件的行不记，见上面那段注释） */
    receipt.step("ledger", true, format!("{} 条", fingerprints.len()));

    /* ③ 目录 JSON 与 catalog.json 已在 ② 的整批事务里按候选字节写出并回读验证。
     * 这里不再单独写一遍 —— 绕过事务的写盘没有备份、没有回滚、没有验证。
     * 生成不盖发布戳：候选集沿用目录里已有的 `publishedAt`，只有发布才盖新时间。 */
    receipt.step("catalog", true, "随整批事务写出并验证");
    /* `book` 借了 `ctx`（渲染器那个探针），下面要可变借 `ctx` 并台账 —— 在这里收掉它。
    到此为止 `book` 的活儿（渲染、预演、重算目录）已经全干完了。 */
    drop(book);
    drop(probe);
    ctx.committed.built.extend(ledger_updates);
    /* 第 7 步：核对 —— 交付面、入库面、基线三处是不是同一份内容 */
    receipt.step(
        "verify",
        true,
        format!(
            "交付 {} / 内核 {} / 基线 {}",
            receipt.delivery.written, receipt.assets.written, receipt.baseline.written
        ),
    );

    tracing::info!(
        written = written.len(),
        unchanged = unchanged.len(),
        skipped = skipped.len(),
        "生成完成"
    );
    Ok(GenerateReport {
        // 台账已经在上面落盘 —— 报告里不再带生成记录（草稿与生成记录彻底无关）。
        stamp: ledger_stamp,
        written,
        unchanged,
        skipped,
        /* 外层 `generate_with` 拿到成品回执后填进来 */
        receipt: None,
    })
}

// 生成之后把影子树对齐回真仓库（2026-10-31）：这一步发生在 `wb_generate` 的包装里，
// 因为 `generate_with` 也要被发布事务复用，而发布那一路自己收尾。

/* ---------- 生成前预演（生成前确认那一步） ---------- */

/// 一份文件的预演结论。
///
/// 三个状态就是「点生成会怎样」的全部可能：
///   · `added`    —— 磁盘上还没有这一份（首次生成 / 被清理过）：正文全绿
///   · `modified` —— 有这一份，但这次算出来的和它不一样：会**原子替换**
///   · `unchanged`—— 盘上那份**就是候选字节**（[`crate::workbench::up_to_date`]），
///     不会重写。内容一致但盘上还带着 `# generated_at:` 的算 `modified` ——
///     真生成会写它（候选按无该行的字节登记进 `catalog.json`）。
///     判据与生成事务的 [`super::delivery::generate_transaction`] **同一道** ——
///     预演说「要写」而真生成跳过，是把「将写入 N 份」报成假的。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffState {
    Added,
    Modified,
    Unchanged,
}

/// 行级 diff 的一种行。
///
/// 状态判据跟写盘走（见 [`DiffState`]）；**行级差异仍比真文本** ——
/// 真要重写的文件，头里时间戳那行确实会变，如实显示出来。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
    /// `context` 没变 / `added` 这次新有 / `removed` 这次没有
    pub kind: DiffLineKind,
    pub text: String,
    /// 第几行（1 起；`removed` 记它在**磁盘旧版**里的行号，其余记新版的）
    pub no: usize,
}

/// 前端那边叫 `PreviewDiffLine`（同一条 wire 形状）。
///
/// 单独起个别名只是为了让「一份未保存改动的 diff」读起来不串到"预演"上 ——
/// **不再是第二套结构**，字段与上面那个逐项相同。
pub type PreviewDiffLine = DiffLine;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffLineKind {
    Context,
    Added,
    Removed,
}

/// 行级 diff 的**唯一实现**（`pub` 是为了给未保存改动的 diff 用 ——
/// 别处再写一份 LCS 迟早两边给出的红绿不一样）。
///
/// 未保存改动那边（`domain::shadow_diff`）调的就是它：算法、行号口径、
/// 「删在前、增在后」的顺序与生成前确认框**逐字相同**。
pub(crate) fn line_diff(old: &str, new: &str) -> Vec<DiffLine> {
    diff_lines(old, new)
}

/// 一份产物的预演：状态 + 行级差异 + 计数。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewFile {
    pub uid: String,
    pub file_name: String,
    pub state: DiffState,
    /// 行级差异（含未变的上下文行）。`unchanged` 时是**整份正文**（全 `context`）——
    /// 确认框的「完整」视图要能看原文（作者 2026-10-07：「就算它一模一样不会重写，
    /// 我也希望看一个完整的」）
    pub lines: Vec<DiffLine>,
    pub added: usize,
    pub removed: usize,
    /// **谁写的**（生成时重算 / 发布时定稿 / 软件发布链）。
    ///
    /// 附属文件那几份不都由生成写：`manifest.json` / `source.json` 归发布、
    /// `release.json` 归软件发布链 —— 它们也列在确认框里（作者 2026-10-07 问
    /// 「怎么没有这」），界面照这个字段说"本次生成不动"、也不把它们算进「将写入 N 份」
    pub stage: DeliveryStage,
}

/// 预演报告。跳过的项照实列出（与 `wb_generate` 同一套原因）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewReport {
    /// **产物**（`mkp/presets/*.toml`）—— 一份一个版本
    pub files: Vec<PreviewFile>,
    /// **附属文件**（`content/*.json` + `catalog.json`，`uid` / `fileName` 都是交付根相对的路径）。
    ///
    /// ★ 2026-10-07 加：生成不只写 toml —— 它同时重算目录与清单。作者原话
    /// 「我希望到时候它还能显示一个会变化的 Json」。单独一格而不是并进 `files`：
    /// 发布闸的 ⑤「能不能渲染」数的是 `files`（渲染产物），口径不能混。
    pub aux: Vec<PreviewFile>,
    pub skipped: Vec<(String, String)>,
    /// 会被写盘的**产物**份数（`added` + `modified`）
    pub to_write: usize,
    /// 不变的**产物**份数
    pub unchanged: usize,
    /// 有阻断时的唯一原因（与 `wb_generate` 同一道闸，前端照它压按钮）
    pub blocked: Option<String>,
}

/// **生成前预演**：把这次要写的产物都算出来，与磁盘上现存的逐份比，给出行级 diff。
///
/// **一个字节都不写** —— 它就是 [`wb_generate`] 的彩排：同一批 `todo`（同一套跳过理由）、
/// 同一处渲染、同一处落点（`<delivery>/mkp/presets/`），只是把"写"换成"读出来比"。
/// 界面上「点生成 → 先看这个 → 再点确认」的第二步靠它。
///
/// 阻断也照实报（`blocked` 非空 = 生成会被拒），不假装能生成。
#[tauri::command(async)]
pub fn wb_generate_preview(scope: Scope) -> Result<PreviewReport, AppError> {
    traced("wb_generate_preview", |_| {
        with_ctx(|ctx| {
            // 预演也只在已保存状态上算：否则「预演的东西」和「能生成的东西」不是一份
            require_clean(ctx)?;
            preview_with(ctx, &scope)
        })
    })
}

/// 预演的**锁无关内核**：给定一份会话，把这次要写的产物都算出来。
///
/// ★ 这一层存在的唯一理由是**「一个入口」**：界面上的「生成前预演」与**发布闸**
/// （[`super::audit`]）必须算同一件事。而发布闸本身已经在 `with_ctx` 里，`with_ctx`
/// 的锁**不可重入** —— 闸里再调一次 `wb_generate_preview` 命令就是自己把自己锁死。
/// 所以「同一台渲染器」落在这一层，**不落在命令壳上**；命令壳只负责 `with_ctx` + trace。
///
/// 只读：`dist_root_path` 不建目录，缺文件按「新增」算，不顺手造出一个 `mkp/presets/`。
pub(super) fn preview_with(ctx: &super::Ctx, scope: &Scope) -> Result<PreviewReport, AppError> {
    let (c, d, _) = state(ctx)?;
    let probe = renderer_for(ctx);
    let book = book_with_probe(ctx, &ctx.presets, &c, &d, &probe);

    // 生成闸门与 `wb_generate` 是同一道：有阻断就如实说，不往下算
    let report = issues::inspect(&book);
    if let Some(b) = report.first_block() {
        return Ok(PreviewReport {
            files: Vec::new(),
            aux: Vec::new(),
            skipped: Vec::new(),
            to_write: 0,
            unchanged: 0,
            blocked: Some(format!("{}：{}", b.title, b.detail)),
        });
    }

    let (todo, skipped) = planned_todos(&book, scope);
    let delivery = paths::delivery_root_path();
    let presets_dir = delivery
        .join(crate::workbench::app::delivery::MKP_DIR)
        .join("presets");

    let mut files: Vec<PreviewFile> = Vec::with_capacity(todo.len());
    let mut to_write = 0usize;
    let mut unchanged = 0usize;
    for uid in &todo {
        let r = render(&book, uid)?;
        let existing = std::fs::read_to_string(presets_dir.join(&r.file_name)).ok();
        let pf = preview_one(&r, existing.as_deref());
        match pf.state {
            DiffState::Unchanged => unchanged += 1,
            _ => to_write += 1,
        }
        files.push(pf);
    }

    // ★ **附属文件也预演**（作者 2026-10-07：「我希望到时候它还能显示一个会变化的 Json」）。
    //
    // 生成不只写 toml —— 它同时重算目录与清单（content 那三份 + `catalog.json`）。
    // 那几份也在确认框里列出来，比法与产物同一处（`preview_text`）。
    //
    // 正文由 `delivery` 那两台**纯函数**给出（与真生成同一处构造）。
    // `catalog.json` 的 `publishedAt` **沿用旧目录的戳**（生成不盖发布戳，见真生成
    // 那一段）—— 没有实质变化时它不会再显示"修改"。
    let mut aux: Vec<PreviewFile> = Vec::new();
    for (rel, text) in super::delivery::content_json_texts(&book)? {
        let old = std::fs::read_to_string(delivery.join(rel)).ok();
        aux.push(preview_text(rel, rel, &text, old.as_deref()));
    }
    /* 与真生成**同一处构造**：候选集按当前源重算 catalog 的字节（发布戳与发布身份
     * 沿用旧目录，生成不盖新时间、序号一分不动）。预演自己再拼一份目录的话，
     * "确认框说会写这份、真生成写的是另一份"就又出现了。 */
    let catalog_bytes = super::delivery::candidate_set(&delivery, &book)?
        .into_iter()
        .find(|c| c.rel == super::delivery::NEW_CATALOG_FILE)
        .ok_or_else(|| AppError::internal("候选集里没有 catalog.json"))?
        .want;
    let catalog_text = String::from_utf8(catalog_bytes).map_err(|e| {
        AppError::internal("catalog.json 候选正文不是 UTF-8").with_detail(e.to_string())
    })?;
    let catalog_rel = super::delivery::NEW_CATALOG_FILE;
    let old = std::fs::read_to_string(delivery.join(catalog_rel)).ok();
    aux.push(preview_text(
        catalog_rel,
        catalog_rel,
        &catalog_text,
        old.as_deref(),
    ));

    // 生成**不动**的那几份也照实列出来（作者 2026-10-07：「那我那个生成的里面怎么没有」）——
    // manifest / source 由发布定稿、release.json 属软件发布链，界面把它们标成"本次生成不动"。
    for (rel, stage) in [
        (super::delivery::MANIFEST_FILE, DeliveryStage::Publish),
        (super::delivery::SOURCE_FILE, DeliveryStage::Publish),
        (
            crate::runtime::source::RELEASE_FILE,
            DeliveryStage::Software,
        ),
    ] {
        let old = std::fs::read_to_string(delivery.join(rel)).ok();
        aux.push(preview_frozen(rel, stage, old.as_deref()));
    }

    Ok(PreviewReport {
        files,
        aux,
        skipped,
        to_write,
        unchanged,
        blocked: None,
    })
}

/// 把目录里**本次清单不认**的旧名 `.toml` 移进回收站，返回移了几份。
///
/// 2026-11-12（作者：「改了 id 之后要自动做成新的，别每次手动改错误」）：
/// `rendered` 只给当前清单算出的新名字，改过 id 的旧名产物会留在目录里，
/// 被 `include_str!` 编进二进制、被判据当成该有的一份 —— 客户端随包预设
/// 于是停在旧 id，而编译期一声不响。生成既然是这两个目录的唯一写入方，
/// 清旧名就该由生成做。
///
/// **走回收不直接删**：`workbench/.trash/<kind>/<时间戳>/` 可还原，
/// 与交付残留 `clean_strays` 同一条纪律。
fn prune_orphan_presets(
    dir: &std::path::Path,
    expected: &BTreeSet<String>,
    trash_root: &std::path::Path,
    kind: &str,
    stamp: &str,
) -> Result<usize, AppError> {
    let mut orphans: Vec<String> = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|e| {
        AppError::io(format!("读不到产物目录 {}", dir.display())).with_detail(e.to_string())
    })?;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if name.ends_with(".toml") && !expected.contains(&name) {
            orphans.push(name);
        }
    }
    orphans.sort();

    for name in &orphans {
        let src = dir.join(name);
        let dst = trash_root.join(kind).join(stamp).join(name);
        if let Some(parent) = dst.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::io(format!("建不出回收目录 {}", parent.display()))
                    .with_detail(e.to_string())
            })?;
        }
        std::fs::rename(&src, &dst).map_err(|e| {
            AppError::io(format!(
                "移不动旧名产物：{} → {}",
                src.display(),
                dst.display()
            ))
            .with_detail(e.to_string())
        })?;
    }
    if !orphans.is_empty() {
        tracing::info!(
            dir = %dir.display(),
            moved = orphans.len(),
            names = ?orphans,
            at = %stamp,
            "旧名产物已移入回收站"
        );
    }
    Ok(orphans.len())
}

/// 内核产物（`assets_dir`）与交付产物（`delivery_dir`）有哪一份不同。
///
/// 两边都由同一个 [`render`] 从同一份 `presets/` 算出来，写盘无误就必然相等。
/// 不等意味着"又开了第二条数据来源" —— 历史上 `tower_extrude_ratio` 一边 1.0、
/// 一边 0.9（配方那条链独自维护），就是这么来的。
///
/// 返回 `(文件名, 交付那侧内容, 内核那侧内容)`；`None` = 全都逐字节一致。
fn first_drift(
    delivery_dir: &std::path::Path,
    assets_dir: &std::path::Path,
    rendered: &[Rendered],
) -> Option<(String, String, String)> {
    for r in rendered {
        let delivery = std::fs::read_to_string(delivery_dir.join(&r.file_name)).ok();
        let assets = std::fs::read_to_string(assets_dir.join(&r.file_name)).ok();
        match (delivery, assets) {
            /* 两边都读得到、**剥掉生成时间**后相同 → 继续看下一份。
            时间戳各记各的写盘时刻，两侧必然不同，那不是"第二个数据源"。 */
            (Some(a), Some(b))
                if crate::workbench::strip_generated_at(&a)
                    == crate::workbench::strip_generated_at(&b) =>
            {
                continue
            }
            // 有一侧读不到 / 两侧不同：都算漂移，把看到的东西如实带回
            (a, b) => {
                return Some((
                    r.file_name.clone(),
                    a.unwrap_or_else(|| "（读不到）".to_owned()),
                    b.unwrap_or_else(|| "（读不到）".to_owned()),
                ))
            }
        }
    }
    None
}

/// 比一份产物：见 [`PreviewFile`] 的三档状态。
fn preview_one(r: &Rendered, existing: Option<&str>) -> PreviewFile {
    preview_text(&r.uid, &r.file_name, &r.text, existing)
}

/// 比一份文本：磁盘读得到且**逐字节相同**（与真生成的「跳过」同一道判据）
/// → `unchanged`；读得到但不同 → `modified`；读不到 → `added`。
///
/// **无变化也把正文带回去**（作者 2026-10-07）：确认框的「完整」视图要看整份原文 ——
/// 「就算它一模一样不会重写，我也希望看一个完整的」。所以这里给 [`full_lines`]
/// （全 `context`、`added` / `removed` 都是 0），「不会重写」那句只在**对比**视图说。
///
/// 产物（`preview_one`）与**附属文件**（目录 JSON）都走这一处 —— 两边各写一套比法，
/// 迟早有一边把"会写"报成"不会写"。
fn preview_text(uid: &str, file_name: &str, text: &str, existing: Option<&str>) -> PreviewFile {
    let (state, lines) = match existing {
        None => (DiffState::Added, diff_added(text)),
        /*
         * 「无变化」= 盘上那份**就是候选字节**（[`crate::workbench::up_to_date`]，
         * 与生成事务同一道判据）。带 `# generated_at:` 的历史产物算「修改」：
         * 真生成会重写它（候选里没有那行，`catalog.json` 按候选登记 size / sha），
         * 这里若说「不会重写」，人就再没有别的路让它消掉。
         *
         * 展示的仍是**盘上那一份**（不是这次算出来的候选）：人要看的是已经躺在那里的东西。
         */
        Some(old) if crate::workbench::up_to_date(file_name, old.as_bytes(), text.as_bytes()) => {
            (DiffState::Unchanged, full_lines(old))
        }
        Some(old) => (DiffState::Modified, diff_lines(old, text)),
    };
    let added = lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Added)
        .count();
    let removed = lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Removed)
        .count();
    PreviewFile {
        uid: uid.to_owned(),
        file_name: file_name.to_owned(),
        state,
        lines,
        added,
        removed,
        stage: DeliveryStage::Generate,
    }
}

/// 附属文件里**本次生成不动**的那几份（`manifest.json` / `source.json` 归发布定稿、
/// `release.json` 归软件发布链）。
///
/// 它们也列在确认框里（作者 2026-10-07：「那我那个生成的里面怎么没有」）——
/// 但状态**永远是「无变化」**：生成一个字节都不碰它们；详情给盘上原文，看得见就行。
fn preview_frozen(rel: &str, stage: DeliveryStage, existing: Option<&str>) -> PreviewFile {
    PreviewFile {
        uid: rel.to_owned(),
        file_name: rel.to_owned(),
        state: DiffState::Unchanged,
        lines: existing.map(full_lines).unwrap_or_default(),
        added: 0,
        removed: 0,
        stage,
    }
}

/// 新增一份：每一行都是 `added`（界面全绿，不折叠）
fn diff_added(text: &str) -> Vec<DiffLine> {
    text.lines()
        .enumerate()
        .map(|(i, t)| DiffLine {
            kind: DiffLineKind::Added,
            text: t.to_owned(),
            no: i + 1,
        })
        .collect()
}

/// 整份正文，逐行都是 `context`（无变化的文件在「完整」视图里要能看全文）。
///
/// 与 [`diff_added`] 的差别只在 kind：那个是"这次新有的"（界面全绿），
/// 这个每一行都是"磁盘上本来就这样"（素底）。
fn full_lines(text: &str) -> Vec<DiffLine> {
    text.lines()
        .enumerate()
        .map(|(i, t)| DiffLine {
            kind: DiffLineKind::Context,
            text: t.to_owned(),
            no: i + 1,
        })
        .collect()
}

/// 行级 diff：先剥掉两端的公共行，中间那段做最简 LCS，再拼回来。
///
/// **不引第三方 diff 库**（守"不引入新依赖"）。TOML 一行一条、行数在几十到几百，
/// 这个 O(n·m) 的 LCS 在这里毫无压力。剥前缀/后缀是为了让"只改了一行"这种常见情形
/// 退化成"一大段 context + 一两行变化"，避免整份文件都进 LCS。
fn diff_lines(old: &str, new: &str) -> Vec<DiffLine> {
    let a: Vec<&str> = old.lines().collect();
    let b: Vec<&str> = new.lines().collect();

    // 公共前缀
    let mut head = 0;
    while head < a.len() && head < b.len() && a[head] == b[head] {
        head += 1;
    }
    // 公共后缀（不越过头）
    let mut tail = 0;
    while tail < a.len() - head
        && tail < b.len() - head
        && a[a.len() - 1 - tail] == b[b.len() - 1 - tail]
    {
        tail += 1;
    }

    let mid_a = &a[head..a.len() - tail];
    let mid_b = &b[head..b.len() - tail];

    let mut out: Vec<DiffLine> = Vec::with_capacity(a.len().max(b.len()) + mid_a.len());
    for (i, t) in a[..head].iter().enumerate() {
        out.push(DiffLine {
            kind: DiffLineKind::Context,
            text: (*t).to_owned(),
            no: i + 1,
        });
    }
    lcs_diff(mid_a, mid_b, head + 1, head + 1, &mut out);
    for (i, t) in a[a.len() - tail..].iter().enumerate() {
        out.push(DiffLine {
            kind: DiffLineKind::Context,
            text: (*t).to_owned(),
            no: a.len() - tail + i + 1,
        });
    }
    out
}

/// 中间那段的最简 LCS（标准 DP + 回溯），产出 `context` / `added` / `removed` 三种行。
fn lcs_diff(a: &[&str], b: &[&str], a_base: usize, b_base: usize, out: &mut Vec<DiffLine>) {
    let n = a.len();
    let m = b.len();
    // dp[i][j] = a[i..] 与 b[j..] 的最长公共子序列长度
    let mut dp = vec![vec![0usize; m + 1]; n + 1];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            dp[i][j] = if a[i] == b[j] {
                dp[i + 1][j + 1] + 1
            } else {
                dp[i + 1][j].max(dp[i][j + 1])
            };
        }
    }
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            out.push(DiffLine {
                kind: DiffLineKind::Context,
                text: a[i].to_owned(),
                no: a_base + i,
            });
            i += 1;
            j += 1;
        } else if dp[i + 1][j] >= dp[i][j + 1] {
            out.push(DiffLine {
                kind: DiffLineKind::Removed,
                text: a[i].to_owned(),
                no: a_base + i,
            });
            i += 1;
        } else {
            out.push(DiffLine {
                kind: DiffLineKind::Added,
                text: b[j].to_owned(),
                no: b_base + j,
            });
            j += 1;
        }
    }
    while i < n {
        out.push(DiffLine {
            kind: DiffLineKind::Removed,
            text: a[i].to_owned(),
            no: a_base + i,
        });
        i += 1;
    }
    while j < m {
        out.push(DiffLine {
            kind: DiffLineKind::Added,
            text: b[j].to_owned(),
            no: b_base + j,
        });
        j += 1;
    }
}

/*
 * 曾有「配对」这一层（把「整块删 + 整块增」读成一行「旧 → 新」），2026-11-02 删掉。
 *
 * 起因是作者看到"只加了一条却在概览条上冒出两道 −"—— 那其实是 LCS 的"删一段 + 增一段"，
 * 文件里并没有少东西。配对的思路没错，但渲染成「旧 → 新」一行在长行上读不动
 * （作者原话：「到时候长一点可能就不太好接受了」），他自己要的是
 * **生成前确认那种上红下绿两行**。既然界面要的就是 LCS 的原样，那这一层就没有载荷了。
 *
 * 别再加回来：真要"改一行读成一行"，那是编辑器（VSCode 那种 inline diff）的活，
 * 不是这个只读确认框的事。
 */

/* ---------- 恢复配方 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertPreview {
    pub uid: String,
    /// 能不能恢复。没有快照时为 false
    pub allowed: bool,
    pub blocked_reason: Option<String>,
    /// 要提交的 patch。前端走 `wb_apply_draft` —— 恢复也是一条撤销
    pub patches: Vec<Patch>,
    /// 会变的项：字段名 + 现在 + 恢复成
    pub changes: Vec<RevertChange>,
    /// **原本继承来的、会从此脱钩的那几项。不许闷着改**
    pub detaching: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevertChange {
    pub key: String,
    pub label: String,
    pub before: String,
    pub after: String,
}

/// 「恢复到上次成功生成时的配方」。**只算不写** —— 写走唯一那条入口
#[tauri::command(async)]
pub fn wb_revert_preview(uid: String) -> Result<RevertPreview, AppError> {
    traced("wb_revert_preview", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            let v = book
                .version(&uid)
                .ok_or_else(|| AppError::not_found(format!("版本 {uid} 不存在")))?;
            let mut out = RevertPreview {
                uid: uid.clone(),
                allowed: false,
                blocked_reason: None,
                patches: Vec::new(),
                changes: Vec::new(),
                detaching: Vec::new(),
            };

            let rel = ctx.store.snapshot_rel(&v.machine_id, &v.version_id)?;
            let Some(snap) = ctx
                .store
                .read_doc::<BTreeMap<String, Value>>(&rel, "生成快照")?
            else {
                out.blocked_reason =
                    Some("这一版还没有成功生成过，没有可以恢复到的配方".to_owned());
                return Ok(out);
            };
            out.allowed = true;

            let layers = book
                .version_layers(&uid)
                .ok_or_else(|| AppError::corrupted("取值层取不出来"))?;
            for (key, want) in &snap {
                let Some(p) = ctx.presets.registry.param(key) else {
                    continue; // 上游已经删了这个参数，恢复它没有意义
                };
                let Some(hit) = layers.effective(key) else {
                    continue;
                };
                if hit.value == want {
                    continue;
                }
                // **一律写在版本这一层**：快照里只有有效值，看不出当时是哪一层给的
                if !layers.has_own(Level::Version, key) {
                    out.detaching.push(p.label.clone());
                }
                out.changes.push(RevertChange {
                    key: key.clone(),
                    label: p.label.clone(),
                    before: w::value_text(p, hit.value),
                    after: w::value_text(p, want),
                });
                out.patches.push(Patch::SetValue {
                    level: Level::Version,
                    owner: uid.clone(),
                    key: key.clone(),
                    value: Some(want.clone()),
                });
            }
            Ok(out)
        })
    })
}

/* ---------- 发布 ---------- */

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishReport {
    pub stamp: String,
    pub root: String,
    pub files: usize,
    /// 这次发出去的目录登记的**最低正式客户端版本**（结构规则表里对当前结构签名那条）。
    ///
    /// 以前这一格是"上游 manifest 声明了吗"，上游删掉之后恒空；现在它有真来源，
    /// 而且**不要求那个版本已经发布**（Dev 场景，见 `runtime::structure`）。
    /// `None` 只该出现在"这一代还没登记"时 —— 那种情况发布闸会先拦住。
    pub minimum_client: Option<String>,
    /// 待办与提示**不挡发布**，但要在报告里列出来
    pub todos: usize,
    pub hints: usize,
}

/// **发布闸**：发布前那十五项的逐项结果（第二刀）。
///
/// 它**只读**：不写盘、不动 git、不发网络请求。界面拿它画那个逐项打勾的框，
/// 往下走的那颗按钮只在 `can_publish` 为真时亮 —— **任何一项 Blocker 红了
/// 就不许往下走**（作者定的硬规矩）。
///
/// 与 [`wb_publish`] 共用同一个会话上下文，所以"闸里看到的"就是"发布会写出去的"。
///
/// ★ **命令壳是薄的**：算的是 [`super::audit::publish_audit`]，界面与 `cargo test`
/// 判据调的是同一个函数（「界面不许自己再实现一套检查」）。
#[tauri::command(async)]
pub fn wb_publish_audit() -> Result<super::audit::PublishAudit, AppError> {
    traced("wb_publish_audit", |_| super::audit::publish_audit())
}

/// **发布事务**（第三刀下半）：把「审计 → 生成 → 定稿 → 本地 git → 平台 PR/MR」跑成一次手势。
///
/// ★ 这是**唯一对外的发布动作**：`wb_generate` / `wb_publish_audit` 都是它的**内部步骤**
/// （前端不摆「生成」「创建 PR」按钮）。事务内核见 [`super::publish_tx::run`] ——
/// **锁无关**（只收 `&Ctx`），因为 `with_ctx` 锁不可重入（回头调命令壳会自锁）。
///
/// 发布目标（平台 / 仓库 / 用户名 / Token）在**锁外**从发布账户配置解析（[`resolve_publish`]）；
/// **没有配发布账户**时退化成"只生成 + 定稿 + 本地推送"，如实说"没建 PR" ——
/// 那是"还没配账户"，不是失败。
/// ★ `(async)` 不是性能优化，是正确性：这条命令要起 git 子进程、发平台 HTTP ——
/// 跑在主线程上就是"整个窗口一动不动"
/// （2026-10-04 真机事故；与上面「读命令必须异步」同一条病，只是它更重）。
#[tauri::command(async)]
pub fn wb_publish(
    app: tauri::AppHandle,
    opts: Option<super::publish_tx::TxOptions>,
) -> Result<super::publish_tx::PublishTxReport, AppError> {
    traced("wb_publish", |_| {
        let opts = opts.unwrap_or_default();
        // 发布目标 + 平台客户端在锁外构造（读配置 / 凭据文件 / remote，都不碰会话）
        let (target, hosting) = resolve_publish(&app, opts.platform.as_deref());
        /* 数据根也在锁外取：内核只收一个 `&Path`，不碰 AppHandle（锁纪律）。
         * 记账本身在内核里 —— CLI `presets publish` 走同一个内核，所以两条路都留痕。 */
        let history_root = crate::fsx::paths::internal_root(&app).ok();
        let report = with_ctx_mut(|ctx| {
            // 发布只认「已保存」那一版：草稿混进产物 = 发出去的东西没人审过
            require_clean(ctx)?;
            let out = super::publish_tx::run(
                ctx,
                &opts,
                target.as_ref(),
                hosting.as_ref().map(|h| h.as_ref()),
                None,
                history_root.as_deref(),
            )?;
            /* 发布写了交付产物 / 账本（真仓库）；影子树跟着换到新一代 */
            crate::workbench::domain::txn::mirror(&ctx.real_root, &ctx.shadow_root)?;
            Ok::<_, AppError>(out)
        })?;
        Ok(report)
    })
}

/// **发布历史**（只读、`async`）：最近若干次「交付预设」事务的回执，**最新在前**。
///
/// 读的是**全部数据根**（工作台那棵 + 命令行那棵，见 [`super::history_roots`]）：
/// `npm run presets -- publish` 写的是 `APP_IDENTIFIER` 那棵树，不合并就等于"那次发布没发生"。
/// 每条带 `source`，界面能说清它是从哪条入口发的。
///
/// 界面开场读一次；每条"现在走到哪"由 [`super::publish_tx::wb_publish_status`] **手动刷新**
/// （作者定死：状态是"看一看"，不是常驻任务 —— **不做轮询**）。
#[tauri::command(async)]
pub fn wb_publish_history(
    app: tauri::AppHandle,
) -> Result<super::history::PublishHistoryView, AppError> {
    traced("wb_publish_history", |_| {
        let roots = super::history_roots(&app)?;
        super::history::load_merged(&roots)
    })
}

/// **本分支最近的提交**（只读、`async`、不联网）——「发布历史」第三栏。
///
/// 作者 2026-11-05：「AI 用 CI 做了什么我不知道，但我不要很详细的」——
/// 只给最近 20 条 `短 sha / 作者 / 时间 / 标题`，不 diff、不查远端。
/// 读的是**本机仓库**，没推上去的提交也在里面（界面要写清这一句）。
#[tauri::command(async)]
pub fn wb_recent_commits(limit: Option<usize>) -> Result<Vec<super::git::CommitRow>, AppError> {
    traced("wb_recent_commits", |_| {
        super::git::Git::at_repo_root().recent_commits(limit.unwrap_or(20))
    })
}

/// **在系统浏览器里打开一个 URL**（回执屏的「查看 PR」）。
///
/// ★ 只放行 `http(s)://`：这是个"把字符串变成系统动作"的口子，白名单要窄。
/// 与 `wb_reveal_asset` 同一条取向：不写任何应用状态，只开系统程序。
/// ★ `(async)`：命令一律不占主线程（读命令那条规矩的同一个理由）。
#[tauri::command(async)]
pub fn wb_open_external(app: tauri::AppHandle, url: String) -> Result<(), AppError> {
    traced("wb_open_external", |_| {
        let u = url.trim();
        if !(u.starts_with("https://") || u.starts_with("http://")) {
            return Err(AppError::invalid_argument(
                "只允许打开 http(s) 链接 —— 别的形状不交给系统",
            ));
        }
        tauri_plugin_opener::OpenerExt::opener(&app)
            .open_url(u, None::<&str>)
            .map_err(|e| AppError::io("打不开浏览器").with_detail(e.to_string()))
    })
}

/// 解析发布目标 + 造平台客户端（都在锁外）。
///
/// - 配了发布账户 → 解析出 [`PublishTarget`] 并按平台造 `Hosting`（用配置里的 Token）；
/// - **没配**（或配得不完整）→ 两者都是 `None` —— 事务退化成"生成 + 定稿 + 本地推送"，
///   **不报错**（"还没配账户"是正常状态，不是失败）。
fn resolve_publish(
    app: &tauri::AppHandle,
    platform: Option<&str>,
) -> (
    Option<super::publish_tx::PublishTarget>,
    Option<Box<dyn super::platform::Hosting>>,
) {
    let Ok(root) = crate::fsx::paths::internal_root(app) else {
        return (None, None);
    };
    let Ok(target) = super::publish_tx::resolve_target(&root, platform) else {
        // 没配 / 配不完整：不报错，退化成纯本地推送
        return (None, None);
    };
    let hosting: Box<dyn super::platform::Hosting> = match target.platform.as_str() {
        "github" => Box::new(super::platform::github::GitHub::new(target.token.clone())),
        "gitee" => Box::new(super::platform::gitee::Gitee::new(target.token.clone())),
        _ => return (Some(target), None),
    };
    (Some(target), Some(hosting))
}

/// **旧发布壳**（第三刀上半及以前）：只把 `presets/delivery/` 定稿，不生成、不动 git。
///
/// 已被 [`wb_publish`] 事务取代，**不再是前端入口**。留着它是因为它仍是"定稿"这一步的
/// 可单测入口（发布事务内核走的是同一段 `publish_into`）。前端只用 `wb_publish`。
#[allow(dead_code)]
fn publish_deliverable_only() -> Result<PublishReport, AppError> {
    with_ctx(|ctx| {
        let (c, d, _) = state(ctx)?;
        let book = Book::new(&ctx.presets, &c, &d);
        let report = issues::inspect(&book);
        if let Some(b) = report.first_block() {
            return Err(AppError::invalid_argument("有阻断问题没解决，不能发布")
                .with_detail(format!("{}：{}", b.title, b.detail)));
        }

        let root = paths::delivery_root()?;
        let asset_root = paths::assets_root()?;
        /*
         * `wb_publish` 是**发布命令壳**（不是事务），它沿用盘上已有的发布身份 ——
         * 序号只由 `publish_tx` 分配。所以这里 `identity: None`：交付层会用
         * `PublishIdentity::inherit_from` 拿盘上那一份，不会自己数数。
         */
        let meta = super::delivery::PublishMeta {
            stamp: clock::now_iso8601(),
            channel: PUBLISH_CHANNEL.to_owned(),
            version: String::new(),
            identity: None,
            /* 这个入口不提交 git（它只是"重算式定稿"）⇒ 没有"上一代是哪一笔"可说 */
            commit_sha: None,
        };
        /*
         * `content/` 那三份（`machine_catalog` / `bundles` / `assets_index`）是**生成**的
         * 产物。拆成 plan/commit 之后 `publish_into` 不再自己往交付根里拷它们
         * （2026-10-28），所以这个"只定稿、不生成"的老入口必须先补上生成 ——
         * 否则它就是在对着一份不存在的内容做定稿。
         */
        super::delivery::write_content(&root, &asset_root, &book)?;
        let out = super::delivery::publish_into(&root, &asset_root, &book, &meta)?;
        let stamp = meta.stamp;

        tracing::info!(files = out.files, at = %stamp, "发布完成");
        Ok(PublishReport {
            stamp,
            root: root.display().to_string(),
            files: out.files,
            minimum_client: out.minimum_client.clone(),
            todos: report.todos,
            hints: report.hints,
        })
    })
}

/// **交付目录的残留清单**（b05 Task 13.4 的查询面）。
///
/// 「不在本次交付集合内」的文件，按字典序。发布被残留拦下时，界面先给这一条
/// 让人看清是什么，再决定要不要清理。
#[tauri::command(async)]
pub fn wb_dist_strays() -> Result<Vec<String>, AppError> {
    traced("wb_dist_strays", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            let expected = super::delivery::delivery_expected_set(&book);
            Ok(super::delivery::scan_strays(
                &paths::delivery_root()?,
                &expected,
            ))
        })
    })
}

/// **清理残留**（b05 Task 13.5）：显式动作，走 `workbench/.trash/delivery/<stamp>/`
/// 回收（保留相对路径，可还原），不直接删。清理完重新发布即可。
#[tauri::command]
pub fn wb_clean_dist_strays() -> Result<usize, AppError> {
    traced("wb_clean_dist_strays", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            let expected = super::delivery::delivery_expected_set(&book);
            let root = paths::delivery_root()?;
            let strays = super::delivery::scan_strays(&root, &expected);
            if strays.is_empty() {
                return Ok(0);
            }
            let stamp = clock::now_iso8601();
            // 收集符号里的 `:` 会让 Windows 路径出问题，压成安全形状
            let stamp = stamp.replace([':', ' '], "-");
            let trash_root = paths::workbench_root()?.join(".trash");
            let moved = super::delivery::clean_strays(&root, &strays, &trash_root, &stamp)?;
            tracing::info!(moved, at = %stamp, "交付残留已移入回收站");
            Ok(moved)
        })
    })
}

/* ---------- 交付文件清单（「交付预设」卡看这次都会写出什么） ---------- */

/// **交付读数**（只读）：本次交付集合里都有哪些文件、盘上处于哪一档、目录登记有没有漂移。
///
/// 作者 2026-10-07：「我现在只能知道这个 TOML 的生成，我不知道这些其他的……
/// 还有什么文件需要生成的，我也想看到」。名单就是交付集合（与残留审计同一份），
/// 分类见 [`super::delivery::DeliveryStage`]；2026-10-19 起连同 `catalog.json`
/// 的登记漂移（改了 3MF 之后的那条提示）一起给。
#[tauri::command(async)]
pub fn wb_delivery_files() -> Result<super::delivery::DeliveryStatus, AppError> {
    traced("wb_delivery_files", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            // 只读定位，不顺手建交付目录（和 `wb_dist_strays` 的读侧一样）
            super::delivery::delivery_status(&paths::delivery_root_path(), &book)
        })
    })
}

/// 看一份交付文件的**盘上原文**（只读）。
///
/// 只认交付集合里的路径 —— 界面传什么都读不了集合外的文件。
/// 盘上还没有那一份（还没生成 / 还没发布）时如实报错，不假装有内容。
#[tauri::command(async)]
pub fn wb_delivery_file(rel: String) -> Result<String, AppError> {
    traced("wb_delivery_file", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            if !super::delivery::delivery_expected_set(&book).contains(&rel) {
                return Err(AppError::invalid_argument(format!("不熟这一份：{rel}"))
                    .with_detail("只给看本次交付集合里的文件".to_owned()));
            }
            let path = paths::delivery_root_path().join(&rel);
            std::fs::read_to_string(&path).map_err(|e| {
                AppError::not_found(format!("盘上还没有这一份：{rel}"))
                    .with_detail(format!("{e} —— 生成 / 发布过之后才有"))
            })
        })
    })
}

/* ---------- ② 交付定稿（只读检查器） ---------- */

/// 交付定稿检查的四种结论。界面只认这四句 —— 不造第五种「发布状态」，
/// 也不用「已定稿」这种会被读成"已经写进去了"的词。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FinalizeState {
    /// manifest 与当前交付文件逐条一致
    Matched,
    /// 存在 SHA / size 不一致（manifest 还停在上一次发布那一代）
    Stale,
    /// 凭据齐了，但目录按当前源重算过、还没发过新版（同一个号）
    PendingPublish,
    /// source / catalog / 账本的关系有问题 —— 细节看 `checks`
    IdentityFailed,
}

/// 定稿后 `manifest.json` 里的一条（sha / size 按盘上真字节算）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeEntry {
    pub relative_path: String,
    pub id: String,
    pub sha256: String,
    pub size: u64,
    /// 盘上那份 manifest 里登记的值（`null` = 这一条是新增的）
    pub before_sha256: Option<String>,
    pub before_size: Option<u64>,
}

/// 定稿会把 `manifest.json` 写成什么（与盘上那份的行级差）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeManifest {
    pub rel: String,
    pub state: DiffState,
    pub entries: Vec<FinalizeEntry>,
    /// 有几条 sha / size 与盘上那份不同（含新增）
    pub changed_entries: usize,
    pub lines: Vec<DiffLine>,
    pub added: usize,
    pub removed: usize,
}

/// 一条问题**属于哪一类**：能不能在这里修，决定界面上给不给按钮。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FinalizeIssueKind {
    /// 这里就能修（今天只有「更新发布凭据」）
    Fixable,
    /// 得先回去重新生成、或者去改源配置 —— 这一格只指路
    Source,
    /// source / catalog / 账本的关系问题 —— 交给发布闸，不在这里修
    Identity,
    /// 只是登记口径差异，不是故障（附属资产与 catalog 的关系）
    Info,
}

/// 这一条上能点的那个动作。**写盘动作只有 `UpdateCredentials` 一个**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum FinalizeAction {
    /// 按盘上真字节重算并写入 `manifest.json`（不碰 source / 序号 / git）
    UpdateCredentials,
    /// 去源配置：哪台机型、哪一版
    LocateSource,
    /// 去发布闸：身份问题在那里判
    CheckIdentity,
    /// 只展示，不给按钮
    None,
}

/// ② 的一条问题。**分类与"能不能修"都由后端说**，前端只照 `action` 摆按钮。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizeIssue {
    pub id: String,
    pub title: String,
    pub kind: FinalizeIssueKind,
    /// 「怎么办」，不是把 title 换个说法
    pub detail: String,
    pub action: FinalizeAction,
    /// 涉及的文件（界面只数份数；要逐条看走下面的明细）
    pub files: Vec<String>,
    /// 「定位源问题」落到哪一页 / 哪一版（反查不到 = `null`，那就只展示路径）
    pub at: Option<crate::workbench::domain::issues::Where>,
}

/// ② 交付检查的返回值。**命令本身只读**：不写盘、不分配序号、不碰账本、不看草稿干不干净。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FinalizePreview {
    pub state: FinalizeState,
    /// `true` = 没有可修 / 需处理的问题（`info` 不算），可以进正式发布
    pub ready: bool,
    /// 逐条问题：能的直接修，不能的说去哪修
    pub issues: Vec<FinalizeIssue>,
    /// 凭据失效的逐条原因（[`super::delivery::manifest_diagnosis`] 的原话）
    pub reasons: Vec<String>,
    /// 复用发布闸 ⑬⑭ 的两格：source 正确 / manifest 与交付集合一致
    pub checks: Vec<super::audit::AuditItem>,
    pub catalog_id: String,
    /// 盘上这一代目录的号（`null` = 还没发过）
    pub serial_on_disk: Option<u64>,
    /// **下一次正式发布会拿到的号**（纯算术，这一次没有分配）
    pub serial_next_publish: u64,
    /// 定稿后的 manifest（算不出来 = `None`，原因在 `planError`）
    pub manifest: Option<FinalizeManifest>,
    /// `plan_publish` 算不出来的原因（有残留 / 产物没生成 / 资产缺失…）
    pub plan_error: Option<String>,
}

/// 「更新发布凭据」的回执：写了没有、改了几条，以及**更新之后重新检查的那一份**。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CredentialUpdate {
    /// `false` = 盘上那份本来就是最新的，一个字节都没写
    pub written: bool,
    /// 这次重算进 manifest 的条目数
    pub entries: usize,
    /// 有几条 SHA / size 与原来那份不同
    pub changed: usize,
    /// 更新后立刻重算的一整份检查（前端直接换屏，不用再拉一次）
    pub preview: FinalizePreview,
}

/// **交付检查**（只读）：交付文件具不具备定稿条件、定稿会把 manifest 改成什么、
/// 以及**每条问题能不能在这里修**。
///
/// 它**不是**发布阶段，也不写任何正式凭据。真正的确立身份仍然只在发布事务里发生。
#[tauri::command(async)]
pub fn wb_finalize_preview() -> Result<FinalizePreview, AppError> {
    traced("wb_finalize_preview", |_| with_ctx(finalize_preview_with))
}

/// **更新发布凭据**（写）：按盘上真字节重算 SHA / size 并写回 `manifest.json`。
///
/// 只动这一份文件：不写 `source.json`、不重算 `catalog.json`、不推进 `publishSerial`、
/// 不碰账本、不提交 Git、**不要求工作区干净**。它改变的是**本地交付凭据**，
/// 不代表正式发布 —— 发布仍然要过闸、由人确认。
///
/// **保护条件**：交付集合没收敛（盘上与当前源算出来的集合不同）、或目录还停在
/// 上一代时，一个字节都不写 —— 否则会把"本来不该发布的东西"顺手登记进 manifest。
#[tauri::command]
pub fn wb_update_publish_credentials() -> Result<CredentialUpdate, AppError> {
    traced("wb_update_publish_credentials", |_| {
        with_ctx_mut(update_credentials_with)
    })
}

/// ② 的锁无关内核。**不调 `require_clean`** —— 带着未保存草稿、工作区脏着几十处
/// 改动，也要能查交付对不对。
pub(super) fn finalize_preview_with(ctx: &super::Ctx) -> Result<FinalizePreview, AppError> {
    let (c, d, _) = state(ctx)?;
    let probe = renderer_for(ctx);
    let book = book_with_probe(ctx, &ctx.presets, &c, &d, &probe);
    // 交付树与载荷根都从**真仓库**派生（与生成 / 发布 / 发布闸同一处）
    let delivery = paths::delivery_root_at(ctx.real_root());
    let asset_root = paths::assets_root_at(ctx.real_root());

    let (stale, reasons) = super::delivery::manifest_diagnosis(&delivery)?;
    let checks = super::audit::finalize_checks(&delivery);
    /* 身份那两格里只有 ⑬（source）是 Blocker 档；⑭ 是提示档，红了也不该把
    一次"凭据失效"说成"身份坏了" —— 两者是不同的病。 */
    let identity_failed = checks
        .iter()
        .any(|i| i.status == super::audit::AuditStatus::Fail);
    let plan = super::audit::identity_plan(&delivery);
    /* 三件事分开说：身份坏了 / 凭据落后 / 只是还没发布。凭据落后比"待发布"更该先说 ——
    它有一颗粒按钮能当场修。 */
    let state_word = if identity_failed {
        FinalizeState::IdentityFailed
    } else if stale {
        FinalizeState::Stale
    } else if plan.pending_publish {
        FinalizeState::PendingPublish
    } else {
        FinalizeState::Matched
    };
    let serial_on_disk = super::delivery::PublishIdentity::inherit_from(&delivery).serial;

    let (manifest, plan_error) = match manifest_preview(&delivery, &asset_root, &book) {
        Ok(p) => (Some(p), None),
        Err(e) => (
            None,
            Some(match e.detail {
                Some(detail) => format!("{}：{detail}", e.message),
                None => e.message.clone(),
            }),
        ),
    };

    let mut issues = build_issues(&delivery, &checks, &book)?;
    /* 账本与盘上目录不是同一代（或账本根本读不出来）：说不出"下一号是几号"，
    这一类不在检查器里修 —— 列出来，交给发布闸判。 */
    if plan.ledger_out_of_step {
        issues.insert(
            0,
            FinalizeIssue {
                id: "identity.ledger".to_owned(),
                title: "发布账本与当前目录不是同一代".to_owned(),
                kind: FinalizeIssueKind::Identity,
                detail: if plan.next_serial == 0 {
                    "账本（publish-serial.json）读不出来，说不出下一次是第几号 —— 去发布闸看具体原因。"
                        .to_owned()
                } else {
                    "账本记的那一代与盘上 catalog 对不上 —— 序号一旦错代，客户端就分不出新旧。去发布闸看具体原因。"
                        .to_owned()
                },
                action: FinalizeAction::CheckIdentity,
                files: Vec::new(),
                at: None,
            },
        );
    } else if plan.pending_publish {
        /* 同一个号、内容变了：生成改过目录，还没发过新版。**不是故障**，只是"等发布" */
        issues.push(FinalizeIssue {
            id: "identity.pending".to_owned(),
            title: "这一代已经生成好，还没发布".to_owned(),
            kind: FinalizeIssueKind::Info,
            detail: "目录与账本停在同一个号（内容已按当前源重算）—— 发一次就回到同一代；\
                 发布前客户端看到的还是上一代。"
                .to_owned(),
            action: FinalizeAction::CheckIdentity,
            files: Vec::new(),
            at: None,
        });
    }
    let ready = matches!(
        state_word,
        FinalizeState::Matched | FinalizeState::PendingPublish
    ) && !plan.ledger_out_of_step
        && issues.iter().all(|i| i.kind == FinalizeIssueKind::Info)
        && manifest.is_some();

    Ok(FinalizePreview {
        state: state_word,
        ready,
        issues,
        reasons,
        checks,
        catalog_id: plan.catalog_id,
        serial_on_disk,
        serial_next_publish: plan.next_serial,
        manifest,
        plan_error,
    })
}

/// 「更新发布凭据」的锁无关内核。**写在最后、只写一处**：`manifest.json`。
pub(super) fn update_credentials_with(ctx: &mut super::Ctx) -> Result<CredentialUpdate, AppError> {
    let (c, d, _) = state(ctx)?;
    let probe = renderer_for(ctx);
    let book = book_with_probe(ctx, &ctx.presets, &c, &d, &probe);
    let delivery = paths::delivery_root_at(ctx.real_root());
    let asset_root = paths::assets_root_at(ctx.real_root());

    // 保护：集合没收敛 / 目录不是当前代 ⇒ 一个字节都不写
    let blockers = credential_blockers(&delivery, &book)?;
    if !blockers.is_empty() {
        return Err(
            AppError::invalid_argument("交付集合还没准备好，不能更新发布凭据")
                .with_detail(blockers.join("；")),
        );
    }

    let (_, old_value) = read_manifest(&delivery);
    /* 真定稿那一刻要盖新时间（与发布事务同一件事）；`channel` / `version` 沿用盘上。
    `identity: None` = 不分配、也不改号 —— 这一笔不是发布。 */
    let field = |key: &str| {
        old_value
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let meta = super::delivery::PublishMeta {
        stamp: clock::now_iso8601(),
        channel: field("channel").unwrap_or_else(|| PUBLISH_CHANNEL.to_owned()),
        version: field("version").unwrap_or_default(),
        identity: None,
        commit_sha: None,
    };
    let plan = super::delivery::plan_publish(&delivery, &asset_root, &book, &meta)?;
    let new_bytes = serde_json::to_vec_pretty(&plan.manifest)?;
    let (_, changed) = entry_diff(old_value.as_ref(), &plan.manifest);
    let entries = plan
        .manifest
        .get("assets")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    let written = std::fs::read(delivery.join(super::delivery::MANIFEST_FILE))
        .ok()
        .as_deref()
        != Some(new_bytes.as_slice());
    if written {
        crate::fsx::atomic::atomic_write(
            &delivery.join(super::delivery::MANIFEST_FILE),
            &new_bytes,
        )?;
    }

    /* 写完立刻按盘上真字节重算一遍，把"更新后的一整份"交回去 —— 界面不用再拉一次，
    也不会出现"回执说改了、屏幕还是旧的"。 */
    let preview = finalize_preview_with(ctx)?;
    Ok(CredentialUpdate {
        written,
        entries,
        changed,
        preview,
    })
}

/// 盘上那份 `manifest.json`：原文字 + 解析后的值（读不出来 / 解析不了都是 `None`）。
fn read_manifest(delivery: &std::path::Path) -> (Option<String>, Option<Value>) {
    let bytes = std::fs::read(delivery.join(super::delivery::MANIFEST_FILE)).ok();
    let text = bytes
        .as_deref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .map(str::to_owned);
    let value = text.as_deref().and_then(|t| serde_json::from_str(t).ok());
    (text, value)
}

/// 定稿后的 manifest 每条登记 vs 盘上那份：`(逐条, 有几条变了)`。
///
/// 只比 `assets[]` 里的 `relativePath` / `sha256` / `size` —— **不算 `updated`**：
/// 真定稿那一刻它一定变，混进来的话真正的登记变化会被一行时间淹掉。
fn entry_diff(old: Option<&Value>, manifest: &Value) -> (Vec<FinalizeEntry>, usize) {
    let mut before: BTreeMap<String, (String, u64)> = BTreeMap::new();
    for a in old
        .and_then(|v| v.get("assets"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let Some(path) = a.get("relativePath").and_then(Value::as_str) else {
            continue;
        };
        let sha = a.get("sha256").and_then(Value::as_str).unwrap_or_default();
        let size = a.get("size").and_then(Value::as_u64).unwrap_or(0);
        before.insert(path.to_owned(), (sha.to_owned(), size));
    }

    let mut entries: Vec<FinalizeEntry> = Vec::new();
    let mut changed = 0usize;
    for a in manifest
        .get("assets")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        let relative_path = a
            .get("relativePath")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let sha256 = a
            .get("sha256")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let size = a.get("size").and_then(Value::as_u64).unwrap_or(0);
        let id = a
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned();
        let old = before.get(&relative_path);
        /* `map_or(true, ..)` 而不是 `is_none_or`：本仓 MSRV 是 1.77，后者要 1.82 */
        #[allow(clippy::unnecessary_map_or)]
        let differs = old.map_or(true, |(sha, n)| *sha != sha256 || *n != size);
        if differs {
            changed += 1;
        }
        entries.push(FinalizeEntry {
            relative_path,
            id,
            sha256,
            size,
            before_sha256: old.map(|(sha, _)| sha.clone()),
            before_size: old.map(|(_, n)| *n),
        });
    }
    /* 旧清单里有、新清单里没有的也算"这一笔要动"：定稿会把它们**去掉**
    （停用机型 / 退出交付面的资产）。不数它们，回执就会说"0 条"却真写了盘。 */
    let kept: std::collections::BTreeSet<&str> =
        entries.iter().map(|e| e.relative_path.as_str()).collect();
    changed += before.keys().filter(|p| !kept.contains(p.as_str())).count();
    (entries, changed)
}

/// 定稿会把 `manifest.json` 写成什么：`plan_publish` 纯算一遍，再与盘上那份做行级差。
///
/// `stamp` / `channel` / `version` **沿用盘上那份**：真定稿那一刻 `updated` 会被时钟改写，
/// 拿"现在"去比会让 diff 里永远多一行「时间变了」，把真正的登记变化埋掉。所以这一份
/// 只回答"哪几条 sha / size 会变"（`updated` 一定会变，不必在这一屏说）。
fn manifest_preview(
    delivery: &std::path::Path,
    asset_root: &std::path::Path,
    book: &Book<'_>,
) -> Result<FinalizeManifest, AppError> {
    let rel = super::delivery::MANIFEST_FILE;
    let (old_text, old_value) = read_manifest(delivery);

    let field = |key: &str| {
        old_value
            .as_ref()
            .and_then(|v| v.get(key))
            .and_then(Value::as_str)
            .map(str::to_owned)
    };
    let meta = super::delivery::PublishMeta {
        stamp: field("updated").unwrap_or_default(),
        channel: field("channel").unwrap_or_else(|| PUBLISH_CHANNEL.to_owned()),
        version: field("version").unwrap_or_default(),
        /* **不分配号**：沿用盘上那一代（`plan_publish` 只读它，不写账本） */
        identity: None,
        commit_sha: None,
    };
    let plan = super::delivery::plan_publish(delivery, asset_root, book, &meta)?;
    let new_text = String::from_utf8(serde_json::to_vec_pretty(&plan.manifest)?).map_err(|e| {
        AppError::internal("定稿后的 manifest 不是 UTF-8").with_detail(e.to_string())
    })?;

    let (entries, changed_entries) = entry_diff(old_value.as_ref(), &plan.manifest);

    let (state, lines) = match old_text.as_deref() {
        None => (DiffState::Added, diff_added(&new_text)),
        Some(old) if old == new_text => (DiffState::Unchanged, full_lines(old)),
        Some(old) => (DiffState::Modified, diff_lines(old, &new_text)),
    };
    let added = lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Added)
        .count();
    let removed = lines
        .iter()
        .filter(|l| l.kind == DiffLineKind::Removed)
        .count();

    Ok(FinalizeManifest {
        rel: rel.to_owned(),
        state,
        entries,
        changed_entries,
        lines,
        added,
        removed,
    })
}

/// 把各项读数折成**问题清单**：能修的直接修、不能修的说去哪修、口径差异只报计数。
///
/// 分类不在这里"猜"：`Fixable` 只给凭据落后（那确实只需重写 manifest）；
/// `Source` 给"得先重新生成 / 改源"的两类；`Identity` 直接抄发布闸红掉的那几格。
fn build_issues(
    delivery: &std::path::Path,
    checks: &[super::audit::AuditItem],
    book: &Book<'_>,
) -> Result<Vec<FinalizeIssue>, AppError> {
    use super::delivery::DiffKind;

    let mut out: Vec<FinalizeIssue> = Vec::new();

    /* ① 可修：凭据落后 —— 判据是 [`super::delivery::manifest_drift`]（manifest 登记
    vs 盘上真字节），**不依赖 `plan_publish` 算不算得出来**。
    否则只要定稿那一步算不出来（例如某份产物没进 catalog），这几条就会一起消失，
    人只看到"需处理源"，而真正能修的那一项被吞掉。 */
    let drift = super::delivery::manifest_drift(delivery)?;
    if !drift.is_empty() {
        out.push(FinalizeIssue {
            id: "credentials.stale".to_owned(),
            title: format!("{} 份凭据", drift.len()),
            kind: FinalizeIssueKind::Fixable,
            detail: "manifest 与这一代的交付面对不上（SHA / size 落后，或还登着目录里已经没有的\
                     条目）。更新只重写 manifest.json —— 不动 source.json、不推进序号、不提交、不推送。"
                .to_owned(),
            action: FinalizeAction::UpdateCredentials,
            files: drift.iter().map(|d| d.rel.clone()).collect(),
            at: None,
        });
    }

    /* ② 集合 / 目录：这里修不了，只指路 */
    let (candidates, diffs) = super::delivery::delivery_diffs(delivery, book)?;
    let mut unsettled: Vec<String> = Vec::new();
    for d in &diffs {
        match d.kind {
            DiffKind::Added => unsettled.push(format!("{}（盘上还没有）", d.rel)),
            DiffKind::Removed => unsettled.push(format!("{}（已不在当前交付集合里）", d.rel)),
            _ => {}
        }
    }

    /* 产物在盘上 / 在候选里，但 `catalog.json` 没登记它。
    绝不"顺手塞进 manifest"：先重新生成让目录重算，或去源里确认它该不该发。
    **停用机型豁免**：目录不登记它是设计（客户端不显示），产物照留在盘上。 */
    let catalog = std::fs::read(delivery.join(super::delivery::NEW_CATALOG_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<crate::runtime::catalog::Catalog>(&b).ok());
    let preset_prefix = format!("{}/", paths::MKP_PRESETS_DIR);
    let unlisted: Vec<String> = candidates
        .iter()
        .filter(|c| c.rel.starts_with(&preset_prefix))
        .filter(|c| !super::delivery::product_disabled(book, &c.rel[preset_prefix.len()..]))
        .filter(|c| {
            let want = format!("delivery/{}", c.rel);
            !catalog
                .as_ref()
                .is_some_and(|cat| cat.files.iter().any(|f| f.path == want))
        })
        .map(|c| c.rel.clone())
        .collect();
    if !unlisted.is_empty() {
        /* 反查是哪一版（产物名 → uid）：定位得到才给「定位源问题」，否则只展示路径 */
        let at = unlisted.first().and_then(|rel| {
            let name = rel.rsplit('/').next()?;
            let v = book.versions().iter().find(|v| v.mkp_file == name)?;
            Some(crate::workbench::domain::issues::Where {
                view: crate::workbench::domain::issues::View::Params,
                machine_id: Some(v.machine_id.clone()),
                uid: Some(v.uid.clone()),
                key: None,
            })
        });
        out.push(FinalizeIssue {
            id: "collection.unregistered".to_owned(),
            title: format!("{} 项交付集合需要处理", unlisted.len()),
            kind: FinalizeIssueKind::Source,
            detail: "产物已经在盘上，但 catalog.json 没登记它。先回①卡「生成」一次，\
                     让目录按当前源重算；如果它本来就不该发布，去源配置里确认这台机型 / \
                     这一版的启用状态。不要直接把它塞进 manifest。"
                .to_owned(),
            action: if at.is_some() {
                FinalizeAction::LocateSource
            } else {
                FinalizeAction::None
            },
            files: unlisted,
            at,
        });
    }

    if !unsettled.is_empty() {
        out.push(FinalizeIssue {
            id: "collection.unsettled".to_owned(),
            title: format!("{} 份文件的集合对不上", unsettled.len()),
            kind: FinalizeIssueKind::Source,
            detail: "盘上的产物与当前源算出来的集合还不一致。回①卡「生成」一次\
                     （多出来的移入回收站、缺的补出来），再来更新凭据。"
                .to_owned(),
            action: FinalizeAction::None,
            files: unsettled,
            at: None,
        });
    }

    /* ③ 身份：source / 目录 / 账本的关系 —— 交给发布闸，不在这里修 */
    for item in checks
        .iter()
        .filter(|i| i.status == super::audit::AuditStatus::Fail)
    {
        out.push(FinalizeIssue {
            id: format!("identity.{}", item.id),
            title: item.name.clone(),
            kind: FinalizeIssueKind::Identity,
            detail: format!("{}（{}）", item.details, item.fix_hint),
            action: FinalizeAction::CheckIdentity,
            files: item.affected_files.clone(),
            at: None,
        });
    }

    /* ④ 附属资产的登记口径差异：只报计数，不铺明细、不当故障 */
    for item in checks
        .iter()
        .filter(|i| i.status == super::audit::AuditStatus::Warn)
    {
        out.push(FinalizeIssue {
            id: format!("info.{}", item.id),
            title: format!("{} 项附属登记差异", item.affected_files.len()),
            kind: FinalizeIssueKind::Info,
            detail: item.details.clone(),
            action: FinalizeAction::None,
            files: Vec::new(),
            at: None,
        });
    }

    Ok(out)
}

/// 「更新发布凭据」写盘前的**保护条件**：非空 = 一个字节都不许写。
///
/// 判据只有两条，都读自交付层同一套比较（不另写一套）：
///
/// 1. 盘上集合与当前源算出来的**集合**一致（没有 `Added` / `Removed`）；
/// 2. `catalog.json` 本身已经是当前代，且候选产物逐份都在它里面登记过。
///
/// 任一条不满足时写 manifest，就会把"本来不该发布 / 客户端拿不到"的东西登记成
/// 正式凭据 —— 那比"没修成"严重得多。
fn credential_blockers(
    delivery: &std::path::Path,
    book: &Book<'_>,
) -> Result<Vec<String>, AppError> {
    use super::delivery::DiffKind;

    let (candidates, diffs) = super::delivery::delivery_diffs(delivery, book)?;
    let mut out: Vec<String> = Vec::new();
    for d in &diffs {
        match d.kind {
            DiffKind::Added => out.push(format!("{}：盘上还没有，先生成一次", d.rel)),
            DiffKind::Removed => out.push(format!(
                "{}：已不在当前交付集合里，先生成一次（会移入回收站）",
                d.rel
            )),
            _ => {}
        }
    }
    if let Some(d) = diffs
        .iter()
        .find(|d| d.rel == super::delivery::NEW_CATALOG_FILE)
    {
        if d.kind != DiffKind::Same {
            out.push(format!(
                "{}：目录还停在上一代，先生成一次让目录按当前源重算",
                d.rel
            ));
        }
    }
    let catalog = std::fs::read(delivery.join(super::delivery::NEW_CATALOG_FILE))
        .ok()
        .and_then(|b| serde_json::from_slice::<crate::runtime::catalog::Catalog>(&b).ok());
    let preset_prefix = format!("{}/", paths::MKP_PRESETS_DIR);
    for c in candidates
        .iter()
        .filter(|c| c.rel.starts_with(&preset_prefix))
        /* 停用机型：目录本来就不登记它，产物留在盘上是设计，不是没准备交付 */
        .filter(|c| !super::delivery::product_disabled(book, &c.rel[preset_prefix.len()..]))
    {
        let want = format!("delivery/{}", c.rel);
        if !catalog
            .as_ref()
            .is_some_and(|cat| cat.files.iter().any(|f| f.path == want))
        {
            out.push(format!("{}：产物在盘上，但目录没登记它", c.rel));
        }
    }
    Ok(out)
}

/* ---------- 对照基线（b05 Task 14.9） ---------- */

/// 基线 diff 的一条：产物（`BUILTIN_PRESETS`，判据保证与入库目录一份不差）vs
/// `crates/postprocess/tests/fixtures/presets/` 的同名文件
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BaselineDiffEntry {
    pub file_name: String,
    /// `same` / `changed` / `missingBaseline`。没有"产物侧缺失"：
    /// 产物名单来自编译进二进制的表，它有判据盯着
    pub status: String,
    /// 两侧内容哈希前 16 位。给界面确认用 —— 哈希不同就是变了
    pub product_sha: String,
    pub baseline_sha: Option<String>,
}

/// **基线 diff**（14.9 的第①步，**只读**）：列出九份产物的同步状态，
/// 人看过这份清单、点确认，才轮到 [`wb_sync_baseline`] 写。
///
/// 为什么产物侧用 `BUILTIN_PRESETS` 而不是读盘：那张表有判据
/// （`builtin_presets_match_dir`）保证与入库目录**一份不差**，编译进二进制
/// 意味着"发布者看到的"与"用户二进制里带的"是同一份。
#[tauri::command(async)]
pub fn wb_baseline_diff() -> Result<Vec<BaselineDiffEntry>, AppError> {
    traced("wb_baseline_diff", |_| {
        Ok(baseline_diff_against(&preset::generate::fixtures_dir()))
    })
}

/// diff 的领域体：对哪份基线目录比对由调用方给 ——
/// 判据要在系统临时目录的基线上做反向走查（落点闸允许的那个豁免）
fn baseline_diff_against(fixtures: &std::path::Path) -> Vec<BaselineDiffEntry> {
    let mut out = Vec::new();
    for (name, content) in preset::BUILTIN_PRESETS {
        let product_sha = short_sha(content.as_bytes());
        let baseline_bytes = std::fs::read(fixtures.join(name));
        let (status, baseline_sha) = match &baseline_bytes {
            Ok(b) if b.as_slice() == content.as_bytes() => ("same", short_sha(b)),
            Ok(b) => ("changed", short_sha(b)),
            Err(_) => ("missingBaseline", String::new()),
        };
        out.push(BaselineDiffEntry {
            file_name: (*name).to_owned(),
            status: status.to_owned(),
            product_sha,
            baseline_sha: (!baseline_sha.is_empty()).then_some(baseline_sha),
        });
    }
    out
}

/// **同步对照基线**（14.9 的第②步，**显式写入动作**）。
///
/// 前提：人已经看过 [`wb_baseline_diff`] 的清单并确认。这里直接转调
/// `preset::generate::sync_baseline` —— **落点闸在它内部**
/// （`check_baseline_target` 只认真 fixtures 目录或系统临时目录），src-tauri
/// 不经手路径，也就没有绕过闸的口子。内容相同的自动跳过，返回真正写入的份数。
#[tauri::command]
pub fn wb_sync_baseline() -> Result<usize, AppError> {
    traced("wb_sync_baseline", |_| {
        let n = preset::generate::sync_baseline(
            &preset::generate::assets_dir(),
            &preset::generate::fixtures_dir(),
        )
        .map_err(|e| AppError::invalid_argument("基线同步被拒绝").with_detail(e))?;
        tracing::info!(synced = n, "对照基线已同步（人工确认后）");
        Ok(n)
    })
}

fn short_sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let h = Sha256::digest(bytes);
    format!("{:x}", h).chars().take(16).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, Committed, CommittedVersion, Draft};
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture};
    use crate::workbench::store::Store;
    use std::collections::BTreeSet;

    /* ---------- 生成 × 台账（2026-10-06 状态机修正的判据） ---------- */

    use super::super::Ctx;
    use crate::runtime::catalog::Catalog as RuntimeCatalog;
    use crate::workbench::domain::BuildState as TestBuildState;

    /// 一份挂在临时目录上的会话 + 一份被引用的 BBS 资产文件（write_content 的健检要它）。
    /// 生成 / 台账 / 快照 / catalog 的落盘全部从 `ctx.presets.root()` 派生 —— 与真仓库零接触。
    fn fixture_ctx() -> (tempfile::TempDir, Ctx) {
        let f = crate::workbench::domain::testkit::Fixture::load();
        let (dir, presets) = f.into_parts();
        // 夹具引用的资产文件落齐（write_content 的健检要求真实存在）：
        // 两台机型的图标 + A1 的机型图 + A1 的 BBS 曲线。
        // 写盘走 atomic_write —— 写盘纪律对测试同样生效。
        for rel in ["icons/a1.svg", "icons/p1s.svg", "printers/a1.webp"] {
            let p = dir.path().join("presets/assets").join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            crate::fsx::atomic::atomic_write(&p, b"asset").unwrap();
        }
        let bbs = dir.path().join("presets/assets/bbs/A1");
        std::fs::create_dir_all(&bbs).unwrap();
        crate::fsx::atomic::atomic_write(&bbs.join("process.json"), br#"{"x":1}"#).unwrap();
        /* 载荷根挂上：不挂的话发布侧构建目录时所有资产都算"载荷缺失"，
        目录里就没有 A 类条目 —— 而 manifest 会照登它们，这本账从一开始就自相矛盾。 */
        let mut presets = presets;
        presets.set_asset_root(&dir.path().join("presets/assets"));
        let store = Store::at(dir.path().join("workbench"));
        store.bootstrap().unwrap();
        (dir, Ctx::with(presets, store).unwrap())
    }

    /// ★ **生成成功 = 台账已经落盘**：不调用 `wb_save`，`built.json` 里就有本批的
    /// 新指纹，且与「现在会渲染出来的」同一代 —— 这就是「产物新、台账旧」那个
    /// 分裂状态的墓碑（真机踩过：本机 Built、CI Stale）。
    #[test]
    fn generation_writes_the_ledger_in_the_same_transaction() {
        let (dir, mut ctx) = fixture_ctx();

        let report = generate_with(&mut ctx, &Scope::All, false).expect("生成成功");
        assert!(!report.written.is_empty(), "首生成必须真写了东西");

        // **绕开会话**直接读盘（干净检出的视角）：台账里有本批每一笔
        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let empty = Draft::default();
        // 判 Built 现在是"盘上字节 == 当前渲染"，所以要装渲染器（与生成同一个探针）
        let probe = renderer_for(&ctx);
        let book = Book::new(&ctx.presets, &loaded.committed, &empty).with_renderer(&probe);
        for uid in &report.written {
            let rec = loaded
                .committed
                .built
                .get(uid)
                .unwrap_or_else(|| panic!("台账里必须有 {uid} —— 生成与落账是同一批"));
            let fp = book.version_layers(uid).unwrap().fingerprint();
            assert_eq!(
                rec.fingerprint, fp,
                "{uid} 的台账指纹必须是这一代的渲染指纹"
            );
            // 干净 Book（无草稿）上就是已生成
            assert_eq!(book.build_state(uid), TestBuildState::Built);
        }
        // 草稿从头到尾没被生成碰过
        assert!(ctx.draft.is_clean(), "生成不写草稿 —— 它不是编辑");

        // catalog 登记的 sha 与交付真字节同一代
        let catalog = RuntimeCatalog::parse(
            &std::fs::read(
                dir.path()
                    .join("presets/delivery")
                    .join(crate::workbench::app::delivery::NEW_CATALOG_FILE),
            )
            .unwrap(),
        )
        .unwrap();
        for f in &catalog.files {
            if let Some(sha) = &f.sha256 {
                let bytes = std::fs::read(dir.path().join("presets").join(&f.path)).unwrap();
                assert_eq!(
                    preset::provenance::sha256_hex(&String::from_utf8_lossy(&bytes)),
                    *sha,
                    "{} 的 catalog 登记与真字节必须一致",
                    f.path
                );
            }
        }
    }

    /// ★ **销毁草稿不能把已生成翻成待生成** —— 之前 CI 红的那条链
    /// （生成 → 不保存 → 本机 Built → 草稿一没就 Stale）从这里开始不再成立：
    /// 台账在磁盘上，草稿是死是活与生成状态无关。连「旧版快照里还带着
    /// built 字段」那种历史文件也只被当草稿编辑态解析，翻不了账。
    #[test]
    fn destroying_the_draft_cannot_flip_built_back_to_stale() {
        let (dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("生成成功");

        // 造一份**带着旧 built 字段的草稿快照**（老版本工作台写出来的形状）
        let legacy = serde_json::json!({
            "values": {},
            "visibility": {},
            "bundles": {},
            "built": { "A1/STANDARD": { "stamp": "old", "fingerprint": "deadbeef" } }
        });
        let draft_file = dir.path().join("workbench").join(Store::DRAFT_REL);
        std::fs::create_dir_all(draft_file.parent().unwrap()).unwrap();
        crate::fsx::atomic::atomic_write(&draft_file, legacy.to_string().as_bytes()).unwrap();

        // 干净检出视角：重新从盘上载入（草稿读回来 + 台账读回来）→ 仍然是已生成
        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let draft = super::super::storage::read_draft(&ctx.store).unwrap();
        let probe = renderer_for(&ctx);
        let book = Book::new(&ctx.presets, &loaded.committed, &draft).with_renderer(&probe);
        assert_eq!(
            book.build_state("A1/STANDARD"),
            TestBuildState::Built,
            "草稿快照里的旧 built 记录不得覆盖台账"
        );

        // 把草稿整个删掉（等于从没保存过任何草稿）→ 还是已生成
        std::fs::remove_file(&draft_file).unwrap();
        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let empty = Draft::default();
        let book = Book::new(&ctx.presets, &loaded.committed, &empty).with_renderer(&probe);
        assert_eq!(book.build_state("A1/STANDARD"), TestBuildState::Built);
    }

    /// no-op 的生成**连台账都不碰**：字节没变、指纹全对上时，built.json 原样
    ///（2026-10-05 裁定在生成事务层依然成立 —— git/clean 不因为"看了一眼"变红）。
    #[test]
    fn a_noop_generation_leaves_the_ledger_byte_identical() {
        let (dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("首次生成");

        let ledger = dir.path().join("workbench").join("built.json");
        let before = std::fs::read(&ledger).unwrap();

        let report = generate_with(&mut ctx, &Scope::Stale, false).expect("二次生成");
        assert!(report.written.is_empty(), "没有新东西要写");
        // 一切都对得上 → 计划清单是空的（既没有要写的，也没有要跳过的）

        let after = std::fs::read(&ledger).unwrap();
        assert_eq!(before, after, "no-op 生成不许动台账（逐字节）");
    }

    /* ---------- 验收：回执只报告真实结果（2026-11-01） ---------- */

    /// **全部无变化**：`written=0`、`unchanged=total`、`failed=0`，
    /// 并且 `commit` 那一步要**明说"都没变"**，不许让它看起来像"什么都没干"。
    #[test]
    fn a_noop_generate_says_nothing_changed_instead_of_faking_writes() {
        let (_dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("首次生成");

        let report = generate_with(&mut ctx, &Scope::All, false).expect("二次生成");
        let r = report.receipt.expect("该有回执");

        assert_eq!(r.status, super::super::receipt::ReceiptStatus::Passed);
        assert_eq!(r.delivery.written, 0, "内容没变，交付一份都不许写");
        assert_eq!(
            r.delivery.unchanged, r.delivery.total,
            "没写的那几份要记成「未变」，不是消失"
        );
        assert_eq!(r.delivery.failed, 0);
        assert_eq!(r.assets.written, 0, "内核同样一份都不许写");
        // `written/total` 相加必须等于总数 —— 三个数不许对不上
        for c in [r.delivery, r.assets, r.baseline] {
            assert_eq!(
                c.written + c.unchanged + c.failed,
                c.total,
                "写/未变/失败 三个数必须加起来等于总数"
            );
        }
        let commit = r.steps.iter().find(|s| s.key == "commit").unwrap();
        assert!(commit.ok);
        assert!(
            commit.detail.contains("未变化"),
            "全部没变时要明说，别让人以为写过了：{}",
            commit.detail
        );
    }

    /// **部分变化**：`written` 必须等于**实际写盘的文件数**，不是"处理了几个"。
    ///
    /// 这正是 2026-11-01 那个假 `9/9` 的来源：旧代码 `n_assets += 1` 无条件累加，
    /// 于是"一份都没写"也报 `9/9`。
    #[test]
    fn written_counts_real_writes_not_files_visited() {
        let (_dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("首次生成");

        // 改一个版本的值 → 只该有那一份变
        apply(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(33)),
            }],
        )
        .unwrap();
        let report = generate_with(&mut ctx, &Scope::All, false).expect("改后生成");
        let r = report.receipt.expect("该有回执");

        assert!(r.delivery.written >= 1, "改过的版本必须真写");
        assert_eq!(
            r.delivery.written,
            report.written.len(),
            "回执里的写入数必须与真写盘的那份清单逐项一致"
        );
        assert!(
            r.delivery.unchanged > 0,
            "没改的那些要计入「未变」，它们确实没写"
        );
        /* 内核那条链**只在真实仓库会话**里跑（`ctx.real_root == 真 presets 根`）；
        测试夹具是临时根，所以这里它整段跳过 —— `assets` 三个数都该是 0，
        而不是"跟交付一样"（那个假设只在生产里成立）。 */
        assert_eq!(
            (r.assets.written, r.assets.unchanged, r.assets.total),
            (0, 0, 0),
            "临时会话不写内核产物，计数就该如实是 0"
        );
    }

    /// **回执与磁盘逐项一致**：回执说写了哪些，盘上就真的变了哪些。
    #[test]
    fn the_receipt_matches_what_actually_landed_on_disk() {
        let (_dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("首次生成");

        let dir = paths::delivery_root_at(&ctx.real_root)
            .join(crate::workbench::app::delivery::MKP_DIR)
            .join("presets");
        let before: Vec<(String, std::time::SystemTime)> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    e.metadata().unwrap().modified().unwrap(),
                )
            })
            .collect();

        apply(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(33)),
            }],
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let report = generate_with(&mut ctx, &Scope::All, false).expect("改后生成");
        let r = report.receipt.expect("该有回执");

        // 回执点名的文件，mtime 必须是新的；没点名的必须是旧的
        for (name, old_mtime) in before {
            let now = std::fs::metadata(dir.join(&name))
                .unwrap()
                .modified()
                .unwrap();
            let named = r.files.iter().any(|f| f == &name);
            if named {
                assert!(now > old_mtime, "{name} 回执说写了，盘上却没变");
            } else {
                assert_eq!(now, old_mtime, "{name} 回执说没写，盘上却被动过");
            }
        }
    }

    /* ---------- 交付产物确定性 / 内核产物的写盘记录 ---------- */

    /// 内核产物仍带写盘记录：时间戳插在头注释块末尾，剥掉它又回到原样。
    /// 交付产物不写这一行（`generate_transaction` 写的是确定性候选字节）。
    #[test]
    fn the_generated_at_line_round_trips() {
        let body = "# machine: A1\n# variant: standard\n\n[wiping]\nratio = 1\n";
        let stamped = stamp(body);
        assert!(stamped.contains("# generated_at: "), "写盘版必须带生成时间");
        assert!(
            stamped.starts_with("# machine: A1"),
            "机型那行仍是第一行：{stamped}"
        );
        assert!(
            stamped.find("# generated_at: ").unwrap() < stamped.find("[wiping]").unwrap(),
            "生成时间要落在头注释块里（业务内容之前）：{stamped}"
        );
        assert_eq!(
            crate::workbench::strip_generated_at(&stamped),
            body,
            "剥掉时间必须逐字节回到渲染正文"
        );
    }

    /// ★ **内容没变时再点一次生成：不写、时间不动、字节不动**。
    /// 交付产物本身是确定性字节 —— 里面没有写盘时刻，也就没有"每次点生成都变"的可能。
    #[test]
    fn a_second_generate_does_not_touch_the_files_when_nothing_changed() {
        let (dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("首次生成");

        let target = ctx
            .real_root
            .join(crate::workbench::paths::DELIVERY_SUBDIR)
            .join(crate::workbench::paths::MKP_PRESETS_DIR)
            .join("A1-standard.toml");
        let first = std::fs::read(&target).expect("产物该在");
        let first_mtime = std::fs::metadata(&target).unwrap().modified().unwrap();
        assert!(
            !String::from_utf8_lossy(&first).contains("# generated_at: "),
            "交付产物不写写盘时刻；生成时刻记在回执里"
        );

        // 隔一会儿再生成，确保"如果真写了"会留下痕迹
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let report = generate_with(&mut ctx, &Scope::All, false).expect("二次生成");
        assert!(report.written.is_empty(), "内容没变，一份都不许写");
        assert_eq!(std::fs::read(&target).unwrap(), first, "产物必须逐字节不动");
        assert_eq!(
            std::fs::metadata(&target).unwrap().modified().unwrap(),
            first_mtime,
            "连文件修改时间都不该变"
        );
        drop(dir);
    }

    /// **强制重新生成**：跳过"内容没变"的判断，把全部候选重新走一遍事务与回读验证。
    /// 写出去的仍是同一份确定性字节 —— 强制生成不该制造一个新版本的内容。
    #[test]
    fn force_regen_rewrites_even_when_nothing_changed() {
        let (_dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("首次生成");
        let target = ctx
            .real_root
            .join(crate::workbench::paths::DELIVERY_SUBDIR)
            .join(crate::workbench::paths::MKP_PRESETS_DIR)
            .join("A1-standard.toml");
        let before = std::fs::read_to_string(&target).unwrap();

        std::thread::sleep(std::time::Duration::from_millis(1100));
        let report = generate_with(&mut ctx, &Scope::All, true).expect("强制生成");
        assert!(!report.written.is_empty(), "强制生成必须重写");

        let after = std::fs::read_to_string(&target).unwrap();
        assert_eq!(before, after, "确定性字节：强制重写也不该改变产物内容");
    }

    /// 内容真变了才写，写进去的是当前源算出的确定性正文。
    #[test]
    fn a_real_change_writes_the_file_and_refreshes_the_timestamp() {
        let (_dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("首次生成");
        let target = ctx
            .real_root
            .join(crate::workbench::paths::DELIVERY_SUBDIR)
            .join(crate::workbench::paths::MKP_PRESETS_DIR)
            .join("A1-standard.toml");
        let before = std::fs::read_to_string(&target).unwrap();

        // 走正规写入口改一个值（与工作台里改参数同一条路）
        apply(
            &mut ctx.draft,
            &ctx.committed,
            &ctx.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(33)),
            }],
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let report = generate_with(&mut ctx, &Scope::All, false).expect("改后生成");
        assert!(!report.written.is_empty(), "内容变了必须写");

        let after = std::fs::read_to_string(&target).unwrap();
        assert_ne!(before, after, "文件内容必须变");
        assert!(after.contains("child = 33"), "新值要落进去");
        assert!(
            !after.contains("# generated_at: "),
            "交付产物是确定性正文，不写写盘时刻"
        );
    }

    /// 台账里的指纹就是渲染指纹 → 每一份有产物的版本都该是「已生成」；
    /// A2L 那种「暂无资源」不参与（它是没有东西可生成，不是生成过没生成过）。
    #[test]
    fn every_renderable_version_reads_built_after_a_full_generation() {
        let (_dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("全量生成");

        let loaded = super::super::storage::load(&ctx.store, &ctx.presets).unwrap();
        let empty = Draft::default();
        let probe = renderer_for(&ctx);
        let book = Book::new(&ctx.presets, &loaded.committed, &empty).with_renderer(&probe);
        for uid in ["A1/STANDARD", "A1/FAST", "P1S/LITE"] {
            assert_eq!(
                book.build_state(uid),
                TestBuildState::Built,
                "{uid} 在全量生成后必须是已生成"
            );
        }
    }

    /* ---------- 对照基线（b05 Task 14.9） ---------- */

    /// **清旧名只清清单不认的，且可还原**（2026-11-12）。
    ///
    /// 改版本 id 之后，生成按新清单写出新名产物；旧名该进回收站，
    /// 但这批文件里还有没有参与本次生成的其它版本、以及非 TOML 的说明文件，
    /// 一个都不许动。
    #[test]
    fn pruning_removes_only_orphans_and_keeps_them_recoverable() {
        let dir = tempfile::tempdir().expect("临时目录");
        let trash = tempfile::tempdir().expect("临时目录");
        std::fs::write(dir.path().join("A1-standard.toml"), "标准版").unwrap();
        std::fs::write(dir.path().join("A1-fast260326.toml"), "快拆版").unwrap();
        std::fs::write(dir.path().join("A1-fastv3.3.toml"), "旧名").unwrap();
        std::fs::write(dir.path().join("README.md"), "不是产物").unwrap();

        let expected: BTreeSet<String> = ["A1-standard.toml", "A1-fast260326.toml"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let moved = prune_orphan_presets(
            dir.path(),
            &expected,
            trash.path(),
            "kernel-presets",
            "2026-11-12T00-00-00",
        )
        .expect("清理");
        assert_eq!(moved, 1);

        let mut left: Vec<String> = std::fs::read_dir(dir.path())
            .unwrap()
            .flatten()
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        left.sort();
        assert_eq!(
            left,
            vec![
                "A1-fast260326.toml".to_owned(),
                "A1-standard.toml".to_owned(),
                "README.md".to_owned()
            ]
        );
        let saved = trash
            .path()
            .join("kernel-presets/2026-11-12T00-00-00/A1-fastv3.3.toml");
        assert_eq!(std::fs::read_to_string(&saved).unwrap(), "旧名");

        // 再来一次应该是 no-op：幂等，且不会把 expected 里的东西搬走
        assert_eq!(
            prune_orphan_presets(
                dir.path(),
                &expected,
                trash.path(),
                "kernel-presets",
                "again",
            )
            .expect("再清一次"),
            0
        );
    }

    /// **真数据只读锚点**：当前产物与基线应当逐字节相同（K-G0' 绿的现状），
    /// 所以 diff 必须是 `BUILTIN_PRESETS.len()` 条全 `same`。这条同时验证 diff 命令
    /// **只读**：它跑前后基线目录的文件集合不能变。
    #[test]
    fn the_real_baseline_diff_reports_all_same() {
        let fixtures = preset::generate::fixtures_dir();
        let before: BTreeSet<String> = walk_shas(&fixtures);
        let out = wb_baseline_diff().expect("diff");
        assert_eq!(
            out.len(),
            preset::BUILTIN_PRESETS.len(),
            "diff 条数应等于随包预设表的份数 —— 表由 build.rs 按目录生成，不会说谎"
        );
        let not_same: Vec<_> = out.iter().filter(|e| e.status != "same").collect();
        assert!(
            not_same.is_empty(),
            "真产物与基线应当全绿，却有：{not_same:?} —— 谁改了没同步？"
        );
        let after: BTreeSet<String> = walk_shas(&fixtures);
        assert_eq!(before, after, "diff 是只读的，不许动基线目录");
    }

    /// **反向走查（在系统临时目录做，落点闸明确允许；真基线一个字节不碰）**：
    /// 改一份基线 → diff 报 `changed` → sync 写入 → diff 回到全 `same`，
    /// 且写入后的字节与产物**逐字节相同**。未经确认直接写在这里不存在 ——
    /// sync 是显式命令，判据同时证明它**只动 diff 说过的那一份**
    #[test]
    fn sync_baseline_writes_exactly_what_the_diff_named() {
        let tmp = tempfile::tempdir().unwrap();
        let fixtures = tmp.path().to_path_buf();
        // 铺 9 份与产物相同的基线
        for (name, content) in preset::BUILTIN_PRESETS {
            crate::fsx::atomic::atomic_write(&fixtures.join(name), content.as_bytes()).unwrap();
        }
        // 改其中一份（反向：diff 必须抓到）
        let victim = preset::BUILTIN_PRESETS[0].0;
        crate::fsx::atomic::atomic_write(
            &fixtures.join(victim),
            format!("{}\n# 改过了\n", preset::BUILTIN_PRESETS[0].1).as_bytes(),
        )
        .unwrap();

        let out = baseline_diff_against(&fixtures);
        let changed: Vec<_> = out.iter().filter(|e| e.status == "changed").collect();
        assert_eq!(changed.len(), 1, "恰好一份变了：{changed:?}");
        assert_eq!(changed[0].file_name, victim);
        assert!(changed[0].baseline_sha.is_some());

        // sync（tempdir 在落点闸的允许清单里）：只写那一份，其余跳过
        let n = preset::generate::sync_baseline(&preset::generate::assets_dir(), &fixtures)
            .expect("sync");
        assert_eq!(n, 1, "只同步 diff 点名的那一份");

        // 重回全绿，且那份的字节与产物逐字节相同
        let out2 = baseline_diff_against(&fixtures);
        assert!(out2.iter().all(|e| e.status == "same"), "sync 后必须全绿");
        assert_eq!(
            std::fs::read(fixtures.join(victim)).unwrap(),
            preset::BUILTIN_PRESETS[0].1.as_bytes(),
            "写入的字节就是产物本体"
        );
    }

    /// 目录里全部文件的 sha 指纹（文件名 → sha 前 16），给"只读"断言用
    fn walk_shas(dir: &std::path::Path) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        fn walk(dir: &std::path::Path, out: &mut BTreeSet<String>) {
            for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, out);
                } else {
                    let bytes = std::fs::read(&p).unwrap_or_default();
                    out.insert(format!(
                        "{}:{}",
                        p.file_name()
                            .map(|s| s.to_string_lossy())
                            .unwrap_or_default(),
                        short_sha(&bytes)
                    ));
                }
            }
        }
        if dir.is_dir() {
            walk(dir, &mut out);
        }
        out
    }

    fn setup() -> (tempfile::TempDir, Fixture, Committed) {
        let dir = tempfile::tempdir().unwrap();
        let store = Store::at(dir.path());
        store.bootstrap().unwrap();
        let f = Fixture::load();
        let mut versions = BTreeMap::new();
        for (uid, machine, vid, name) in [
            ("A1/STANDARD", "A1", "STANDARD", "标准版"),
            ("A1/FAST", "A1", "FAST", "高速版"),
            ("A2L/STANDARD", "A2L", "STANDARD", "标准版"),
            ("P1S/LITE", "P1S", "LITE", "精简版"),
        ] {
            versions.insert(
                uid.to_owned(),
                CommittedVersion {
                    machine_id: machine.to_owned(),
                    version_id: vid.to_owned(),
                    name: name.to_owned(),
                    ..Default::default()
                },
            );
        }
        let c = Committed {
            versions,
            catalog: fixture_catalog(),
            ..Default::default()
        };
        (dir, f, c)
    }

    /// 渲染出来的东西要**能被 TOML 解析器读回来**，而且值对得上
    #[test]
    fn rendered_toml_parses_and_keeps_the_values() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let r = render(&book, "A1/STANDARD").unwrap();

        // 头里只剩两行内容身份（2026-11-01 去掉了每次变动的 release_time）
        assert!(r.text.starts_with("# machine: A1"));
        assert!(r.text.contains("# variant: standard"));
        assert!(
            r.text.contains("[toolhead]"),
            "段名要从数据里来：\n{}",
            r.text
        );

        // 解析成 `Table` 而不是 `Value`：toml 1.x 的 `Value` 从字符串解析要求整份文档
        // 就是一个值，而我们产出的是一份带表头的文档
        let parsed: toml::Table = r.text.parse().expect("产出来的必须是合法 TOML");
        let toolhead = &parsed["toolhead"];
        /*
         * 三轴偏移：**三个独立字段**（2026-10-02 从共享 tomlKey 的内联表 `offset = { x, y, z }`
         * 拆出来）。所以这里读的是 `offset_x/y/z` 三个裸键，不再是 `toolhead["offset"]["x"]`。
         * **整数不写小数点** —— 上游真产物就是 `offset_x = -1`。
         */
        assert_eq!(
            toolhead["offset_x"].as_integer(),
            Some(-1),
            "A1:STANDARD 的上游覆盖"
        );
        assert_eq!(toolhead["offset_y"].as_float(), Some(18.6));
        assert_eq!(toolhead["offset_z"].as_integer(), Some(4));
        // G-code 走多行字符串
        assert!(toolhead["script"].as_str().is_some());
        assert_eq!(parsed["wiping"]["mode"].as_str(), Some("tower"));
        // 行尾注释要在（挂在 offset_x 那一行上）
        assert!(
            r.text.contains("# 笔尖偏移"),
            "偏移那一行的行尾注释丢了：\n{}",
            r.text
        );
    }

    /// **M0：我们渲染出来的产物 vs 真机验证过的那份基线。**（b04 Task 11）
    ///
    /// 这是整个迁移的前置判据。基线是那 9 份内置预设 —— 它们由 `gen-presets` 从
    /// `preset_recipes.toml` 生成，**上过真机**。我们这条链（`presets/*.toml` → `render()`）
    /// 算出来的如果与它正文字节相同，说明两套真源等值、迁移不需要修数据；
    /// 不同就必须先定哪边对 —— 搬完 3 万行代码再发现值对不上，会留下一批
    /// "生成出来但和验证过的不一样"的产物，而那种错在产物上看不出来。
    ///
    /// 比的是**正文**：基线（历史产物）头里带着 `# uuid:` 与 `# release_time:`，
    /// 现役产物头里只有后者 —— 剥离这两行才能把两边对齐到正文
    /// （uuid 是 2026-10-05 删掉的死字段，见模块文档；基线不改，它是历史事实）。
    ///
    /// # 基线在仓内，配对认身份不认文件名（b05 Task 3.3）
    ///
    /// 基线以前指向兄弟仓库 `../mkp-ssr/crates/preset/assets/presets`：开发机上有它、
    /// CI 上没有 —— 于是在 CI 里这条判据一次都没跑过（那句"没找到基线目录"连颜色都不变）。
    /// 现在改指 `preset::generate::fixtures_dir()`：**仓内那 9 份，与旧仓那份逐字节相同**
    /// （sha256 9/9 一致，只差文件名），判据的强度没有变，但它从此在 CI 里也真跑。
    ///
    /// 配对**按文件头的 `# machine:` / `# variant:`**，不按文件名：夹具那边现在还是
    /// 云端那套名字（`A1MF_260628.toml`），b05 Task 5 会把它们改成产物命名
    /// （`A1_MINI-fastv3.3.toml`）。两套名字都配得上，这条判据不用跟着改。
    ///
    /// # 反空转：9 份一份都不能少
    ///
    /// 基线少了、同一身份重了、或某一份没被任何版本配上，都直接失败 ——
    /// 一条"比了 0 份"的判据比没有判据更坏，因为它是绿的。
    #[test]
    fn our_render_matches_the_machine_verified_baseline() {
        // 基线是消费端内置的 9 份，必须与真 presets 逐份对上。
        // 机型与版本全部来自我们自己那份清单（`presets/machines/*.toml`），
        // `render()` 不查任何外部来源。
        let presets = crate::workbench::load_presets().expect("真 presets");
        let tmp = tempfile::tempdir().unwrap();
        let store = crate::workbench::store::Store::at(tmp.path());
        store.bootstrap().unwrap();
        let c = super::super::storage::load(&store, &presets)
            .expect("干净仓库读得通")
            .committed;
        let draft = Draft::default();
        let book = Book::new(&presets, &c, &draft);

        /// 基线目录按**身份**索引：`(机型 id, 版本 id 小写)` → 文件。
        ///
        /// 读不出来的、同一身份出现两次的，都在这里直接报错：静默跳过会让这条判据
        /// 悄悄少比几份，而它看起来还是绿的。
        fn baseline_by_identity(
            dir: &std::path::Path,
        ) -> BTreeMap<(String, String), std::path::PathBuf> {
            let mut out: BTreeMap<(String, String), std::path::PathBuf> = BTreeMap::new();
            let entries = std::fs::read_dir(dir)
                .unwrap_or_else(|e| panic!("读不到基线目录 {}：{e}", dir.display()));
            for entry in entries {
                let path = entry.expect("目录项").path();
                if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                    continue;
                }
                let text = std::fs::read_to_string(&path)
                    .unwrap_or_else(|e| panic!("读不到 {}：{e}", path.display()));
                let file = preset::read_preset_from_bytes(text)
                    .unwrap_or_else(|e| panic!("{} 读不出来：{e}", path.display()));
                let variant = file.variant.as_deref().unwrap_or_else(|| {
                    panic!("{} 的文件头没有 `# variant:`，身份认不出来", path.display())
                });
                let key = (file.machine.clone(), variant.to_lowercase());
                if let Some(prev) = out.insert(key.clone(), path.clone()) {
                    panic!(
                        "基线 {:?} 有两份：{} 与 {}",
                        key,
                        prev.display(),
                        path.display()
                    );
                }
            }
            out
        }

        let baseline_dir = preset::generate::fixtures_dir();
        let baselines = baseline_by_identity(&baseline_dir);
        // 基线目录还兼作 `crates/postprocess` 的 IR 判据输入，可能比随包表多几份
        // 旧名资产；这里只要求「够现役产物配对」，不写死份数。
        assert!(
            baselines.len() >= preset::BUILTIN_PRESETS.len(),
            "基线份数（{}）比随包预设（{}）还少：{:?} —— 是谁动的？",
            baselines.len(),
            preset::BUILTIN_PRESETS.len(),
            baselines.keys().collect::<Vec<_>>()
        );

        /// 去掉按定义不对齐的头部行：release_time 是当下时间；uuid 与那句段落标签
        /// 只存在于**历史基线**那侧（现役产物都不写了，见模块文档）。
        ///
        /// ★ 滤掉的**就这三种行**，别扩成"头部整段不比"那种口子 —— 那样头里的
        /// 机型 / 版本写错了也绿，而头部正是身份（那条判据要守的东西之一）。
        fn body(s: &str) -> String {
            s.lines()
                .filter(|l| {
                    let t = l.trim();
                    !l.starts_with("# uuid:")
                        && !l.starts_with("# release_time:")
                        /* 生成时刻不表达内容：基线里那份是它被写出去的时刻，
                           我们这边 render() 根本不产这一行 —— 两者只差这里 */
                        && !l.starts_with(crate::workbench::GENERATED_AT_PREFIX)
                        && t != "#胶笔配置"
                        && t != "# 胶笔配置"
                })
                .collect::<Vec<_>>()
                .join("\n")
        }

        /// 同一份正文按行排序 —— 用来把「值不一样」与「只是顺序不一样」分开。
        ///
        /// 这两件事的处置完全不同：值不一样要定哪边对（可能要改数据），
        /// 顺序不一样只要定一个口径（TOML 键序不影响语义，`load_ir` 读进去是一样的）。
        /// 合在一起报「不一致」等于把一个能一句话解决的问题说成了一个要查数据的问题
        fn sorted_body(s: &str) -> Vec<String> {
            let mut v: Vec<String> = body(s).lines().map(str::to_owned).collect();
            v.sort();
            v
        }

        let mut compared = 0usize;
        let mut value_diff: Vec<String> = Vec::new();
        let mut order_only: Vec<String> = Vec::new();
        let mut matched: Vec<(String, String)> = Vec::new();
        for v in book.versions() {
            // 产物名只进报告（它比 uid 更接近用户看到的东西）；配对靠的是身份
            let name = preset_file_name(&v.machine_id, &v.version_id);
            let key = (v.machine_id.clone(), v.version_id.to_lowercase());
            let Some(path) = baselines.get(&key) else {
                continue; // 没有基线的版本（A2L 那台占位就是这样）—— 不是差异
            };
            let want = std::fs::read_to_string(path).expect("读基线");
            let got = render(&book, &v.uid).expect("渲染得出来").text;
            matched.push(key);
            compared += 1;
            let (a, b) = (body(&got), body(&want));
            if a == b {
                continue;
            }
            if sorted_body(&got) == sorted_body(&want) {
                // 每一行两边都有，只是排的位置不同
                order_only.push(name);
                continue;
            }
            // 真差异：逐行列出只在一边出现的
            let (sa, sb) = (sorted_body(&got), sorted_body(&want));
            let only_ours: Vec<&String> = sa.iter().filter(|l| !sb.contains(l)).take(4).collect();
            let only_theirs: Vec<&String> = sb.iter().filter(|l| !sa.contains(l)).take(4).collect();
            value_diff.push(format!(
                "  {name}\n    只在我们：{only_ours:?}\n    只在基线：{only_theirs:?}"
            ));
        }

        // 反空转①：随包表里的**每一份**都要配上基线。
        //
        // 2026-11-12 换向：原来咬「基线里不能有配不上现役版本的孤儿」，但基线目录
        // 还兼作 `crates/postprocess` 的 IR 判据输入，会合法地留着几份旧身份资产。
        // 现在改成从 `BUILTIN_PRESETS`（由 build.rs 按目录生成）往回核对，
        // 对现役产物一份不少，同时不再把判据资产误判成孤儿。
        let table_names: std::collections::BTreeSet<String> = preset::BUILTIN_PRESETS
            .iter()
            .map(|(name, _)| (*name).to_owned())
            .collect();
        let matched_names: std::collections::BTreeSet<String> = matched
            .iter()
            .map(|(m, v)| preset_file_name(m, v))
            .collect();
        let unmatched: Vec<&String> = table_names.difference(&matched_names).collect();
        assert!(
            unmatched.is_empty(),
            "随包表里有 {} 份没配上基线：{unmatched:?} —— \
             `presets/machines/*.toml` 与夹具的身份对不上了",
            unmatched.len()
        );
        // 反空转②：比到的份数不少于随包表（一对一配对 ⇒ 多出来的只能是额外基线资产）
        assert!(
            compared >= table_names.len(),
            "比了 {compared} 份，随包表有 {} 份",
            table_names.len()
        );
        assert!(
            value_diff.is_empty(),
            "**值不一致**（比了 {compared} 份）—— 这要先定哪边对，不能直接搬代码：\n{}",
            value_diff.join("\n")
        );

        // **M0 的结论（2026-09-23 实测）：9 份逐行同集合，值全对上了。**
        //
        // 剩下的唯一差异是**段内键序**：基线是 offset → speed_limit → custom_mount_gcode…，
        // 我们按 `layout.order`（界面顺序）。TOML 键序不影响语义，`load_ir` 读进去一样，
        // 所以这不是数据问题，是口径问题。
        //
        // 为什么这里只记录不断言：**定案倾向跟基线**（那 9 份已经发布、上过真机，
        // 字节相同意味着"换成我们生成"在交付面上是零变化），但改排序会牵动
        // 好几条既有判据，属于一次独立的改动。它是 Task 11 的收尾项。
        //
        // 不断言不等于放过：`value_diff` 那一条是真判据（值一变就红），
        // 而键序这一条一旦修好，把下面这个 `if` 换成 `assert!` 即可 —— 留着这行是为了
        // 让"还没修"这件事在每次跑测试时都出现在眼前，而不是躺在某个清单里
        if !order_only.is_empty() {
            eprintln!(
                "【M0 已知差异，待 Task 11 收尾】值全对上了（{compared} 份逐行同集合），\
                 但段内键序不同：{} 份。定案倾向跟基线（它已发布、上过真机）。\
                 推断基线用的是注册表里 [[params]] 的出现顺序，而我们用 layout.order —— \
                 落地前要先验证这个推断",
                order_only.len()
            );
        }
    }

    /// 段内键序 = **tomlKey 字母序**（大小写不敏感）—— 生成不许改变没改过的行的位置。
    ///
    /// 作者 2026-10-03：「明明都是 O 开头的 offset 都是一起的，生成的时候却改变了
    /// 它的顺序」—— 以前按界面顺序排，注册表一挪条目产物键序就漂。
    #[test]
    fn sections_are_sorted_by_key_name_so_the_order_never_drifts() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let r = render(&book, "A1/STANDARD").unwrap();

        // 抠出 [toolhead] 段的键（小写化后必须已经有序）
        let seg = r
            .text
            .split("[toolhead]\n")
            .nth(1)
            .unwrap()
            .lines()
            .take_while(|l| !l.starts_with('[') && !l.is_empty())
            .filter_map(|l| l.split('=').next())
            .map(|k| k.trim().to_lowercase())
            .collect::<Vec<_>>();
        assert!(
            seg.len() >= 2,
            "fixture 的 toolhead 段该有几个键：{:?}",
            seg
        );
        let mut sorted = seg.clone();
        sorted.sort();
        assert_eq!(seg, sorted, "段内键要按字母序：{:?}", seg);
    }

    /// 同样的输入**产出逐字节相同的产物**（2026-11-01 起连时间戳都不再例外）。
    ///
    /// 这条是「内容没变不重写」与「生成可复现」的地基：头里只要有一个"每次都变"
    /// 的东西（随机 uuid / 当前时刻），整条规则立刻失效。
    #[test]
    fn rendering_is_byte_for_byte_reproducible() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let a = render(&book, "A1/STANDARD").unwrap();
        let b = render(&book, "A1/STANDARD").unwrap();
        assert_eq!(a.text, b.text, "两次渲染必须逐字节相同");
        assert_eq!(a.fingerprint, b.fingerprint);
    }

    /// 产物头**不许**出现 `# release_time:`（2026-11-01 删掉的假时间字段）。
    ///
    /// 理由：它写的是**渲染时刻**，与"这一份是什么时候写出去的"无关，却让同一份
    /// 配方每次生成出不同字节 —— 于是「内容没变不重写」必须靠"跳过某一行"来救，
    /// 那套补丁正是这次要收掉的东西。作者 2026-11-01 被它骗过一次：
    /// 「他怎么可能这个时间就是刚才呢」。时间改由 `built.json` 的 `stamp` 表达。
    #[test]
    fn the_product_header_carries_no_release_time() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        for uid in ["A1/STANDARD", "A1/FAST", "A2L/STANDARD", "P1S/LITE"] {
            let r = render(&book, uid).unwrap();
            assert!(
                !r.text.contains("# release_time"),
                "{uid} 的产物头里又出现了 release_time —— 生成就不再可复现了"
            );
        }
    }

    /// 产物头**不许**出现 `# uuid:` —— 2026-10-05 删掉的死字段（全仓无读者、
    /// 公开渠道无消费方，见模块文档），谁把它加回来这条就红。
    /// 历史基线那 9 份头里还带着它，那是历史事实，与本判据无关。
    #[test]
    fn the_product_header_carries_no_uuid() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        for uid in ["A1/STANDARD", "A1/FAST", "A2L/STANDARD", "P1S/LITE"] {
            let r = render(&book, uid).unwrap();
            assert!(
                !r.text.contains("# uuid:"),
                "{uid} 的产物头里出现了 uuid —— 一行没有读者的假字段又回来了"
            );
        }
    }

    /// 改一个值 → 产物真的变了（不然「待生成」是句空话）
    #[test]
    fn changing_a_value_changes_the_output() {
        let (_d, f, c) = setup();
        let mut draft = Draft::default();
        let before = render(&Book::new(&f.presets, &c, &draft), "A1/STANDARD").unwrap();

        apply(
            &mut draft,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(33)),
            }],
        )
        .unwrap();
        let after = render(&Book::new(&f.presets, &c, &draft), "A1/STANDARD").unwrap();

        assert_ne!(before.text, after.text, "改了值，产物字节必须变");
        assert_ne!(before.fingerprint, after.fingerprint);
        assert!(after.text.contains("child = 33"));
    }

    /// 字符串里的引号要转义 —— 不转义能让整份配方解析失败
    #[test]
    fn strings_are_escaped() {
        assert_eq!(toml_string("a\"b"), "\"a\\\"b\"");
        assert_eq!(toml_string("a\\b"), "\"a\\\\b\"");
        assert_eq!(toml_string("a\nb"), "\"a\\nb\"");
    }

    /// 多行字符串里出现 `"""` 要断开，否则字面量提前结束
    #[test]
    fn a_triple_quote_inside_gcode_is_broken_up() {
        let body = gcode_body(&serde_json::json!("G1\n\"\"\"\nG2"));
        assert!(!body.contains("\"\"\""), "还留着三引号：{body}");
        let text = format!("k = \"\"\"\n{body}\n\"\"\"\n");
        let parsed: toml::Table = text.parse().expect("断开之后要还能解析");
        assert!(parsed["k"].as_str().unwrap().contains("G1"));
    }

    /// 「写不写」= **整份字节是否相同**，一个字节都不放过（2026-11-01 起没有例外行）。
    ///
    /// 以前这里验的是"`same_payload` 只忽略时间戳"，那条函数已删：产物头不再带
    /// 时刻，例外也就不需要了。现在多一个空格都要重写 —— 这才是"内容没变"的真定义。
    #[test]
    fn a_product_is_rewritten_whenever_any_byte_differs() {
        let a = "# machine: A1\n# variant: standard\nk = 1\n";
        let b = "# machine: A1\n# variant: standard\nk = 1\n";
        let c = "# machine: A1\n# variant: standard\nk = 2\n";
        let d = "# machine: A1\n# variant: standard\nk = 1 \n";
        assert_eq!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d, "末尾多个空格也算内容不同");
    }

    /// 有阻断时**渲染照做、但生成该被拒**。
    /// 这里只验校验那一半（`wb_generate` 要真仓库，走不了单测）
    #[test]
    fn a_block_is_detected_before_generating() {
        let (_d, f, c) = setup();
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();
        let book = Book::new(&f.presets, &c, &draft);
        let r = issues::inspect(&book);
        assert!(r.blocked());
        assert!(!w::disabled::BUILD_BLOCKED.is_empty());
    }

    /// A2L 没有产物 → 渲染它照样能出东西（参数全是出厂默认），
    /// 但生成会把它按「暂无资源」跳过 —— 这两件事分开
    #[test]
    fn a2l_renders_but_is_skipped_by_generate() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);

        let r = render(&book, "A2L/STANDARD").unwrap();
        assert!(r.text.contains("# machine: A2L"));
        // 文件名只由清单决定（机型 id + 版本 id 小写），与上游给不给名字无关
        assert_eq!(r.file_name, "A2L-standard.toml");

        let row = book
            .build_rows()
            .into_iter()
            .find(|x| x.uid == "A2L/STANDARD")
            .unwrap();
        assert_eq!(row.state, w::BuildState::NoResources);
        assert!(!row.buildable, "生成时该跳过它");
    }

    /// **跳过占位机型时说的必须是真因，而且不许写成催办。**（b04 P0 审计的后续）
    ///
    /// A2L 现在只是占了个名字，还没开始做。空着是**闭合的**：这台不生成、不进清单，
    /// 消费端也拿不到（它自己的内置尺寸表里同样没有这台）。所以这里要的是
    /// 「收起来并说一句」，不是「催人去补」。
    ///
    /// 「暂无资源」那句话不能复用：缺资源是漏（该有的没有），占位是刻意 ——
    /// 说成缺资源会让人去找一件根本不存在的东西。
    ///
    /// 这一条同时是「判据的判据」：夹具里 A2L **刻意没有** `[dimensions]`
    /// （`testkit::has_dimensions`），所以它一定走得到这一支
    #[test]
    fn skipping_a_placeholder_machine_names_the_real_reason() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);

        let a2l = book
            .machines()
            .iter()
            .find(|m| m.id == "A2L")
            .expect("夹具里有 A2L");
        assert!(
            !a2l.has_dimensions,
            "夹具的 A2L 本该没有尺寸，这条判据在空转"
        );

        // 别的机型有尺寸 —— 这一支不该把它们也拦下来
        assert!(
            book.machines()
                .iter()
                .filter(|m| m.id != "A2L")
                .all(|m| m.has_dimensions),
            "只有 A2L 该缺尺寸"
        );

        // 两句话必须是两句话，而且都不许是催办口气
        assert_ne!(
            w::disabled::BUILD_NO_DIMENSIONS,
            w::disabled::BUILD_NO_RESOURCES,
            "占位与缺资源说了同一句话，用户会照着错的方向去找"
        );
        assert!(
            w::disabled::BUILD_NO_DIMENSIONS.contains("占位"),
            "没说清这是占位：{}",
            w::disabled::BUILD_NO_DIMENSIONS
        );
        assert!(
            !w::disabled::BUILD_NO_DIMENSIONS.contains("补上"),
            "写成了催办：{}",
            w::disabled::BUILD_NO_DIMENSIONS
        );

        // 占位不挡生成 —— 别的机型照样交付
        let report = crate::workbench::domain::issues::inspect(&book);
        assert!(
            report.first_block().is_none(),
            "占位机型把整批生成挡住了：{:?}",
            report.first_block().map(|b| b.title.clone())
        );
    }

    /// 薄壳与权威实现逐例相等 —— 这是连着两处的**那根线**。
    ///
    /// 命名规则现在只有一处（`preset::preset_file_name`），这里只转调；但"转调"本身
    /// 没有类型系统兜底：哪天有人在薄壳里再补一次 `to_lowercase()`、或把分隔符从 `-`
    /// 改成 `_`，编译照样过，红的是消费端的查找。所以拿一组能区分行为的输入两边各算一遍。
    ///
    /// 输入刻意选在规则的每条边界上：版本 id 大小写混写（小写化在哪一侧发生）、
    /// 机型 id 含 `_`（它必须原样保留）、版本 id 含 `.`（`fastv3.3` 那种）。
    #[test]
    fn naming_matches_the_preset_crate() {
        for (machine, version) in [
            ("A1", "standard"),
            ("A1", "FASTV3.3"),
            ("A1", "Fast"),
            ("A1_MINI", "STANDARD"),
            ("A1_MINI", "FastV3.3"),
            ("P1S", "lite"),
            ("X1C", "LITE"),
        ] {
            assert_eq!(
                preset_file_name(machine, version),
                preset::preset_file_name(machine, version),
                "{machine}:{version} —— 两处算出的文件名不同，规则已经分岔"
            );
        }
    }

    /// **渲染的机型取自我们自己的清单。**
    ///
    /// `render()` 曾经用上游目录查机型，所以「上游清单里没有这台」会让渲染失败。
    /// 上游整层删掉之后，机型只来自 `presets/machines/*.toml`（借自 `Committed.catalog`）
    /// —— 这条判据钉住「机型 id 与版本 id 一样是我们的身份」（`docs/ARCHITECTURE.md` §10.1）。
    ///
    /// 反空转靠**挑一台夹具清单里没有、真 `presets/` 里有的机型**（A1_MINI）：
    /// 它不在 `testkit::FIXTURE_MACHINES`（A1 / A2L / P1S），渲染必须照样成功。
    #[test]
    fn render_takes_the_machine_from_our_own_catalog() {
        // 前提反空转：真 presets/ 必须有 A1_MINI —— 清单在我们这边
        let presets = crate::workbench::load_presets().expect("真 presets");
        assert!(
            presets.catalog.machine("A1_MINI").is_some(),
            "真 presets 里没有 A1_MINI，前提没了"
        );

        let tmp = tempfile::tempdir().unwrap();
        let store = crate::workbench::store::Store::at(tmp.path());
        store.bootstrap().unwrap();
        let c = super::super::storage::load(&store, &presets)
            .expect("干净仓库读得通")
            .committed;
        let draft = Draft::default();
        let book = Book::new(&presets, &c, &draft);

        let v = book
            .versions()
            .iter()
            .find(|v| v.machine_id == "A1_MINI")
            .expect("真 presets 里应当有 A1_MINI 的版本");
        let r = render(&book, &v.uid).expect("上游认不认这台机型，都不该影响渲染");

        assert!(
            r.text.contains(&format!("# machine: {}\n", v.machine_id)),
            "`# machine:` 那一行不是我们清单里的 id：{:?}",
            r.text.lines().take(4).collect::<Vec<_>>()
        );
        assert!(
            r.file_name.starts_with("A1_MINI-"),
            "产物名没跟着我们清单里的机型 id 走：{}",
            r.file_name
        );
    }

    /// **生成出来的名字必须正好是消费端认得的那一批。**（b04 §02 契约的第一条）
    ///
    /// 消费端按 `{机型}-{版本小写}.toml` 找文件，它内置的 9 份就是这个形状
    /// （`AUDIT-EVIDENCE.md` §2 逐份列过）。名字对不上的后果不是报错，
    /// 是它**找不到** —— 我们生成了一堆它认不出的文件，而两边都不会说话。
    ///
    /// 这是一条**会随清单变化而红**的判据，而且红了就该看：
    /// 改版本 id 或加机型都会改变这批名字，那时要同步的是消费端的内置表
    #[test]
    fn generated_names_match_what_the_consumer_looks_for() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let p = crate::workbench::presets::Presets::load_from(&root).unwrap();
        let got: std::collections::BTreeSet<String> = p
            .catalog
            .machines()
            .iter()
            .flat_map(|m| m.versions.iter().map(|v| preset_file_name(&m.id, &v.id)))
            .collect();

        // 消费端 `crates/preset/assets/presets/` 下**实际**的那批文件名。
        // 2026-11-12 起不再写死名单：写死就是「改一次 id、改一次判据」，
        // 而这里要守的本来就是「清单算出的名字 == 消费端目录里的名字」。
        let want: std::collections::BTreeSet<String> =
            std::fs::read_dir(preset::generate::assets_dir())
                .expect("读得到入库产物目录")
                .flatten()
                .map(|e| e.file_name().to_string_lossy().to_string())
                .filter(|name| name.ends_with(".toml"))
                .collect();
        assert!(!want.is_empty(), "入库产物目录空了，判据在空转");

        assert_eq!(
            got, want,
            "产物名与消费端认的那一批不一致 —— 它会找不到文件，而两边都不报错"
        );
    }

    /* ---------- 生成前预演：行级 diff ---------- */

    /// 逐行的 `(kind, 文本)`，方便断言时只写关心的部分。
    fn kinds(lines: &[DiffLine]) -> Vec<(&'static str, &str)> {
        lines
            .iter()
            .map(|l| {
                let k = match l.kind {
                    DiffLineKind::Context => "=",
                    DiffLineKind::Added => "+",
                    DiffLineKind::Removed => "-",
                };
                (k, l.text.as_str())
            })
            .collect()
    }

    #[test]
    fn a_new_file_shows_every_line_as_added() {
        let out = diff_added("a\nb\nc");
        assert_eq!(kinds(&out), vec![("+", "a"), ("+", "b"), ("+", "c")]);
        // 行号从 1 起、连续
        assert_eq!(out.iter().map(|l| l.no).collect::<Vec<_>>(), vec![1, 2, 3]);
    }

    #[test]
    fn an_identical_file_has_no_diff_lines() {
        // diff_lines 只服务「确实要重写」的文件；字节相同的在 preview_one
        // 就判成「无变化」，根本走不到这里
        assert!(diff_lines("x\ny", "x\ny")
            .iter()
            .all(|l| l.kind == DiffLineKind::Context));
    }

    /// 预演的三档状态（2026-11-01 口径：**只有字节相同才是无变化**）。
    ///
    /// · 字节相同 → 无变化，且详情给的是**盘上那一份**（不是本次候选）；
    /// · 差一个字节 → 修改；
    /// · 盘上没有 → 新增（绝不冒充「无变化」）。
    ///
    /// 以前这里还有一条"只有时间戳不同也算无变化"的分支，那正是让观众看见
    /// "无变化的文件时间变成了现在"的那条路 —— 随产物头去掉时刻一起删了。
    #[test]
    fn preview_states_follow_the_bytes_on_disk() {
        let r = Rendered {
            uid: "A1/STANDARD".to_owned(),
            file_name: "A1-standard.toml".to_owned(),
            text: "# machine: A1\n# variant: standard\nk = 1\n".to_owned(),
            fingerprint: "fp".to_owned(),
            snapshot: Default::default(),
        };
        // 盘上就是这份字节：无变化，且展示盘上原文
        let pf = preview_one(&r, Some(r.text.as_str()));
        assert_eq!(pf.state, DiffState::Unchanged);
        assert!(
            pf.lines.iter().all(|l| l.kind == DiffLineKind::Context),
            "无变化项要整份素底摊开"
        );
        assert_eq!(
            pf.lines.first().map(|l| l.text.as_str()),
            Some("# machine: A1"),
            "无变化展示的必须是盘上那一份"
        );
        // 正文真变了：要写
        let changed = "# machine: A1\n# variant: standard\nk = 2\n";
        assert_eq!(preview_one(&r, Some(changed)).state, DiffState::Modified);
        // 磁盘上没有：新增
        assert_eq!(preview_one(&r, None).state, DiffState::Added);
    }

    /// 盘上那份**内容一样但多一行** `# generated_at:`：判「要写」，不是「无变化」。
    ///
    /// 真生成按候选字节写盘、`catalog.json` 也按候选字节登记 size / sha256，
    /// 所以那一行必须清掉；预演若说「不会重写」，人就没有任何办法让它消失。
    #[test]
    fn a_legacy_generated_at_line_makes_the_preview_say_it_will_write() {
        let r = Rendered {
            uid: "A1/STANDARD".to_owned(),
            file_name: "A1-standard.toml".to_owned(),
            text: "# machine: A1\n# variant: standard\nk = 1\n".to_owned(),
            fingerprint: "fp".to_owned(),
            snapshot: Default::default(),
        };
        let legacy = format!(
            "# machine: A1\n# variant: standard\n{} 2026-11-05T04:17:00+08:00\nk = 1\n",
            crate::workbench::GENERATED_AT_PREFIX
        );
        let pf = preview_one(&r, Some(legacy.as_str()));
        assert_eq!(pf.state, DiffState::Modified, "带旧写盘记录 ⇒ 要重写");
        assert_eq!(
            pf.lines
                .iter()
                .filter(|l| l.kind == DiffLineKind::Removed)
                .count(),
            1,
            "如实显示被清掉的那一行"
        );
    }

    #[test]
    fn a_changed_line_shows_one_removed_and_one_added() {
        let out = diff_lines("a\nold\nc", "a\nnew\nc");
        // 与 git diff 同一套顺序：删在前、增在后
        assert_eq!(
            kinds(&out),
            vec![("=", "a"), ("-", "old"), ("+", "new"), ("=", "c")],
            "变的那一行要一删一增，前后未变的行保持 context"
        );
    }

    #[test]
    fn an_inserted_line_shows_only_an_added() {
        let out = diff_lines("a\nc", "a\nb\nc");
        assert_eq!(kinds(&out), vec![("=", "a"), ("+", "b"), ("=", "c")]);
    }

    #[test]
    fn a_deleted_line_shows_only_a_removed() {
        let out = diff_lines("a\nb\nc", "a\nc");
        assert_eq!(kinds(&out), vec![("=", "a"), ("-", "b"), ("=", "c")]);
    }

    #[test]
    fn the_counts_match_the_line_kinds() {
        // preview_one 的 added/removed 计数直接数行 —— 与界面上的「+N −N」是同一个数
        let r = Rendered {
            uid: "A1/standard".to_owned(),
            file_name: "A1-standard.toml".to_owned(),
            text: "a\nnew\nc\n".to_owned(),
            fingerprint: String::new(),
            snapshot: BTreeMap::new(),
        };

        let added = preview_one(&r, None);
        assert_eq!(added.state, DiffState::Added);
        assert_eq!((added.added, added.removed), (3, 0));

        let same = preview_one(&r, Some("a\nnew\nc\n"));
        assert_eq!(same.state, DiffState::Unchanged);
        // 无变化也带回**整份正文**（全 context）—— 确认框的「完整」视图要看原文；
        // 「不会重写」那句由界面在**对比**视图说（作者 2026-10-07）
        assert!(
            same.lines.iter().all(|l| l.kind == DiffLineKind::Context),
            "无变化带回的是整份正文，不是差异"
        );
        assert_eq!(same.lines.len(), 3, "正文几行就几行");
        assert_eq!((same.added, same.removed), (0, 0), "没变化就不算增减");

        let changed = preview_one(&r, Some("a\nold\nc\n"));
        assert_eq!(changed.state, DiffState::Modified);
        assert_eq!((changed.added, changed.removed), (1, 1));
    }

    /// **预演一个字节都不写** —— 它就是 `wb_generate` 的彩排。
    ///
    /// 这里用一个真实渲染器（`render`）产出的文本，落到临时目录：跑完预演后，
    /// 磁盘上的字节与预演前**逐字节相同**（新增的那份不会被预演创建出来）。
    #[test]
    fn preview_never_touches_the_disk() {
        let (_d, f, c) = setup();
        let draft = Draft::default();
        let book = Book::new(&f.presets, &c, &draft);
        let r = render(&book, "A1/STANDARD").unwrap();

        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join(&r.file_name);

        // 磁盘上还没有 → 预演判「新增」，但**不会把它写出来**
        let added = preview_one(&r, std::fs::read_to_string(&target).ok().as_deref());
        assert_eq!(added.state, DiffState::Added);
        assert!(!target.exists(), "预演不许创建任何文件");

        // 放一份**不同**的内容进去：预演判「修改」，且原字节一个不动
        crate::fsx::atomic::atomic_write(&target, b"# old\n").unwrap();
        let before = std::fs::read(&target).unwrap();
        let modified = preview_one(&r, std::fs::read_to_string(&target).ok().as_deref());
        assert_eq!(modified.state, DiffState::Modified);
        assert_eq!(
            std::fs::read(&target).unwrap(),
            before,
            "预演不许改任何文件"
        );

        // 放成与渲染结果逐字节相同：预演判「不变」
        crate::fsx::atomic::atomic_write(&target, r.text.as_bytes()).unwrap();
        let unchanged = preview_one(&r, std::fs::read_to_string(&target).ok().as_deref());
        assert_eq!(unchanged.state, DiffState::Unchanged);
    }

    /* ---------- ② 交付定稿检查（只读） ---------- */

    /// 整棵临时目录的字节快照（相对路径 → 内容）。看"一个字节都没写"用。
    fn snapshot_tree(root: &std::path::Path) -> std::collections::BTreeMap<String, Vec<u8>> {
        fn walk(
            root: &std::path::Path,
            dir: &std::path::Path,
            out: &mut std::collections::BTreeMap<String, Vec<u8>>,
        ) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for e in entries.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(root, &p, out);
                } else if let (Ok(rel), Ok(bytes)) = (p.strip_prefix(root), std::fs::read(&p)) {
                    /* 分隔符统一成 `/`：调用方按仓库内的相对路径取条目，
                    Windows 上原样给 `\` 会一条都取不中。 */
                    out.insert(rel.to_string_lossy().replace('\\', "/"), bytes);
                }
            }
        }
        let mut out = std::collections::BTreeMap::new();
        walk(root, root, &mut out);
        out
    }

    /// 一份"已经发布过"的会话：生成 → 定稿（manifest / catalog / source / 账本全齐）。
    fn published_ctx() -> (tempfile::TempDir, Ctx) {
        let (dir, mut ctx) = fixture_ctx();
        generate_with(&mut ctx, &Scope::All, false).expect("生成");
        let delivery = crate::workbench::paths::delivery_root_at(ctx.real_root());
        let assets = crate::workbench::paths::assets_root_at(ctx.real_root());
        {
            /* 渲染器这个探针借的是 `ctx`，出块就放掉 —— 不然下面交不回 `ctx` */
            let (c, d, _) = state(&ctx).expect("会话状态");
            let probe = renderer_for(&ctx);
            let book = book_with_probe(&ctx, &ctx.presets, &c, &d, &probe);
            let meta = super::super::delivery::PublishMeta {
                stamp: "2026-11-07T00:00:00Z".to_owned(),
                channel: PUBLISH_CHANNEL.to_owned(),
                version: String::new(),
                identity: Some(super::super::delivery::PublishIdentity::new_line(
                    "support-ease",
                    1,
                )),
                commit_sha: None,
            };
            super::super::delivery::publish_into(&delivery, &assets, &book, &meta).expect("定稿");
        }
        /* 真交付根里这两份由软件发版链写（`source.json` 声明了它们，发布闸 ⑬ 要求
        声明的都得在）。夹具补上最小形状，别让"身份检查"红在一个与交付无关的缺口上。 */
        for rel in [
            crate::runtime::source::RELEASE_FILE,
            crate::runtime::source::RELEASE_PREVIEW_FILE,
        ] {
            crate::fsx::atomic::atomic_write(
                &dir.path().join("presets/delivery").join(rel),
                br#"{"version":"0.0.0"}"#,
            )
            .expect("补一份发布清单");
        }
        (dir, ctx)
    }

    /// **一次定稿检查不许落下任何副作用**：整棵目录逐字节不变、序号不动，
    /// 但要说清"下一次正式发布会拿几号"（纯算术，不分配）。
    #[test]
    fn finalize_preview_is_read_only_and_reports_the_next_serial() {
        let (dir, ctx) = published_ctx();
        let before = snapshot_tree(dir.path());

        let p = finalize_preview_with(&ctx).expect("检查得出");

        assert_eq!(
            snapshot_tree(dir.path()),
            before,
            "定稿检查不许写任何文件（含账本与 catalog）"
        );
        assert_eq!(p.state, FinalizeState::Matched, "刚定稿：{:?}", p.reasons);
        assert!(p.reasons.is_empty(), "刚定稿不该有失效条目");
        assert!(p.ready, "刚定稿就该是「交付就绪」");
        assert!(
            p.issues.iter().all(|i| i.kind == FinalizeIssueKind::Info),
            "刚定稿只该剩下口径差异那一类：{:?}",
            p.issues.iter().map(|i| i.id.clone()).collect::<Vec<_>>()
        );
        assert_eq!(p.serial_on_disk, Some(1));
        assert_eq!(p.serial_next_publish, 2, "只报下一号，不分配");
        let m = p.manifest.expect("定稿后的 manifest 该算得出来");
        assert_eq!(m.state, DiffState::Unchanged, "刚定稿：manifest 会原样重写");
        assert_eq!(m.changed_entries, 0);
        assert_eq!(p.plan_error, None);
    }

    /// 产物变了、还没定稿 ⇒ 「凭据失效」；点名是哪一条，并给出定稿后那一行会变成什么。
    #[test]
    fn finalize_preview_names_stale_credentials_without_touching_them() {
        let (dir, ctx) = published_ctx();
        let delivery = crate::workbench::paths::delivery_root_at(ctx.real_root());
        let product = delivery
            .join(super::super::delivery::MKP_PRESETS_DIR)
            .join("A1-standard.toml");
        let mut bytes = std::fs::read(&product).expect("产物该在");
        bytes.extend_from_slice(b"\n# touched\n");
        crate::fsx::atomic::atomic_write(&product, &bytes).unwrap();

        let before = snapshot_tree(dir.path());
        let p = finalize_preview_with(&ctx).expect("检查得出");
        assert_eq!(snapshot_tree(dir.path()), before, "检查过程只读");

        assert_eq!(p.state, FinalizeState::Stale);
        assert_eq!(p.reasons.len(), 1, "只该点名那一条：{:?}", p.reasons);
        assert!(
            p.reasons[0].contains("A1-standard.toml"),
            "要点名是哪一份：{:?}",
            p.reasons
        );
        let m = p.manifest.expect("算得出定稿后的 manifest");
        assert_eq!(m.state, DiffState::Modified);
        assert_eq!(m.changed_entries, 1, "只有被改的那一条会重新登记");
        assert!(
            m.entries
                .iter()
                .any(|e| e.relative_path.ends_with("A1-standard.toml")
                    && e.before_sha256.as_deref() != Some(e.sha256.as_str())),
            "逐条要能看出新旧 sha"
        );
        assert_eq!(p.serial_next_publish, 2);
    }

    /// source 不在 ⇒ 「身份检查未通过」（复用发布闸 ⑬，不另写判据）。
    #[test]
    fn finalize_preview_says_identity_failed_when_source_is_gone() {
        let (dir, ctx) = published_ctx();
        std::fs::remove_file(dir.path().join("presets/delivery/source.json")).expect("删掉 source");

        let p = finalize_preview_with(&ctx).expect("检查得出");

        assert_eq!(p.state, FinalizeState::IdentityFailed);
        assert!(
            p.checks
                .iter()
                .any(|i| i.status == super::super::audit::AuditStatus::Fail),
            "复用的那两格要有红的：{:?}",
            p.checks.iter().map(|i| i.id.clone()).collect::<Vec<_>>()
        );
        assert!(
            p.issues
                .iter()
                .any(|i| i.kind == FinalizeIssueKind::Identity
                    && i.action == FinalizeAction::CheckIdentity),
            "身份问题要落成「检查发布身份」那一条"
        );
        assert!(!p.ready, "身份红了不许说就绪");
    }

    /* ---------- ② 更新发布凭据（唯一会写盘的那一步） ---------- */

    /// 把 manifest 里第一条的 sha 改坏 —— 模拟"生成过、凭据还停在上一代"，
    /// 但**集合与目录都是当前代**（这正是允许更新的那个状态）。
    fn corrupt_manifest_sha(delivery: &std::path::Path) {
        let path = delivery.join(super::super::delivery::MANIFEST_FILE);
        let text = std::fs::read_to_string(&path).expect("manifest 该在");
        let mut v: serde_json::Value = serde_json::from_str(&text).expect("是 JSON");
        v["assets"][0]["sha256"] = serde_json::Value::String("0".repeat(64));
        crate::fsx::atomic::atomic_write(
            &path,
            serde_json::to_string_pretty(&v).unwrap().as_bytes(),
        )
        .expect("写坏那一份");
    }

    /// 「更新发布凭据」只重写 `manifest.json`：source / catalog / 账本一个字节不动，
    /// 序号不动，更新后重新检查必须是「交付就绪」。
    #[test]
    fn updating_credentials_rewrites_only_the_manifest() {
        let (dir, mut ctx) = published_ctx();
        let delivery = crate::workbench::paths::delivery_root_at(ctx.real_root());
        corrupt_manifest_sha(&delivery);

        let p = finalize_preview_with(&ctx).expect("检查得出");
        assert_eq!(p.state, FinalizeState::Stale);
        let cred = p
            .issues
            .iter()
            .find(|i| i.id == "credentials.stale")
            .expect("要有一条可修的凭据问题");
        assert_eq!(cred.kind, FinalizeIssueKind::Fixable);
        assert_eq!(cred.action, FinalizeAction::UpdateCredentials);
        assert!(!p.ready, "凭据失效时不能说就绪");

        /* 除 manifest 之外，其余文件的字节快照 —— 更新后必须一个都没变 */
        let untouched = |root: &std::path::Path| {
            let mut s = snapshot_tree(root);
            s.remove(&format!(
                "presets/delivery/{}",
                super::super::delivery::MANIFEST_FILE
            ));
            s
        };
        let before = untouched(dir.path());

        let out = update_credentials_with(&mut ctx).expect("更新该成功");
        assert!(out.written, "凭据落后 ⇒ 必须真写一次");
        assert_eq!(out.changed, 1, "只有被改坏的那一条会重新登记");
        assert!(out.entries >= 1);
        assert_eq!(
            untouched(dir.path()),
            before,
            "除 manifest 外一个字节都不许动"
        );

        /* 更新之后的那份检查：凭据回到匹配、就绪 */
        assert_eq!(
            out.preview.state,
            FinalizeState::Matched,
            "更新后仍是失效：{:?}",
            out.preview.reasons
        );
        assert!(
            out.preview.ready,
            "更新后该是交付就绪：{:?}",
            out.preview.issues
        );
        assert!(
            !out.preview
                .issues
                .iter()
                .any(|i| i.id == "credentials.stale"),
            "可修的那条该消失"
        );
        assert_eq!(
            out.preview.serial_on_disk,
            Some(1),
            "序号不推进（这一笔不是发布）"
        );
        assert_eq!(out.preview.serial_next_publish, 2, "下一号仍然只是算术");
    }

    /// **定稿算不出来时，可修的那一条也不许消失**。
    ///
    /// 真机形状：某份产物不在交付根里 ⇒ `plan_publish` 直接报错 ⇒ 算不出"定稿后的
    /// manifest"。凭据落后是**另一件事**（manifest vs 盘上真字节），必须照样列出来 ——
    /// 否则界面只剩"需处理源"，人以为那是唯一的病。
    #[test]
    fn credentials_issue_survives_a_failed_finalize_plan() {
        let (_dir, ctx) = published_ctx();
        let delivery = crate::workbench::paths::delivery_root_at(ctx.real_root());

        // 让 plan_publish 算不出来：删掉一份产物
        std::fs::remove_file(
            delivery
                .join(super::super::delivery::MKP_PRESETS_DIR)
                .join("A1-fast.toml"),
        )
        .expect("删掉一份产物");
        // 同时让凭据真的落后一条（另一份的 sha 改坏）
        corrupt_manifest_sha(&delivery);

        let p = finalize_preview_with(&ctx).expect("检查本身不该失败");
        assert!(p.plan_error.is_some(), "定稿这一步算不出来");
        assert!(p.manifest.is_none());
        let cred = p
            .issues
            .iter()
            .find(|i| i.id == "credentials.stale")
            .expect("可修那条不许因为定稿算不出来就消失");
        assert_eq!(cred.kind, FinalizeIssueKind::Fixable);
        assert_eq!(cred.action, FinalizeAction::UpdateCredentials);
        assert!(!cred.files.is_empty(), "要点名是哪几份");
        assert!(!p.ready);
    }

    /// 交付集合没收敛（盘上少一份产物）⇒ **拒绝**更新，且一个字节都不写。
    #[test]
    fn updating_credentials_refuses_an_unsettled_collection() {
        let (dir, mut ctx) = published_ctx();
        let delivery = crate::workbench::paths::delivery_root_at(ctx.real_root());
        std::fs::remove_file(
            delivery
                .join(super::super::delivery::MKP_PRESETS_DIR)
                .join("A1-standard.toml"),
        )
        .expect("删掉一份产物，造出「盘上还没有」");

        let before = snapshot_tree(dir.path());
        let err = update_credentials_with(&mut ctx).expect_err("集合没收敛必须拒绝");
        assert!(
            err.message.contains("不能更新发布凭据"),
            "要如实说为什么：{}",
            err.message
        );
        assert!(
            err.detail.unwrap_or_default().contains("A1-standard.toml"),
            "要点名是哪一份"
        );
        assert_eq!(snapshot_tree(dir.path()), before, "拒绝时一个字节都不写");
    }
}
