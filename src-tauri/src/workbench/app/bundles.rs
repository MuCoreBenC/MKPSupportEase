//! 「套餐管理」的后端：读 `presets/bundles.toml`（套餐域①层，b05 Task 10）。
//!
//! # P4 起：读视图 + 一处写
//!
//! 套餐唯一真源就是这份 `bundles.toml`（Task 13.6：发布 manifest v3 删掉了从上游
//! 透传的 `bundles` 字段 —— 交付侧 `content/bundles.json` 由这里直出）。这一层做两件事：
//!
//! 1. **读**（[`wb_bundles`]）：定义五字段，外加两块派生 ——
//!    - 每个 ref join 资产域的解析结果（类型 / 名字 / 文件在不在 / 交付身份）；
//!    - **谁在指向它**。指向模型是 C14 第十九轮定稿、作者 2026-09-30 拍板移植的
//!      **一版一套**：版本的 `recommendedBundle` 是主指向（[`BundleView::users`]），
//!      机型 `defaultBundle` 是生成侧的回退（[`BundleView::default_for`]），分开列 ——
//!      两档混在一起的话，「改指向会动到谁」就说不清了。
//! 2. **写**（[`wb_set_bundle_refs`]）：换一份套餐的文件清单。**即时落盘，不走参数草稿**
//!    —— 套餐定义与「改一个数」在撤销语义上不是一类事（同 `wb_set_version_field` 的
//!    取舍，见 `app/machines.rs` 头注）。
//!
//! # `Patch::SetBundle` overlay 的下落（记录在案）
//!
//! 交付侧那份按套餐 id 存 `{presets, bbs}` 清单的草稿 overlay 是 Task 7 时代的遗产
//! （当时套餐只存在于上游 manifest）。真源收口到 `bundles.toml` 之后它**不再有界面
//! 生产者**；判据层（`bundle_bbs` 的 overlay 优先）原样保留 —— 存量 store 里可能
//! 还躺着那份 overlay，读路径不能装它不存在。
//!
//! # 可见性为什么走会话
//!
//! ref 的交付身份（在菜单 / 仅归档）要**连草稿态一起**报出来：`wb_apply_draft`
//! 刚写进去还没保存的那笔，页面上必须立刻看见。所以这一层走 `Ctx`（committed ⊕
//! draft 合并），而不是像 Task 10 那样每次 `Presets::load()`。

use serde::Serialize;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::clock;
use crate::workbench::domain::derive::Book;
use crate::workbench::domain::patch::Visibility;
use crate::workbench::domain::wording::BuildState;
use crate::workbench::presets::{AssetKind, Presets};

use super::{state, with_ctx, with_ctx_mut};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleRefView {
    pub id: String,
    /// 资产类型。前端按它把 refs 分成 MKP / 切片器两组 —— 资产域只有这四种
    /// （MKP 预设不建资产条目，doc §12.5，所以 MKP 组恒空是 honest 的）
    pub kind: AssetKind,
    /// 能不能解析到一条真实资产。**加载期它是 error**，这里恒 true 是"没问题"，
    /// 不是"没查"
    pub resolvable: bool,
    /// 是不是 BBS 预设。**每条套餐至少一条 true**（doc §12.4：MKP 与 BBS 成套配发）
    pub is_bbs: bool,
    /// 资产域登记的名字。解析不到时是空串（加载期拦过了，真数据上不会出现）
    pub name: String,
    /// 文件在不在
    pub present: bool,
    /// 交付身份（在菜单 / 仅归档）。**含草稿态** —— 刚设还没保存的也要看得见
    pub visibility: Visibility,
}

