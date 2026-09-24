//! 「套餐管理」的后端：读 `presets/bundles.toml`（套餐域①层，b05 Task 10）。
//!
//! # 读之外有了「建套餐」（b05 Task 15.3）
//!
//! 空白初始化那台设备上要能建出第一个套餐，所以这条写入口先落命令层
//! （界面归 Task 17）。它比「加一个版本」多一道：**套餐的每条引用都要先在别的
//! 文件里站得住**（机型在 `machines/`、资产在 `assets.toml`），而那三件事
//! 加载期会查成 Corrupted —— 写坏了不是报一次错，是整个 `presets/` 读不回来。
//! 所以 [`add_bundle`] 的原则是**先查再写**：不合格就一个字节都不写。
//!
//! # 为什么「至少一条 BBS」在这里拦而不是等加载期
//!
//! doc §12.4：MKP 与 BBS 成套配发，发了 MKP 不发 BBS，用户打出来的结果是错的。
//! 加载期那条（`check_bundle_refs`）管的是**盘上已有的数据**；写入口管的是
//! 「不许把盘写成那个样子」。两条是同一条判据的两端 —— 只留加载期那一端的话，
//! 人点一次「建套餐」就把工作台变成了起不来的状态。
//!
//! # 为什么每个 ref 都带解析状态
//!
//! 加载期已经把"解析不到"拦成 error（`Presets::check_bundle_refs`），所以真数据上
//! `resolvable` 恒为 true。把它报出来不是防御，是**让界面不用再猜**：
//! 校验层（Task 11.1）要列的就是这些对象，DTO 先把形状立好。

use serde::Serialize;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::clock;
use crate::workbench::presets::{AssetKind, Bundle, Presets};

