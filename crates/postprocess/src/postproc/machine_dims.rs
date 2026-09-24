//! 机型表：尺寸 + 别名 + 禁区。**唯一来源是 `presets/`**（b04 Task 13 / M3）。
//!
//! # 为什么不再是编译期快照
//!
//! 搬进来之前这里是 `include_str!` 嵌进二进制的两份 JSON —— 那在源仓库是合理的
//! （那边的机型集是构建期常量，换机型集 = 出新版本）。在我们的仓库里不成立：
//! `presets/machines/*.toml` 是**可编辑的唯一真源**，工作台就靠改它来交付。
//! 快照与它并存的话，改完尺寸再生成，产物用的还是旧几何 —— 而且不会报错。
//!
//! 所以数据改成**注入**：进程启动时由调用方给一份（[`install`]），
//! 或者从 `presets/` 目录读（[`load_presets_dir`]）。查不到仍然返回 zero-value
//! （M017「禁止机型回退」的语义保留，与旧侧一致）。
//!
//! # 为什么允许"没人装过"时自己去读 `presets/`
//!
//! 这个 crate 的判据有 246 条，其中相当一部分会走到这里（几何、涂胶、擦拭、标定）。
//! 要在每条判据里插一行全局安装是不现实的，而且 `OnceLock` 只能装一次 ——
//! 测试之间会互相依赖执行顺序。
//!
//! 所以默认值是一条**有顺序的逃生链**：显式装过 → 环境变量 `MKPSE_PRESETS_DIR` →
//! 编译期记下的仓库相对路径 `../../presets`。前两条是给发布物与命令行用的，
//! 第三条只在"在仓库里跑"时成立。三条都不成立就 **panic 并说明怎么装** ——
//! 静默用空表会让产物在几何上悄悄错掉，那是这个文件最不能接受的失败方式。

#![allow(clippy::collapsible_if)] // 机型检测四优先级循环与 Go 同形，不改写

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

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

/// 一份完整的机型表：尺寸 + 别名 + 禁区。
///
/// 它**只是数据的容器**，不含任何规则 —— 规则在下面的查询函数里。
#[derive(Debug, Default)]
pub struct MachineTables {
    /// 规范机型 id → 尺寸。**没有 `[dimensions]` 的机型不在这里**（A2L 就是这种，
    /// 而"没有尺寸的机型要硬错"那条判据正靠这个差集算出来）
    dimensions: HashMap<String, MachineDimensions>,
    /// **大写** alias → 规范机型 id。查询侧只做大写化，与旧快照同语义
    alias_map: HashMap<String, String>,
    /// 规范机型 id → 禁区多边形（键是**原样的 id**，不做大小写归一）
    forbidden_zones: HashMap<String, Vec<ZonePolygon>>,
}

impl MachineTables {
    pub fn dimensions(&self) -> &HashMap<String, MachineDimensions> {
        &self.dimensions
    }
    pub fn alias_map(&self) -> &HashMap<String, String> {
        &self.alias_map
    }
    pub fn forbidden_zones(&self) -> &HashMap<String, Vec<ZonePolygon>> {
        &self.forbidden_zones
    }
}

/// 那份进程级的表。**只装一次**（见 [`install`]）。
static TABLES: OnceLock<MachineTables> = OnceLock::new();

/// 装一次（进程级）。**第二次会返回 Err 而不是覆盖** ——
/// 静默替换会把"两处数据"变成"看谁先跑"，那正是这个文件要消灭的东西。
pub fn install(tables: MachineTables) -> Result<(), String> {
    TABLES
        .set(tables)
        .map_err(|_| "机型表已经装过了（OnceLock 只能装一次）".to_string())
}

/// 从 `presets/` 目录读一份机型表：
/// `machines/*.toml` 的 `[dimensions]` 与 `externalAliases`，以及
/// `forbidden_zones/*.toml` 的 `[[zones]]`。
pub fn load_presets_dir(dir: &Path) -> Result<MachineTables, String> {
    let mut tables = MachineTables::default();

    let machines_dir = dir.join("machines");
    let entries = std::fs::read_dir(&machines_dir)
        .map_err(|e| format!("读不出 {}（{e}）", machines_dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("toml") {
            continue;
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| format!("读不到 {}（{e}）", path.display()))?;
        let file: MachineFile =
            toml::from_str(&raw).map_err(|e| format!("{} 解析失败：{e}", path.display()))?;

        // 别名：**键一律大写** —— 查询侧 `lookup_alias` 只做大写化（与旧快照同语义）
        tables
            .alias_map
            .insert(file.id.to_uppercase(), file.id.clone());
        for alias in &file.external_aliases {
            tables
                .alias_map
                .insert(alias.to_uppercase(), file.id.clone());
        }

        // 没有 `[dimensions]` 的机型**不入表**（不是填 zero-value）：
        // 「别名认识但尺寸表里没有」这个差集本身是一条判据的输入
        if let Some(dims) = file.dimensions {
            tables.dimensions.insert(file.id.clone(), dims);
        }
    }

    let zones_dir = dir.join("forbidden_zones");
    if let Ok(entries) = std::fs::read_dir(&zones_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) != Some("toml") {
                continue;
            }
            let Some(id) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let raw = std::fs::read_to_string(&path)
                .map_err(|e| format!("读不到 {}（{e}）", path.display()))?;
            let file: ZoneFile =
                toml::from_str(&raw).map_err(|e| format!("{} 解析失败：{e}", path.display()))?;
            if !file.zones.is_empty() {
                tables.forbidden_zones.insert(id.to_string(), file.zones);
            }
        }
    }

    Ok(tables)
}

