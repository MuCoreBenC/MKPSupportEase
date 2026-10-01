//! 坐标提取 / 解析 / 替换（对照 coord_extract.go / offset_byte.go / offset_replace.go）。
//!
//! 三个函数分别复刻三个**正则语义**（旧侧已把它们改写成字节扫描以省分配，
//! Rust 侧直接按字节扫描移植——语义描述全部保留在各自文档里）：
//! - `extract_coord`       ≡ `regexp "X([\d\.-]+)"` 的首个匹配
//! - `parse_xyze`          ≡ `regexp "([XYZE])(\d+\.\d+)"` 的 FindAll（最后者胜）
//! - `replace_axis_value`  ≡ X/Y/Z: `axis-?\d+\.\d+`；E: `E-?\.?\d+\.?\d*`
//! - `format_xyze_string`  ≡ `([XYZE])([\d.]+)` → `axis + %.3f`

/// `ExtractCoord`：轴字母后最长的 `[\d\.-]+` 串，ParseFloat 失败即「未找到」。
///
/// 注意首个匹配若不可解析是**终止扫描**（返回 false），不是跳过找下一个——
/// 这是旧实现的真实行为（对照 coord_extract.go:47-50）。
pub fn extract_coord(line: &[u8], axis: u8) -> Option<f64> {
    let mut i = 0;
    while i < line.len() {
        if line[i] != axis {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut end = start;
        while end < line.len()
            && (line[end].is_ascii_digit() || line[end] == b'.' || line[end] == b'-')
        {
            end += 1;
        }
        if end == start {
            // 轴字母后没有数字字符——不是匹配，继续扫
            i += 1;
            continue;
        }
        let token = std::str::from_utf8(&line[start..end]).ok()?;
        return crate::gcode::parse_float_go(token);
    }
    None
}

/// XYZE 四轴提取结果（对照 `XYZEValues`；Has* 区分「没出现」与「值为 0」）。
#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct XyzeValues {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub e: f64,
    pub has_x: bool,
    pub has_y: bool,
    pub has_z: bool,
    pub has_e: bool,
}

/// `ParseXYZE`：≡ `([XYZE])(\d+\.\d+)` 的 FindAll + ParseFloat，最后者胜。
///
/// 语义要点（与正则一字不差）：轴字母后必须是 `\d+\.\d+`（点两侧都要有数字）；
/// **负数不匹配**（'-' 不在模式里）；"X10"（无小数点）不匹配。
pub fn parse_xyze(line: &[u8]) -> XyzeValues {
    let mut result = XyzeValues::default();
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        if !matches!(c, b'X' | b'Y' | b'Z' | b'E') {
            i += 1;
            continue;
        }
        let num_start = i + 1;
        let mut j = num_start;
        while j < line.len() && line[j].is_ascii_digit() {
            j += 1;
        }
        if j == num_start {
            i += 1;
            continue;
        }
        if j >= line.len() || line[j] != b'.' {
            i += 1;
            continue;
        }
        j += 1; // 消耗点
        let mut digits_after_dot = 0;
        while j < line.len() && line[j].is_ascii_digit() {
            j += 1;
            digits_after_dot += 1;
        }
        if digits_after_dot == 0 {
            i += 1;
            continue;
        }
        let token = std::str::from_utf8(&line[num_start..j]).ok();
        let value = token.and_then(crate::gcode::parse_float_go);
        let Some(value) = value else {
            i += 1;
            continue;
        };
        match c {
            b'X' => {
                result.x = value;
                result.has_x = true;
            }
            b'Y' => {
                result.y = value;
                result.has_y = true;
            }
            b'Z' => {
                result.z = value;
                result.has_z = true;
            }
            b'E' => {
                result.e = value;
                result.has_e = true;
            }
            _ => {}
        }
        // 整段跳过（非重叠，同 FindAllStringSubmatch）
        i = j;
    }
    result
}

