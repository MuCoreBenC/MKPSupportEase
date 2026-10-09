//! 旧世代（`mkp-ssr`）数据根 —— `<系统文稿目录>/MKPSupportSSR`。
//!
//! # 为什么这一个模块要去读另一棵树
//!
//! 后处理这件事今天**不在 SupportEase 里跑**：用户把「复制后处理脚本」得到的
//! `mkp-ssr.exe --Toml … --Gcode` 贴进切片器（Bambu Studio）的后处理栏，
//! 由切片器导出时调用。那个钩子跑完把**执行报告**（`gcode_history/<日期>/*_meta.json`）
//! 与**校准模型缓存**（`models/*.3mf`）都写在它自己的数据根里
//! （`mkp-ssr` 的 `settings.rs`：默认 `~/Documents/MKPSupportSSR`，`MKP_SSR_DATA` 覆盖）。
//!
//! 报告页要回答"后处理跑得怎么样"，**唯一真实的账就在那一棵树里** ——
//! 界面上不许拿假数据凑，所以只能去读真账。模型同理：用户机器上已经缓存的
//! 三个 3mf 就在它的 `models/` 下，「从本地缓存打开」该认它。
//!
//! # 纪律
//!
//! - **只读**。这棵树的主人是 mkp-ssr，本模块一个字节都不写（连日志也不写进去）。
//! - 找不到是**正常状态**（用户还没装 / 还没跑过后处理），返回 `None`，不报错 ——
//!   界面据此显示"还没有执行记录"，与"读不出来"是两回事。
//! - `MKP_SSR_DATA` 与 mkp-ssr 认的是**同一个环境变量**：用户把数据根挪了，
//!   两边看到的是同一棵树。
//!
//! # 与 `fsx::paths` 的关系
//!
//! `fsx::paths` 管的是 **SupportEase 自己的**两层数据根（那里的判据是"不许碰
//! `~/Documents`"——那是给自家数据定的规矩）。本模块读的是**别人的数据根**，
//! 是产品行为的一部分（报告页的数据源），不是把自家数据搬回 Documents。

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

/// 后处理执行账（`<根>/gcode_history`）—— 钩子跑完落的 `_meta.json` 都在这下面的日期目录里。
pub fn gcode_history_dir(root: &std::path::Path) -> PathBuf {
    root.join("gcode_history")
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
        assert!(gcode_history_dir(&root).ends_with("MKPSupportSSR/gcode_history"));
    }
}
