//! 「默认配置」页的后端：读写 `presets/client-defaults.toml`。
//!
//! 出厂值的真值只有这一份 —— 三个消费者都 `include_str!` 它：
//! 内核 `postprocess::ir::PrefsIr::default()`、客户端 `runtime::app_state::PostprocPrefs::resolved()`
//! 的那 12 项（`[Prefs]` 段），以及客户端 `runtime::client_defaults` 那一批界面偏好（`[Client]` 段）。
//! 这一页改的是**新装**的默认值：已经装好的客户端不受影响，要重新构建才进包。
//!
//! 写入走 [`with_struct_ctx`]：一个文件、一条撤销点、失败回滚 ——
//! 与本仓改机型显示名同一条路，不另造事务。
//!
//! # 两段都认，但都不是"随便写"
//!
//! [`EDITABLE`] 是这一页能改的全部键：多一个不认（改错文件比报错更难查），少一个就是
//! 客户端那边有一项没有出厂值可配。取值类型与枚举档也在这里收口 —— 写进来的值先过
//! [`Spec::kind`]，落盘之后再整份读一遍，坏档绝不进影子树。
//!
//! 键名与标签**照客户端的说法**（`src/i18n/zh-CN.ts` 那一套）：同一个设置在两边叫两个
//! 名字时，配置的人没法把两边对上。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::ipc::traced;
use crate::runtime::client_defaults::ClientDefaults;

use super::{one_field, with_ctx, with_struct_ctx, Ctx};

/// 相对影子根（`presets/`）的文件名
const FILE: &str = "client-defaults.toml";

/// 一段（TOML 表名）。两段的读者不同：`Prefs` 决定切出来的 G-code，`Client` 只管界面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Section {
    Prefs,
    Client,
}

impl Section {
    fn name(self) -> &'static str {
        match self {
            Self::Prefs => "Prefs",
            Self::Client => "Client",
        }
    }
}

/// 一格能写什么值。**写之前必须过这一道**：枚举档只认列出来的那几个字面量。
#[derive(Debug, Clone, Copy)]
enum Kind {
    Bool,
    Number,
    /// 文本 + 允许的取值（顺序就是界面上那一排的顺序）
    Choice(&'static [&'static str]),
}

/// 一格出厂值：写在哪一段、叫什么、界面写什么字、能填什么。
#[derive(Debug, Clone, Copy)]
struct Spec {
    section: Section,
    key: &'static str,
    label: &'static str,
    kind: Kind,
}

