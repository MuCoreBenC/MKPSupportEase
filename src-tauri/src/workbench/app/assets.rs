//! 「资产库」的后端：读 `presets/assets.toml`（资产域①层，b05 Task 8）。
//!
//! # P4 起：读视图带派生与筛选 + 删除
//!
//! 这一页的正源是**资产域定义**（21 条：机型图 / 图标 / 模型 / BBS），不是
//! `wb_stock` 那份上游交付镜像 —— C14 那页的列表、引用反查、套餐 join 说的都是
//! 资产域的事（套餐 assetRefs 与这里同一 id 空间）。由此：
//!
//! - **切片器三根轴**（C14 第二十八轮：切片器 BBS/Orca · 喷嘴 · 层高）在后端做：
//!   `slicer` 是登记字段；喷嘴与层高资产定义刻意不存（doc §12.5 那条「不存第二份
//!   真相」），由 [`slicer_axes`] 从路径与文件名派生 —— 判据与原型
//!   `derive.slicerMetaOf` 同一条，只是搬进了后端（「前端不算业务」）。
//! - **交付身份三态** [`AssetView::assign`] 在后端算：可见性（含草稿态）压过
//!   「进没进套餐」，与 `wb_stock` 行上的三态同一条口径。
//! - **筛选参数**走命令参数（同 `wb_matrix` 的 `tab` / `query`），选项表
//!   （喷嘴 / 层高的候选值）从**全部**切片器条目取 —— 选中某个值不能让其他选项消失。
//! - **删除**（[`wb_remove_asset`]）：反查守卫在数据层
//!   （[`crate::workbench::presets::Presets::remove_asset`]），有人引用整次拒绝。
//!   即时落盘、不走参数草稿（清单编辑与「改一个数」不是一类撤销语义）。
//!
//! # `present` 为什么现在就报
//!
//! 条目与文件一起落地之后它普遍是 true。这不是错误值：界面要能说出
//! "这一条登记了、文件还没搬"，而不是显示一张空图让人猜。
//!
//! # URL 由后端给，前端负责编码
//!
//! `url` 是 `/assets/<path>`（vite 的 `public/` 直通）。**路径里可能有空格**
//! （实测 BBS 文件名就是 `MKPProcess A1 0.2 0.10.json`），所以前端用它之前要
//! `encodeURI` —— 前缀只有这一处，别在 TSX 里再拼一遍。

use serde::Serialize;
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::domain::wording::{AssetIdentity, BuildState};
use crate::workbench::domain::Visibility;
use crate::workbench::load_presets;
use crate::workbench::paths;
use crate::workbench::presets::{AssetKind, Presets};

