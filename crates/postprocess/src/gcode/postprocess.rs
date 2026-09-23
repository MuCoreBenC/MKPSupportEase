//! E 值读写与行级后处理原语（对照 postprocess.go）。
//!
//! 这些函数只按「运动指令 + 参数字段」的字节语义工作，不理解 MKP 业务。

/// 运动指令集合（G0/G1/G2/G3 的两种写法）。
pub(crate) fn is_motion_command(code: &str) -> bool {
    matches!(
        code,
        "G0" | "G1" | "G00" | "G01" | "G2" | "G3" | "G02" | "G03"
    )
}

/// `parseGCode`：拆出指令码（大写）与参数表。私有 helper 的公开形态，
/// 供本模块与测试共用；参数表保留**最后出现者胜**的 map 语义。
pub(crate) fn parse_g_code(line: &str) -> (String, std::collections::HashMap<String, f64>) {
    let mut params = std::collections::HashMap::new();
    let code_part = extract_code_part(line);
    if code_part.is_empty() {
        return (String::new(), params);
    }
    let mut fields = code_part.split_whitespace();
    let Some(first) = fields.next() else {
        return (String::new(), params);
    };
    let code = first.to_uppercase();
    for f in fields {
        if f.len() < 2 {
            continue;
        }
        let param = f[..1].to_uppercase();
        let Some(val) = crate::gcode::parse_float_go(&f[1..]) else {
            continue;
        };
        params.insert(param, val);
    }
    (code, params)
}

/// 注释剥离后的指令部分（Go `extractCodePart`）。
pub(crate) fn extract_code_part(line: &str) -> &str {
    let trimmed = line.trim();
    match trimmed.find(';') {
        Some(idx) => trimmed[..idx].trim(),
        None => trimmed.trim(),
    }
}

/// `HasEParam`：运动指令且参数字段以 E<数字|-> 形式存在。
/// 注意它扫描的是**原始 codePart 字节**（含前导空格判断），不是拆分后的字段。
pub fn has_e_param(line: &str) -> bool {
    let trimmed = line.trim();
    let code_part = match trimmed.find(';') {
        Some(idx) => &trimmed[..idx],
        None => trimmed,
    };
    let code_part = code_part.trim();
    if code_part.is_empty() {
        return false;
    }
    let Some(first) = code_part.split_whitespace().next() else {
        return false;
    };
    let code = first.to_uppercase();
    if !is_motion_command(&code) {
        return false;
    }
    let bytes = code_part.as_bytes();
    for i in 0..bytes.len().saturating_sub(1) {
        if bytes[i] == b'E'
            && (bytes[i + 1] == b'-' || bytes[i + 1].is_ascii_digit())
            && (i == 0 || bytes[i - 1] == b' ')
        {
            return true;
        }
    }
    false
}

/// `GetEValue`：运动指令的 E 参数值（区分回抽负值与挤出正值）。
pub fn get_e_value(line: &str) -> Option<f64> {
    let code_part = extract_code_part(line);
    if code_part.is_empty() {
        return None;
    }
    let mut fields = code_part.split_whitespace();
    let first = fields.next()?;
    let code = first.to_uppercase();
    if !is_motion_command(&code) {
        return None;
    }
    for f in fields {
        if f.len() < 2 {
            continue;
        }
        if !f[..1].eq_ignore_ascii_case("E") {
            continue;
        }
        if let Some(v) = crate::gcode::parse_float_go(&f[1..]) {
            return Some(v);
        }
    }
    None
}

