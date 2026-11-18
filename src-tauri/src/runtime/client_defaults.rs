//! `presets/client-defaults.toml` 的 `[Client]` 段 —— 客户端壳自己的偏好初值。
//!
//! # 它管的是哪一批
//!
//! `[Prefs]` 段那 11 项决定**切出来的 G-code**（内核与客户端都读它）。这里这批**不进管线**：
//! 主题、界面语言、默认打开模型的软件、保存后的重新切片提示、通知、显示方式、置顶、
//! 强制校准、预设页那三项、自动清理、自动检查更新 —— 都是"用户点出来的界面偏好"，
//! 只决定新装的客户端第一眼长什么样。
//!
//! # 与「没选过」的关系
//!
//! 盘上（`run/app-state.json`）每一格仍是 `Option`：`None` = 这一项从没动过 ⇒ 用这里的值。
//! 于是"没动过"与"明确选了出厂档"分得开，改这里的默认值只影响前者。
//!
//! # 缺字段就是缺
//!
//! 整段用一个 `expect` 解析：文件里少一个键 / 枚举值认不出来，**编译期进二进制时就带着错**，
//! 第一次用到当场炸。不拿别的值顶上 —— 那会让「工作台写着 A、客户端是 B」变成没人能解释的事。
//! 与 `postprocess::ir::client_prefs_default` 同一条口径。

use serde::Deserialize;
use serde_json::Value;

use super::app_state::{LocalePref, PostprocUiMode, ThemePref};

/// 「默认打开模型的软件」出厂档。
///
/// 只认三档：`custom`（从磁盘指定的那个）没有出厂意义 —— 它要一条本机路径。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlicerDefault {
    /// 交给系统关联程序（`.3mf` 那个）
    System,
    Bambu,
    Orca,
}

impl SlicerDefault {
    /// 枚举 → 线上字符串。只写这一处（盘上 / DTO / 工作台共用同一个字面量）
    pub fn wire(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Bambu => "bambu",
            Self::Orca => "orca",
        }
    }
}

/// `[Client]` 段整份。字段名 → 盘上键名的换算是 `PascalCase`（见 `serde` 属性）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct ClientDefaults {
    pub theme: ThemePref,
    pub locale: LocalePref,
    pub slicer: SlicerDefault,
    pub guide_show: bool,
    pub notify_on_finish: bool,
    pub ui_mode: PostprocUiMode,
    pub auto_top_on_ask: bool,
    pub force_calibration_mode: bool,
    pub copies_follow_cloud: bool,
    pub auto_copy_script: bool,
    pub auto_use_after_download: bool,
    pub auto_clean_history: bool,
    pub auto_check_update: bool,
}

impl ClientDefaults {
    /// 这一段认哪些键（工作台「默认配置」页的白名单拿它自检：两边不许各认一套）。
    pub const KEYS: &'static [&'static str] = &[
        "Theme",
        "Locale",
        "Slicer",
        "GuideShow",
        "NotifyOnFinish",
        "UiMode",
        "AutoTopOnAsk",
        "ForceCalibrationMode",
        "CopiesFollowCloud",
        "AutoCopyScript",
        "AutoUseAfterDownload",
        "AutoCleanHistory",
        "AutoCheckUpdate",
    ];

    /// 一格 → JSON 值（工作台照它画控件）。键不认识给 `None`（调用方报错）。
    pub fn value_of(&self, key: &str) -> Option<Value> {
        Some(match key {
            "Theme" => Value::String(self.theme.wire().to_owned()),
            "Locale" => Value::String(self.locale.wire().to_owned()),
            "Slicer" => Value::String(self.slicer.wire().to_owned()),
            "GuideShow" => Value::Bool(self.guide_show),
            "NotifyOnFinish" => Value::Bool(self.notify_on_finish),
            "UiMode" => Value::String(self.ui_mode.wire().to_owned()),
            "AutoTopOnAsk" => Value::Bool(self.auto_top_on_ask),
            "ForceCalibrationMode" => Value::Bool(self.force_calibration_mode),
            "CopiesFollowCloud" => Value::Bool(self.copies_follow_cloud),
            "AutoCopyScript" => Value::Bool(self.auto_copy_script),
            "AutoUseAfterDownload" => Value::Bool(self.auto_use_after_download),
            "AutoCleanHistory" => Value::Bool(self.auto_clean_history),
            "AutoCheckUpdate" => Value::Bool(self.auto_check_update),
            _ => return None,
        })
    }
}

