//! 校准模型档 —— 校准图案的几何与判据口径，由 `presets/calibration_models.toml`
//! 一份数据文件驱动。
//!
//! # 为什么要有它
//!
//! 这些数原来写死在 `postproc/disk/calibration.rs` 的常量里（`Z_PATTERN_MODEL_WIDTH`
//! = 119.58、`Z_PATTERN_MODEL_HEIGHT` = 13.063、`Z_PATTERN_OFFSET_X/Y` = 0.079 / 3.755、
//! 方块间距 11.0、XY 图案那七个偏移……）。于是「同一台机器换一块校准板」= 改代码重发版。
//! 现在：**加模型 = 往 `presets/calibration_models.toml` 加一条**，代码一行不动。
//!
//! # 谁读它
//!
//! 后处理那一趟（[`crate::postproc::disk::calibration`]）：尺寸校验的标称外形、
//! 图案基准偏移、XY 模板偏移、质心旋转判据的期望，四处都从这里取。
//!
//! 装载由 [`crate::postproc::machine_dims`] 统一做（`load_presets_dir` 顺手读它），
//! 读取链与机型表**同一条**：显式 `install` → `MKPSE_PRESETS_DIR` → 仓库相对
//! `../../presets`。不另造一份路径依赖，也不另开一个进程级 `OnceLock`。
//!
//! # 坏了会怎样
//!
//! `deny_unknown_fields` + 逐档校验：字段名敲错、mode 不认识、该 mode 要用的数缺了
//! （ZOffset 缺 `datumY`、XY 缺 `pattern`……）= 装载期**硬错**，带文件名与档 id。
//! 不静默退化成 0.0 —— 那会让一个错尺寸混进认档，把「模型被缩放」这种真问题盖掉
//! （口径与 `diag/messages.rs` 那条一致：坏就让它看得见）。

use serde::Deserialize;
use std::path::Path;

/// 默认容差（mm）。切片器的挤出宽度会让包围盒有零点几毫米抖动，
/// 卡太死会把正常模型拦下来 —— 与 `generator_new.go` 的两道检测同值。
const DEFAULT_TOLERANCE: f64 = 0.5;

fn default_tolerance() -> f64 {
    DEFAULT_TOLERANCE
}

/// 一档校准模型。字段名与表里的键一一对应（camelCase）。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelProfile {
    /// 档位 id（唯一）。报错、判据、前端选版都用它。
    pub id: String,
    /// `ZOffset` / `XY` / `Repetition` —— 分组见 [`mode_group`]。
    pub mode: String,
    /// catalog 里的模型资产 id（`presets/assets.toml` 那条）。可缺省。
    #[serde(default)]
    pub asset: Option<String>,
    /// 标称外形宽 / 高（**切片后实测**口径，即打印区网格 − 一个挤出宽度内缩）。
    /// `0.0` = 这一档不参与尺寸校验（Repetition）。
    #[serde(default)]
    pub footprint_w: f64,
    #[serde(default)]
    pub footprint_h: f64,
    /// 尺寸匹配容差：两轴各自比，都落在容差内才算命中。
    #[serde(default = "default_tolerance")]
    pub tolerance: f64,
    /// 图案基准（相对打印区包围盒**左下角**的毫米偏移）。
    #[serde(default)]
    pub datum_x: f64,
    #[serde(default)]
    pub datum_y: f64,
    /// Z 方块梯子的间距 / 个数。
    #[serde(default)]
    pub square_spacing: f64,
    #[serde(default)]
    pub square_count: usize,
    /// XY 图案的七个模板偏移（只有 `XY` 档要写）。
    #[serde(default)]
    pub pattern: Option<XyPattern>,
    /// 质心旋转判据的期望。**必填**：缺了它这一档就没法判旋转，
    /// 而"悄悄不判"比"报一条看不太懂的错"更坏。
    pub centroid: CentroidExpect,
}

impl ModelProfile {
    /// 这一档要不要参与尺寸认档。`Repetition` 的 L 形件没有标称外形 ⇒ 不参与。
    pub fn has_footprint(&self) -> bool {
        self.footprint_w > 0.0 && self.footprint_h > 0.0
    }
}

