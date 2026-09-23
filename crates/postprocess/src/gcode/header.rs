//! 切片器头部注释的刮取（纯函数，行为规格 = 旧 Go `engine/process.go` 与
//! `engine/helpers.go`）。
//!
//! 这四条在 Go 侧都发生在 `machine-detect` / `parse` 之间、纯读 `content`，与
//! MKP 预设无关，故落在 rank 0 而不是 engine 里内联——engine 只编排。
//!
//! 移植对照：
//! - `machine_preset_name`     ← process.go:364-366（`; printer_settings_id = `）
//! - `process_preset_name`     ← process.go:367-369（`; default_print_profile = `）
//! - `slicer_ironing_enabled`  ← process.go:358-363（`; enable_support_ironing = ` 且首数 == 1）
//! - `three_mf_file_name`      ← helpers.go:114-139（`; filename_format` 占位符展开）
//!
//! **Go 的两处形态被逐字保留，不"顺手修正"**：
//! 1. 判定用 `Contains` 而取值用 `TrimPrefix` —— 标记不在行首时 `TrimPrefix`
//!    什么也不去，整行被当成值。此处用 `strip_prefix().unwrap_or(该行)` 复刻。
//! 2. 循环不 break ⇒ **最后一次命中生效**（不是第一次）。

use std::path::Path;

const PREFIX_PRINTER_SETTINGS_ID: &str = "; printer_settings_id = ";
const PREFIX_DEFAULT_PRINT_PROFILE: &str = "; default_print_profile = ";
const PREFIX_ENABLE_SUPPORT_IRONING: &str = "; enable_support_ironing = ";
const MARKER_FILENAME_FORMAT: &str = "; filename_format";

/// 按 Go 的「Contains 判定 + TrimPrefix 取值 + 不 break」刮一个头部键。
fn scrape_last(content: &[String], prefix: &str) -> String {
    let mut found = String::new();
    for line in content {
        let trimmed = line.trim();
        if trimmed.contains(prefix) {
            found = trimmed
                .strip_prefix(prefix)
                .unwrap_or(trimmed)
                .trim()
                .to_string();
        }
    }
    found
}

/// `; printer_settings_id = Bambu Lab A1 mini 0.4 nozzle` → 机器预设名。
pub fn machine_preset_name(content: &[String]) -> String {
    scrape_last(content, PREFIX_PRINTER_SETTINGS_ID)
}

/// `; default_print_profile = 0.20mm Standard @BBL A1M` → 工艺预设名。
pub fn process_preset_name(content: &[String]) -> String {
    scrape_last(content, PREFIX_DEFAULT_PRINT_PROFILE)
}

/// 切片器自身是否开了支撑熨平：`; enable_support_ironing = 1`。
///
/// Go 取的是 `ir.NumStrip(trimmed)` 的**首个数字**是否 `== 1`，且一旦为 true
/// 后续行不会把它改回 false。`NumStrip` 在 rank 2（`mkp-ir`）不可用于本层，
/// 这里只需要"首个数字"，故就地扫一遍（语义与 `NumStrip` 的首元素一致：
/// 负号只在数字起始处生效、最多一个小数点）。
pub fn slicer_ironing_enabled(content: &[String]) -> bool {
    let mut enabled = false;
    for line in content {
        let trimmed = line.trim();
        if trimmed.contains(PREFIX_ENABLE_SUPPORT_IRONING) && first_number(trimmed) == Some(1.0) {
            enabled = true;
        }
    }
    enabled
}

/// `NumStrip` 的首元素：按 Go `ir.NumStrip`（build.go:816）的扫描规则取第一个数。
fn first_number(line: &str) -> Option<f64> {
    let mut buf = String::new();
    let mut has_dot = false;
    for c in line.chars() {
        if c == '-' && buf.is_empty() {
            buf.push('-');
        } else if c.is_ascii_digit() {
            buf.push(c);
        } else if c == '.' && !has_dot && !buf.is_empty() {
            buf.push('.');
            has_dot = true;
        } else {
            // Go 侧遇到分隔符时才收束一个数；只有 "-" 不算数。
            if buf.len() > 1 || (buf.len() == 1 && buf != "-") {
                return buf.parse().ok();
            }
            buf.clear();
            has_dot = false;
        }
    }
    if buf.len() > 1 || (buf.len() == 1 && buf != "-") {
        return buf.parse().ok();
    }
    None
}