/// 这一页能改的键。**顺序只影响自检日志**，界面顺序由 `DefaultsPage.tsx` 决定（照客户端）。
const EDITABLE: &[Spec] = &[
    /* ---- 外观与语言 ---- */
    Spec {
        section: Section::Client,
        key: "Theme",
        label: "主题",
        kind: Kind::Choice(&["system", "light", "dark"]),
    },
    Spec {
        section: Section::Client,
        key: "Locale",
        label: "界面语言",
        kind: Kind::Choice(&["system", "zh-CN", "zh-TW", "ja", "en"]),
    },
    /* ---- 切片与引导 ---- */
    Spec {
        section: Section::Client,
        key: "Slicer",
        label: "默认打开模型的软件",
        /* `custom`（从磁盘指定的那个）没有出厂意义：它要一条本机路径 */
        kind: Kind::Choice(&["system", "bambu", "orca"]),
    },
    Spec {
        section: Section::Client,
        key: "GuideShow",
        label: "保存参数后的重新切片提示",
        kind: Kind::Bool,
    },
    /* ---- 后处理：通知与显示 ---- */
    Spec {
        section: Section::Client,
        key: "NotifyOnFinish",
        label: "处理完成发送系统通知",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Client,
        key: "UiMode",
        label: "显示方式",
        kind: Kind::Choice(&["gui", "silent"]),
    },
    Spec {
        section: Section::Client,
        key: "AutoTopOnAsk",
        label: "等待回答时置顶",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Prefs,
        key: "GlueBottomEnabled",
        label: "支撑面底面涂胶",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Prefs,
        key: "AutoTowerPosition",
        label: "智能塔位置",
        kind: Kind::Choice(&["ask", "auto", "off"]),
    },
    /* ---- 校准与补偿 ---- */
    Spec {
        section: Section::Client,
        key: "ForceCalibrationMode",
        label: "强制校准模式",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Prefs,
        key: "SkipVibrationCalibration",
        label: "跳过震动校准检测",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Prefs,
        key: "BeginStageRetractEnabled",
        label: "开始阶段收笔",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Prefs,
        key: "BeginStageRetractZLiftMm",
        label: "收笔抬升高度",
        kind: Kind::Number,
    },
    Spec {
        section: Section::Prefs,
        key: "CalibrationSafeLiftMm",
        label: "校准抬升高度",
        kind: Kind::Number,
    },
    Spec {
        section: Section::Prefs,
        key: "ZCalibrationDwellSec",
        label: "Z 轴校准停留时间",
        kind: Kind::Number,
    },
    Spec {
        section: Section::Prefs,
        key: "XyCalibrationDwellSec",
        label: "XY 轴校准停留时间",
        kind: Kind::Number,
    },
    /* ---- 下方支撑余量 ---- */
    Spec {
        section: Section::Prefs,
        key: "UnderSupportEnabled",
        label: "启用下方支撑余量",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Prefs,
        key: "UnderSupportMm",
        label: "余量阈值",
        kind: Kind::Number,
    },
    /* ---- 热床预涂胶 ---- */
    Spec {
        section: Section::Prefs,
        key: "BedGlueEnabled",
        label: "启用热床预涂胶",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Prefs,
        key: "BedGlueAllModels",
        label: "所有模型首层也涂",
        kind: Kind::Bool,
    },
    /* ---- 预设 ---- */
    Spec {
        section: Section::Client,
        key: "CopiesFollowCloud",
        label: "副本跟随云端更新",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Client,
        key: "AutoCopyScript",
        label: "使用预设后复制后处理脚本",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Client,
        key: "AutoUseAfterDownload",
        label: "下载后自动使用",
        kind: Kind::Bool,
    },
    /* ---- 数据与维护 / 软件更新 ---- */
    Spec {
        section: Section::Client,
        key: "AutoCleanHistory",
        label: "自动清理执行记录",
        kind: Kind::Bool,
    },
    Spec {
        section: Section::Client,
        key: "AutoCheckUpdate",
        label: "自动检查更新",
        kind: Kind::Bool,
    },
];

/// 一行默认值
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsField {
    /// `Prefs` / `Client` —— 界面不必认它（键名全局唯一），排查时用
    pub section: String,
    pub key: String,
    pub label: String,
    pub value: Value,
}

impl DefaultsField {
    fn of(spec: &Spec, value: Value) -> Self {
        Self {
            section: spec.section.name().to_owned(),
            key: spec.key.to_owned(),
            label: spec.label.to_owned(),
            value,
        }
    }
}

/// 整页视图
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsView {
    /// 相对仓库根的路径，界面那行「改的是这一份」用
    pub path: String,
    pub fields: Vec<DefaultsField>,
}

/// 整份文件的外形：两段都在这儿，缺一段 / 缺一个键都读不出来（不兜底）
#[derive(Debug, Deserialize)]
struct DefaultsFile {
    #[serde(rename = "Prefs")]
    prefs: postprocess::ir::PrefsIr,
    #[serde(rename = "Client")]
    client: ClientDefaults,
}

/// 读这一份：影子树优先（草稿态），没有就退回真仓库那一份。
///
/// 影子树是 `presets/` 的镜像，正常两张都在；退回真仓库只为「镜像还没铺到它」
/// 这种边界，读得到就先给人看，写的时候仍只写影子。
fn read_text(ctx: &Ctx) -> Result<String, AppError> {
    let shadow = ctx.shadow_root.join(FILE);
    match std::fs::read_to_string(&shadow) {
        Ok(s) => Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            std::fs::read_to_string(ctx.real_root.join(FILE))
                .map_err(|err| AppError::io(format!("读不出 presets/{FILE}：{}", err)))
        }
        Err(e) => Err(AppError::io(format!(
            "读不出影子树里的 presets/{FILE}：{e}"
        ))),
    }
}