/// `ReplaceEValue`：替换 E 参数值（`E` + `FormatEValue`），保留其他参数与注释；
/// 非运动指令或无 E 时原样返回。
pub fn replace_e_value(line: &str, new_e: f64) -> String {
    let trimmed = line.trim();
    let (code_part, comment_part) = match trimmed.find(';') {
        Some(idx) => (&trimmed[..idx], Some(&trimmed[idx..])),
        None => (trimmed, None),
    };
    let code_part = code_part.trim();
    if code_part.is_empty() {
        return line.to_string();
    }
    let fields: Vec<&str> = code_part.split_whitespace().collect();
    let Some(first) = fields.first() else {
        return line.to_string();
    };
    let code = first.to_uppercase();
    if !is_motion_command(&code) {
        return line.to_string();
    }
    let mut new_fields: Vec<String> = Vec::with_capacity(fields.len());
    new_fields.push(fields[0].to_string());
    let mut e_replaced = false;
    for f in &fields[1..] {
        if f.len() < 2 {
            new_fields.push((*f).to_string());
            continue;
        }
        if f[..1].eq_ignore_ascii_case("E") {
            new_fields.push(format!("E{}", crate::gcode::format::format_e_value(new_e)));
            e_replaced = true;
        } else {
            new_fields.push((*f).to_string());
        }
    }
    if !e_replaced {
        return line.to_string();
    }
    let mut result = new_fields.join(" ");
    if let Some(comment) = comment_part {
        result.push(' ');
        result.push_str(comment);
    }
    result
}

/// `FilterEMoves`：剔除真实挤出移动行（`HasEParam` 且非 virtual extrusion）。
pub fn filter_e_moves(lines: &[String]) -> Vec<String> {
    lines
        .iter()
        .filter(|l| !(has_e_param(l) && !is_virtual_extrusion_line(l)))
        .cloned()
        .collect()
}

fn is_virtual_extrusion_line(line: &str) -> bool {
    line.contains("virtual extrusion")
}

fn is_calibration_home_line(line: &str) -> Option<f64> {
    let (code, params) = parse_g_code(line);
    if !is_motion_command(&code) {
        return None;
    }
    let &x = params.get("X")?;
    let &y = params.get("Y")?;
    let &z = params.get("Z")?;
    const HOME_XY: f64 = 100.0;
    const TOLERANCE: f64 = 1.0;
    if !(HOME_XY - TOLERANCE..=HOME_XY + TOLERANCE).contains(&x) {
        return None;
    }
    if !(HOME_XY - TOLERANCE..=HOME_XY + TOLERANCE).contains(&y) {
        return None;
    }
    Some(z)
}

fn is_safety_raise_line(line: &str, home_z: f64) -> Option<f64> {
    let (code, params) = parse_g_code(line);
    if !is_motion_command(&code) {
        return None;
    }
    let &x = params.get("X")?;
    let &y = params.get("Y")?;
    let &z = params.get("Z")?;
    const HOME_XY: f64 = 100.0;
    const TOLERANCE: f64 = 2.0;
    if !(HOME_XY - TOLERANCE..=HOME_XY + TOLERANCE).contains(&x) {
        return None;
    }
    if !(HOME_XY - TOLERANCE..=HOME_XY + TOLERANCE).contains(&y) {
        return None;
    }
    if z < home_z + 20.0 {
        return None;
    }
    Some(z)
}

/// `RemoveFinalLowerZ`：校准流程尾部「降 Z」行的清除（对照 postprocess.go:236）。
/// 两个分支：有/无 safety-raise 锚点行，阈值分别为 safetyRaiseZ / homeZ；
/// 危险区（"; lower z aittle" 注释后）内带 XY 的降 Z 行只剥 Z 参数。
pub fn remove_final_lower_z(lines: &[String]) -> Vec<String> {
    let Some((home_idx, home_z)) = lines
        .iter()
        .enumerate()
        .find_map(|(i, l)| is_calibration_home_line(l).map(|z| (i, z)))
    else {
        return lines.to_vec();
    };

    let safety = lines[home_idx + 1..]
        .iter()
        .enumerate()
        .find_map(|(offset, l)| {
            is_safety_raise_line(l, home_z).map(|z| (home_idx + 1 + offset, z))
        });

    let mut result: Vec<String> = Vec::with_capacity(lines.len());
    result.extend_from_slice(&lines[..=home_idx]);

    match safety {
        None => {
            let threshold_z = home_z;
            filter_lower_z_tail(lines, home_idx + 1, threshold_z, &mut result);
        }
        Some((raise_idx, raise_z)) => {
            result.extend_from_slice(&lines[home_idx + 1..=raise_idx]);
            filter_lower_z_tail(lines, raise_idx + 1, raise_z, &mut result);
        }
    }
    result
}