/// 指向这份套餐的一个版本（一版一套的主指向：`recommendedBundle`）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleUserView {
    pub machine_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleView {
    pub id: String,
    pub display: String,
    pub machine_id: String,
    pub asset_refs: Vec<BundleRefView>,
    /// 配发的 MKP 预设（uid 直引，**文件可不存在** —— 作者 2026-10-03）
    pub presets: Vec<PresetRefView>,
    /// 上一次改动日期（迁移照抄旧值；真改动由 `set_refs` 盖新值）
    pub updated_at: Option<String>,
    /// **一版一套**：`recommendedBundle` 指着这份套餐的版本
    pub users: Vec<BundleUserView>,
    /// `defaultBundle` 指着它的机型 —— 生成侧「版本没自己指」时回退的就是这一档。
    /// 与 [`BundleView::users`] 分开列：改这份套餐的内容，这两批人都会跟着变
    pub default_for: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleList {
    pub bundles: Vec<BundleView>,
    /// 过滤前一共几份 —— 页脚「筛出 X / Y 个」的 Y
    pub total: usize,
    /// MKP 组的候选池：能生成的版本（作者 2026-10-03：套餐要能挂 MKP 预设，
    /// **文件不存在也能先挂** —— 预设是生成产物，生成之后文件才落）
    pub preset_candidates: Vec<PresetCandidate>,
}

/// 套餐里挂着的一条 MKP 预设（uid 直引，不经过资产库 —— doc §12.5 的本义）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetRefView {
    /// 版本 uid（`A1/STANDARD`），与 `bundles.toml` 的 `presets` 字段同形
    pub uid: String,
    /// 产物文件名（命名规则算出的那份）。没生成过也有 —— 名字是算出来的
    pub file_name: String,
    /// 磁盘上有没有这份产物。false = 挂了名字还没生成，不是错误
    pub generated: bool,
}

/// MKP 组的候选：一棵版本（能生成的），带当前生成态
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetCandidate {
    pub uid: String,
    /// 「机型 版本名」，给人看的
    pub name: String,
    pub file_name: String,
    /// 原始状态档（built / stale / neverBuilt / noResources），词由前端词表挑
    pub state: BuildState,
}

/// committed ⊕ draft 的可见性合并（与 `Book` 里 stock_rows 用的同一条口径）：
/// 草稿里有的压过已落盘的，两边都没有就是「在菜单」
fn visibility_of(
    committed: &std::collections::BTreeMap<String, Visibility>,
    draft: &std::collections::BTreeMap<String, Visibility>,
    id: &str,
) -> Visibility {
    draft
        .get(id)
        .or_else(|| committed.get(id))
        .copied()
        .unwrap_or(Visibility::Menu)
}

/// 视图组装。`query` 是 id / 显示名的子串筛选（大小写不敏感），命令与测试共用
fn list_of(
    presets: &Presets,
    book: &Book<'_>,
    committed: &std::collections::BTreeMap<String, Visibility>,
    draft: &std::collections::BTreeMap<String, Visibility>,
    query: Option<&str>,
) -> BundleList {
    let q = query.unwrap_or_default().trim().to_lowercase();
    let all = presets.bundles.items();
    // MKP 组的候选池：能生成的版本（含还没生成过的 —— 文件不存在也能先挂）
    let preset_candidates: Vec<PresetCandidate> = book
        .build_rows()
        .iter()
        .filter(|r| r.buildable)
        .map(|r| PresetCandidate {
            uid: r.uid.clone(),
            name: format!("{} {}", r.machine, r.name),
            file_name: r.mkp_file.clone().unwrap_or_default(),
            state: r.state,
        })
        .collect();
    let bundles = all
        .iter()
        .filter(|b| {
            q.is_empty()
                || b.id.to_lowercase().contains(&q)
                || b.display.to_lowercase().contains(&q)
        })
        .map(|b| {
            let refs = b
                .asset_refs
                .iter()
                .map(|r| {
                    let a = presets.assets.get(r);
                    BundleRefView {
                        id: r.clone(),
                        kind: a.map_or(AssetKind::SlicerProfile, |a| a.kind),
                        resolvable: a.is_some(),
                        is_bbs: a.is_some_and(|a| {
                            a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs")
                        }),
                        name: a.map_or(String::new(), |a| a.name.clone()),
                        present: a.is_some_and(|a| presets.assets.present(a)),
                        visibility: visibility_of(committed, draft, r),
                    }
                })
                .collect();
            let lower = b.id.to_lowercase();
            let mut users = Vec::new();
            let mut default_for = Vec::new();
            for m in presets.catalog.machines() {
                if m.default_bundle
                    .as_deref()
                    .is_some_and(|s| s.trim().to_lowercase() == lower)
                {
                    default_for.push(m.id.clone());
                }
                for v in &m.versions {
                    if v.recommended_bundle
                        .as_deref()
                        .is_some_and(|s| s.trim().to_lowercase() == lower)
                    {
                        users.push(BundleUserView {
                            machine_id: m.id.clone(),
                            version_id: v.id.clone(),
                        });
                    }
                }
            }
            BundleView {
                id: b.id.clone(),
                display: b.display.clone(),
                machine_id: b.machine_id.clone(),
                asset_refs: refs,
                // MKP 组：uid 直引。文件在不在照实说（挂了名字还没生成 = false，
                // 不是错误 —— 作者 2026-10-03：套餐可以先挂）
                presets: b
                    .presets
                    .iter()
                    .map(|uid| PresetRefView {
                        uid: uid.clone(),
                        file_name: book
                            .version(uid)
                            .map(|v| v.mkp_file.clone())
                            .unwrap_or_default(),
                        generated: book.build_state(uid) == BuildState::Built,
                    })
                    .collect(),
                updated_at: b.updated_at.clone(),
                users,
                default_for,
            }
        })
        .collect();
    BundleList {
        bundles,
        total: all.len(),
        preset_candidates,
    }
}