/// `[Prefs]` 的一格 → JSON 值。键不认识就是 `None`（调用方报错）
fn value_of_prefs(prefs: &postprocess::ir::PrefsIr, key: &str) -> Option<Value> {
    use postprocess::ir::TowerRelocateModeIr;
    Some(match key {
        "UnderSupportEnabled" => Value::Bool(prefs.under_support_enabled),
        "UnderSupportMm" => Value::from(prefs.under_support_mm),
        "GlueBottomEnabled" => Value::Bool(prefs.glue_bottom_enabled),
        "SkipVibrationCalibration" => Value::Bool(prefs.skip_vibration_calibration),
        "BeginStageRetractEnabled" => Value::Bool(prefs.begin_stage_retract_enabled),
        "BeginStageRetractZLiftMm" => Value::from(prefs.begin_stage_retract_z_lift_mm),
        "CalibrationSafeLiftMm" => Value::from(prefs.calibration_safe_lift_mm),
        "ZCalibrationDwellSec" => Value::from(prefs.z_calibration_dwell_sec),
        "XyCalibrationDwellSec" => Value::from(prefs.xy_calibration_dwell_sec),
        "BedGlueEnabled" => Value::Bool(prefs.bed_glue_enabled),
        "BedGlueAllModels" => Value::Bool(prefs.bed_glue_all_models),
        "AutoTowerPosition" => Value::String(
            match prefs.auto_tower_position {
                TowerRelocateModeIr::Off => "off",
                TowerRelocateModeIr::Auto => "auto",
                TowerRelocateModeIr::Ask => "ask",
            }
            .to_owned(),
        ),
        _ => return None,
    })
}

/// 一格出厂值的当前值。键不认识 = `None`（`EDITABLE` 与两段结构体打架时在这里露出来）
fn value_of(file: &DefaultsFile, spec: &Spec) -> Option<Value> {
    match spec.section {
        Section::Prefs => value_of_prefs(&file.prefs, spec.key),
        Section::Client => file.client.value_of(spec.key),
    }
}

fn parse(text: &str) -> Result<DefaultsFile, AppError> {
    /* `toml_edit::de` 而不是 `toml`：本 crate 的生产依赖里只有前者
    （`toml` 在 `[dev-dependencies]`，测试用），两者对这份文件等价 */
    toml_edit::de::from_str(text)
        .map_err(|e| AppError::corrupted(format!("presets/{FILE} 解析不了：{e}")))
}

/// 组装视图（读命令）
pub fn view_from(ctx: &Ctx) -> Result<DefaultsView, AppError> {
    let file = parse(&read_text(ctx)?)?;
    let fields = EDITABLE
        .iter()
        .map(|spec| {
            let value = value_of(&file, spec).ok_or_else(|| {
                AppError::corrupted(format!(
                    "presets/{FILE} 的 [{}] 里没有可读的 {}",
                    spec.section.name(),
                    spec.key
                ))
            })?;
            Ok(DefaultsField::of(spec, value))
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(DefaultsView {
        path: format!("presets/{FILE}"),
        fields,
    })
}

/// JSON 值 → TOML 值。**只认布尔 / 数字 / 文本**：数组与对象不是默认值
fn to_toml(value: &Value) -> Result<toml_edit::Value, AppError> {
    match value {
        Value::Bool(b) => Ok((*b).into()),
        Value::Number(n) => match (n.as_i64(), n.as_f64()) {
            (Some(i), _) => Ok(i.into()),
            (None, Some(f)) => Ok(f.into()),
            _ => Err(AppError::invalid_argument("这个数字读不出来")),
        },
        Value::String(s) => Ok(s.as_str().into()),
        _ => Err(AppError::invalid_argument(
            "默认值只认开关、数字与文本三种形态",
        )),
    }
}

/// 写进来的值先过这一格的类型与档位。**认不出就整次拒绝** —— 不静默改成出厂档。
fn check_kind(spec: &Spec, value: &Value) -> Result<(), AppError> {
    match (spec.kind, value) {
        (Kind::Bool, Value::Bool(_)) => Ok(()),
        (Kind::Number, Value::Number(_)) => Ok(()),
        (Kind::Choice(allowed), Value::String(s)) => {
            if allowed.contains(&s.as_str()) {
                Ok(())
            } else {
                Err(AppError::invalid_argument(format!(
                    "{} 只认 {}，给的是 {s}",
                    spec.key,
                    allowed.join(" / ")
                )))
            }
        }
        (Kind::Bool, _) => Err(AppError::invalid_argument(format!(
            "{} 是开关，只认 true / false",
            spec.key
        ))),
        (Kind::Number, _) => Err(AppError::invalid_argument(format!(
            "{} 是数字，给的不是数字",
            spec.key
        ))),
        (Kind::Choice(_), _) => Err(AppError::invalid_argument(format!(
            "{} 是选项，只认那几档文字",
            spec.key
        ))),
    }
}

/// 纯编辑：把一格写进这份文本（保注释、保格式、只动那一格）。
///
/// 判据全在这一层：键在不在 [`EDITABLE`] 里、值过不过 [`check_kind`]、写在哪一段。
/// 落盘与记账留给 [`apply_in`] —— 这样"改一格会写出什么字节"能在测试里直接看。
fn edit_text(before_text: &str, spec: &Spec, value: &Value) -> Result<String, AppError> {
    check_kind(spec, value)?;

    let mut doc: toml_edit::DocumentMut = before_text
        .parse()
        .map_err(|e| AppError::corrupted(format!("presets/{FILE} 解析不了：{e}")))?;
    let section = spec.section.name();
    let table = doc[section]
        .as_table_mut()
        .ok_or_else(|| AppError::corrupted(format!("presets/{FILE} 里没有 [{section}] 段")))?;
    if !table.contains_key(spec.key) {
        return Err(AppError::corrupted(format!(
            "presets/{FILE} 的 [{section}] 里没有 {} —— 缺字段就是缺，不替你补一行",
            spec.key
        )));
    }
    table[spec.key] = toml_edit::Item::Value(to_toml(value)?);
    Ok(doc.to_string())
}

/// 改一格：写影子文件 + 记一条字段账
fn apply_in(ctx: &mut Ctx, key: &str, value: &Value) -> Result<DefaultsView, AppError> {
    let spec = EDITABLE.iter().find(|s| s.key == key).ok_or_else(|| {
        AppError::invalid_argument(format!(
            "{key} 不是这一页能改的键（只认 `presets/{FILE}` 里登记过的那些）"
        ))
    })?;

    let before_text = read_text(ctx)?;
    let before = parse(&before_text)?;
    let section = spec.section.name();
    let next_text = edit_text(&before_text, spec, value)?;

    /* 写完先自己读一遍：坏档绝不落进影子树（撤销栈里那份字节要能被人看懂） */
    let after = parse(&next_text)?;

    let target = ctx.shadow_root.join(FILE);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| AppError::io(format!("建不了 {}：{e}", parent.display())))?;
    }
    atomic_write(&target, next_text.as_bytes())
        .map_err(|e| AppError::io(format!("写不进 {}：{}", target.display(), e.message)))?;

    let fmt = |v: Option<Value>| v.map(|v| v.to_string());
    ctx.pending_fields.push(one_field(
        "client-defaults".to_owned(),
        spec.key,
        spec.label.to_owned(),
        fmt(value_of(&before, spec)),
        fmt(value_of(&after, spec)),
    ));
    tracing::info!(key = spec.key, section, "默认值已改（草稿）");
    Ok(DefaultsView {
        path: format!("presets/{FILE}"),
        fields: EDITABLE
            .iter()
            .filter_map(|s| value_of(&after, s).map(|v| DefaultsField::of(s, v)))
            .collect(),
    })
}

