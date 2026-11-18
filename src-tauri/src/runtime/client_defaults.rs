//! 客户端「预设」偏好的出厂值 —— `presets/client-defaults.toml` 的 `[Client]` 段。
//!
//! # 与 `[Prefs]` 的分界
//!
//! `[Prefs]` 那十一项**真改变切出来的 G-code**（内核 `PrefsIr::default()` 读它）；
//! 这一段一项都不进管线，只决定客户端那两步替不替用户做掉：下载完直接设为使用中、
//! 换完使用中顺手把后处理脚本抄进剪贴板。两段的唯一共同点是住同一份文件、随源码进二进制。
//!
//! # 为什么读这一份而不是写字面量
//!
//! 同一份默认值有三个入口：新装的客户端（这里）、工作台「默认配置」页（改的就是这一份）、
//! 以及用户点过之后落在 `run/app-state.json` 的那一档。默认值只此一份 —— 写死了的话，
//! 工作台里改完，新装的客户端还是老默认，没人能解释。
//!
//! 缺字段 / 语法错直接 panic：出厂值读不出来时不拿别的值顶上（与
//! [`postprocess::ir::client_prefs_default`] 同一条口径）。

use serde::Deserialize;

/// `[Client]` 段的三格。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ClientDefaults {
    /// 复制出来的那一份跟不跟云端
    pub copies_follow_cloud: bool,
    /// 使用之后自动复制后处理脚本
    pub auto_copy_script: bool,
    /// 下载之后自动设为使用中
    pub auto_use_after_download: bool,
}

#[derive(Deserialize)]
struct File {
    #[serde(rename = "Client")]
    client: ClientDefaults,
}

/// 出厂值（进程内只解析一次）。
pub fn get() -> &'static ClientDefaults {
    static CELL: std::sync::OnceLock<ClientDefaults> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        const RAW: &str = include_str!("../../../presets/client-defaults.toml");
        let file: File = toml_edit::de::from_str(RAW).expect(
            "presets/client-defaults.toml 的 [Client] 段必须能解析（客户端偏好出厂值的唯一源）",
        );
        file.client
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 那两步出厂都替用户做掉（作者 2026-11-18：「默认都是开的」）。
    #[test]
    fn the_two_steps_are_on_by_default() {
        let d = get();
        assert!(d.auto_copy_script, "使用后自动复制脚本：出厂要开");
        assert!(d.auto_use_after_download, "下载后自动使用：出厂要开");
        assert!(!d.copies_follow_cloud, "复制件跟云端：出厂保持关");
    }
}
