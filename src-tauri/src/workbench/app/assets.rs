//! 「资产库」的后端：读 `presets/assets.toml`（资产域①层，b05 Task 8）。
//!
//! # 现在只有读
//!
//! 定义、导入、反查、删除是 b05 Task 9 / Task 14 的事。这一版只做**读得到、看得见**：
//! 条目 + 它指向的文件在不在。写入口（`Assets::add` / `write`）已经在数据层就位，
//! 接上它要有界面 —— 那时这一步不该顺手加进来。
//!
//! # `present` 为什么现在就报
//!
//! Task 9 之前它**普遍是 false**（条目与文件一起在那边落地）。这不是错误值：
//! 界面要能说出"这一条登记了、文件还没搬"，而不是显示一张空图让人猜。
//!
//! # URL 由后端给，前端负责编码
//!
//! `url` 是 `/assets/<path>`（vite 的 `public/` 直通）。**路径里可能有空格**
//! （实测 BBS 文件名就是 `MKPProcess A1 0.2 0.10.json`），所以前端用它之前要
//! `encodeURI` —— 前缀只有这一处，别在 TSX 里再拼一遍。

use serde::Serialize;

use crate::error::AppError;
use crate::ipc::traced;
use crate::workbench::paths;
use crate::workbench::presets::{AssetKind, Presets};

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
    /// 文件在不在。**Task 9 之前普遍 `false`** —— 那是还没搬，不是错
    pub present: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetList {
    pub assets: Vec<AssetView>,
    /// 资产根的绝对路径，显示在状态条上
    pub root: String,
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

/// **谁在用它**（b05 Task 9.4）。删资产之前先问这一条 ——
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

/// 资产库清单。**只读**（写入口在数据层，还没接到命令）
#[tauri::command]
pub fn wb_assets() -> Result<AssetList, AppError> {
    traced("wb_assets", |_| {
        let presets = Presets::load()?;
        let root = paths::assets_root()?;
        let assets = presets
            .assets
            .items()
            .iter()
            .map(|a| AssetView {
                id: a.id.clone(),
                kind: a.kind,
                machine_id: a.machine_id.clone(),
                name: a.name.clone(),
                path: a.path.clone(),
                url: format!("/assets/{}", a.path),
                slicer: a.slicer.clone(),
                profile: a.profile.clone(),
                present: presets.assets.present(a),
            })
            .collect();
        Ok(AssetList {
            assets,
            root: root.display().to_string(),
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **真数据上的一条**：条目数与文件都在（b05 Task 9 的验收）。
    ///
    /// 它比 `presets::tests` 那条多看一层：命令返回的 DTO 是不是也对
    /// （`present`、`url` 前缀、类型都在）。
    #[test]
    fn the_real_asset_list_is_complete_and_present() {
        let list = wb_assets().expect("真 presets 读得通");
        assert_eq!(
            list.assets.len(),
            21,
            "机型图 6 + 图标 3 + 模型 3 + BBS 9 —— 条数变了就该在提交里说清为什么"
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
}