/// postprocess.go 两段共享的尾部过滤循环（原文是两份近乎复制的循环，此处合并；
/// 行为逐字对照：空行/注释行原样、"; lower z a little" 进危险区、非运动行原样、
/// 降 Z 行按 hasXY 剥 Z 或整行丢弃）。
fn filter_lower_z_tail(lines: &[String], from: usize, threshold_z: f64, out: &mut Vec<String>) {
    let mut in_danger_zone = false;
    for line in &lines[from..] {
        let mut line = line.clone();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            out.push(line);
            continue;
        }
        if trimmed.starts_with(';') {
            if trimmed.contains("; lower z a little") {
                in_danger_zone = true;
            }
            out.push(line);
            continue;
        }
        let (code, params) = parse_g_code(&line);
        if !is_motion_command(&code) {
            out.push(line);
            continue;
        }
        let z_val = params.get("Z").copied();
        let has_z = z_val.is_some();
        let has_xy = params.contains_key("X")
            || params.contains_key("Y")
            || params.contains_key("I")
            || params.contains_key("J");
        let should_filter = in_danger_zone || (has_z && z_val.is_some_and(|z| z < threshold_z));
        if should_filter && has_z {
            let z = z_val.unwrap();
            if z >= threshold_z {
                out.push(line);
                continue;
            }
            if has_xy {
                line = remove_param_from_line(&line, "Z");
                if extract_code_part(&line).trim().is_empty() {
                    continue;
                }
            } else {
                continue;
            }
        }
        out.push(line);
    }
}

