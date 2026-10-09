//! 旧世代（`mkp-ssr`）数据根 —— `<系统文稿目录>/MKPSupportSSR`。
//!
//! # 为什么这一个模块还留着
//!
//! 2026-10-09 起**后处理在本程序里跑**（切片器带 `--Toml/--Gcode` 调的就是我们自己的
//! 钩子）：执行记录与三件套备份落在自己的用户根（见 [`crate::archive`]），
//! 报告页已经**不再读这棵树**。
//!
//! 本模块只剩**一处**消费者：**校准模型缓存**（`models/*.3mf`，`ipc/mod.rs` 的回落
//! 分支）—— 用户机器上 mkp-ssr 时代缓存下来的那三个 3mf 还有用，这里找不到再退回
//! 内置资产。哪天那个回落也退役，本模块整个删掉即可（旧目录用户自己删；
//! 本模块**不搬旧数据、不写迁移逻辑**）。
//!
//! # 纪律
//!
//! - **只读**。这棵树的主人是 mkp-ssr，本模块一个字节都不写（连日志也不写进去）。
//! - 找不到是**正常状态**（用户还没装 / 还没跑过后处理），返回 `None`，不报错 ——
//!   界面据此走内置资产那条回落。
//! - `MKP_SSR_DATA` 与 mkp-ssr 认的是**同一个环境变量**：用户把数据根挪了，
//!   两边看到的是同一棵树。
//!
//! # 与 `fsx::paths` 的关系
//!
//! `fsx::paths` 管的是 **SupportEase 自己的**两层数据根（那里的判据是"不许碰
//! `~/Documents`"——那是给自家数据定的规矩）。本模块读的是**别人的数据根**，
//! 只为那一处缓存回落服务。

use std::path::PathBuf;

use tauri::{AppHandle, Manager};

/// 数据根本名（mkp-ssr 那侧同样只认这个名字）
const ROOT_DIR_NAME: &str = "MKPSupportSSR";

/// 覆盖数据根位置的环境变量（与 mkp-ssr 同名同义）
const ROOT_ENV: &str = "MKP_SSR_DATA";

/// 旧世代数据根。`None` = 这台机器上还没有这一棵树（没装 / 没跑过钩子）。
///
/// 环境变量优先（`MKP_SSR_DATA`）；没设就去系统文稿目录下找 `MKPSupportSSR`。
/// 文稿目录本身解不出来（极少数精简环境）也返回 `None` —— 与"没有数据根"同一个答案，
/// 界面上都是"还没有记录"，不需要区分到那个粒度。
pub fn data_root(app: &AppHandle) -> Option<PathBuf> {
    if let Ok(over) = std::env::var(ROOT_ENV) {
        let trimmed = over.trim();
        if !trimmed.is_empty() {
            return Some(PathBuf::from(trimmed));
        }
    }
    let docs = app.path().document_dir().ok()?;
    Some(docs.join(ROOT_DIR_NAME))
}

/// 纯函数版：给定文稿目录，算出数据根（判据用，不需要 AppHandle）。
pub fn data_root_in(documents: &std::path::Path) -> PathBuf {
    documents.join(ROOT_DIR_NAME)
}

/// 模型缓存目录（`<根>/models`）—— 三个校准 3mf 的旧缓存。
pub fn models_dir(root: &std::path::Path) -> PathBuf {
    root.join("models")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_root_is_the_documents_subdir_named_after_the_old_tool() {
        let docs = std::path::Path::new("/tmp/fake-documents");
        assert_eq!(
            data_root_in(docs),
            docs.join("MKPSupportSSR"),
            "数据根 = 文稿目录下的 MKPSupportSSR（与 mkp-ssr 的默认值同一个）"
        );
    }

    #[test]
    fn sub_dirs_sit_under_the_root() {
        let root = data_root_in(std::path::Path::new("/tmp/d"));
        assert!(models_dir(&root).ends_with("MKPSupportSSR/models"));
    }
}