/// 进程级的机型表：显式装过就用它，否则按模块文档那条逃生链去找 `presets/`。
pub fn tables() -> &'static MachineTables {
    TABLES.get_or_init(|| {
        if let Ok(dir) = std::env::var("MKPSE_PRESETS_DIR") {
            return load_presets_dir(Path::new(&dir))
                .unwrap_or_else(|e| panic!("MKPSE_PRESETS_DIR={dir} 读不出来：{e}"));
        }
        let repo_default = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets");
        if repo_default.is_dir() {
            return load_presets_dir(&repo_default)
                .unwrap_or_else(|e| panic!("{} 读不出来：{e}", repo_default.display()));
        }
        panic!(
            "没有机型表。发布物必须在启动时调 `machine_dims::install(..)`；\
             开发与测试可以设 MKPSE_PRESETS_DIR 指向 presets/ 目录"
        );
    })
}

/// `presets/machines/*.toml` 里我们用到的那几个字段。
///
/// **必须有 `rename_all = "camelCase"`**：清单里的键是 `externalAliases`，
/// 少了这一行它会静默变成空数组 —— 于是所有别名解析成空串、机型识别全挂，
/// 而不会有任何报错。（M3 落地时 `presets_reproduce_the_legacy_alias_map`
/// 就是这么抓到的：23 条变 6 条。那条判据的存在理由就是这种"读出来了但读空了"。）
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MachineFile {
    id: String,
    #[serde(default)]
    external_aliases: Vec<String>,
    /// 没有这一段的机型不入尺寸表（`presets/machines/*.toml` 里只有 A2L 是这样）
    #[serde(default)]
    dimensions: Option<MachineDimensions>,
}

#[derive(Debug, Deserialize)]
struct ZoneFile {
    #[serde(default)]
    zones: Vec<ZonePolygon>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ZonePolygon {
    #[serde(default)]
    pub points: Vec<ZonePoint>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ZonePoint {
    #[serde(default)]
    pub x: f64,
    #[serde(default)]
    pub y: f64,
}

/// `lookupAlias`（machine/normalize.go）：大写 alias → Canonical Machine ID。
fn lookup_alias(raw: &str) -> Option<String> {
    let upper = raw.trim().to_uppercase();
    tables().alias_map.get(&upper).cloned()
}

/// `NormalizeToCanonical`（machine/normalize.go:308）：alias 归一；未命中返回空串
/// （Go ResolveModelID 语义；preset 按约定填 Canonical ID）。
pub fn normalize_to_canonical(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }
    lookup_alias(raw).unwrap_or_default()
}

/// `PopulateForbiddenZones`（ir/forbidden_zones.go:12）：从机型表的禁区段填充 IR；
/// 未命中/空 → 保留空（警告记日志，不阻塞）。数据来自 `presets/forbidden_zones/*.toml`。
pub fn populate_forbidden_zones(ir: &mut crate::ir::Ir, machine_type: &str) {
    if machine_type.is_empty() {
        return;
    }
    let Some(zone_list) = tables().forbidden_zones.get(machine_type) else {
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
    let map = &tables().dimensions;
    if let Some(d) = map.get(machine_type) {
        return d.clone();
    }
    let lower = machine_type.to_lowercase();
    for (k, d) in map {
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
/// 别名表认 6 个规范名（含 `A2L`），而尺寸表只有 5 个 ⇒
/// `check --set Machine.MachineType=A2L` 曾经退 0 报「配置可用」，
/// 打印出 `X 0.0..0.0 / Y 0.0..0.0`。
/// 判断「命中没命中」必须问这个函数，不能靠看返回值是不是零。
///
/// 查法与 [`get_machine_dimensions`] **逐字一致**（先精确、再大小写不敏感），
/// 否则两者会对同一个名字给出不同答案。
pub fn has_machine_dimensions(machine_type: &str) -> bool {
    let map = &tables().dimensions;
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
