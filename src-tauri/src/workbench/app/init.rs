//! 空白初始化（b05 Task 15，裁定②）—— **显式初始化动作，不放宽根目录判定**。
//!
//! # 裁定如何落在这里
//!
//! - `presets_root()` 的标志文件判定**不改**：初始化命令建出最小骨架
//!   （`registry/param_registry.toml` 是标志文件），建完根目录才被识别；
//! - **未初始化时只开放初始化入口**：`wb_boot` 返回 `initialized: false`，
//!   其余命令因 `Presets::load()` 失败自然不可用 —— 每个命令不必处理
//!   "半初始化环境"；
//! - **重复执行有明确行为**：干净空白才成功；任一骨架文件已存在（半初始化
//!   或已初始化）→ 拒绝并列出现状，**绝不覆盖已有数据**；
//! - **骨架只含合法的最小结构**：六件全空表（结构体字段全 `#[serde(default)]`），
//!   不预填任何虚构机型、版本或参数。`forbidden_zones/` 不建
//!   （`load_zones` 明示目录不存在是合法态）。
//!
//! # 骨架清单（勘察定稿，2026-09-24）
//!
//! ```text
//! presets/
//! ├── brands.toml              空表
//! ├── layout_schema.toml       空表
//! ├── registry/
//! │   └── param_registry.toml  空表 ← 标志文件
//! ├── machines/                空目录（零机型是合法态）
//! ├── assets.toml              空表
//! └── bundles.toml             空表
//! ```
//!
//! `workbench/` 的四个子目录由 `store::bootstrap` 在首次会话时自动建。

use std::path::Path;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::ipc::traced;

use super::paths;

/// 骨架文件：相对 `presets/` 根的路径 → 内容。**这份清单是唯一的**。
const SKELETON: [(&str, &str); 5] = [
    (
        "brands.toml",
        "# 品牌清单（空白初始化的合法空起点；机型页新增机型时可选品牌）\n",
    ),
    (
        "layout_schema.toml",
        "# 参数摆放（空白初始化的合法空起点：没有任何页签与分组）\n",
    ),
    (
        "registry/param_registry.toml",
        "# 参数注册表（空白初始化的合法空起点：没有任何参数定义）。\n\
         # 这份文件是工作台根目录的标志：它在 = 已初始化。\n\
         # 在这里定义参数之前，预检会报「缺少可用参数注册数据」—— 那是诚实的阻断。\n",
    ),
    (
        "assets.toml",
        "# 资产定义（空白初始化的合法空起点：没有任何条目）\n",
    ),
    (
        "bundles.toml",
        "# 套餐定义（空白初始化的合法空起点：没有任何套餐）\n",
    ),
];

/// 初始化报告：实际创建的文件（相对 `presets/` 根）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InitReport {
    pub created: Vec<String>,
}

/// 领域体：对哪个根初始化由调用方给（判据用临时目录）。
///
/// **只在干净空白时成功**：六个骨架目标全部不存在、`machines/` 为空或不存在的
/// 才算干净。任一已存在 → 拒绝并列出现状 —— 那说明有人手动放过数据或已经
/// 初始化过，自动补齐会把"数据是什么状态"这件事藏起来。
pub fn init_workbench(root: &Path) -> Result<InitReport, AppError> {
    let existing: Vec<String> = SKELETON
        .iter()
        .filter(|(rel, _)| root.join(rel).exists())
        .map(|(rel, _)| (*rel).to_owned())
        .collect();
    let machines = root.join("machines");
    let machines_dirty = machines
        .read_dir()
        .map(|mut entries| entries.next().is_some())
        .unwrap_or(false);

    if !existing.is_empty() || machines_dirty {
        let mut detail = Vec::new();
        if !existing.is_empty() {
            detail.push(format!("已有文件：{}", existing.join("、")));
        }
        if machines_dirty {
            detail.push("machines/ 目录不是空的".to_owned());
        }
        return Err(AppError::invalid_argument(
            "工作台数据已经存在，初始化拒绝执行（不会覆盖任何已有数据）",
        )
        .with_detail(detail.join("；")));
    }

    let mut created = Vec::new();
    for (rel, content) in SKELETON {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::io(format!("建不出目录：{}", parent.display())).with_detail(e.to_string())
            })?;
        }
        atomic_write(&path, content.as_bytes())?;
        created.push(rel.to_owned());
    }
    // machines/ 是空目录（不是文件），单独建 —— 零机型是合法态
    std::fs::create_dir_all(&machines).map_err(|e| {
        AppError::io(format!("建不出机型目录：{}", machines.display())).with_detail(e.to_string())
    })?;
    tracing::info!(count = created.len(), root = %root.display(), "工作台数据已初始化（空白骨架）");
    Ok(InitReport { created })
}

