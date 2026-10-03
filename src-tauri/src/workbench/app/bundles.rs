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
//! 2. **写**（[`wb_set_bundle_refs`]）：换一份套餐的文件清单（`assetRefs` 一份，
//!    **MKP 预设也在里面** —— 那一类 2026-10-03 进了资产库，`type = 'mkPreset'`，
//!    文件在不在都能选用）。**即时落盘，不走参数草稿**
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
use crate::workbench::domain::patch::Visibility;
use crate::workbench::presets::{AssetKind, Presets};

use super::{state, with_ctx, with_ctx_mut};

/// 资产在界面上的**真名**（与资产库同一处派生：MKP 预设 = 版本名、切片器 = 文件名）。
/// 套餐里那些行显示的也是它 —— 两处不许各显示一份不一样的名字
fn display_name(presets: &Presets, a: &crate::workbench::presets::Asset) -> String {
    super::assets::display_name(presets, a)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleRefView {
    pub id: String,
    /// 资产类型。前端按它把 refs 分成 MKP / 切片器两组 ——
    /// MKP 预设 2026-10-03 进了资产库（`type = 'mkPreset'`），MKP 组不再恒空
    pub kind: AssetKind,
    /// 能不能解析到一条真实资产。**加载期它是 error**，这里恒 true 是"没问题"，
    /// 不是"没查"
    pub resolvable: bool,
    /// 是不是 BBS 预设。**每条套餐至少一条 true**（doc §12.4：MKP 与 BBS 成套配发）
    pub is_bbs: bool,
    /// 资产域登记的名字。解析不到时是空串（加载期拦过了，真数据上不会出现）
    pub name: String,
    /// 文件在不在。**MKP 预设类按生成状态判**（产物不在资产根里；
    /// 「已生成」= 生成页四档里的 `built`）—— 与资产库列表同一条口径
    pub present: bool,
    /// **生成状态**（只 MKP 预设类有）：这一版的产物处于哪一档，
    /// 词与判据都取生成页那一套（同资产库的徽章）。其余类恒 `None`
    pub build_state: Option<crate::workbench::domain::BuildState>,
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
    book: &crate::workbench::domain::derive::Book<'_>,
    committed: &std::collections::BTreeMap<String, Visibility>,
    draft: &std::collections::BTreeMap<String, Visibility>,
    query: Option<&str>,
) -> BundleList {
    let q = query.unwrap_or_default().trim().to_lowercase();
    let all = presets.bundles.items();
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
                    // MKP 预设的「文件在不在」是生成页那套判据（产物不在资产根，
                    // 按资产域判会恒 false）；与资产库列表同一条口径
                    let build_state = a.filter(|a| a.kind == AssetKind::MkPreset).map(|a| {
                        let uid = format!(
                            "{}/{}",
                            a.machine_id.as_deref().unwrap_or_default(),
                            a.version_id.as_deref().unwrap_or_default()
                        );
                        book.build_state(&uid)
                    });
                    BundleRefView {
                        id: r.clone(),
                        kind: a.map_or(AssetKind::SlicerProfile, |a| a.kind),
                        resolvable: a.is_some(),
                        is_bbs: a.is_some_and(|a| {
                            a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs")
                        }),
                        name: a.map_or(String::new(), |a| display_name(presets, a)),
                        present: match (&a, &build_state) {
                            (Some(a), None) => presets.assets.present(a),
                            (_, Some(s)) => *s == crate::workbench::domain::BuildState::Built,
                            (None, None) => false,
                        },
                        build_state,
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
                updated_at: b.updated_at.clone(),
                users,
                default_for,
            }
        })
        .collect();
    BundleList {
        bundles,
        total: all.len(),
    }
}

