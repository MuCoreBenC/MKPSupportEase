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

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::domain::{BbsAssign, Visibility};
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
    /// 交付身份三态。可见性（含草稿态）压过「进没进套餐」，两件事正交
    pub assign: BbsAssign,
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

/// 交付身份三态。与 `Book::stock_rows` 那条同口径：可见性压过「进没进套餐」
fn assign_of(vis: Visibility, in_bundle: bool) -> BbsAssign {
    if vis == Visibility::ArchiveOnly {
        BbsAssign::ArchiveOnly
    } else if in_bundle {
        BbsAssign::Assigned
    } else {
        BbsAssign::Optional
    }
}

/// 视图组装。筛选全在后端做；`None` 的轴表示「这一轴不过滤」。
/// 命令与测试共用 —— 过滤判据只写这一遍
#[allow(clippy::too_many_arguments)]
fn list_of(
    presets: &Presets,
    committed: &std::collections::BTreeMap<String, Visibility>,
    draft: &std::collections::BTreeMap<String, Visibility>,
    kind: Option<AssetKind>,
    slicer: Option<&str>,
    nozzle: Option<&str>,
    layer: Option<&str>,
    assign: Option<BbsAssign>,
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
    // 全量的三态计数（页脚读数，不随筛选变）与逐条的归属，一次循环算完
    let mut total = 0usize;
    let mut optional_count = 0usize;
    let mut archive_count = 0usize;
    let mut assign_of_all: std::collections::BTreeMap<String, BbsAssign> =
        std::collections::BTreeMap::new();
    for a in presets.assets.items() {
        total += 1;
        let vis = draft
            .get(&a.id)
            .or_else(|| committed.get(&a.id))
            .copied()
            .unwrap_or(Visibility::Menu);
        let asg = assign_of(vis, bundled.contains(&a.id.to_lowercase()));
        match asg {
            BbsAssign::Optional => optional_count += 1,
            BbsAssign::ArchiveOnly => archive_count += 1,
            BbsAssign::Assigned => {}
        }
        assign_of_all.insert(a.id.to_lowercase(), asg);
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
            assign.map_or(true, |want| {
                assign_of_all.get(&a.id.to_lowercase()) == Some(&want)
            })
        })
        .filter(|a| {
            q.is_empty() || a.name.to_lowercase().contains(&q) || a.id.to_lowercase().contains(&q)
        })
        .map(|a| {
            let (nozzle, layer) = slicer_axes(a.kind, &a.path);
            AssetView {
                id: a.id.clone(),
                kind: a.kind,
                machine_id: a.machine_id.clone(),
                name: a.name.clone(),
                path: a.path.clone(),
                url: format!("/assets/{}", a.path),
                slicer: a.slicer.clone(),
                profile: a.profile.clone(),
                present: presets.assets.present(a),
                nozzle,
                layer,
                assign: assign_of_all
                    .get(&a.id.to_lowercase())
                    .copied()
                    .unwrap_or(BbsAssign::Optional),
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

/// 资产库清单。类型 / 三根轴 / 交付身份 / 搜索全部在这里筛
#[tauri::command]
pub fn wb_assets(
    kind: Option<String>,
    slicer: Option<String>,
    nozzle: Option<String>,
    layer: Option<String>,
    assign: Option<String>,
    query: Option<String>,
) -> Result<AssetList, AppError> {
    traced("wb_assets", |_| {
        with_ctx(|ctx| {
            let (committed, draft, _) = state(ctx)?;
            let kind = kind.as_deref().and_then(parse_kind);
            let assign = assign.as_deref().and_then(parse_assign);
            Ok(list_of(
                &ctx.presets,
                &committed.visibility,
                &draft.visibility,
                kind,
                slicer.as_deref(),
                nozzle.as_deref(),
                layer.as_deref(),
                assign,
                query.as_deref(),
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
            Ok(list_of(
                &ctx.presets,
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
#[tauri::command]
pub fn wb_asset_usage(asset_id: String) -> Result<AssetUsageView, AppError> {
    traced("wb_asset_usage", |_| {
        let presets = Presets::load()?;
        let usage = presets.asset_usage(&asset_id)?;
        Ok(AssetUsageView {
            id: asset_id.clone(),
            machines: usage.machines,
            bundles: usage.bundles,
        })
    })
}

fn parse_kind(s: &str) -> Option<AssetKind> {
    [
        AssetKind::Image,
        AssetKind::Icon,
        AssetKind::Model,
        AssetKind::SlicerProfile,
    ]
    .into_iter()
    .find(|k| k.key() == s)
}

fn parse_assign(s: &str) -> Option<BbsAssign> {
    match s {
        "assigned" => Some(BbsAssign::Assigned),
        "optional" => Some(BbsAssign::Optional),
        "archiveOnly" => Some(BbsAssign::ArchiveOnly),
        _ => None,
    }
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
            19,
            "机型图 4 + 图标 3 + 模型 3 + BBS 9 —— 条数变了就该在提交里说清为什么"
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

    /// 交付身份三态在真数据上的分布：5 条套餐各引 1 条 BBS → 恰好 5 条已分配；
    /// 机型图 / 图标 / 没进套餐的 BBS → 可选。**没人动过可见性，不该有仅归档**
    #[test]
    fn the_real_assign_states_match_bundle_membership() {
        let list = wb_assets(None, None, None, None, None, None).expect("真 presets 读得通");
        let assigned = list
            .assets
            .iter()
            .filter(|a| a.assign == BbsAssign::Assigned)
            .count();
        assert_eq!(
            assigned, 5,
            "5 条套餐各引一条 BBS —— 进套餐的就 5 条；条数变了就核对 bundles.toml 再改这里的期望"
        );
        assert!(
            list.assets
                .iter()
                .all(|a| a.assign != BbsAssign::ArchiveOnly),
            "没人动过可见性 —— 出现仅归档说明草稿/交付层串了"
        );
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

        // 机型图各归各的
        let u = wb_asset_usage("a1-image".to_owned()).expect("反查");
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
