//! 「默认配置」页的后端：读写 `presets/client-defaults.toml`。
//!
//! 出厂值的真值只有这一份 —— 内核 `postprocess::ir::PrefsIr::default()` 与客户端
//! `runtime::app_state::PostprocPrefs::resolved()` 都 `include_str!` 它。
//! 这一页改的是**新装**的默认值：已经装好的客户端不受影响，要重新构建才进包。
//!
//! 写入走 [`with_struct_ctx`]：一个文件、一条撤销点、失败回滚 ——
//! 与本仓改机型显示名同一条路，不另造事务。
//!
//! **只做 `[Prefs]` 段**：那里是「真改变切出来 G-code」的 11 项。外观 / 切片 / 备份 /
//! 高级 / 引导图是客户端私有偏好，不在这个文件里，页面照客户端顺序摆出来并注明
//! 只在客户端设置（见 `src/workbench/views/DefaultsPage.tsx`）。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::ipc::traced;

use super::{one_field, with_ctx, with_struct_ctx, Ctx};

/// 相对影子根（`presets/`）的文件名
const FILE: &str = "client-defaults.toml";

/// 这一页能改的键。**只认这些** —— 多一个不认：改错文件比报错更难查。
const EDITABLE: &[(&str, &str)] = &[
    ("UnderSupportEnabled", "启用下方支撑余量"),
    ("UnderSupportMm", "下方支撑余量阈值"),
    ("GlueBottomEnabled", "支撑面下方一层也涂胶"),
    ("SkipVibrationCalibration", "跳过震动校准检测"),
    ("BeginStageRetractEnabled", "开始阶段收笔"),
    ("BeginStageRetractZLiftMm", "收笔抬升高度"),
    ("ZCalibrationDwellSec", "Z 轴校准停留时间"),
    ("XyCalibrationDwellSec", "XY 轴校准停留时间"),
    ("BedGlueEnabled", "热床预涂胶"),
    ("BedGlueAllModels", "热床涂胶覆盖所有模型"),
    ("AutoTowerPosition", "智能塔位置"),
];

/// 一行默认值
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsField {
    pub key: String,
    pub label: String,
    pub value: Value,
}

/// 整页视图
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DefaultsView {
    /// 相对仓库根的路径，界面那行「改的是这一份」用
    pub path: String,
    pub fields: Vec<DefaultsField>,
}

#[derive(Debug, Deserialize)]
struct DefaultsFile {
    #[serde(rename = "Prefs")]
    prefs: postprocess::ir::PrefsIr,
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

/// `PrefsIr` 的一格 → JSON 值。键不认识就是 `None`（调用方报错）
fn value_of(prefs: &postprocess::ir::PrefsIr, key: &str) -> Option<Value> {
    use postprocess::ir::TowerRelocateModeIr;
    Some(match key {
        "UnderSupportEnabled" => Value::Bool(prefs.under_support_enabled),
        "UnderSupportMm" => Value::from(prefs.under_support_mm),
        "GlueBottomEnabled" => Value::Bool(prefs.glue_bottom_enabled),
        "SkipVibrationCalibration" => Value::Bool(prefs.skip_vibration_calibration),
        "BeginStageRetractEnabled" => Value::Bool(prefs.begin_stage_retract_enabled),
        "BeginStageRetractZLiftMm" => Value::from(prefs.begin_stage_retract_z_lift_mm),
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
        .map(|(key, label)| {
            let value = value_of(&file.prefs, key)
                .ok_or_else(|| AppError::corrupted(format!("presets/{FILE} 里没有可读的 {key}")))?;
            Ok(DefaultsField {
                key: (*key).to_owned(),
                label: (*label).to_owned(),
                value,
            })
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

/// 改一格：写影子文件 + 记一条字段账
fn apply_in(ctx: &mut Ctx, key: &str, value: &Value) -> Result<DefaultsView, AppError> {
    let label = EDITABLE
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, l)| *l)
        .ok_or_else(|| {
            AppError::invalid_argument(format!(
                "{key} 不是这一页能改的键（只认 `[Prefs]` 段里那 11 项）"
            ))
        })?;

    let before_text = read_text(ctx)?;
    let before = parse(&before_text)?.prefs;

    let mut doc: toml_edit::DocumentMut = before_text
        .parse()
        .map_err(|e| AppError::corrupted(format!("presets/{FILE} 解析不了：{e}")))?;
    let table = doc["Prefs"]
        .as_table_mut()
        .ok_or_else(|| AppError::corrupted(format!("presets/{FILE} 里没有 [Prefs] 段")))?;
    if !table.contains_key(key) {
        return Err(AppError::corrupted(format!(
            "presets/{FILE} 的 [Prefs] 里没有 {key} —— 缺字段就是缺，不替你补一行"
        )));
    }
    table[key] = toml_edit::Item::Value(to_toml(value)?);
    let next_text = doc.to_string();

    /* 写完先自己读一遍：坏档绝不落进影子树（撤销栈里那份字节要能被人看懂） */
    let after = parse(&next_text)?.prefs;

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
        key,
        label.to_owned(),
        fmt(value_of(&before, key)),
        fmt(value_of(&after, key)),
    ));
    tracing::info!(key, "默认值已改（草稿）");
    Ok(DefaultsView {
        path: format!("presets/{FILE}"),
        fields: EDITABLE
            .iter()
            .filter_map(|(k, l)| {
                value_of(&after, k).map(|value| DefaultsField {
                    key: (*k).to_owned(),
                    label: (*l).to_owned(),
                    value,
                })
            })
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

    /// 与两个 crate 的 `include_str!` **同一份文件** —— 这里用相对路径再打开一次，
    /// 只为让判据在「文件被挪走 / 改名」时先红，而不是等两边都编译失败。
    const RAW: &str = include_str!("../../../../presets/client-defaults.toml");

    #[test]
    fn the_shared_file_parses_and_covers_every_editable_key() {
        let file = parse(RAW).expect("出厂值文件必须能解析");
        for (key, label) in EDITABLE {
            assert!(
                value_of(&file.prefs, key).is_some(),
                "{key}（{label}）在这份文件里读不出来"
            );
        }
    }

    #[test]
    fn writing_one_key_keeps_comments_and_the_other_values() {
        let before = parse(RAW).expect("改之前");
        let mut doc: toml_edit::DocumentMut = RAW.parse().expect("DocumentMut");
        doc["Prefs"]["UnderSupportEnabled"] = toml_edit::Item::Value(true.into());
        let next = doc.to_string();
        let after = parse(&next).expect("改完之后还得能解析");

        assert!(after.prefs.under_support_enabled, "改的那一格没生效");
        assert_eq!(
            after.prefs.under_support_mm, before.prefs.under_support_mm,
            "只改一格不许连带别的"
        );
        assert!(
            next.contains("只此一份"),
            "注释被吃掉了（说明写回路径不对）"
        );
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
    fn value_of_rejects_keys_outside_the_whitelist() {
        let file = parse(RAW).expect("parse");
        assert!(value_of(&file.prefs, "DebugWaitX").is_none());
        assert!(value_of(&file.prefs, "").is_none());
    }
}