/// XY 图案模板（原 `XY_PATTERN_*` 七个常量）。全部是相对包围盒左下角的偏移。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct XyPattern {
    pub line_spacing: f64,
    pub h_line_offset_y: f64,
    pub h_line_start_x: f64,
    pub h_line_end_x: f64,
    pub v_line_offset_x: f64,
    pub v_line_start_y: f64,
    pub v_line_end_y: f64,
}

/// 质心旋转判据的期望（原 `check_rotation_by_centroid` 的那四个入参）。
///
/// 期望符号为 `0` 表示「这一轴没有期望」，不参与反向判定。
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CentroidExpect {
    pub dx_sign: i32,
    pub dy_sign: i32,
    /// 偏移比（偏移 / 该轴尺寸）低于它就"判不出来就不判"。
    pub min_ratio: f64,
    /// 正方形模型（XY 板）：转 90° 的 BBox 看不出来，只有质心方向能判。
    pub square: bool,
}

/// 表根：一个 `[[models]]` 数组。
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelsFile {
    #[serde(default)]
    models: Vec<ModelProfile>,
}

/// 模式串 → 档位分组。表里写的是面板级的名字（`XY`），运行时给的是内核那三个
/// 稳定 id（`Precise` / `Rough`）—— 它们是同一张板，归一组。
///
/// 认不认识由它回答（空串 = 不认识）。
pub fn mode_group(mode: &str) -> &'static str {
    match mode {
        "ZOffset" => "ZOffset",
        "XY" | "Precise" | "Rough" => "XY",
        "Repetition" => "Repetition",
        _ => "",
    }
}

/// 认档结果。
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ProfileMatch<'a> {
    /// 同一朝向命中：尺寸对得上，用这一档的基准与判据。
    Direct(&'a ModelProfile),
    /// 宽高互换命中：疑似把板转了 90°。只有非正方形板看得出来
    /// （正方形板转 90° 的包围盒一模一样，得靠质心 —— 见 `CentroidExpect::square`）。
    Swapped(&'a ModelProfile),
}

/// 按实测包围盒认档 —— 把 `generator_new.go` 那两道尺寸检测（宽高互换 / 尺寸不符）
/// 泛化成「在已知档位里找一档」。
///
/// 判定顺序**不可重排**：先试同一朝向（命中就用它），再试宽高互换（命中即旋转）。
/// 顺序反了会把「摆放正确的另一种模型」误判成旋转。
///
/// 命中判定：两轴偏差都在该档 `tolerance` 内；多档同时命中时取偏差和最小的那一档。
/// 都不命中返回 `None` —— 由调用方报「尺寸已变化」并把已知档位列出来。
pub fn find_profile<'a>(
    profiles: &'a [ModelProfile],
    mode: &str,
    width: f64,
    height: f64,
) -> Option<ProfileMatch<'a>> {
    let group = mode_group(mode);
    if group.is_empty() {
        return None;
    }

    // Repetition 不做尺寸校验（没有标称外形）⇒ 该组唯一那一档直接命中。
    if group == "Repetition" {
        return profiles
            .iter()
            .find(|p| mode_group(&p.mode) == group)
            .map(ProfileMatch::Direct);
    }

    let mut direct: Option<(f64, &ModelProfile)> = None;
    let mut swapped: Option<(f64, &ModelProfile)> = None;
    for p in profiles.iter().filter(|p| mode_group(&p.mode) == group) {
        if !p.has_footprint() {
            continue;
        }
        let dw = (width - p.footprint_w).abs();
        let dh = (height - p.footprint_h).abs();
        if dw <= p.tolerance && dh <= p.tolerance {
            let score = dw + dh;
            if direct.is_none_or(|(s, _)| score < s) {
                direct = Some((score, p));
            }
        }
        if p.footprint_w != p.footprint_h {
            let sw = (width - p.footprint_h).abs();
            let sh = (height - p.footprint_w).abs();
            if sw <= p.tolerance && sh <= p.tolerance {
                let score = sw + sh;
                if swapped.is_none_or(|(s, _)| score < s) {
                    swapped = Some((score, p));
                }
            }
        }
    }

    direct
        .map(|(_, p)| ProfileMatch::Direct(p))
        .or_else(|| swapped.map(|(_, p)| ProfileMatch::Swapped(p)))
}

