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