/// 套餐清单。refs 带解析结果与交付身份，指向关系按一版一套分两档报
#[tauri::command(async)]
pub fn wb_bundles(query: Option<String>) -> Result<BundleList, AppError> {
    traced("wb_bundles", |_| {
        with_ctx(|ctx| {
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            Ok(list_of(
                &ctx.presets,
                &book,
                &c.visibility,
                &d.visibility,
                query.as_deref(),
            ))
        })
    })
}

/// 换一份套餐的文件清单（P4 套餐内容编辑）。**即时落盘**，回一份新的清单。
///
/// 校验在领域层（[`crate::workbench::presets::Presets::set_bundle_refs`]）：
/// 每条 ref 都要解析到真实资产、每条 preset uid 都要解析到**本机型**的真版本
///（**文件不存在没关系** —— 预设是生成产物，作者 2026-10-03：套餐先挂名字）、
/// 改完至少一条 BBS、内容没变不写盘。
/// `updatedAt` 由那次写盖上当天 —— 内容变了就是改了套餐。
#[tauri::command]
pub fn wb_set_bundle_refs(
    bundle_id: String,
    asset_ids: Vec<String>,
    preset_uids: Vec<String>,
) -> Result<BundleList, AppError> {
    traced("wb_set_bundle_refs", |_| {
        with_ctx_mut(|ctx| {
            // 时间戳由工作台这一侧给（时间源只有 `clock` 一处，见它的模块头）
            ctx.presets.set_bundle_refs(
                &bundle_id,
                &asset_ids,
                &preset_uids,
                &clock::now_iso8601(),
            )?;
            let (c, d, _) = state(ctx)?;
            let book = Book::new(&ctx.presets, &c, &d);
            Ok(list_of(
                &ctx.presets,
                &book,
                &c.visibility,
                &d.visibility,
                None,
            ))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::app::assets::wb_asset_usage;

    /// **真数据上的一条**：5 份套餐、每条的引用都解析得到、每条至少一条 BBS。
    /// 旧仓 `updatedAt = '2026-07-12'` 照实搬 —— 迁移不改内容，日期照旧
    #[test]
    fn the_real_bundle_list_is_complete() {
        let list = wb_bundles(None).expect("真 presets 读得通");
        assert_eq!(list.total, 5);
        assert_eq!(
            list.bundles.len(),
            5,
            "旧仓实测 5 份套餐（A1 / A1_MINI / P1S / P2S / X1C 各一）—— 条数变了就说清为什么"
        );

        let a1 = list
            .bundles
            .iter()
            .find(|b| b.id == "A1_default")
            .expect("A1_default 必须在");
        assert_eq!(a1.display, "官方推荐");
        assert_eq!(a1.machine_id, "A1");
        assert_eq!(a1.updated_at.as_deref(), Some("2026-07-12"));
        assert_eq!(
            a1.asset_refs
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a1-bbs-04-020"],
            "旧仓 A1_default 的 assetRefs 实测就这一条（换了我们的资产 id）"
        );
        // join 资产域：类型 / 名字 / 文件在不在都要跟上来（P4 读视图的增量）
        let r = &a1.asset_refs[0];
        assert_eq!(r.kind, AssetKind::SlicerProfile);
        assert!(r.is_bbs && r.resolvable && r.present);
        assert!(!r.name.is_empty(), "ref 要带上给人看的名字");
        assert_eq!(r.visibility, Visibility::Menu, "没人动过可见性，就是在菜单");

        let mut bbs_total = 0usize;
        for b in &list.bundles {
            assert!(
                b.asset_refs.iter().all(|r| r.resolvable),
                "{} 有解析不到的引用 —— 加载期就该拦下，到这里还在说明检查没接上",
                b.id
            );
            assert!(
                b.asset_refs.iter().any(|r| r.is_bbs),
                "{} 一条 BBS 都没有（10.8：MKP 与 BBS 必须成套配发）",
                b.id
            );
            bbs_total += b.asset_refs.iter().filter(|r| r.is_bbs).count();
        }
        // 反空转：5 条套餐各引 1 条 BBS —— 少于 5 说明上面的判定路径没走通
        assert!(
            bbs_total >= 5,
            "BBS 引用只对上 {bbs_total} 条 —— 判据在空转"
        );
    }

    /// **一版一套的指向在真数据上说得清**（P4）：版本层 9 处非空引用、机型层 5 处
    /// defaultBundle —— 两档分开报，合在一起的话「改套餐会动到谁」就数不出来
    #[test]
    fn the_real_pointing_is_reported_in_two_tiers() {
        let list = wb_bundles(None).expect("真 presets 读得通");

        let users_total: usize = list.bundles.iter().map(|b| b.users.len()).sum();
        assert_eq!(
            users_total, 9,
            "实测 9 处非空的版本 recommendedBundle（A2L 那处空串不算）—— 条数变了就说清为什么"
        );
        let defaults_total: usize = list.bundles.iter().map(|b| b.default_for.len()).sum();
        assert_eq!(
            defaults_total, 5,
            "实测 5 处机型 defaultBundle —— 一台一条，条数变了就说清为什么"
        );

        let a1 = list
            .bundles
            .iter()
            .find(|b| b.id == "A1_default")
            .expect("A1_default 必须在");
        assert!(
            a1.users.iter().all(|u| u.machine_id == "A1"),
            "套餐归属 A1，指向它的版本也该都在 A1 名下"
        );
        assert_eq!(
            a1.default_for,
            vec!["A1".to_owned()],
            "A1 的 defaultBundle 指着它"
        );
    }

    /// 筛选（页脚的「筛出 X / Y 个」）：命中 id 与显示名，过滤前的总数不变
    #[test]
    fn the_query_filters_without_losing_the_total() {
        let hit = wb_bundles(Some("a1".to_owned())).expect("筛选");
        assert!(
            hit.bundles
                .iter()
                .all(|b| b.id.to_lowercase().contains("a1")),
            "命中的都该含 a1：{:?}",
            hit.bundles.iter().map(|b| b.id.clone()).collect::<Vec<_>>()
        );
        assert!(
            hit.bundles.len() < hit.total,
            "5 份套餐里 A1 / A1_MINI / P1S / P2S / X1C 各一，筛 a1 不该是全部"
        );
        assert_eq!(hit.total, 5);
        assert!(
            wb_bundles(Some("不存在的套餐".to_owned())).is_ok(),
            "筛不中是空清单不是错误"
        );
    }

    /// 反查的**套餐那一档**在真数据上说得清是谁（b05 Task 10）
    #[test]
    fn the_real_usage_lookup_names_the_bundles_too() {
        let u = wb_asset_usage("a1-bbs-04-020".to_owned()).expect("反查");
        assert!(u.machines.is_empty(), "BBS 预设不被机型直接引用");
        assert_eq!(
            u.bundles,
            vec!["A1_default".to_owned()],
            "引用它的套餐要说得出是谁"
        );

        // 图标那一份没有套餐引用（归属 ≠ 引用）：p1s-icon 归 P1S，用的是 P2S / X1C
        let u = wb_asset_usage("p1s-icon".to_owned()).expect("反查");
        assert!(u.bundles.is_empty(), "机型图标不该被套餐引用");
    }
}
