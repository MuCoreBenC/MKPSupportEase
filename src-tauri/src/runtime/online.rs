//! **「联网功能」那道闸** —— 关着的时候，这个程序一个字节都不出网。
//!
//! # 它挡的是什么
//!
//! 用户能点到的每一条出网路：预设 / 模型下载、目录检查、检查更新、匿名使用统计。
//! 判据只有一条（[`super::app_state::online`]），拦在各 IPC 命令的**开头**：
//! 挡在入参校验之后、真正动手之前 —— 不许出现"文件已经落了一半才发现不让联网"。
//!
//! # 为什么拦在命令开头而不是拦在 `net.rs`
//!
//! `net.rs` 也服务工作台的发布通道（那是一颗独立的二进制，同一个 `release` 模块要发版）。
//! 把闸放在那一层，用户关掉联网功能会顺手把发布能力也关掉。所以闸在**客户端命令**这一层，
//! `net.rs` 仍然只负责"怎么取字节"。
//!
//! # 读不到状态怎么办
//!
//! 不兜底：`app_state` 读不出来就把那个错原样抛出去（§8.1 缺字段就报错）。
//! 只有"读得出来、值是关"才算"用户不想联网"。

use std::path::Path;

use postprocess::diag::messages;

use crate::error::AppError;

use super::app_state;

/// 关着时给用户读的那一句（文案表里的键）
pub const OFF_MESSAGE: &str = "ui.online.001";

/// 稳定错误码：`W_*` = 这条是**你自己让它停的**，不是故障。
pub const OFF_CODE: &str = "W_ONLINE_OFF_001";

/// 联网功能开着吗。读不到状态是**错误**，不当成"关着"。
pub fn is_on(root: &Path) -> Result<bool, AppError> {
    app_state::online(root)
}

/// 命令开头的闸：关着就拒绝，并说清是"你关的"。
pub fn require(root: &Path) -> Result<(), AppError> {
    if is_on(root)? {
        return Ok(());
    }
    Err(
        AppError::permission_denied(messages::text(OFF_MESSAGE, &[]))
            .with_message_code(OFF_CODE),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root() -> tempfile::TempDir {
        tempfile::tempdir().unwrap()
    }

    /// 没选过 = 开（出厂口径），命令放行
    #[test]
    fn fresh_install_lets_network_through() {
        let d = root();
        assert!(is_on(d.path()).unwrap());
        assert!(require(d.path()).is_ok());
    }

    /// 关掉之后命令被拒，且码是 `W_*`（自己让它停的，不是故障）
    #[test]
    fn switched_off_refuses_with_its_own_code() {
        let d = root();
        app_state::set_online(d.path(), false).unwrap();
        assert!(!is_on(d.path()).unwrap());
        let e = require(d.path()).unwrap_err();
        assert_eq!(e.message_code.as_deref(), Some(OFF_CODE));
        assert!(e.text().contains(OFF_CODE), "那一句要带上码：{}", e.text());
    }

    /// 再打开就放行（开关是双向的，不是一次性的刹车）
    #[test]
    fn switching_back_on_lets_network_through_again() {
        let d = root();
        app_state::set_online(d.path(), false).unwrap();
        assert!(require(d.path()).is_err());
        app_state::set_online(d.path(), true).unwrap();
        assert!(require(d.path()).is_ok());
    }

    /// 状态文件坏了要**报错**，不许悄悄当成"关着"（§8.1）
    #[test]
    fn a_broken_state_file_is_an_error_not_a_silent_off() {
        let d = root();
        std::fs::create_dir_all(d.path().join("run")).unwrap();
        crate::fsx::atomic::atomic_write(&d.path().join("run/app-state.json"), b"{ not json")
            .unwrap();
        assert!(is_on(d.path()).is_err());
    }
}
