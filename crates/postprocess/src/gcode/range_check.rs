//! 工具头运动范围检测（对照 range_check.go）。
//!
//! 消费方：首笔活化（revitalization_block）与热床预涂胶。返回违规列表交给
//! 上层发警告——本 crate 不做 IO、不打日志。
//!
//! 与旧侧的一处刻意偏离：Go 在 movement_range 全零时 `fmt.Println` 一条提示
//! （污染 stdout）。Rust 侧静默返回空——CLI 的 stdout 是结果通道
//!（tasks.md 15.4），诊断走返回值。行为差异仅限诊断噪声，检测语义不变。

/// 机型软件运动极限（与 `ir::MachineIR` 同名字段一一对应；独立定义避免
/// gcode→ir 反向依赖——Rust 侧同样必须维持这个方向）。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct MachineMovementRange {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

/// 一次越界事件（四方向各最多一条，取该方向最大超出量）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RangeViolation {
    /// 如 "first_pen_revitalization" / "bed_pre_print_glue"
    pub source: &'static str,
    /// "X+" | "X-" | "Y+" | "Y-"
    pub direction: &'static str,
    pub actual_value: f64,
    pub machine_limit: f64,
    pub wiper_x: f64,
    pub wiper_y: f64,
    pub toolhead_x_off: f64,
    pub toolhead_y_off: f64,
}

/// `CheckXYRange`：扫描所有 `G1 X.. Y..` 行的机器坐标，返回越界列表
/// （按 X+ / X- / Y+ / Y- 固定顺序）。范围全零 = 机型未识别 → 返回空
/// （M017：禁止回退默认值）。
pub fn check_xy_range(
    gcode_lines: &[String],
    movement_range: MachineMovementRange,
    source: &'static str,
    wiper_x: f64,
    wiper_y: f64,
    toolhead_x_off: f64,
    toolhead_y_off: f64,
) -> Vec<RangeViolation> {
    if movement_range.min_x == 0.0
        && movement_range.max_x == 0.0
        && movement_range.min_y == 0.0
        && movement_range.max_y == 0.0
    {
        return Vec::new();
    }

    let mut x_plus_max = 0.0;
    let mut x_minus_min = 0.0;
    let mut y_plus_max = 0.0;
    let mut y_minus_min = 0.0;
    let mut has_x_plus = false;
    let mut has_x_minus = false;
    let mut has_y_plus = false;
    let mut has_y_minus = false;

    for line in gcode_lines {
        let Some((x, y)) = extract_xy(line) else {
            continue;
        };
        if x > movement_range.max_x && (!has_x_plus || x > x_plus_max) {
            x_plus_max = x;
            has_x_plus = true;
        }
        if x < movement_range.min_x && (!has_x_minus || x < x_minus_min) {
            x_minus_min = x;
            has_x_minus = true;
        }
        if y > movement_range.max_y && (!has_y_plus || y > y_plus_max) {
            y_plus_max = y;
            has_y_plus = true;
        }
        if y < movement_range.min_y && (!has_y_minus || y < y_minus_min) {
            y_minus_min = y;
            has_y_minus = true;
        }
    }

    let mut violations = Vec::with_capacity(4);
    let make = |direction, actual, limit| RangeViolation {
        source,
        direction,
        actual_value: actual,
        machine_limit: limit,
        wiper_x,
        wiper_y,
        toolhead_x_off,
        toolhead_y_off,
    };
    if has_x_plus {
        violations.push(make("X+", x_plus_max, movement_range.max_x));
    }
    if has_x_minus {
        violations.push(make("X-", x_minus_min, movement_range.min_x));
    }
    if has_y_plus {
        violations.push(make("Y+", y_plus_max, movement_range.max_y));
    }
    if has_y_minus {
        violations.push(make("Y-", y_minus_min, movement_range.min_y));
    }
    violations
}

/// `extractXY`：仅处理 trim 后以 "G1 " 开头的行；剥离注释；X、Y 都要有且可解析。
fn extract_xy(line: &str) -> Option<(f64, f64)> {
    let trimmed = line.trim();
    if !trimmed.starts_with("G1 ") {
        return None;
    }
    let cmd_part = match trimmed.find(';') {
        Some(idx) => &trimmed[..idx],
        None => trimmed,
    };
    if !cmd_part.contains('X') || !cmd_part.contains('Y') {
        return None;
    }
    let x_str = extract_param(cmd_part, "X")?;
    let y_str = extract_param(cmd_part, "Y")?;
    Some((
        crate::gcode::parse_float_go(&x_str)?,
        crate::gcode::parse_float_go(&y_str)?,
    ))
}

/// `extractParam`：严格匹配——参数字母前必须是空格/制表符/行首（防 `EX`/`FY`
/// 误命中），数值字符为 `[.\-0-9]+`。
fn extract_param(line: &str, param: &str) -> Option<String> {
    let target = param.as_bytes()[0];
    let bytes = line.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] != target {
            continue;
        }
        if i > 0 && bytes[i - 1] != b' ' && bytes[i - 1] != b'\t' {
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < bytes.len()
            && (bytes[end] == b'.' || bytes[end] == b'-' || bytes[end].is_ascii_digit())
        {
            end += 1;
        }
        if start < end {
            return Some(line[start..end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn zero_range_returns_empty() {
        let ls = lines(&["G1 X999 Y999"]);
        assert!(
            check_xy_range(
                &ls,
                MachineMovementRange::default(),
                "s",
                0.0,
                0.0,
                0.0,
                0.0
            )
            .is_empty()
        );
    }

    #[test]
    fn violations_in_fixed_order_with_maxima() {
        let ls = lines(&[
            "G1 X10 Y10",
            "G1 X200 Y-5", // X+ (200>180) 与 Y- (-5<0)
            "G1 X300 Y-1", // X+ 更大 (300)；Y- -1 不比 -5 更小
            "G1 X-20 Y50", // X- (-20<0)
            "G1 X1 Y1 ; comment X999",
            "G2 X999 Y999", // 非 "G1 " 开头，忽略
            "G1 X150",      // 无 Y，忽略
        ]);
        let range = MachineMovementRange {
            min_x: 0.0,
            max_x: 180.0,
            min_y: 0.0,
            max_y: 180.0,
        };
        let v = check_xy_range(
            &ls,
            range,
            "first_pen_revitalization",
            20.0,
            20.0,
            -1.0,
            18.6,
        );
        let dirs: Vec<&str> = v.iter().map(|x| x.direction).collect();
        assert_eq!(dirs, vec!["X+", "X-", "Y-"], "固定顺序且每方向一条");
        assert_eq!(v[0].actual_value, 300.0, "X+ 取最大超出量");
        assert_eq!(v[1].actual_value, -20.0);
        assert_eq!(v[2].actual_value, -5.0);
        assert_eq!(v[0].machine_limit, 180.0);
        assert_eq!(v[0].toolhead_x_off, -1.0);
        assert_eq!(v[0].wiper_y, 20.0);
    }

    #[test]
    fn extract_param_is_strict_about_preceding_char() {
        assert_eq!(extract_param("G1 X10.5 Y2", "X").as_deref(), Some("10.5"));
        assert_eq!(
            extract_param("G1 X10.5 FY2", "Y"),
            None,
            "FY 的 Y 前面不是空格"
        );
        assert_eq!(extract_param("G1 E-1 X2", "X").as_deref(), Some("2"));
    }
}