/// 读整页：出厂默认值的当前草稿态
#[tauri::command]
pub fn wb_defaults_view() -> Result<DefaultsView, AppError> {
    traced("wb_defaults_view", |_| with_ctx(view_from))
}

/// 改一格：写进影子树，保存才进真仓库
#[tauri::command]
pub fn wb_defaults_apply(key: String, value: Value) -> Result<DefaultsView, AppError> {
    traced("wb_defaults_apply", move |_| {
        with_struct_ctx(
            "",
            |_| vec![FILE.to_owned()],
            |ctx| apply_in(ctx, &key, &value),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 与三个 crate 的 `include_str!` **同一份文件** —— 这里用相对路径再打开一次，
    /// 只为让判据在「文件被挪走 / 改名」时先红，而不是等别处都编译失败。
    const RAW: &str = include_str!("../../../../presets/client-defaults.toml");

    #[test]
    fn the_shared_file_parses_and_covers_every_editable_key() {
        let file = parse(RAW).expect("出厂值文件必须能解析");
        for spec in EDITABLE {
            assert!(
                value_of(&file, spec).is_some(),
                "{}（{}）在这份文件里读不出来",
                spec.key,
                spec.label
            );
        }
    }

    /// 两段里的键一个都不能漏：客户端 `[Client]` 段自己有一张键表（`KEYS`），
    /// 这一页漏登记一个，那一项就没有出厂值可配。
    #[test]
    fn every_client_key_is_editable_here_too() {
        let listed: Vec<&str> = EDITABLE
            .iter()
            .filter(|s| s.section == Section::Client)
            .map(|s| s.key)
            .collect();
        assert_eq!(
            listed.as_slice(),
            ClientDefaults::KEYS,
            "[Client] 段的键表与这一页的登记不一致（顺序也要一致）"
        );
    }

    #[test]
    fn writing_one_key_keeps_comments_and_the_other_values() {
        let before = parse(RAW).expect("改之前");
        let spec = EDITABLE
            .iter()
            .find(|s| s.key == "UnderSupportEnabled")
            .expect("那一格");
        let next = edit_text(RAW, spec, &Value::Bool(true)).expect("改得动");
        let after = parse(&next).expect("改完之后还得能解析");

        assert!(after.prefs.under_support_enabled, "改的那一格没生效");
        assert_eq!(
            after.prefs.under_support_mm, before.prefs.under_support_mm,
            "只改一格不许连带别的"
        );
        assert_eq!(
            after.client.theme, before.client.theme,
            "改 [Prefs] 不许连带 [Client]"
        );
        assert!(
            next.contains("只此一份"),
            "注释被吃掉了（说明写回路径不对）"
        );
    }

    /// `[Client]` 那一段也照同一条路写：写进去的是它自己那一段，别的段一个字不动。
    #[test]
    fn a_client_key_is_written_into_the_client_table() {
        use crate::runtime::app_state::ThemePref;

        let spec = EDITABLE
            .iter()
            .find(|s| s.key == "Theme")
            .expect("主题那一格");
        let before = parse(RAW).expect("改之前");
        let next = edit_text(RAW, spec, &Value::String("dark".to_owned())).expect("改得动");
        let after = parse(&next).expect("改完之后还得能解析");

        assert_eq!(after.client.theme, ThemePref::Dark, "主题没写进去");
        assert_eq!(
            after.client.locale, before.client.locale,
            "只改一格不许连带别的"
        );
        assert_eq!(
            after.prefs.under_support_mm, before.prefs.under_support_mm,
            "改 [Client] 不许碰 [Prefs]"
        );
        assert!(next.contains("[Client]"), "整段被写丢了");
    }

    /// 文件里没有的那一格：报错，**不替它补一行**（补了就是凭空发明一个出厂值）。
    #[test]
    fn editing_a_missing_key_is_an_error() {
        let spec = EDITABLE
            .iter()
            .find(|s| s.key == "Theme")
            .expect("主题那一格");
        let text = RAW
            .lines()
            .filter(|l| !l.starts_with("Theme ="))
            .collect::<Vec<_>>()
            .join("\n");
        let err = edit_text(&text, spec, &Value::String("dark".to_owned()));
        assert!(err.is_err(), "缺这一格该报错，不该补上");
    }

    #[test]
    fn only_bool_number_and_text_are_defaults() {
        assert!(to_toml(&Value::Bool(true)).is_ok());
        assert!(to_toml(&Value::from(1.5)).is_ok());
        assert!(to_toml(&Value::from(3)).is_ok());
        assert!(to_toml(&Value::String("ask".to_owned())).is_ok());
        assert!(to_toml(&Value::Null).is_err(), "空值不是默认值");
        assert!(
            to_toml(&Value::Array(Vec::new())).is_err(),
            "数组不是默认值"
        );
    }

    #[test]
    fn kind_gate_rejects_wrong_types_and_unknown_choices() {
        let tower = EDITABLE
            .iter()
            .find(|s| s.key == "AutoTowerPosition")
            .expect("塔位那一格");
        assert!(check_kind(tower, &Value::String("ask".to_owned())).is_ok());
        assert!(
            check_kind(tower, &Value::String("每次提问".to_owned())).is_err(),
            "选项只认盘上那个字面量"
        );
        assert!(
            check_kind(tower, &Value::Bool(true)).is_err(),
            "档位不是开关"
        );

        let theme = EDITABLE.iter().find(|s| s.key == "Theme").expect("主题");
        assert!(check_kind(theme, &Value::String("dark".to_owned())).is_ok());
        assert!(check_kind(theme, &Value::String("dim".to_owned())).is_err());

        let notify = EDITABLE
            .iter()
            .find(|s| s.key == "NotifyOnFinish")
            .expect("通知");
        assert!(check_kind(notify, &Value::Bool(false)).is_ok());
        assert!(check_kind(notify, &Value::from(0)).is_err());
    }

    #[test]
    fn value_of_rejects_keys_outside_the_whitelist() {
        let file = parse(RAW).expect("parse");
        assert!(file.client.value_of("DebugWaitX").is_none());
        assert!(file.client.value_of("").is_none());
        assert!(value_of_prefs(&file.prefs, "DebugWaitX").is_none());
        assert!(value_of_prefs(&file.prefs, "").is_none());
    }
}