/// `FormatXYZEString`：把文本中所有 `轴字母+[\d.]+` 归一为 `%.3f`（尾零保留）。
/// 负坐标不匹配（'-' 不在字符类）——与正则一致。无匹配时原样返回。
pub fn format_xyze_string(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut buf: Vec<u8> = Vec::new();
    let mut started = false;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if !matches!(c, b'X' | b'Y' | b'Z' | b'E') {
            if started {
                buf.push(c);
            }
            i += 1;
            continue;
        }
        let mut j = i + 1;
        while j < bytes.len() && (bytes[j].is_ascii_digit() || bytes[j] == b'.') {
            j += 1;
        }
        if j == i + 1 {
            if started {
                buf.push(c);
            }
            i += 1;
            continue;
        }
        let token = std::str::from_utf8(&bytes[i + 1..j]).ok();
        let value = token.and_then(crate::gcode::parse_float_go);
        match value {
            Some(v) => {
                if !started {
                    started = true;
                    buf.extend_from_slice(&bytes[..i]);
                }
                buf.push(c);
                buf.extend_from_slice(format!("{v:.3}").as_bytes());
            }
            None => {
                // ParseFloat 失败（如 "1.2.3"）——原样保留匹配段
                if started {
                    buf.extend_from_slice(&bytes[i..j]);
                }
            }
        }
        i = j;
    }
    if !started {
        return text.to_string();
    }
    String::from_utf8(buf).unwrap_or_else(|e| {
        // 输入是合法 UTF-8，替换只动 ASCII 段，这里实际不可达；保底原样返回
        let _ = e;
        text.to_string()
    })
}

/// `ReplaceAxisValue`：把给定轴的全部坐标替换为 replacement（"" 即剥离）。
///
/// 模式（与旧正则一字不差）：
/// - X/Y/Z：`axis-?\d+\.\d+`（点两侧必须有数字，可负）
/// - E：`E-?\.?\d+\.?\d*`（宽松：E5 / E.5 / E5. 都匹配）
pub fn replace_axis_value(s: &str, axis: u8, replacement: &str) -> String {
    let bytes = s.as_bytes();
    let mut buf: Vec<u8> = Vec::new();
    let mut started = false;
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != axis {
            if started {
                buf.push(bytes[i]);
            }
            i += 1;
            continue;
        }
        match match_axis_coord(bytes, i, axis) {
            Some(end) => {
                if !started {
                    started = true;
                    buf.extend_from_slice(&bytes[..i]);
                }
                buf.extend_from_slice(replacement.as_bytes());
                i = end;
            }
            None => {
                if started {
                    buf.push(bytes[i]);
                }
                i += 1;
            }
        }
    }
    if !started {
        return s.to_string();
    }
    String::from_utf8(buf).unwrap_or_else(|_| s.to_string())
}