/// 认不出档时给用户看的清单：这一组我们认识哪些外形。
///
/// 认档失败只有两种可能 —— 模型被缩放，或者这是一块我们还没登记的板；
/// 把已知尺寸摆出来，两种都能自己看出来。
pub fn known_footprints(profiles: &[ModelProfile], mode: &str) -> String {
    let group = mode_group(mode);
    let list: Vec<String> = profiles
        .iter()
        .filter(|p| mode_group(&p.mode) == group && p.has_footprint())
        .map(|p| format!("{}={}x{}", p.id, p.footprint_w, p.footprint_h))
        .collect();
    if list.is_empty() {
        "（这一组没有登记任何档位）".to_string()
    } else {
        list.join(" ")
    }
}

/// 读 `presets/calibration_models.toml`。
///
/// 文件不存在由调用方决定（`machine_dims` 那边给 `None`）—— 这里只管这份文件本身
/// 能不能用：解析不了、一条都没有、字段缺、id 撞名，都是硬错，消息里带文件名与档 id。
pub fn load(path: &Path) -> Result<Vec<ModelProfile>, String> {
    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("读不到 {}（{e}）", path.display()))?;
    let file: ModelsFile =
        toml::from_str(&raw).map_err(|e| format!("{} 解析失败：{e}", path.display()))?;
    if file.models.is_empty() {
        return Err(format!(
            "{} 里一条 [[models]] 都没有 —— 这张表就是校准图案的来源，空表等于没有校准能力",
            path.display()
        ));
    }
    for p in &file.models {
        validate(p).map_err(|e| format!("{}：{e}", path.display()))?;
    }
    for (i, p) in file.models.iter().enumerate() {
        if file.models[..i].iter().any(|q| q.id == p.id) {
            return Err(format!(
                "{}：档 id '{}' 撞名 —— 报错与认档都靠 id 区分",
                path.display(),
                p.id
            ));
        }
    }
    Ok(file.models)
}