/// 套餐清单。refs 带解析结果与交付身份，指向关系按一版一套分两档报
#[tauri::command(async)]
pub fn wb_bundles(query: Option<String>) -> Result<BundleList, AppError> {
    traced("wb_bundles", |_| {
        with_ctx(|ctx| {
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
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
) -> Result<BundleList, AppError> {
    traced("wb_set_bundle_refs", |_| {
        with_ctx_mut(|ctx| {
            // 时间戳由工作台这一侧给（时间源只有 `clock` 一处，见它的模块头）
            ctx.presets
                .set_bundle_refs(&bundle_id, &asset_ids, &clock::now_iso8601())?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
            ))
        })
    })
}

/* ---------- 套餐 CRUD（作者 2026-10-03：C15 的能力，产品侧真写） ---------- */

/// 新建一条套餐。**新建时就得挑内容**（至少一条 BBS）—— 落一份空套餐出去，
/// 下一次加载就会被「成套配发」判据拦住
#[tauri::command]
pub fn wb_add_bundle(
    id: String,
    machine_id: String,
    display: String,
    asset_ids: Vec<String>,
) -> Result<BundleList, AppError> {
    traced("wb_add_bundle", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.add_bundle(
                &id,
                &machine_id,
                &display,
                &asset_ids,
                &clock::now_iso8601(),
            )?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
            ))
        })
    })
}

/// 编辑一条套餐：改 id 与/或显示名。**改 id 会连带重指机型文件里的引用**
/// （`defaultBundle` + 各版本的 `recommendedBundle`），两边一起落盘
#[tauri::command]
pub fn wb_rename_bundle(
    bundle_id: String,
    new_id: String,
    display: Option<String>,
) -> Result<BundleList, AppError> {
    traced("wb_rename_bundle", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.rename_bundle(
                &bundle_id,
                &new_id,
                display.as_deref(),
                &clock::now_iso8601(),
            )?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
            ))
        })
    })
}

/// 复制一条套餐：内容照抄、id 必须是新的；复制出来的那份没人指着
#[tauri::command]
pub fn wb_copy_bundle(
    bundle_id: String,
    new_id: String,
    display: Option<String>,
) -> Result<BundleList, AppError> {
    traced("wb_copy_bundle", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.copy_bundle(
                &bundle_id,
                &new_id,
                display.as_deref(),
                &clock::now_iso8601(),
            )?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
            ))
        })
    })
}

/// **把一批版本指到这份套餐**（作者 2026-10-03：多选 + 确认）。
/// 回一份新的清单；改动细节（哪些真的变了）在 toasts 里说 —— 界面在确认前
/// 已经用预览框把影响摆出来
#[tauri::command]
pub fn wb_assign_bundle_versions(
    bundle_id: String,
    uids: Vec<String>,
) -> Result<BundleList, AppError> {
    traced("wb_assign_bundle_versions", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.assign_versions(&bundle_id, &uids)?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
            ))
        })
    })
}

