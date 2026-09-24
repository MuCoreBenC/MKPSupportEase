//! 「套餐管理」的后端：读 `presets/bundles.toml`（套餐域①层，b05 Task 10）。
//!
//! # 现在只有读
//!
//! 与 [`crate::workbench::app::assets`] 同一条纪律：写入口（`Bundles::add` / `write` /
//! `drop_asset_refs`）已经在数据层就位，接上它要有界面 —— Task 14，那时这一步
//! 不该顺手加进来。
//!
//! # 为什么每个 ref 都带解析状态
//!
//! 加载期已经把"解析不到"拦成 error（`Presets::check_bundle_refs`），所以真数据上
//! `resolvable` 恒为 true。把它报出来不是防御，是**让界面不用再猜**：
//! 校验层（Task 11.1）要列的就是这些对象，DTO 先把形状立好。

use serde::Serialize;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::presets::{AssetKind, Presets};

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

/// 套餐清单。**只读**（写入口在数据层，还没接到命令）
#[tauri::command]
pub fn wb_bundles() -> Result<BundleList, AppError> {
    traced("wb_bundles", |_| {
        let presets = Presets::load()?;
        let bundles = presets
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
            .collect();
        Ok(BundleList { bundles })
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