/// `matchAxisCoord`：从 start 起尝试匹配该轴的坐标模式，返回结束位置。
fn match_axis_coord(s: &[u8], start: usize, axis: u8) -> Option<usize> {
    let mut i = start + 1;
    if i >= s.len() {
        return None;
    }
    if axis == b'E' {
        // E-?\.?\d+\.?\d*
        if s[i] == b'-' {
            i += 1;
            if i >= s.len() {
                return None;
            }
        }
        if s[i] == b'.' {
            i += 1;
            if i >= s.len() {
                return None;
            }
        }
        if !s[i].is_ascii_digit() {
            return None;
        }
        while i < s.len() && s[i].is_ascii_digit() {
            i += 1;
        }
        if i < s.len() && s[i] == b'.' {
            i += 1;
            while i < s.len() && s[i].is_ascii_digit() {
                i += 1;
            }
        }
        return Some(i);
    }
    // X/Y/Z: axis-?\d+\.\d+
    if s[i] == b'-' {
        i += 1;
        if i >= s.len() {
            return None;
        }
    }
    if !s[i].is_ascii_digit() {
        return None;
    }
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    if i >= s.len() || s[i] != b'.' {
        return None;
    }
    i += 1;
    if i >= s.len() || !s[i].is_ascii_digit() {
        return None;
    }
    while i < s.len() && s[i].is_ascii_digit() {
        i += 1;
    }
    Some(i)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧侧 coord_extract_test.go 的行集 × {X,Y,E} 轴（期望值按正则语义手推）。
    #[test]
    fn extract_coord_matches_regex_semantics() {
        let cases: &[(&str, u8, Option<f64>)] = &[
            ("G1 X10.5 Y20.2 E0.3 F1200", b'X', Some(10.5)),
            ("G1 X10.5 Y20.2 E0.3 F1200", b'Y', Some(20.2)),
            ("G1 X10.5 Y20.2 E0.3 F1200", b'E', Some(0.3)),
            ("G1 X-10.5 Y-20.2 E-0.3", b'X', Some(-10.5)),
            ("G1 X-10.5 Y-20.2 E-0.3", b'E', Some(-0.3)),
            ("G1 X100 Y200", b'X', Some(100.0)),
            ("G1 X0.0 Y0.0 E0.0", b'E', Some(0.0)),
            ("G1 X.5 Y.25 E.1", b'X', Some(0.5)), // ".5" 合法浮点
            ("G0 X10 Y20", b'X', Some(10.0)),
            ("M106 S100", b'X', None),
            ("G1 X10.5 Y20.2 E0.3 ; comment", b'E', Some(0.3)),
            ("G1 X123.456 Y789.012 E345.678", b'Y', Some(789.012)),
            ("G1 X-123.456 Y-789.012 E-345.678", b'Y', Some(-789.012)),
            ("", b'X', None),
            ("; comment only", b'X', None),
            ("G1 X1 Y2 E3", b'E', Some(3.0)),
            ("G1 X-1 Y-2 E-3", b'Y', Some(-2.0)),
        ];
        for (line, axis, want) in cases {
            assert_eq!(
                &extract_coord(line.as_bytes(), *axis),
                want,
                "line={line:?}"
            );
        }
    }

    /// 首个匹配不可解析即整体失败（旧实现行为，不是跳过继续）。
    #[test]
    fn extract_coord_first_bad_match_stops() {
        // "X1.2.3" 的贪婪捕获 "1.2.3" 不可解析 → found=false
        assert_eq!(extract_coord(b"G1 X1.2.3 Y4", b'X'), None);
    }

    #[test]
    fn parse_xyze_last_wins_and_requires_dot() {
        let v = parse_xyze(b"G1 X10.500 Y20.200 E0.300 X11.000");
        assert_eq!(v.x, 11.0, "重复轴最后者胜");
        assert_eq!(v.y, 20.2);
        assert_eq!(v.e, 0.3);
        assert!(v.has_x && v.has_y && v.has_e);
        assert!(!v.has_z);

        // 无小数点 / 负数不匹配（正则语义）
        let v = parse_xyze(b"G1 X10 Y-10.500 Z.5 E5");
        assert!(!v.has_x, "X10 无点不匹配");
        assert!(!v.has_y, "负数不匹配");
        assert!(!v.has_z, ".5 点前无数字不匹配");
        assert!(!v.has_e, "E5 无点不匹配");

        // 紧邻轴字母：XY10.500 → X 不匹配、Y 匹配
        let v = parse_xyze(b"G1 XY10.500");
        assert!(!v.has_x);
        assert!(v.has_y);
        assert_eq!(v.y, 10.5);
    }

    #[test]
    fn format_xyze_string_normalizes_to_3_decimals() {
        // 模式是 ([XYZE])([\d.]+)：Y20（无点）与 E.5（点前无数字）都**匹配**，
        // 与 ParseXYZE 的 \d+\.\d+ 不同；负数 Z-3 不匹配（'-' 不在字符类）。
        assert_eq!(
            format_xyze_string("G1 X10.5 Y20 Z-3 E.5"),
            "G1 X10.500 Y20.000 Z-3 E0.500"
        );
        assert_eq!(
            format_xyze_string("G1 X1.2.3 Y4.5"),
            "G1 X1.2.3 Y4.500",
            "不可解析的捕获段原样保留"
        );
        assert_eq!(format_xyze_string("M104 S0"), "M104 S0");
        assert_eq!(format_xyze_string("X100.0000"), "X100.000");
    }

    #[test]
    fn replace_axis_value_patterns() {
        // X/Y/Z：axis-?\d+\.\d+
        assert_eq!(
            replace_axis_value("G1 X10.500 Y2.0", b'X', "X12.345"),
            "G1 X12.345 Y2.0"
        );
        assert_eq!(replace_axis_value("G1 X-10.500", b'X', ""), "G1 ");
        assert_eq!(replace_axis_value("G1 X10 Y10.5", b'X', ""), "G1 X10 Y10.5");
        assert_eq!(replace_axis_value("G1 X.5 Y10.5", b'X', ""), "G1 X.5 Y10.5");
        // E：宽松模式 E-?\.?\d+\.?\d*
        assert_eq!(replace_axis_value("G1 X1 Y1 E5", b'E', ""), "G1 X1 Y1 ");
        assert_eq!(replace_axis_value("G1 X1 Y1 E.5", b'E', ""), "G1 X1 Y1 ");
        assert_eq!(replace_axis_value("G1 X1 Y1 E-0.25", b'E', ""), "G1 X1 Y1 ");
        assert_eq!(replace_axis_value("G1 X1 Y1 E5.", b'E', ""), "G1 X1 Y1 ");
        // 多处替换
        assert_eq!(replace_axis_value("X1.0 X2.0", b'X', "X9"), "X9 X9");
    }
}