use super::{state, with_ctx, with_ctx_mut};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetView {
    pub id: String,
    pub kind: AssetKind,
    /// 归属机型；不属于任何机型时是 `null`
    pub machine_id: Option<String>,
    /// 归属版本（**只 `mkPreset` 类有** —— 那一类靠「机型 + 版本」定位它的产物）。
    /// 界面拿它跳到「机型与版本」页的那一版（改名字改的是同一个字段）
    pub version_id: Option<String>,
    /// **给人看的名字 —— 一律用真名**（作者 2026-10-03：「这些预设的名字不是真实的
    /// 名字，我希望显示原本的真实名字 / 版本名称」）：
    ///
    /// | 类 | 显示名 | 真源（能改的地方） |
    /// |---|---|---|
    /// | `mkPreset` | **版本名称**（`标准版` / `快拆版6月以前`） | 机型与版本页的「版本名称」 |
    /// | `slicerProfile` | **文件名**（`MKPProcess A1 mini 0.4 0.20`） | 资产根里的文件 |
    /// | 其余三类 | 台账登记的 `name` | 资产台账 |
    ///
    /// 界面显示这一格；[`Self::name`] 保留登记名作副行对照。派生不出来时如实
    /// 回落登记名（不显示空白）
    pub display: String,
    /// 台账登记的名字（`presets/assets.toml` 的 `name`）。**不是显示真源** ——
    /// 留着是为了详情卡能对照「登记名叫什么」
    pub name: String,
    /// 相对资产根的一段
    pub path: String,
    /// 前端可直接用的 URL（**用之前 encodeURI**）
    pub url: String,
    pub slicer: Option<String>,
    pub profile: Option<String>,
    /// 文件在不在。**还没搬的话它是 false** —— 那是状态，不是错
    pub present: bool,
    /// 切片器三根轴之二：喷嘴。从路径的 `0.4mm` 那一段派生；只有切片器条目有
    pub nozzle: Option<String>,
    /// 三根轴之三：层高。从文件名尾部的数字派生（`… 0.10.json` → `0.10`）
    pub layer: Option<String>,
    /// **交付身份四态**（作者 2026-10-03 定的模型，判定在 [`identity_of`]）：
    /// 进套餐 / 可选 / 随包 / 仅归档
    pub identity: AssetIdentity,
    /// **交付档位**（作者 2026-10-03）：`download` = 客户端按需下载（默认）、
    /// `bundled` = **随包不下载**（整机图：台账里可管、可换图，客户端不下载）。
    /// 四态视图里的「随包」就是它给的；与可见性正交，视图按优先级合成一个身份
    pub delivery: crate::presetdata::assets::Delivery,
    /// **生成状态**（作者 2026-10-03，只 `mkPreset` 类有）：这一版的产物处于
    /// 「未生成 / 待更新 / 已生成 / 暂无资源」哪一档 —— 与生成页**同一套判据**
    /// （快照指纹比对，不看文件时间）。资产库列表与详情照它显示徽章：
    /// 没生成过 = 待生成；生成过、参数改完还没重新生成 = **待更新**。
    /// 其余四类恒为 `null`（它们的"文件在不在"由 `present` 说）
    pub build_state: Option<crate::workbench::domain::wording::BuildState>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetList {
    pub assets: Vec<AssetView>,
    /// 资产根的绝对路径，显示在状态条上
    pub root: String,
    /// 喷嘴轴的候选值。从**全部**切片器条目取，不随筛选变 ——
    /// 选中「0.4」之后别的选项还得在
    pub nozzles: Vec<String>,
    /// 层高轴的候选值。同上
    pub layers: Vec<String>,
    /// 过滤前一共几条 —— 页脚「筛出 X / Y 个」的 Y
    pub total: usize,
    /// 全量里的「可选」条数（页脚读数，不随筛选变）
    pub optional_count: usize,
    /// 全量里的「仅归档」条数（页脚读数，不随筛选变）
    pub archive_count: usize,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetUsageView {
    pub id: String,
    /// 直接引用它的机型（`image` / `icon` 字段写着这个 id）
    pub machines: Vec<String>,
    /// 引用它的套餐（`assetRefs` 写着这个 id 的套餐 id，b05 Task 10 起）。
    /// **归属不是引用**：`p1s-icon` 归 P1S，但借它当图标的另有其人
    pub bundles: Vec<String>,
}

/// 资产**检查面板**（作者 2026-10-03 锁定的字段清单）：把详情卡从"登记表"
/// 变成"资产检查面板" —— 盘上那个文件到底是什么样，它自己说。
///
/// # 为什么不并进 [`AssetView`]（列表里那一条）
///
/// 这几格里最贵的两格（`sha256` / `bytes`）要**读真实字节** —— 实测模型
/// 3.2 MB + 377 KB + 284 KB。而列表每次筛选 / 搜索词一变就重取（搜索框逐键触发），
/// 把哈希算进列表就是"每敲一个字读 4 MB"。所以它单独一条读命令：
/// **选中才问**（与 [`wb_asset_usage`] 同一形状）。
///
/// # 两条线各说各的落点
///
/// - 普通资产（有 `path`）：`absPath` = **源文件**（`presets/assets/<path>`），
///   工作台永远读源（与交付档位无关）；
/// - `mkPreset`（没有源文件，是生成产物）：`fileName` = **产物名**（命名规则算出）、
///   `absPath` = 产物文件、`productPath` = 产物相对仓库根的一段。
///
/// **文件不存在时 `absPath` 同时就是"期望路径"** —— 界面按 `exists` 换标题，
/// 不去猜第二个落点。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInspectView {
    pub id: String,
    /// 真实文件名（盘上那个名字）。普通资产 = `path` 的文件名；
    /// `mkPreset` = **产物名**（`A1-fastv3.3.toml`）
    pub file_name: String,
    /// 盘上绝对路径。普通资产 = 源文件；`mkPreset` = 产物文件。
    /// **文件不存在时它就是期望路径**
    pub abs_path: String,
    /// 文件在不在这条链的期望位置（普通 = 源文件；`mkPreset` = 产物文件）
    pub exists: bool,
    /// 文件大小（字节）。不存在时 `None`
    pub bytes: Option<u64>,
    /// **源文件字节**的 SHA-256（小写 hex，与交付侧同一算法）。不存在时 `None`
    pub sha256: Option<String>,
    /// 图片像素宽 / viewBox 宽（webp / png / svg 读得出时给）。非图片或读不出 = `None`
    pub width: Option<u32>,
    /// 同上：高
    pub height: Option<u32>,
    /// 格式（小写扩展名：`webp` / `svg` / `3mf` / `json` / `toml`…）。
    /// 没有扩展名时 `None`
    pub format: Option<String>,
    /// 产物**相对仓库根**的一段（仅 `mkPreset`）：`presets/delivery/mkp/presets/<产物名>`。
    /// 绝对的那一份在 [`Self::abs_path`]
    pub product_path: Option<String>,
}

/// 切片器三根轴之二三（C14 `derive.slicerMetaOf` 同一条判据，搬进后端）：
///
/// - 喷嘴：路径里 `0.2mm` 那一段（去掉 `mm` 后还得是数字，防止把 `mm` 目录误伤）；
/// - 层高：文件名尾部那串数字（`MKPProcess A1 0.2 0.10.json` → `0.10`）。
///
/// 只对切片器条目有意义，其他类型返回 `(None, None)`。
fn slicer_axes(kind: AssetKind, path: &str) -> (Option<String>, Option<String>) {
    if kind != AssetKind::SlicerProfile {
        return (None, None);
    }
    let numeric = |s: &str| {
        !s.is_empty()
            && s.bytes().all(|b| b.is_ascii_digit() || b == b'.')
            && s.bytes().any(|b| b.is_ascii_digit())
    };
    let nozzle = path
        .split(['/', '\\'])
        .find_map(|seg| seg.strip_suffix("mm"))
        .filter(|s| numeric(s))
        .map(str::to_owned);
    // 文件名尾部：先摘掉 .json 扩展名，再取结尾那串数字与点（"… 0.2 0.10" → "0.10"）
    let file = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = file
        .rfind('.')
        .filter(|&i| file[i + 1..].eq_ignore_ascii_case("json"))
        .map_or(file, |i| &file[..i]);
    let layer = stem
        .trim_end_matches('.')
        .rsplit_once(' ')
        .map(|(_, tail)| tail.trim())
        .filter(|tail| numeric(tail))
        .map(str::to_owned);
    (nozzle, layer)
}

/// **显示名的派生，只写这一处**（作者 2026-10-03：资产库的名字要用真名）。
///
/// - `mkPreset` → **版本名称**（`machines/{机型}.toml` 的 `versions[].name`）。
///   台账里那条 `name`（当初写死的「A1 标准版预设」）是**会过期的第二份真相** ——
///   版本改名之后它不会跟着变，所以不拿它当显示名。真源改的地方是「机型与版本」页
///   的版本名称，两边显示的是同一个值（后端也是同一个字段）。
/// - `slicerProfile` → **文件名去掉扩展名**（`MKPProcess A1 mini 0.4 0.20.json`
///   → `MKPProcess A1 mini 0.4 0.20`）。台账里手写的「A1 mini：0.4 喷头 0.20 层高」
///   同样不是真名。
/// - 其余三类（整机图 / 图标 / 模型）→ 台账登记名（它们没有别的真源）。
///
/// 派生不出来（版本名空 / 文件没有 path）时如实回落到登记名，不显示空白
pub(super) fn display_name(presets: &Presets, a: &crate::workbench::presets::Asset) -> String {
    match a.kind {
        AssetKind::MkPreset => {
            let (Some(mid), Some(vid)) = (
                a.machine_id.as_deref().map(str::trim),
                a.version_id.as_deref().map(str::trim),
            ) else {
                return a.name.clone();
            };
            presets
                .catalog
                .machine(mid)
                .and_then(|m| m.versions.iter().find(|v| v.id == vid))
                .map(|v| v.name.trim().to_owned())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| a.name.clone())
        }
        AssetKind::SlicerProfile => a
            .path
            .rsplit('/')
            .next()
            .unwrap_or(&a.path)
            .rsplit_once('.')
            .map(|(stem, _)| stem)
            .filter(|s| !s.is_empty())
            .unwrap_or(&a.name)
            .to_owned(),
        _ => a.name.clone(),
    }
}