/// 删一条套餐。被机型默认或版本指着时整次拒绝并点名是谁
/// （界面把它转成拦截页：先去解除那些指向）
#[tauri::command]
pub fn wb_remove_bundle(bundle_id: String) -> Result<BundleList, AppError> {
    traced("wb_remove_bundle", |_| {
        with_ctx_mut(|ctx| {
            ctx.presets.remove_bundle(&bundle_id)?;
            let (committed, draft, _) = state(ctx)?;
            let book =
                crate::workbench::domain::derive::Book::new(&ctx.presets, &committed, &draft);
            Ok(list_of(
                &ctx.presets,
                &book,
                &committed.visibility,
                &draft.visibility,
                None,
            ))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::app::assets::wb_asset_usage;

    /// **真数据上的一条**：9 份套餐（一版一套）、每条的引用都解析得到、每条至少一条 BBS。
    /// 2026-10-03 作者裁决照 C15 模型重排：A1 / A1_MINI 三个版本各一份，
    /// 单版本机型沿用 `_default` 旧名
    #[test]
    fn the_real_bundle_list_is_complete() {
        use crate::workbench::domain::BuildState;
        let list = wb_bundles(None).expect("真 presets 读得通");
        assert_eq!(
            list.total, 9,
            "一版一套：A1 三版 + A1_MINI 三版 + P1S / P2S / X1C 各一 —— 条数变了就说清为什么"
        );
        assert_eq!(list.bundles.len(), 9);

        let a1 = list
            .bundles
            .iter()
            .find(|b| b.id == "A1_STANDARD")
            .expect("A1_STANDARD 必须在");
        assert_eq!(a1.display, "官方推荐");
        assert_eq!(a1.machine_id, "A1");
        assert_eq!(a1.updated_at.as_deref(), Some("2026-10-03"));
        assert_eq!(
            a1.asset_refs
                .iter()
                .map(|r| r.id.as_str())
                .collect::<Vec<_>>(),
            vec!["a1-standard", "a1-bbs-04-020"],
            "一份套餐 = 该版本的 MKP 预设 + 本机型 BBS"
        );
        // join 资产域：MKP 那条带生成状态（built.json 入库且与配方同代 → 已生成）
        let mkp = &a1.asset_refs[0];
        assert_eq!(mkp.kind, AssetKind::MkPreset);
        assert!(!mkp.is_bbs && mkp.resolvable);
        assert_eq!(mkp.build_state, Some(BuildState::Built));
        assert!(
            mkp.present,
            "MKP 预设的「在不在」按生成状态判，不该恒 false"
        );
        // BBS 那条照旧走资产域
        let bbs = &a1.asset_refs[1];
        assert_eq!(bbs.kind, AssetKind::SlicerProfile);
        assert!(bbs.is_bbs && bbs.resolvable && bbs.present);
        assert_eq!(bbs.build_state, None, "只有 MKP 预设类有生成状态");
        assert_eq!(
            bbs.visibility,
            Visibility::Menu,
            "没人动过可见性，就是在菜单"
        );

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
            assert_eq!(
                b.asset_refs.len(),
                2,
                "{} 一版一套后每份恰两条（MKP + BBS）",
                b.id
            );
            bbs_total += b.asset_refs.iter().filter(|r| r.is_bbs).count();
        }
        // 反空转：9 条套餐各引 1 条 BBS —— 少于 9 说明上面的判定路径没走通
        assert!(
            bbs_total >= 9,
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
            .find(|b| b.id == "A1_STANDARD")
            .expect("A1_STANDARD 必须在");
        assert!(
            a1.users.iter().all(|u| u.machine_id == "A1"),
            "套餐归属 A1，指向它的版本也该都在 A1 名下"
        );
        assert_eq!(
            a1.users.len(),
            1,
            "一版一套：A1_STANDARD 只被 A1/STANDARD 指着"
        );
        assert_eq!(
            a1.users[0].version_id, "STANDARD",
            "各版本指各的套餐，不再三版共指一份"
        );
        assert_eq!(
            a1.default_for,
            vec!["A1".to_owned()],
            "A1 的 defaultBundle（机型默认 = STANDARD 那份）指着它"
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
            "9 份套餐里 A1 / A1_MINI 占六份，筛 a1 不该是全部"
        );
        assert_eq!(hit.total, 9);
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
            vec![
                "A1_STANDARD".to_owned(),
                "A1_FAST".to_owned(),
                "A1_FASTV3.3".to_owned()
            ],
            "A1 的三个版本共用这一条 BBS —— 三份套餐各装它一次"
        );

        // 图标那一份没有套餐引用（归属 ≠ 引用）：p1s-icon 归 P1S，用的是 P2S / X1C
        let u = wb_asset_usage("p1s-icon".to_owned()).expect("反查");
        assert!(u.bundles.is_empty(), "机型图标不该被套餐引用");

        // MKP 预设那一条只被自己那版指向的套餐装着
        let u = wb_asset_usage("a1-fast".to_owned()).expect("反查");
        assert_eq!(u.bundles, vec!["A1_FAST".to_owned()]);
    }
}