/// `presets/client-defaults.toml` → `[Client]` 段（进程内只解析一次）。
pub fn client_defaults() -> &'static ClientDefaults {
    static CELL: std::sync::OnceLock<ClientDefaults> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        /// 文件外形：这一段与 `[Prefs]` 同住一份文件，这里只要 `[Client]`
        #[derive(Deserialize)]
        struct File {
            #[serde(rename = "Client")]
            client: ClientDefaults,
        }
        const RAW: &str = include_str!("../../../presets/client-defaults.toml");
        let file: File = toml_edit::de::from_str(RAW).expect(
            "presets/client-defaults.toml 的 [Client] 段必须能解析（客户端偏好初值的唯一源）",
        );
        file.client
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 白名单与结构体一一对应：多一个键 / 少一个键都在这里先红。
    #[test]
    fn every_key_in_the_segment_is_listed_and_readable() {
        let d = client_defaults();
        for key in ClientDefaults::KEYS {
            assert!(d.value_of(key).is_some(), "{key} 读不出来");
        }
        assert_eq!(
            ClientDefaults::KEYS.len(),
            13,
            "加了键要同时加进 KEYS（工作台那一页照它画）"
        );
        assert!(d.value_of("Theme2").is_none(), "不认识的键不该有值");
    }

    /// 出厂档与客户端老口径一致：这一条是「改成可配之后新装行为不变」的判据。
    #[test]
    fn shipped_defaults_match_the_old_hardcoded_ones() {
        let d = client_defaults();
        assert!(matches!(d.theme, ThemePref::System));
        assert!(matches!(d.locale, LocalePref::System));
        assert_eq!(d.slicer.wire(), "system");
        assert!(d.guide_show, "没设过 = 显示（老口径）");
        assert!(d.notify_on_finish);
        assert!(matches!(d.ui_mode, PostprocUiMode::Gui));
        assert!(d.auto_top_on_ask);
        assert!(!d.force_calibration_mode);
        assert!(!d.copies_follow_cloud);
        assert!(d.auto_copy_script, "使用后自动复制脚本：出厂改开");
        assert!(d.auto_use_after_download, "下载后自动使用：出厂开");
        assert!(d.auto_clean_history);
        assert!(d.auto_check_update);
    }

    /// 缺字段要炸，不许拿默认值圆过去（§8.1）。
    #[test]
    fn a_missing_key_is_an_error_not_a_fallback() {
        #[derive(Deserialize)]
        struct File {
            #[serde(rename = "Client")]
            client: ClientDefaults,
        }
        let text = "[Client]\nTheme = \"dark\"\n";
        assert!(
            toml_edit::de::from_str::<File>(text)
                .map(|f| f.client)
                .is_err(),
            "缺字段该报错"
        );
        /* 枚举值认不出同样是错 */
        let bad = "[Client]\nTheme = \"dim\"\nLocale = \"system\"\nSlicer = \"system\"\nGuideShow = true\n\
                   NotifyOnFinish = true\nUiMode = \"gui\"\nAutoTopOnAsk = true\nForceCalibrationMode = false\n\
                   CopiesFollowCloud = false\nAutoCopyScript = true\nAutoUseAfterDownload = true\n\
                   AutoCleanHistory = true\nAutoCheckUpdate = true\n";
        assert!(
            toml_edit::de::from_str::<File>(bad)
                .map(|f| f.client)
                .is_err(),
            "认不出的档该报错"
        );
    }
}