/// **四态身份的判定，只写这一处**（作者 2026-10-03 定的模型）：
/// 归档 > 随包 > 进套餐 > 可选。可见性与交付档位在数据上正交，
/// 视图按这个优先级合成一个身份说话
fn identity_of(
    vis: Visibility,
    delivery: crate::presetdata::assets::Delivery,
    in_bundle: bool,
) -> AssetIdentity {
    if vis == Visibility::ArchiveOnly {
        AssetIdentity::ArchiveOnly
    } else if delivery == crate::presetdata::assets::Delivery::Bundled {
        AssetIdentity::Bundled
    } else if in_bundle {
        AssetIdentity::InBundle
    } else {
        AssetIdentity::Optional
    }
}

/// 视图组装。筛选全在后端做；`None` 的轴表示「这一轴不过滤」。
/// 命令与测试共用 —— 过滤判据只写这一遍
#[allow(clippy::too_many_arguments)]
fn list_of(
    presets: &Presets,
    book: &crate::workbench::domain::derive::Book<'_>,
    committed: &std::collections::BTreeMap<String, Visibility>,
    draft: &std::collections::BTreeMap<String, Visibility>,
    kind: Option<AssetKind>,
    slicer: Option<&str>,
    nozzle: Option<&str>,
    layer: Option<&str>,
    identity: Option<AssetIdentity>,
    query: Option<&str>,
) -> AssetList {
    // 装着它的套餐集合（bundles.toml 反查，大小写不敏感）
    let bundled: std::collections::BTreeSet<String> = presets
        .bundles
        .items()
        .iter()
        .flat_map(|b| b.asset_refs.iter().map(|r| r.trim().to_lowercase()))
        .collect();

    // 选项表从全部切片器条目取 —— 不随筛选变
    let mut nozzles: Vec<String> = Vec::new();
    let mut layers: Vec<String> = Vec::new();
    for a in presets.assets.items() {
        let (n, l) = slicer_axes(a.kind, &a.path);
        if let Some(n) = n {
            nozzles.push(n);
        }
        if let Some(l) = l {
            layers.push(l);
        }
    }
    sort_numeric(&mut nozzles);
    sort_numeric(&mut layers);

    let q = query.unwrap_or_default().trim().to_lowercase();
    // 全量的四态计数（页脚读数，不随筛选变）与逐条的身份，一次循环算完
    let mut total = 0usize;
    let mut optional_count = 0usize;
    let mut archive_count = 0usize;
    let mut identity_of_all: std::collections::BTreeMap<String, AssetIdentity> =
        std::collections::BTreeMap::new();
    for a in presets.assets.items() {
        total += 1;
        let vis = draft
            .get(&a.id)
            .or_else(|| committed.get(&a.id))
            .copied()
            .unwrap_or(Visibility::Menu);
        let idt = identity_of(vis, a.delivery, bundled.contains(&a.id.to_lowercase()));
        match idt {
            AssetIdentity::Optional => optional_count += 1,
            AssetIdentity::ArchiveOnly => archive_count += 1,
            _ => {}
        }
        identity_of_all.insert(a.id.to_lowercase(), idt);
    }

    let assets = presets
        .assets
        .items()
        .iter()
        .filter(|a| kind.map_or(true, |k| a.kind == k))
        .filter(|a| slicer.map_or(true, |s| a.slicer.as_deref() == Some(s)))
        .filter(|a| {
            let (n, l) = slicer_axes(a.kind, &a.path);
            nozzle.map_or(true, |x| n.as_deref() == Some(x))
                && layer.map_or(true, |x| l.as_deref() == Some(x))
        })
        .filter(|a| {
            identity.map_or(true, |want| {
                identity_of_all.get(&a.id.to_lowercase()) == Some(&want)
            })
        })
        .filter(|a| {
            q.is_empty() || a.name.to_lowercase().contains(&q) || a.id.to_lowercase().contains(&q)
        })
        .map(|a| {
            let (nozzle, layer) = slicer_axes(a.kind, &a.path);
            // 生成状态：只有 MKP 预设类有（那一版的产物处于哪一档）
            let build_state = if a.kind == AssetKind::MkPreset {
                let uid = format!(
                    "{}/{}",
                    a.machine_id.as_deref().unwrap_or_default(),
                    a.version_id.as_deref().unwrap_or_default()
                );
                Some(book.build_state(&uid))
            } else {
                None
            };
            AssetView {
                id: a.id.clone(),
                kind: a.kind,
                machine_id: a.machine_id.clone(),
                version_id: a.version_id.clone(),
                display: display_name(presets, a),
                name: a.name.clone(),
                path: a.path.clone(),
                url: format!("/assets/{}", a.path),
                slicer: a.slicer.clone(),
                profile: a.profile.clone(),
                nozzle,
                layer,
                identity: identity_of_all
                    .get(&a.id.to_lowercase())
                    .copied()
                    .unwrap_or(AssetIdentity::Optional),
                delivery: a.delivery,
                // MKP 预设在资产库里就是「那一版的登记」—— 文件在不在看生成状态
                // （`present` 对它没有意义：产物不在资产根里）
                present: match a.kind {
                    AssetKind::MkPreset => build_state.is_some_and(|s| s == BuildState::Built),
                    _ => presets.assets.present(a),
                },
                build_state,
            }
        })
        .collect();
    AssetList {
        assets,
        root: paths::assets_root().map_or_else(|_| String::new(), |p| p.display().to_string()),
        nozzles,
        layers,
        total,
        optional_count,
        archive_count,
    }
}

/// 「0.2 / 0.4 / 1.0」按数值排，不按字典序（"10" < "2" 是错的顺序）
fn sort_numeric(values: &mut Vec<String>) {
    values.sort_by(|a, b| {
        let (x, y) = (
            a.parse::<f64>().unwrap_or(0.0),
            b.parse::<f64>().unwrap_or(0.0),
        );
        x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)
    });
    values.dedup();
}