/// 逐档校验：该 mode 要用的数必须在。
///
/// 「必填」不写进类型（那些字段跨 mode 共用），所以在这里逐 mode 点名 ——
/// 报出来的话必须直接说清是**哪一档缺哪个键**，否则改表的人得自己猜。
fn validate(p: &ModelProfile) -> Result<(), String> {
    let group = mode_group(&p.mode);
    if group.is_empty() {
        return Err(format!(
            "档 '{}' 的 mode '{}' 不认识（只有 ZOffset / XY / Repetition）",
            p.id, p.mode
        ));
    }
    if !p.has_footprint() && group != "Repetition" {
        return Err(format!(
            "档 '{}' 缺 footprintW/footprintH —— {group} 档要给尺寸校验一个标称外形",
            p.id
        ));
    }
    match group {
        "ZOffset" => {
            if p.datum_x == 0.0 || p.datum_y == 0.0 {
                return Err(format!("档 '{}' 缺 datumX/datumY", p.id));
            }
            if p.square_count == 0 || p.square_spacing <= 0.0 {
                return Err(format!("档 '{}' 缺 squareSpacing/squareCount", p.id));
            }
        }
        "XY" => {
            if p.pattern.is_none() {
                return Err(format!(
                    "档 '{}' 缺 [models.pattern] —— XY 图案那七个偏移没有别的来源",
                    p.id
                ));
            }
        }
        "Repetition" => {
            if p.datum_x == 0.0 || p.datum_y == 0.0 {
                return Err(format!("档 '{}' 缺 datumX/datumY", p.id));
            }
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, mode: &str, w: f64, h: f64) -> ModelProfile {
        ModelProfile {
            id: id.to_string(),
            mode: mode.to_string(),
            asset: None,
            footprint_w: w,
            footprint_h: h,
            tolerance: DEFAULT_TOLERANCE,
            datum_x: 0.079,
            datum_y: 3.755,
            square_spacing: 11.0,
            square_count: 11,
            pattern: None,
            centroid: CentroidExpect {
                dx_sign: 1,
                dy_sign: 0,
                min_ratio: 0.01,
                square: false,
            },
        }
    }

    /// 生产清单那两档：同一 mode 下按尺寸分得开。
    fn two_z_offsets() -> Vec<ModelProfile> {
        vec![
            profile("zoffset", "ZOffset", 119.58, 13.063),
            profile("zoffset-ext", "ZOffset", 119.58, 16.063),
        ]
    }

    #[test]
    fn old_model_hits_the_old_profile() {
        let ps = two_z_offsets();
        match find_profile(&ps, "ZOffset", 119.511, 13.055) {
            Some(ProfileMatch::Direct(p)) => assert_eq!(p.id, "zoffset"),
            other => panic!("旧模型没认到旧档: {other:?}"),
        }
    }

    /// 2026-10-18 真机那份：119.5111 × 16.0630 —— 必须落到延长版那一档。
    #[test]
    fn extended_model_hits_the_extended_profile() {
        let ps = two_z_offsets();
        match find_profile(&ps, "ZOffset", 119.5111, 16.0630) {
            Some(ProfileMatch::Direct(p)) => assert_eq!(p.id, "zoffset-ext"),
            other => panic!("新模型没认到新档: {other:?}"),
        }
    }

    /// 未知尺寸 ⇒ 不认档（调用方据此报「尺寸已变化」），清单要把两档都列出来。
    #[test]
    fn unknown_size_matches_nothing_and_lists_both() {
        let ps = two_z_offsets();
        assert!(find_profile(&ps, "ZOffset", 100.0, 16.0).is_none());
        let list = known_footprints(&ps, "ZOffset");
        assert!(list.contains("zoffset=119.58x13.063"), "{list}");
        assert!(list.contains("zoffset-ext=119.58x16.063"), "{list}");
    }

    /// 宽高互换 ⇒ `Swapped`（报旋转），而不是「尺寸已变化」。
    #[test]
    fn swapped_dimensions_are_reported_as_rotation() {
        let ps = vec![profile("zoffset", "ZOffset", 119.58, 13.063)];
        match find_profile(&ps, "ZOffset", 13.063, 119.58) {
            Some(ProfileMatch::Swapped(p)) => assert_eq!(p.id, "zoffset"),
            other => panic!("互换尺寸应判成旋转: {other:?}"),
        }
    }

    /// 正方形板（XY 50.5×50.5）看不出互换 —— 两条检测都不能命中，
    /// 方向只能靠质心（与 `generator_new.go` 同口径）。
    #[test]
    fn square_profile_never_matches_swapped() {
        let ps = vec![profile("precise", "XY", 50.5, 50.5)];
        assert!(matches!(
            find_profile(&ps, "Precise", 50.5, 50.5),
            Some(ProfileMatch::Direct(_))
        ));
        assert!(find_profile(&ps, "Rough", 40.0, 30.0).is_none());
    }

    /// 分组：Precise / Rough 走同一组，ZOffset 的档不会被 XY 认走。
    #[test]
    fn modes_do_not_cross_groups() {
        let ps = two_z_offsets();
        assert!(find_profile(&ps, "Precise", 119.58, 13.063).is_none());
        assert!(find_profile(&ps, "None", 119.58, 13.063).is_none());
    }

    /// Repetition 不参与尺寸校验 ⇒ 无视尺寸直接命中该组那一档。
    #[test]
    fn repetition_matches_without_size_check() {
        let ps = vec![profile("repetition", "Repetition", 0.0, 0.0)];
        match find_profile(&ps, "Repetition", 12.0, 34.0) {
            Some(ProfileMatch::Direct(p)) => assert_eq!(p.id, "repetition"),
            other => panic!("Repetition 应无视尺寸命中: {other:?}"),
        }
    }
}