/// 套餐里一个 `assetRef` 的解析状态
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleRefView {
    pub id: String,
    /// 能不能解析到一条真实资产。**加载期它是 error**，这里恒 true 是"没问题"，
    /// 不是"没查"
    pub resolvable: bool,
    /// 是不是 BBS 预设。**每条套餐至少一条 true**（doc §12.4：MKP 与 BBS 成套配发）
    pub is_bbs: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleView {
    pub id: String,
    pub display: String,
    pub machine_id: String,
    pub asset_refs: Vec<BundleRefView>,
    /// 上一次改动日期（迁移照抄旧值，不写"搬运日"）
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleList {
    pub bundles: Vec<BundleView>,
}

/// 套餐清单。**只读**
#[tauri::command]
pub fn wb_bundles() -> Result<BundleList, AppError> {
    traced("wb_bundles", |_| Ok(list_of(&Presets::load()?)))
}

fn list_of(presets: &Presets) -> BundleList {
    BundleList {
        bundles: presets
            .bundles
            .items()
            .iter()
            .map(|b| BundleView {
                id: b.id.clone(),
                display: b.display.clone(),
                machine_id: b.machine_id.clone(),
                asset_refs: b
                    .asset_refs
                    .iter()
                    .map(|r| {
                        let a = presets.assets.get(r);
                        BundleRefView {
                            id: r.clone(),
                            resolvable: a.is_some(),
                            is_bbs: a.is_some_and(|a| {
                                a.kind == AssetKind::SlicerProfile
                                    && a.slicer.as_deref() == Some("bbs")
                            }),
                        }
                    })
                    .collect(),
                updated_at: b.updated_at.clone(),
            })
            .collect(),
    }
}

/* ---------- 建套餐（b05 Task 15.3） ---------- */

/// 写之前先查一遍：**这三条不合格，写下去 `presets/` 就再也读不回来**。
///
/// 数据层的 `Bundles::add` 只看得见 `bundles.toml` 自己（id 撞、空 `assetRefs`、
/// 空串项）。下面这三条要同时看得见机型与资产，所以只能在这一层查 ——
/// 与 `Presets::check_bundle_refs` 是同一套判据，**不是两套**：这里拦「不许写成
/// 那个样子」，那边拦「盘上不许有那个样子」。
fn check_addable(p: &Presets, b: &Bundle) -> Result<(), AppError> {
    if b.asset_refs.is_empty() {
        return Err(
            AppError::invalid_argument(format!("套餐 {} 的 assetRefs 是空的", b.id)).with_detail(
                "套餐的内容就是 BBS 引用（doc §12.4）：空的套餐等于交付了一半 —— \
             MKP 预设与 BBS 预设必须成套配发"
                    .to_owned(),
            ),
        );
    }
    // ① 归属机型必须是真机型
    if p.catalog.machine(&b.machine_id).is_none() {
        return Err(AppError::not_found(format!("没有机型 {}", b.machine_id))
            .with_detail("套餐是按机型配的：先建这台机型，再来建它的套餐".to_owned()));
    }
    // ② 每条引用都要解析到真资产；③ 其中至少一条是 BBS
    let mut has_bbs = false;
    for r in &b.asset_refs {
        let a = p.assets.get(r).ok_or_else(|| {
            AppError::not_found(format!(
                "套餐 {} 的 assetRefs 里有一条指向不存在的资产：{r}",
                b.id
            ))
            .with_detail(
                "资产定义在 presets/assets.toml。先用「导入资产」把它登记进来，\
                     再来建套餐 —— 写一条解析不到的引用会让整个工作台起不来"
                    .to_owned(),
            )
        })?;
        if a.kind == AssetKind::SlicerProfile && a.slicer.as_deref() == Some("bbs") {
            has_bbs = true;
        }
    }
    if !has_bbs {
        return Err(AppError::invalid_argument(format!(
            "套餐 {} 的 assetRefs 里没有一条 BBS 预设",
            b.id
        ))
        .with_detail(
            "MKP 预设与配套 BBS 预设必须成套配发（doc §12.4）：发了 MKP 不发 BBS，\
             用户打出来的结果是错的。先把一条 BBS 预设导入进来"
                .to_owned(),
        ));
    }
    Ok(())
}

/// 领域体：加一条套餐并**立刻落盘**。
///
/// `updated_at` 由调用方给 —— 迁移照抄来的旧值不能写今天
/// （那是把「搬了个文件」记成「改了套餐」），新建的才是今天。
pub fn add_bundle(p: &mut Presets, bundle: Bundle) -> Result<(), AppError> {
    let candidate = Bundle {
        id: bundle.id.trim().to_owned(),
        display: bundle.display.trim().to_owned(),
        machine_id: bundle.machine_id.trim().to_owned(),
        asset_refs: bundle
            .asset_refs
            .iter()
            .map(|s| s.trim().to_owned())
            .collect(),
        updated_at: bundle.updated_at,
    };
    check_addable(p, &candidate)?;
    p.bundles.add(candidate)?;
    p.bundles.write()
}

/// **建一份套餐**（b05 Task 15.3）。`updatedAt` 写今天 —— 它就是今天建的。
///
/// 建完之后**从盘上重读**再返回：界面看到的必须是落盘的结果。
/// 「被机型引用」走 [`super::machines::wb_set_machine_field`] 的 `defaultBundle`
/// （版本那一格是 `recommendedBundle`），那是另一次独立的写。
#[tauri::command]
pub fn wb_add_bundle(
    id: String,
    display: String,
    machine_id: String,
    asset_refs: Vec<String>,
) -> Result<BundleList, AppError> {
    traced("wb_add_bundle", |_| {
        let mut p = Presets::load()?;
        add_bundle(
            &mut p,
            Bundle {
                id: id.clone(),
                display,
                machine_id,
                asset_refs,
                updated_at: Some(clock::today()),
            },
        )?;
        tracing::info!(bundle = %id, "建了一份套餐");
        Ok(list_of(&Presets::load()?))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::app::assets::wb_asset_usage;
    use crate::workbench::domain::testkit::Fixture;
    use crate::workbench::presets::MachineField;

    /* ---------- 建套餐（b05 Task 15.3） ---------- */

    /// **建第一个套餐**：落盘 → 重读得到 → **整机 presets 仍然读得通**。
    ///
    /// 最后那一句才是这一条的重心：`add_bundle` 写下去的东西要过得了加载期的
    /// 跨文件检查（`check_bundle_refs`）。只看"条目加进去了"的话，一个把工作台
    /// 写成起不来的实现也会绿。
    ///
    /// 顺带把「被机型引用」也走一遍（`defaultBundle` 那一格），并确认引用之后
    /// 加载期照样读得通 —— 15.3 说的"建套餐并被机型引用"是这两步
    #[test]
    fn a_new_bundle_lands_and_can_be_referenced_by_its_machine() {
        let (fx_dir, _up, mut p) = Fixture::load().into_parts();
        // P1S 是夹具里唯一"有 BBS 却还没有套餐"的机型 —— 正是新建套餐的形状
        add_bundle(
            &mut p,
            Bundle {
                id: "P1S_default".to_owned(),
                display: "官方推荐".to_owned(),
                machine_id: "P1S".to_owned(),
                asset_refs: vec!["p1s-bbs-02-010".to_owned()],
                updated_at: Some("2026-09-24".to_owned()),
            },
        )
        .expect("建套餐");

        let again = Presets::load_from(p.root()).expect("建完之后整个 presets 仍读得通");
        let got = again.bundles.get("P1S_default").expect("落盘了");
        assert_eq!(got.machine_id, "P1S");
        assert_eq!(got.asset_refs, vec!["p1s-bbs-02-010".to_owned()]);

        // 被机型引用：写 `defaultBundle`，写完整机仍读得通
        let mut p = Presets::load_from(p.root()).expect("重读");
        p.catalog
            .machine_mut("P1S")
            .unwrap()
            .set_field(MachineField::DefaultBundle, Some("P1S_default"))
            .unwrap();
        p.catalog.write_machine("P1S").unwrap();
        let again = Presets::load_from(p.root()).expect("引用之后仍读得通");
        assert_eq!(
            again
                .catalog
                .machine("P1S")
                .unwrap()
                .default_bundle
                .as_deref(),
            Some("P1S_default")
        );
        assert!(
            again.bundles.get("P1S_default").is_some(),
            "机型引用的那条套餐必须查得到 —— 悬空引用在加载期是 error"
        );
        drop(fx_dir);
    }

    /// **先查再写**：四条不合格的形状一律拒绝，而且**一个字节都不写**
    /// （落盘了就说明套餐被写成了加载期读不回来的样子）
    #[test]
    fn a_bundle_that_would_not_load_is_refused_before_writing() {
        let (fx_dir, _up, mut p) = Fixture::load().into_parts();
        let before = std::fs::read_to_string(p.bundles.file()).expect("原文");

        let bad = [
            (
                "假机型",
                Bundle {
                    id: "X_default".to_owned(),
                    display: "官方推荐".to_owned(),
                    machine_id: "NO_SUCH".to_owned(),
                    asset_refs: vec!["p1s-bbs-02-010".to_owned()],
                    updated_at: None,
                },
            ),
            (
                "假资产",
                Bundle {
                    id: "P1S_default".to_owned(),
                    display: "官方推荐".to_owned(),
                    machine_id: "P1S".to_owned(),
                    asset_refs: vec!["no-such-asset".to_owned()],
                    updated_at: None,
                },
            ),
            (
                "没有 BBS",
                Bundle {
                    id: "P1S_default".to_owned(),
                    display: "官方推荐".to_owned(),
                    machine_id: "P1S".to_owned(),
                    asset_refs: vec!["p1s-icon".to_owned()],
                    updated_at: None,
                },
            ),
            (
                "空引用",
                Bundle {
                    id: "P1S_default".to_owned(),
                    display: "官方推荐".to_owned(),
                    machine_id: "P1S".to_owned(),
                    asset_refs: Vec::new(),
                    updated_at: None,
                },
            ),
        ];
        for (why, b) in bad {
            let err = add_bundle(&mut p, b).expect_err(&format!("{why} 必须被拦，实测通过了"));
            assert!(!err.message.is_empty(), "{why} 的拒绝要说清原因");
        }
        // 撞 id（大小写不敏感）也拦
        let dup = Bundle {
            id: "a1_default".to_owned(),
            display: "官方推荐".to_owned(),
            machine_id: "A1".to_owned(),
            asset_refs: vec!["a1-bbs-04-020".to_owned()],
            updated_at: None,
        };
        assert!(add_bundle(&mut p, dup).is_err(), "撞 id 必须被拦");

        assert_eq!(
            std::fs::read_to_string(p.bundles.file()).expect("原文"),
            before,
            "被拦下就不该动 bundles.toml"
        );
        drop(fx_dir);
    }

    /// **真数据上的一条**：5 份套餐、每条的引用都解析得到、每条至少一条 BBS。
    /// 旧仓 `updatedAt = '2026-07-12'` 照实搬 —— 迁移不改内容，日期照旧
    #[test]
    fn the_real_bundle_list_is_complete() {
        let list = wb_bundles().expect("真 presets 读得通");
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