/// 资产库清单。类型 / 三根轴 / 交付身份四态 / 搜索全部在这里筛
#[tauri::command(async)]
pub fn wb_assets(
    kind: Option<String>,
    slicer: Option<String>,
    nozzle: Option<String>,
    layer: Option<String>,
    identity: Option<String>,
    query: Option<String>,
) -> Result<AssetList, AppError> {
    traced("wb_assets", |_| {
        with_ctx(|ctx| {
            let (committed, draft, _) = state(ctx)?;
            let kind = kind.as_deref().and_then(parse_kind);
            let identity = identity.as_deref().and_then(parse_identity);
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                kind,
                slicer.as_deref(),
                nozzle.as_deref(),
                layer.as_deref(),
                identity,
                query.as_deref(),
            ))
        })
    })
}

/// 改一条资产的交付档位（`download` ↔ `bundled`）。**即时落盘**（清单编辑，
/// 不走参数草稿），回一份新的清单。
///
/// 校验：资产要存在；**mkPreset 不许设成 bundled** —— 它的文件是生成产物、
/// 落点在交付根 `mkp/presets/`，没有「随包复制」这条路径
#[tauri::command]
pub fn wb_set_asset_delivery(asset_id: String, delivery: String) -> Result<AssetList, AppError> {
    traced("wb_set_asset_delivery", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.set_asset_delivery(&asset_id, &delivery)?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
                None,
                None,
                None,
                None,
                None,
            ))
        })
    })
}

/// 删一条资产。**反查守卫在数据层**：有人引用（机型 image/icon、套餐 assetRefs）
/// 整次拒绝并说清是谁 —— 界面把它转成拦截页。即时落盘，回一份新的清单
#[tauri::command]
pub fn wb_remove_asset(asset_id: String) -> Result<AssetList, AppError> {
    traced("wb_remove_asset", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.remove_asset(&asset_id)?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
                None,
                None,
                None,
                None,
                None,
            ))
        })
    })
}

/// 「谁在用它」（b05 Task 9.4）。删资产之前先问这一条 ——
/// 删掉一张还被机型引用着的图，界面上只表现为"那台机型的图没了"
#[tauri::command(async)]
pub fn wb_asset_usage(asset_id: String) -> Result<AssetUsageView, AppError> {
    traced("wb_asset_usage", |_| {
        let presets = load_presets()?;
        let usage = presets.asset_usage(&asset_id)?;
        Ok(AssetUsageView {
            id: asset_id.clone(),
            machines: usage.machines,
            bundles: usage.bundles,
        })
    })
}

/// 一条资产在**盘上**的落点：期望文件名 + 绝对路径 + （仅 `mkPreset`）产物相对路径。
///
/// - 普通资产（有 `path`）：绝对路径 = `presets/assets/<path>` ——
///   工作台永远读源，与 `delivery` 无关（作者定死的五层里的第①层）；
/// - `mkPreset`：没有源文件，落点是**生成产物** —— 产物名走命名规则
///   （[`super::build::preset_file_name`]，与生成端**同一个函数**，不是抄一份），
///   落在交付根 `mkp/presets/` 下（`sourceRoot + path` 那条路对它不成立）。
///
/// 文件存不存在**不在这里判** —— 它只算"该在哪"，`exists` 由调用方 `is_file()` 说话
/// （所以这条路径同时就是"期望路径"）。
fn target_of(
    presets: &Presets,
    asset: &crate::workbench::presets::Asset,
) -> Result<(String, std::path::PathBuf, Option<String>), AppError> {
    if asset.kind == AssetKind::MkPreset {
        let (Some(mid), Some(vid)) = (
            asset.machine_id.as_deref().map(str::trim),
            asset.version_id.as_deref().map(str::trim),
        ) else {
            return Err(AppError::invalid_argument(format!(
                "「{}」是 MKP 预设但没写 machineId / versionId，算不出产物名",
                asset.id
            )));
        };
        let name = super::build::preset_file_name(mid, vid);
        let abs = paths::delivery_root_path()
            .join(paths::MKP_PRESETS_DIR)
            .join(&name);
        // 相对那一段给界面看 / 给人复制：`presets/delivery/mkp/presets/<产物名>`
        let rel = paths::delivery_root_path()
            .strip_prefix(paths::repo_root())
            .map(|p| {
                p.join(paths::MKP_PRESETS_DIR)
                    .join(&name)
                    .display()
                    .to_string()
            })
            .unwrap_or_else(|_| abs.display().to_string());
        Ok((name, abs, Some(rel)))
    } else {
        let abs = presets.assets.resolve(asset)?;
        let name = asset
            .path
            .rsplit(['/', '\\'])
            .next()
            .filter(|s| !s.is_empty())
            .unwrap_or(&asset.id)
            .to_owned();
        Ok((name, abs, None))
    }
}

/// 一条资产的**检查面板**读数（见 [`AssetInspectView`]）。命令与测试共用。
///
/// 只有文件真在时才读字节：不存在就照实给"期望路径 + 空读数"，不去编大小 / 哈希。
fn inspect_of(
    presets: &Presets,
    asset: &crate::workbench::presets::Asset,
) -> Result<AssetInspectView, AppError> {
    let (file_name, abs, product_path) = target_of(presets, asset)?;
    let exists = abs.is_file();
    let format = abs
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .filter(|s| !s.is_empty());
    let (bytes, sha256, width, height) = if exists {
        let data = std::fs::read(&abs).map_err(|e| {
            AppError::io(format!("读不动「{}」", abs.display())).with_detail(e.to_string())
        })?;
        let dims = format.as_deref().and_then(|f| image_dims(f, &data));
        (
            Some(data.len() as u64),
            Some(sha256_hex(&data)),
            dims.map(|(w, _)| w),
            dims.map(|(_, h)| h),
        )
    } else {
        (None, None, None, None)
    };
    Ok(AssetInspectView {
        id: asset.id.clone(),
        file_name,
        abs_path: abs.display().to_string(),
        exists,
        bytes,
        sha256,
        width,
        height,
        format,
        product_path,
    })
}