/// 从 `; filename_format = ...` 推 3MF 文件名；没有该行时退回「输入名 + .3mf」。
///
/// Go 侧占位符只处理三个（`{input_filename_base}` / `{filament_type[0]}` /
/// `{print_time}`），其余占位符**原样留在结果里**——这是 Go 的现状，照抄。
///
/// 边界（Go 用 `filepath.Ext` + `TrimSuffix`，这里用 `file_stem`）：对
/// `".gcode"` 这种纯扩展名文件，Go 得 `""` 而 `file_stem` 得 `".gcode"`。
/// 真实输入总有名字部分，故该角落在本链上不可达，登记而不绕。
pub fn three_mf_file_name(content: &[String], gcode_path: &str) -> String {
    let name_without_ext = Path::new(gcode_path)
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_default();
    for line in content {
        // Go 这里读的是**未 trim** 的原行（helpers.go:116）。
        if !line.contains(MARKER_FILENAME_FORMAT) {
            continue;
        }
        let Some(idx) = line.find('=') else { continue };
        let format = line[idx + 1..].trim();
        let mut result = format
            .replace("{input_filename_base}", &name_without_ext)
            .replace("{filament_type[0]}", "")
            .replace("{print_time}", "")
            .replace("__", "_");
        result = result.trim_end_matches('_').to_string();
        if result.is_empty() {
            result = name_without_ext.clone();
        }
        return format!("{result}.3mf");
    }
    format!("{name_without_ext}.3mf")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn scrapes_preset_names_and_takes_the_last_hit() {
        let c = lines(&[
            "; printer_settings_id = first",
            "; default_print_profile = 0.20mm Standard @BBL A1M",
            "; printer_settings_id = Bambu Lab A1 mini 0.4 nozzle",
        ]);
        // 不 break ⇒ 最后一次命中生效（Go 形态）。
        assert_eq!(machine_preset_name(&c), "Bambu Lab A1 mini 0.4 nozzle");
        assert_eq!(process_preset_name(&c), "0.20mm Standard @BBL A1M");
        assert_eq!(machine_preset_name(&[]), "");
    }

    #[test]
    fn ironing_needs_the_exact_marker_and_value_one() {
        // 真实输入里 `different_settings_to_system` 那行**含**这个词但没有
        // `; enable_support_ironing = ` 这个串 ⇒ 不得命中。
        let noise = lines(&["; different_settings_to_system = a;enable_support_ironing;b"]);
        assert!(!slicer_ironing_enabled(&noise));
        assert!(slicer_ironing_enabled(&lines(&[
            "; enable_support_ironing = 1"
        ])));
        assert!(!slicer_ironing_enabled(&lines(&[
            "; enable_support_ironing = 0"
        ])));
    }

    #[test]
    fn three_mf_expands_placeholders_like_go() {
        let c = lines(&[
            "; filename_format = {input_filename_base}_{filament_type[0]}_{print_time}.gcode",
        ]);
        assert_eq!(
            three_mf_file_name(&c, "/tmp/42274.2.gcode"),
            "42274.2_.gcode.3mf"
        );
        // 无 filename_format 行 ⇒ 退回输入名。
        assert_eq!(three_mf_file_name(&[], "/tmp/42274.2.gcode"), "42274.2.3mf");
        // 占位符展开后全空 ⇒ 退回输入名（Go helpers.go:128-130）。
        let empty = lines(&["; filename_format = {filament_type[0]}{print_time}"]);
        assert_eq!(three_mf_file_name(&empty, "/tmp/m.gcode"), "m.3mf");
    }

    #[test]
    fn first_number_matches_numstrip_head() {
        assert_eq!(first_number("; a = 1"), Some(1.0));
        assert_eq!(first_number("x-2.5y3"), Some(-2.5));
        assert_eq!(first_number("no digits"), None);
        // Go 侧第二个 '-' 落进 else 分支 ⇒ 只含 "-" 的 buf 被丢弃后重新起数 ⇒ 是 7 不是 -7。
        assert_eq!(first_number("--7"), Some(7.0));
    }
}
