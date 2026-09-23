//! 机型维度快照 —— 从旧仓库 seeds/machine_catalog.json 的 `dimensions` 字段
//! 抽取（A1/A1_MINI/P1S/P2S/X1C 五机型），`include_str!` 嵌入。
//!
//! 这是 design.md §3.4「参数元数据不走内容分发管线」同一决定的延伸：
//! 旧侧经 ContentProvider 读 catalog（M017：catalog SSOT、禁止机型回退），
//! Rust 侧把快照编进二进制，换机型集 = 出新版本。查不到 = zero-value
//! （与旧侧「未命中返回 zero-value + 错误日志」语义一致，M017 同守）。

#![allow(clippy::collapsible_if)] // 机型检测四优先级循环与 Go 同形，不改写

use serde::Deserialize;
use std::collections::HashMap;

pub const MACHINE_DIMENSIONS_JSON: &str = include_str!("../../assets/machine_dimensions.json");
pub const MACHINE_CATALOG_EXTRA_JSON: &str =
    include_str!("../../assets/machine_catalog_extra.json");

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BedSize {
    #[serde(default)]
    pub width: f64,
    #[serde(default)]
    pub depth: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MovementRange {
    #[serde(default)]
    pub min_x: f64,
    #[serde(default)]
    pub max_x: f64,
    #[serde(default)]
    pub min_y: f64,
    #[serde(default)]
    pub max_y: f64,
    #[serde(default)]
    pub max_z: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlueArea {
    #[serde(default)]
    pub glue_max_y: f64,
    #[serde(default)]
    pub glue_min_y: f64,
    #[serde(default)]
    pub glue_max_x: f64,
    #[serde(default)]
    pub glue_min_x: f64,
    #[serde(default)]
    pub wipe_x: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Calibration {
    #[serde(default)]
    pub y_line_x: f64,
    #[serde(default)]
    pub y_line_x_end: f64,
    #[serde(default)]
    pub y_line_y: f64,
    #[serde(default)]
    pub x_line_y: f64,
    #[serde(default)]
    pub x_line_y_end: f64,
    #[serde(default)]
    pub x_line_x: f64,
    #[serde(default)]
    pub z_start_x: f64,
    #[serde(default)]
    pub z_start_y: f64,
    #[serde(default)]
    pub l_shape_base_x: f64,
    #[serde(default)]
    pub l_shape_base_y: f64,
}

/// 机型能力标志（pass1 的风扇格式判定消费 `has_second_fan`）。
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Flags {
    #[serde(default)]
    pub has_second_fan: bool,
    #[serde(default)]
    pub gcode_marker: String,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MachineDimensions {
    #[serde(default)]
    pub bed_size: BedSize,
    #[serde(default)]
    pub movement_range: MovementRange,
    #[serde(default)]
    pub edge_zone: f64,
    #[serde(default)]
    pub glue_area: GlueArea,
    #[serde(default)]
    pub calibration: Calibration,
    #[serde(default)]
    pub flags: Flags,
}

/// 机型目录扩展快照（aliasMap + forbiddenZones，从 machine_catalog.json 抽取）。
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CatalogExtra {
    #[serde(default)]
    alias_map: std::collections::HashMap<String, String>,
    #[serde(default)]
    forbidden_zones: std::collections::HashMap<String, Vec<ZonePolygon>>,
}

#[derive(Debug, Deserialize)]
struct ZonePolygon {
    #[serde(default)]
    points: Vec<ZonePoint>,
}

#[derive(Debug, Deserialize)]
struct ZonePoint {
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
}

fn catalog_extra() -> &'static CatalogExtra {
    static EXTRA: std::sync::OnceLock<CatalogExtra> = std::sync::OnceLock::new();
    EXTRA.get_or_init(|| {
        serde_json::from_str(MACHINE_CATALOG_EXTRA_JSON).expect("机型目录扩展快照损坏")
    })
}

/// `lookupAlias`（machine/normalize.go）：大写 alias → Canonical Machine ID。
fn lookup_alias(raw: &str) -> Option<String> {
    let upper = raw.trim().to_uppercase();
    catalog_extra().alias_map.get(&upper).cloned()
}

/// `NormalizeToCanonical`（machine/normalize.go:308）：alias 归一；未命中返回空串
/// （Go ResolveModelID 语义；preset 按约定填 Canonical ID）。
pub fn normalize_to_canonical(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    lookup_alias(raw).unwrap_or_default()
}

/// `PopulateForbiddenZones`（ir/forbidden_zones.go:12）：从 catalog 顶层
/// forbiddenZones 填充 IR；未命中/空 → 保留空（警告记日志，不阻塞）。
pub fn populate_forbidden_zones(ir: &mut crate::ir::Ir, machine_type: &str) {
    if machine_type.is_empty() {
        return;
    }
    let Some(zone_list) = catalog_extra().forbidden_zones.get(machine_type) else {
        tracing::warn!(
            machine_type,
            "禁区数据为空，跳过禁区填充（碰撞检测将不包含禁区）"
        );
        return;
    };
    if zone_list.is_empty() {
        tracing::warn!(
            machine_type,
            "禁区数据为空，跳过禁区填充（碰撞检测将不包含禁区）"
        );
        return;
    }
    let mut result: Vec<crate::ir::Polygon> = Vec::new();
    for z in zone_list {
        if z.points.len() < 3 {
            tracing::warn!(
                count = z.points.len(),
                "禁区多边形顶点不足 3 个，跳过该多边形"
            );
            continue;
        }
        result.push(z.points.iter().map(|p| vec![p.x, p.y]).collect());
    }
    ir.machine.forbidden_zones = result;
}

/// `DetectFromGcode`（machine/normalize.go:134）四优先级：
/// 长标记(带 label) → 长标记(无 label) → printer_model 后缀匹配 → 短注释
/// （最长到最短）。全不命中返回空串（调用方按 UNKNOWN 展示）。
pub fn detect_machine_from_gcode(content: &str) -> String {
    // 优先级 1+2：`;===== machine: <model> =====` / `;===== A1mini 20251031 =====`
    for line in content.lines() {
        let line = line.trim();
        if !line.starts_with(";=") {
            continue;
        }
        // 去掉首尾 ';='* 前缀与 '=' 尾部后取中段，识别两种形态
        if let Some(model) = parse_long_machine_marker_label(line) {
            if let Some(c) = lookup_alias(&model) {
                return c;
            }
        }
        if let Some(first_token) = parse_long_machine_marker_no_label(line) {
            if let Some(c) = lookup_alias(&first_token) {
                return c;
            }
        }
    }
    // 优先级 3：`; printer_model = Bambu Lab A1 mini`（去品牌前缀逐步缩短匹配）
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with(';') && line.to_lowercase().contains("printer_model") {
            if let Some(eq_idx) = line.find('=') {
                if eq_idx < line.len() - 1 {
                    let value = line[eq_idx + 1..].trim();
                    if let Some(c) = lookup_printer_model_value(value) {
                        return c;
                    }
                }
            }
        }
    }
    // 优先级 4：短注释 `;A1M`（最长到最短）
    for line in content.lines() {
        let line = line.trim();
        if let Some(c) = parse_short_machine_comment(line) {
            return c;
        }
    }
    String::new()
}

/// `^;=+\s*machine\s*:\s*(.+?)\s*=*$`（label 形态）。
fn parse_long_machine_marker_label(line: &str) -> Option<String> {
    let rest = line.trim_start_matches(';').trim_start_matches('=');
    let rest = rest.trim();
    let model = rest
        .strip_prefix("machine")?
        .trim()
        .strip_prefix(':')?
        .trim();
    let model = model.trim_end_matches('=').trim();
    if model.is_empty() {
        return None;
    }
    Some(model.to_string())
}

/// `^;=+\s*([A-Za-z0-9_]+)\s+\S.*=*$`（无 label：第一段为机型标识）。
fn parse_long_machine_marker_no_label(line: &str) -> Option<String> {
    let rest = line.trim_start_matches(';').trim_start_matches('=').trim();
    let mut chars = rest.chars();
    let mut token = String::new();
    for c in chars.by_ref() {
        if c.is_ascii_alphanumeric() || c == '_' {
            token.push(c);
        } else {
            break;
        }
    }
    if token.is_empty() {
        return None;
    }
    // 后面必须跟空白 + 至少一个非空白字符（日期/版本），避免误吃标准标记
    let after = rest[token.len()..].trim_start();
    if after.is_empty() || after.trim_end_matches('=').trim().is_empty() {
        return None;
    }
    Some(token)
}

/// `lookupPrinterModelValue`：从最长后缀开始去词匹配（"Bambu Lab A1 mini" → A1MINI）。
fn lookup_printer_model_value(value: &str) -> Option<String> {
    if value.is_empty() {
        return None;
    }
    let words: Vec<&str> = value.split_whitespace().collect();
    for i in 0..words.len() {
        let candidate: String = words[i..].concat().to_uppercase();
        if let Some(c) = lookup_alias(&candidate) {
            return Some(c);
        }
    }
    None
}

/// `parseShortMachineComment`：`;` 后标识符从最长到最短匹配。
fn parse_short_machine_comment(line: &str) -> Option<String> {
    if line.len() < 2 || !line.starts_with(';') {
        return None;
    }
    let rest = line[1..].trim();
    if rest.is_empty() {
        return None;
    }
    let upper = rest.to_uppercase();
    for end in (2..=upper.len()).rev() {
        if !upper.is_char_boundary(end) {
            continue;
        }
        if let Some(c) = lookup_alias(&upper[..end]) {
            return Some(c);
        }
    }
    None
}

/// `FillMachineDimsFromToml`（engine/helpers.go:103）：维度填充（含涂胶区）。
pub fn fill_machine_dims_from_toml(toml_machine: &str, ir: &mut crate::ir::Ir) {
    if toml_machine.is_empty() {
        tracing::error!("fill_machine_dims_from_toml 收到空 tomlMachine（防御性处理）");
        return;
    }
    let dims = get_machine_dimensions(toml_machine);
    ir.machine.max_x = dims.movement_range.max_x;
    ir.machine.min_x = dims.movement_range.min_x;
    ir.machine.max_y = dims.movement_range.max_y;
    ir.machine.min_y = dims.movement_range.min_y;
    ir.machine.glue_max_y = dims.glue_area.glue_max_y;
    ir.machine.glue_min_y = dims.glue_area.glue_min_y;
    ir.machine.glue_max_x = dims.glue_area.glue_max_x;
    ir.machine.glue_min_x = dims.glue_area.glue_min_x;
}

/// 按 Canonical Machine ID 查维度；未命中 zero-value（M017：禁止机型回退）。
/// 大小写不敏感兜底与旧侧 lookupCaseInsensitive 一致。
pub fn get_machine_dimensions(machine_type: &str) -> MachineDimensions {
    let map: HashMap<String, MachineDimensions> =
        serde_json::from_str(MACHINE_DIMENSIONS_JSON).expect("机型维度快照损坏");
    if let Some(d) = map.get(machine_type) {
        return d.clone();
    }
    let lower = machine_type.to_lowercase();
    for (k, d) in &map {
        if k.to_lowercase() == lower {
            return d.clone();
        }
    }
    MachineDimensions::default()
}

/// 尺寸表里**有没有**这个 Canonical Machine ID。
///
/// 为什么需要这张单独的脸：[`get_machine_dimensions`] 未命中时返回 zero-value
/// （M017 禁止机型回退，那个语义要保留），于是「A2L 这台机器的运动范围是 0×0」
/// 与「尺寸表里没有 A2L」在返回值上**长得一模一样**。实测后果：
/// `machine_catalog_extra.json` 的 aliasMap 认 6 个规范名（含 `A2L`），
/// 而 `machine_dimensions.json` 只有 5 个 ⇒ `check --set Machine.MachineType=A2L`
/// 曾经退 0 报「配置可用」，打印出 `X 0.0..0.0 / Y 0.0..0.0`。
/// 判断「命中没命中」必须问这个函数，不能靠看返回值是不是零。
///
/// 查法与 [`get_machine_dimensions`] **逐字一致**（先精确、再大小写不敏感），
/// 否则两者会对同一个名字给出不同答案。
pub fn has_machine_dimensions(machine_type: &str) -> bool {
    let map: HashMap<String, MachineDimensions> =
        serde_json::from_str(MACHINE_DIMENSIONS_JSON).expect("机型维度快照损坏");
    if map.contains_key(machine_type) {
        return true;
    }
    let lower = machine_type.to_lowercase();
    map.keys().any(|k| k.to_lowercase() == lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 反空转 + A1_MINI 校准坐标锚定（Task 4 参考导出实测值）。
    #[test]
    fn snapshot_has_all_machines() {
        let d = get_machine_dimensions("A1_MINI");
        assert!((d.calibration.y_line_x - 66.523).abs() < 1e-9);
        assert!((d.calibration.y_line_x_end - 76.523).abs() < 1e-9);
        assert!((d.calibration.x_line_x - 76.523).abs() < 1e-9);
        assert!((d.calibration.z_start_x - 30.21).abs() < 1e-9);
        for m in ["A1", "A1_MINI", "P1S", "P2S", "X1C"] {
            assert!(
                get_machine_dimensions(m).calibration.z_start_x != 0.0,
                "{m} 维度缺失"
            );
        }
        assert_eq!(get_machine_dimensions("NOPE"), MachineDimensions::default());
    }
}