/// 字节的 SHA-256（小写 hex）—— 与交付侧（`app::dist`）同一套写法，
/// 因为检查面板要报的就是"交付账上那个哈希的源"
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// 图片尺寸（像素）。**只认我们有把握的三种**：png / webp / svg。
///
/// 自己读文件头（几十行）而不是拖进 `image` 这种整库解码器 ——
/// 我们要的只是宽高两个数，不是像素；读不出（或格式不认识）就如实 `None`，
/// **不猜**（界面那一格空着比编一个数诚实）。
fn image_dims(format: &str, data: &[u8]) -> Option<(u32, u32)> {
    match format {
        "png" => png_dims(data),
        "webp" => webp_dims(data),
        "svg" => svg_dims(data),
        _ => None,
    }
}

/// PNG：签名 + 第一个块必须是 IHDR，宽高是大端 u32（偏移 16 / 20）
fn png_dims(data: &[u8]) -> Option<(u32, u32)> {
    const SIG: [u8; 8] = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
    if data.len() < 24 || data[..8] != SIG || &data[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes(data[16..20].try_into().ok()?);
    let h = u32::from_be_bytes(data[20..24].try_into().ok()?);
    (w > 0 && h > 0).then_some((w, h))
}

/// WebP：三种块头（VP8X 画布 / VP8 有损帧 / VP8L 无损）
fn webp_dims(data: &[u8]) -> Option<(u32, u32)> {
    if data.len() < 25 || &data[..4] != b"RIFF" || &data[8..12] != b"WEBP" {
        return None;
    }
    let chunk: &[u8; 4] = data[12..16].try_into().ok()?;
    let payload = &data[20..];
    match chunk {
        b"VP8X" if payload.len() >= 10 => {
            let w = 1 + u32::from_le_bytes([payload[4], payload[5], payload[6], 0]);
            let h = 1 + u32::from_le_bytes([payload[7], payload[8], payload[9], 0]);
            Some((w, h))
        }
        b"VP8 " if payload.len() >= 10 && payload[3..6] == [0x9d, 0x01, 0x2a] => {
            let w = u16::from_le_bytes([payload[6], payload[7]]) as u32 & 0x3fff;
            let h = u16::from_le_bytes([payload[8], payload[9]]) as u32 & 0x3fff;
            (w > 0 && h > 0).then_some((w, h))
        }
        b"VP8L" if payload.len() >= 5 && payload[0] == 0x2f => {
            let bits = u32::from_le_bytes([payload[1], payload[2], payload[3], payload[4]]);
            Some(((bits & 0x3fff) + 1, ((bits >> 14) & 0x3fff) + 1))
        }
        _ => None,
    }
}

/// SVG：优先 `viewBox` 的第 3 / 4 个数（原始坐标尺寸），回落 `width` / `height`
/// 属性（去掉单位）。只看文件头 4 KB —— viewBox 不可能写在文件尾巴上
fn svg_dims(data: &[u8]) -> Option<(u32, u32)> {
    let head = &data[..data.len().min(4096)];
    let text = String::from_utf8_lossy(head);
    if let Some(value) = attr_value(&text, "viewBox") {
        let nums: Vec<f64> = value
            .split(|c: char| c.is_whitespace() || c == ',')
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        if let (Some(w), Some(h)) = (nums.get(2), nums.get(3)) {
            return positive_dims(*w, *h);
        }
    }
    let w = attr_value(&text, "width").and_then(|v| parse_length(&v))?;
    let h = attr_value(&text, "height").and_then(|v| parse_length(&v))?;
    positive_dims(w, h)
}

/// 从 XML/HTML 味道的文本里取一个属性的值（引号成对；找不到 = `None`）
fn attr_value(text: &str, name: &str) -> Option<String> {
    let at = text.find(name)?;
    let rest = &text[at + name.len()..];
    let rest = rest.get(rest.find('=')? + 1..)?;
    let quote = rest.chars().find(|c| *c == '"' || *c == '\'')?;
    let start = rest.find(quote)? + 1;
    let end = rest[start..].find(quote)? + start;
    Some(rest[start..end].to_string())
}

/// `"48"` / `"48px"` / `"48.5"` → 数值；带别的单位（`%` / `em`）视为读不出
fn parse_length(value: &str) -> Option<f64> {
    let v = value.trim().trim_end_matches("px").trim();
    v.parse::<f64>().ok()
}

/// 只收正整数尺寸（viewBox 允许小数，四舍五入；≤0 视为读不出）
fn positive_dims(w: f64, h: f64) -> Option<(u32, u32)> {
    let (w, h) = (w.round(), h.round());
    (w > 0.0 && h > 0.0).then_some((w as u32, h as u32))
}

/// **资产检查面板**：选中一条才问（见 [`AssetInspectView`] 的两条理由）。只读
#[tauri::command(async)]
pub fn wb_asset_inspect(asset_id: String) -> Result<AssetInspectView, AppError> {
    traced("wb_asset_inspect", |_| {
        let presets = load_presets()?;
        let asset = presets
            .assets
            .get(&asset_id)
            .ok_or_else(|| AppError::not_found(format!("没有资产「{asset_id}」")))?;
        inspect_of(&presets, asset)
    })
}

/// 「在访达中显示」的落点：**必须真在**才给（不在就如实拒绝，附期望路径 ——
/// 不去猜、也不打开一个没选中的空目录）。
fn reveal_target(
    presets: &Presets,
    asset: &crate::workbench::presets::Asset,
) -> Result<std::path::PathBuf, AppError> {
    let (name, target, _) = target_of(presets, asset)?;
    if !target.is_file() {
        return Err(
            AppError::not_found(format!("「{name}」的文件不在，没什么可显示的"))
                .with_detail(format!("期望路径：{}", target.display())),
        );
    }
    Ok(target)
}

/// 「在访达中显示」（Windows 上是资源管理器）：打开系统文件管理器**并选中**这一条。
///
/// 纪律（照客户端 `ipc::mine::reveal_in_folder` 那一套，作者 2026-10-03 锁定）：
///
/// - **前端只传资产 id**，路径由后端自己算（源根 + 台账 `path` / 产物路径）——
///   不给前端传任意路径的机会；
/// - **只读、只开窗口、一个状态都不碰** —— 定位是文件管理器的活，不是我们的；
/// - 文件不在就如实拒绝（附期望路径），不去猜、也不打开一个没选中的空目录；
/// - 插件只在 Rust 侧调（`tauri-plugin-opener`），所以不需要在
///   `capabilities/workbench.json` 里开权限（与客户端那条同一条纪律）。
#[tauri::command(async)]
pub fn wb_reveal_asset(app: tauri::AppHandle, asset_id: String) -> Result<(), AppError> {
    traced("wb_reveal_asset", |_| {
        let presets = load_presets()?;
        let asset = presets
            .assets
            .get(&asset_id)
            .ok_or_else(|| AppError::not_found(format!("没有资产「{asset_id}」")))?;
        let target = reveal_target(&presets, asset)?;
        app.opener()
            .reveal_item_in_dir(&target)
            .map_err(|e| AppError::io("打不开系统文件管理器").with_detail(e.to_string()))
    })
}

fn parse_kind(s: &str) -> Option<AssetKind> {
    [
        AssetKind::Image,
        AssetKind::Icon,
        AssetKind::Model,
        AssetKind::SlicerProfile,
        // mkPreset 2026-10-03 进了台账 —— 漏了它的话「MKP 预设」那档会被当成
        // 「这一轴不过滤」，列表把整机图 / 图标全放出来（作者截图点名）
        AssetKind::MkPreset,
    ]
    .into_iter()
    .find(|k| k.key() == s)
}

fn parse_identity(s: &str) -> Option<AssetIdentity> {
    [
        AssetIdentity::InBundle,
        AssetIdentity::Optional,
        AssetIdentity::Bundled,
        AssetIdentity::ArchiveOnly,
    ]
    .into_iter()
    .find(|k| k.key() == s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::testkit::Fixture;

    /// **真数据上的一条**：条目数与文件都在（b05 Task 9 的验收）。
    ///
    /// 它比 `presets::tests` 那条多看一层：命令返回的 DTO 是不是也对
    /// （`present`、`url` 前缀、类型都在）。
    #[test]
    fn the_real_asset_list_is_complete_and_present() {
        let list = wb_assets(None, None, None, None, None, None).expect("真 presets 读得通");
        assert_eq!(
            list.assets.len(),
            29,
            "MKP 预设 9 + 整机图 4 + 品牌字标 1 + 图标 3 + 模型 3 + BBS 9 ——
             2026-10-03：整机图回到台账（bundled 档）、MKP 预设也进了台账（mkPreset 类）、
             品牌 logo 也正式进了台账（`bambu-lab-logo`，公共素材：不写 machineId）。
             条数变了要说清为什么"
        );

        let missing: Vec<&str> = list
            .assets
            .iter()
            .filter(|a| !a.present)
            .map(|a| a.id.as_str())
            .collect();
        assert!(missing.is_empty(), "这些资产的文件不在：{missing:?}");

        assert!(
            list.assets.iter().any(|a| a.kind == AssetKind::Model),
            "模型这一类要在（裁决：保留 model 类型）"
        );
        assert!(
            list.assets.iter().all(|a| a.url.starts_with("/assets/")),
            "URL 前缀只有后端一处，别在 TSX 里再拼"
        );
        assert!(
            list.assets.iter().all(|a| !a.name.trim().is_empty()),
            "每条都要有给人看的名字"
        );
    }

    /// **切片器三根轴**（C14 第二十八轮，P4 搬进后端）：真数据里 BBS 条目的
    /// 喷嘴 / 层高从路径与文件名派生得出来，选项表按数值排序且不随筛选变
    #[test]
    fn the_real_slicer_axes_derive_from_path_and_name() {
        let list = wb_assets(None, None, None, None, None, None).expect("真 presets 读得通");

        let bbs: Vec<&AssetView> = list
            .assets
            .iter()
            .filter(|a| a.kind == AssetKind::SlicerProfile)
            .collect();
        assert_eq!(bbs.len(), 9, "实测 9 份 BBS —— 条数变了就说清为什么");
        for a in &bbs {
            assert!(
                a.nozzle.is_some() && a.layer.is_some(),
                "{} 的轴派生不出来：nozzle={:?} layer={:?}（path={}）",
                a.id,
                a.nozzle,
                a.layer,
                a.path
            );
        }
        let a1 = bbs.iter().find(|a| a.id == "a1-bbs-04-020").unwrap();
        assert_eq!(a1.nozzle.as_deref(), Some("0.4"));
        assert_eq!(a1.layer.as_deref(), Some("0.20"));

        // 选项表：喷嘴两档（0.2 / 0.4），层高三档（P1S / P2S / X1C 的 0.4 喷头是 0.24），
        // 按数值序；且带筛选重取时仍在
        assert_eq!(list.nozzles, vec!["0.2".to_owned(), "0.4".to_owned()]);
        assert_eq!(
            list.layers,
            vec!["0.10".to_owned(), "0.20".to_owned(), "0.24".to_owned()]
        );
        let filtered = wb_assets(
            None,
            Some("bbs".to_owned()),
            Some("0.4".to_owned()),
            None,
            None,
            None,
        )
        .expect("按喷嘴筛");
        assert_eq!(
            filtered.nozzles,
            vec!["0.2".to_owned(), "0.4".to_owned()],
            "选项表不随筛选变 —— 选中一个值不能让别的选项消失"
        );
        assert!(filtered
            .assets
            .iter()
            .all(|a| a.nozzle.as_deref() == Some("0.4")));
    }

    /// **类型轴每档都筛得干净**：选中一档就只出那一档。
    /// mkPreset 那档是作者截图点名的 bug —— parse_kind 漏了它，选中「MKP 预设」
    /// 被当成「不过滤」，整机图 / 图标全放出来
    #[test]
    fn the_kind_axis_filters_each_kind_cleanly() {
        let all = wb_assets(None, None, None, None, None, None).expect("真 presets 读得通");
        let want_of = [
            // image 档 5 = 整机图 4 + 品牌字标 1（2026-10-03 品牌图正式进台账）
            (AssetKind::Image, 5usize),
            (AssetKind::Icon, 3),
            (AssetKind::Model, 3),
            (AssetKind::SlicerProfile, 9),
            (AssetKind::MkPreset, 9),
        ];
        for (kind, want) in want_of {
            let got = wb_assets(Some(kind.key().to_owned()), None, None, None, None, None)
                .unwrap_or_else(|e| panic!("按 {} 筛：{e}", kind.key()));
            assert_eq!(
                got.assets.len(),
                want,
                "{} 档条数变了就说清为什么",
                kind.key()
            );
            assert!(
                got.assets.iter().all(|a| a.kind == kind),
                "{} 档里混进了别的类型：{:?}",
                kind.key(),
                got.assets
                    .iter()
                    .filter(|a| a.kind != kind)
                    .map(|a| a.id.as_str())
                    .collect::<Vec<_>>()
            );
        }
        // 「全部」= 五档之和
        assert_eq!(
            all.assets.len(),
            want_of.iter().map(|(_, n)| n).sum::<usize>()
        );
    }

    /// **四态身份在真数据上的分布**（作者 2026-10-03 定的模型）：
    /// 进套餐 14（9 条 mkPreset + 5 条被套餐引用的 BBS）、随包 4（整机图）、
    /// 仅归档 0（没人动过可见性）、其余可选 10
    #[test]
    fn the_real_identity_states_match_the_model() {
        use AssetIdentity::{ArchiveOnly, Bundled, InBundle, Optional};
        let list = wb_assets(None, None, None, None, None, None).expect("真 presets 读得通");
        let count = |want: AssetIdentity| {
            list.assets
                .iter()
                .filter(|a| a.identity == want)
                .map(|a| a.id.as_str())
                .collect::<Vec<_>>()
        };
        let in_bundle = count(InBundle);
        assert_eq!(
            in_bundle.len(),
            14,
            "9 条 mkPreset + 5 条被套餐引用的 BBS —— 条数变了就核对 bundles.toml 再改这里的期望：{in_bundle:?}"
        );
        assert!(
            in_bundle.contains(&"a1-standard") && in_bundle.contains(&"a1-bbs-04-020"),
            "MKP 预设进套餐后也是「进套餐」身份"
        );
        let bundled = count(Bundled);
        assert_eq!(
            bundled.len(),
            5,
            "四条整机图 + 品牌字标是随包档（品牌图 2026-10-03 进台账）：{bundled:?}"
        );
        assert_eq!(count(ArchiveOnly).len(), 0, "没人动过可见性");
        assert_eq!(
            count(Optional).len(),
            10,
            "29 - 14 进套餐 - 5 随包 = 10 可选（图标 3 + 模型 3 + 没进套餐的 BBS 4）"
        );

        // 身份轴筛选：每一档筛出来都恰好是那一档
        let filtered =
            wb_assets(None, None, None, None, Some("bundled".to_owned()), None).expect("按随包筛");
        assert!(filtered
            .assets
            .iter()
            .all(|a| a.identity == AssetIdentity::Bundled));
        assert_eq!(
            filtered.assets.len(),
            5,
            "按随包身份筛 = 4 张整机图 + 品牌字标（品牌图 2026-10-03 进台账）"
        );
    }

    /// **检查面板在真数据上读的是真文件**（第四刀）：文件名 / 绝对路径 / SHA-256 /
    /// 尺寸都读得出来，一条不空。
    #[test]
    fn the_inspection_panel_reads_the_real_files() {
        let presets = load_presets().expect("真 presets 读得通");

        let image = presets.assets.get("a1-image").expect("a1-image");
        let v = inspect_of(&presets, image).expect("读得出");
        assert_eq!(v.file_name, "a1.webp", "真实文件名 = path 的文件名");
        assert!(
            std::path::Path::new(&v.abs_path).ends_with("printers/a1.webp"),
            "绝对路径要指到源文件：{}",
            v.abs_path
        );
        assert!(v.exists, "源文件在");
        assert!(v.product_path.is_none(), "普通资产没有产物路径");
        assert_eq!(v.format.as_deref(), Some("webp"));
        let bytes = v.bytes.expect("大小读得出");
        assert!(bytes > 0, "大小 {bytes}");
        let sha = v.sha256.as_deref().expect("哈希读得出");
        assert_eq!(sha.len(), 64, "SHA-256 是 64 位小写 hex：{sha}");
        assert!(sha
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()));
        assert!(
            v.width.is_some_and(|w| w > 0) && v.height.is_some_and(|h| h > 0),
            "webp 尺寸读得出：{:?} × {:?}",
            v.width,
            v.height
        );

        // svg 走 viewBox 那一支（含小数：品牌字标 485.05 × 175.15 → 四舍五入）
        let logo = presets.assets.get("bambu-lab-logo").expect("品牌字标");
        let v = inspect_of(&presets, logo).expect("读得出");
        assert_eq!(v.format.as_deref(), Some("svg"));
        assert_eq!(
            (v.width, v.height),
            (Some(485), Some(175)),
            "viewBox 的小数四舍五入，不是截断"
        );
    }

    /// **mkPreset 没有源文件，面板给的是产物**（第四刀）：产物名走命名规则、
    /// 相对路径给人复制、绝对路径落在交付根下。
    #[test]
    fn the_inspection_panel_locates_a_mk_preset_product() {
        let presets = load_presets().expect("真 presets 读得通");
        let a = presets.assets.get("a1-standard").expect("a1-standard");
        let v = inspect_of(&presets, a).expect("读得出");
        assert_eq!(v.file_name, "A1-standard.toml", "产物名 = 命名规则算出来的");
        assert_eq!(
            v.product_path.as_deref(),
            Some("presets/delivery/mkp/presets/A1-standard.toml"),
            "相对仓库根那一段是给人看的 / 复制的"
        );
        assert!(
            std::path::Path::new(&v.abs_path)
                .ends_with("presets/delivery/mkp/presets/A1-standard.toml"),
            "绝对路径：{}",
            v.abs_path
        );
        assert_eq!(v.format.as_deref(), Some("toml"));
        assert_eq!((v.width, v.height), (None, None), "toml 不是图片");
        assert!(v.exists, "交付根里那份产物在（dist 是入库的）");
        assert!(v.bytes.is_some_and(|b| b > 0), "产物大小读得出");
    }

    /// 不存在的 id 如实拒绝（命令这一层，不是空面板）
    #[test]
    fn inspecting_an_unknown_asset_says_so() {
        let err = wb_asset_inspect("no-such-asset".to_owned()).expect_err("不存在的 id");
        assert!(err.message.contains("没有资产"), "实测：{}", err.message);
    }

    /// **文件不在时如实说**：给期望路径 + 空读数（不编大小 / 哈希），
    /// 「在访达中显示」也据实拒绝。
    #[test]
    fn a_registered_file_that_is_not_there_reports_the_expected_path() {
        let mut f = Fixture::load();
        let root = f.presets.root().join("inspect-assets");
        std::fs::create_dir_all(&root).expect("建临时资产根");
        f.presets.set_asset_root(&root);

        let a = f.presets.assets.get("a1-image").expect("夹具里有 a1-image");
        let v = inspect_of(&f.presets, a).expect("文件不在不是错误，是读数");
        assert!(!v.exists);
        assert_eq!(v.file_name, "a1.webp", "真实文件名照给");
        assert!(
            std::path::Path::new(&v.abs_path).ends_with("inspect-assets/printers/a1.webp"),
            "这条路径同时就是期望路径：{}",
            v.abs_path
        );
        assert_eq!(
            (v.bytes, v.sha256, v.width, v.height),
            (None, None, None, None),
            "文件不在 → 读数全空，不猜"
        );
        assert_eq!(
            v.format.as_deref(),
            Some("webp"),
            "格式从期望文件名读，照样给"
        );

        let err = reveal_target(&f.presets, a).expect_err("文件不在就必须拒绝");
        assert!(err.message.contains("文件不在"), "实测：{}", err.message);
        assert!(
            err.detail.is_some_and(|d| d.contains("期望路径")),
            "拒绝时要附期望路径"
        );
    }

    /// 图片尺寸的三个读取器：**认得准的给数、认不出的如实 None**（不猜）。
    /// 纯字节输入 —— 不碰文件系统，也不依赖真数据里恰好有哪种格式。
    #[test]
    fn image_size_readers_are_honest_about_what_they_know() {
        // png：签名 + 第一个块 IHDR（宽高各 4 字节大端）
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        png.extend_from_slice(&13u32.to_be_bytes());
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&640u32.to_be_bytes());
        png.extend_from_slice(&480u32.to_be_bytes());
        assert_eq!(image_dims("png", &png), Some((640, 480)));

        // webp（VP8X 画布）：24 位宽-1 / 高-1，小端
        let mut webp = b"RIFF".to_vec();
        webp.extend_from_slice(&0u32.to_le_bytes());
        webp.extend_from_slice(b"WEBP");
        webp.extend_from_slice(b"VP8X");
        webp.extend_from_slice(&10u32.to_le_bytes());
        webp.extend_from_slice(&[0, 0, 0, 0]);
        webp.extend_from_slice(&[0xff, 0x03, 0x00]); // 1024 - 1
        webp.extend_from_slice(&[0xbf, 0x02, 0x00]); // 704 - 1
        assert_eq!(image_dims("webp", &webp), Some((1024, 704)));

        // webp（VP8L 无损）：14 位宽 + 14 位高包在一个 u32 里
        let mut lossless = b"RIFF".to_vec();
        lossless.extend_from_slice(&0u32.to_le_bytes());
        lossless.extend_from_slice(b"WEBP");
        lossless.extend_from_slice(b"VP8L");
        lossless.extend_from_slice(&5u32.to_le_bytes());
        lossless.push(0x2f);
        let bits: u32 = (32 - 1) | ((16 - 1) << 14);
        lossless.extend_from_slice(&bits.to_le_bytes());
        assert_eq!(image_dims("webp", &lossless), Some((32, 16)));

        // svg：viewBox 优先（小数四舍五入），回落 width / height 属性（去 px）
        assert_eq!(
            image_dims("svg", br#"<svg viewBox="0 0 48.5 24">"#),
            Some((49, 24))
        );
        assert_eq!(
            image_dims("svg", br#"<svg width="32px" height="16"></svg>"#),
            Some((32, 16))
        );

        // 认不出的：如实 None —— 界面那一格空着比编一个数诚实
        assert_eq!(image_dims("webp", b"RIFFxxxxWEBP????"), None);
        assert_eq!(image_dims("json", br#"{"width":100,"height":50}"#), None);
        assert_eq!(image_dims("svg", b"<svg></svg>"), None);
        assert_eq!(image_dims("png", b"not a png"), None);
    }

    /// 反查在真数据上也说得清是谁在用（b05 Task 9.4）
    #[test]
    fn the_real_usage_lookup_names_the_machines() {
        let u = wb_asset_usage("p1s-icon".to_owned()).expect("反查");
        assert_eq!(
            u.machines,
            vec!["P1S".to_owned(), "P2S".to_owned(), "X1C".to_owned()],
            "三个机型共用这一份图标（归属写 P1S，借用的是另外两台）"
        );
        assert!(u.bundles.is_empty(), "图标不是套餐的配发内容");

        // 一台机型一份图标：A1 只引自己那份（整机图 2026-10-01 已剥离台账，
        // 反查这一档改用图标举例）
        let u = wb_asset_usage("a1-icon".to_owned()).expect("反查");
        assert_eq!(u.machines, vec!["A1".to_owned()]);
    }

    /// 夹具上的**删除路径**：反查拦人引用的、没人引用的删得掉且真落盘。
    /// 删除守卫本体在数据层有判据，这里验的是命令这一层接得对
    #[test]
    fn removing_an_unreferenced_asset_deletes_the_entry() {
        let f = Fixture::load();
        // 直接走领域层（命令层要会话；判据同一条，别为它造一个会话出来）
        let mut presets = f.presets;
        let err = presets
            .remove_asset("a1-image")
            .expect_err("A1 的机型图被引用着，必须拦");
        assert!(err.message.contains("还被引用"), "实测：{}", err.message);

        // a1-extra-image 刻意没人引用 —— 删得掉，重读真的没了
        presets.remove_asset("a1-extra-image").expect("删掉");
        assert!(
            presets.assets.get("a1-extra-image").is_none(),
            "内存里要没了"
        );
        presets.assets.write().expect("写");
        let again = Presets::load_from(presets.root()).expect("重读");
        assert!(
            again.assets.get("a1-extra-image").is_none(),
            "落盘之后重读也没了"
        );
    }
}