/// 初始化工作台数据（b05 Task 15 裁定②）。**幂等安全**：已有任何数据一律拒绝，
/// 列出现状，绝不覆盖。
#[tauri::command]
pub fn wb_init_workbench() -> Result<InitReport, AppError> {
    traced("wb_init_workbench", |_| {
        // 仓库根下的 presets/（与 paths::presets_root 同一个根，但**不做标志文件
        // 判定** —— 未初始化时它返回 None，而初始化恰恰要对那个状态执行）
        let root = paths::repo_root().join("presets");
        init_workbench(&root)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 干净空白 → 六件全建出来，且**建完就能被 `Presets::load_from` 读通**
    /// （空表是每个子域的合法输入）—— 这一条把"骨架只含合法最小结构"锁死
    #[test]
    fn init_creates_a_loadable_empty_skeleton() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("presets");
        let out = init_workbench(&root).expect("干净目录初始化");
        assert_eq!(out.created.len(), 5, "五个骨架文件（machines/ 是目录）");
        assert!(root.join("registry/param_registry.toml").is_file());
        assert!(root.join("machines").is_dir());
        assert!(!root.join("forbidden_zones").exists(), "禁区目录不预建");

        // 空骨架能整个读通 —— 跨域判据（assets/bundles/registry/catalog 一起）
        let p = crate::workbench::presets::Presets::load_from(&root).expect("空骨架应可读");
        assert!(p.catalog.machines().is_empty());
        assert!(p.registry.params().is_empty());
        assert!(p.assets.items().is_empty());
        assert!(p.bundles.items().is_empty());
    }

    /// 重复执行：**拒绝且不覆盖** —— 已初始化（标志文件在）时列出已有文件
    #[test]
    fn re_init_is_rejected_and_overwrites_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("presets");
        init_workbench(&root).unwrap();

        // 骨架之外放一份"真数据"，然后重跑 —— 数据必须原样保留
        let real = root.join("machines/A1.toml");
        crate::fsx::atomic::atomic_write(&real, b"# my data\n").unwrap();
        let err = init_workbench(&root).expect_err("已初始化必须拒绝");
        assert!(err.message.contains("拒绝"), "实测：{}", err.message);
        let detail = err.detail.unwrap_or_default();
        assert!(detail.contains("machines/"), "要点出现状：{detail}");
        assert_eq!(
            std::fs::read(&real).unwrap(),
            b"# my data\n",
            "已有数据一个字节不动"
        );
    }

    /// 半初始化（部分骨架文件已存在）→ 拒绝并列出现状，
    /// 不自动补齐 —— 补齐会把"数据是什么状态"藏起来
    #[test]
    fn partial_skeleton_is_rejected_with_the_facts() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("presets");
        std::fs::create_dir_all(&root).unwrap();
        crate::fsx::atomic::atomic_write(&root.join("brands.toml"), b"# hand-placed\n").unwrap();

        let err = init_workbench(&root).expect_err("半初始化必须拒绝");
        let detail = err.detail.unwrap_or_default();
        assert!(
            detail.contains("brands.toml"),
            "要点名已存在的文件：{detail}"
        );
    }

    /// **裁定③**：空 registry 的预检必须报**阻断**，不空集通过 ——
    /// 骨架本身就是空 registry 的夹具，这条判据把两件事闭环：
    /// 初始化后的第一眼校验是「可理解的待办清单」（15.6），其中
    /// 「缺少可用参数注册数据」是一条阻断，而不是绿色假象
    #[test]
    fn preflight_blocks_on_the_empty_registry() {
        use crate::workbench::domain::issues::{self, Severity};

        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("presets");
        init_workbench(&root).unwrap();
        let p = crate::workbench::presets::Presets::load_from(&root).unwrap();
        let f = crate::workbench::domain::testkit::Fixture::load();
        let committed = crate::workbench::domain::patch::Committed::default();
        let draft = crate::workbench::domain::patch::Draft::default();
        let book = crate::workbench::domain::derive::Book::new(Some(&f.up), &p, &committed, &draft);
        let recipe =
            preset::recipe::Recipe::parse(preset::PRESET_RECIPES_TOML).map_err(|e| e.to_string());
        let report = issues::preflight(&book, recipe.as_ref().map_err(String::as_str));
        assert!(
            report
                .issues
                .iter()
                .any(|i| i.id == "registry.empty" && i.severity == Severity::Block),
            "空 registry 要有一条阻断：{:?}",
            report
                .issues
                .iter()
                .map(|i| (&i.id, i.severity))
                .collect::<Vec<_>>()
        );
    }
}