/// `removeParamFromLine`：剥掉指定轴参数字段，注释保留。
pub fn remove_param_from_line(line: &str, param: &str) -> String {
    let trimmed = line.trim();
    let (code_part, comment_part) = match trimmed.find(';') {
        Some(idx) => (&trimmed[..idx], Some(&trimmed[idx..])),
        None => (trimmed, None),
    };
    let code_part = code_part.trim();
    let upper_param = param.to_uppercase();
    let mut new_fields = Vec::new();
    for f in code_part.split_whitespace() {
        if f.is_empty() {
            continue;
        }
        let first = f[..1].to_uppercase();
        if first == upper_param {
            continue;
        }
        new_fields.push(f);
    }
    let mut result = new_fields.join(" ");
    if let Some(comment) = comment_part {
        result.push(' ');
        result.push_str(comment);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 旧侧 get_evalue_test.go 全表移植。
    #[test]
    fn get_e_value_table() {
        let cases: &[(&str, Option<f64>)] = &[
            ("G1 E-5 F1800", Some(-5.0)),
            ("G1 E5.5 F1500", Some(5.5)),
            ("G1 E0 F1800", Some(0.0)),
            ("G1 X100 Y50 E3.2 F600", Some(3.2)),
            ("G1 X100 Y50 E-3.2 F600", Some(-3.2)),
            ("G1 X100 Y50 F600", None),
            ("G1 F30000", None),
            ("G1 Z10.7", None),
            ("G92 E0", None),
            ("G92 E5", None),
            ("M106 S[AUTO]", None),
            ("M204 S10000", None),
            ("", None),
            ("; this is a comment", None),
            ("G0 X10 E-2 F1000", Some(-2.0)),
            ("G00 X10 E2 F1000", Some(2.0)),
            ("G1 E-5 F1800 ; retract", Some(-5.0)),
            ("  G1 E-5 F1800", Some(-5.0)),
        ];
        for (line, want) in cases {
            assert_eq!(&get_e_value(line), want, "get_e_value({line:?})");
        }
    }

    /// 旧侧 replace_e_value_test.go 全表移植。
    #[test]
    fn replace_e_value_table() {
        let cases: &[(&str, f64, &str)] = &[
            ("G1 E-5 F1800", -1.0, "G1 E-1 F1800"),
            ("G1 E5.5 F1500", 1.5, "G1 E1.5 F1500"),
            ("G1 E0 F1800", -1.0, "G1 E-1 F1800"),
            ("G1 X100 Y50 E-5 F1800", -1.0, "G1 X100 Y50 E-1 F1800"),
            ("G1 Z10.7 E-5 F1800", -1.0, "G1 Z10.7 E-1 F1800"),
            ("G1 E-5 F1800 ; retract", -1.0, "G1 E-1 F1800 ; retract"),
            ("  G1 E-5 F1800", -1.0, "G1 E-1 F1800"),
            ("G92 E0", -1.0, "G92 E0"),
            ("M106 S255", -1.0, "M106 S255"),
            ("G1 X100 Y50 F1800", -1.0, "G1 X100 Y50 F1800"),
            ("", -1.0, ""),
            ("; just comment", -1.0, "; just comment"),
            ("G0 X10 E-2 F1000", -1.0, "G0 X10 E-1 F1000"),
        ];
        for (line, new_e, want) in cases {
            assert_eq!(
                &replace_e_value(line, *new_e),
                want,
                "replace_e_value({line:?})"
            );
        }
    }

    #[test]
    fn has_e_param_variants() {
        assert!(has_e_param("G1 E-5 F1800"));
        assert!(has_e_param("G1 X1 E0.4"));
        assert!(has_e_param("G01 E1"));
        // E 前必须是空格或行首（EX 不算）；且必须在注释剥离后的指令部分
        assert!(!has_e_param("G1 X10 ; E5"));
        assert!(!has_e_param("M106 E1")); // 非运动指令
        assert!(!has_e_param("G1 X1 Y2"));
    }

    #[test]
    fn filter_e_moves_drops_real_extrusion() {
        let lines: Vec<String> = [
            "G1 X1 Y1",
            "G1 X2 Y2 E0.5",
            "G1 X3 Y3 E0.5 ; virtual extrusion",
            "; comment",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let got = filter_e_moves(&lines);
        assert_eq!(
            got,
            vec!["G1 X1 Y1", "G1 X3 Y3 E0.5 ; virtual extrusion", "; comment"]
        );
    }

    /// 旧侧 postprocess.go RemoveFinalLowerZ 的关键分支（home 行 100,100 容差 ±1）。
    #[test]
    fn remove_final_lower_z_drops_lower_moves_after_home() {
        let lines: Vec<String> = [
            "G1 X100 Y100 Z10 F600", // home（校准回原点形态）
            "; lower z a little",
            "G1 X100 Y100 Z9.5", // 危险区内降 Z 且带 XY → 剥 Z
            "G1 Z9.0",           // 危险区内纯 Z 降 → 整行丢弃
            "G1 Z10.5",          // 高于阈值 → 保留
            "M104 S0",           // 非运动行 → 保留
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let got = remove_final_lower_z(&lines);
        assert_eq!(
            got,
            vec![
                "G1 X100 Y100 Z10 F600",
                "; lower z a little",
                "G1 X100 Y100", // Z 被剥掉
                "G1 Z10.5",
                "M104 S0",
            ]
        );
    }

    #[test]
    fn remove_final_lower_z_without_home_is_identity() {
        let lines: Vec<String> = ["G1 Z1", "G1 X5 Y5 Z0.2"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert_eq!(remove_final_lower_z(&lines), lines);
    }

    #[test]
    fn remove_param_from_line_strips_axis_keeps_comment() {
        assert_eq!(
            remove_param_from_line("G1 X1 Y2 Z0.2 ; c", "Z"),
            "G1 X1 Y2 ; c"
        );
        // Go 语义：字段级剔除，指令码 "G1" 保留
        assert_eq!(remove_param_from_line("G1 Z0.2", "Z"), "G1");
    }
}
