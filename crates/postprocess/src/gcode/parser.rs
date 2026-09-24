//! 行级识别工具（对照 parser.go）。

/// `TrimLine`：去掉行尾 `\r` / `\n`。
pub fn trim_line(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

/// `IsFeatureLine`：`; FEATURE: xxx` / `;FEATURE: xxx`，返回 feature 名。
pub fn is_feature_line(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    if !crate::gcode::markers::has_slicer_comment_prefix(trimmed, "FEATURE: ") {
        return None;
    }
    Some(
        trimmed
            .trim_start_matches("; FEATURE: ")
            .trim_start_matches(";FEATURE: "),
    )
}

/// `IsLayerNumLine`：含 `; layer num/total_layer_count`。
pub fn is_layer_num_line(line: &str) -> bool {
    line.trim().contains("; layer num/total_layer_count")
}

/// `IsZHeightLine`：`; Z_HEIGHT: <v>`，返回首个浮点。
pub fn is_z_height_line(line: &str) -> Option<f64> {
    let trimmed = line.trim();
    if !crate::gcode::markers::has_slicer_comment_prefix(trimmed, "Z_HEIGHT: ") {
        return None;
    }
    let rest = trimmed
        .trim_start_matches("; Z_HEIGHT: ")
        .trim()
        .trim_start_matches(";Z_HEIGHT: ")
        .trim();
    first_float(rest)
}

/// `firstFloat`：提取字符串中首个浮点（如 "0.2 mm" → 0.2）。
fn first_float(s: &str) -> Option<f64> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() && !is_num_byte(bytes[i]) {
        i += 1;
    }
    if i >= bytes.len() {
        return None;
    }
    let mut j = i;
    while j < bytes.len() && is_num_byte(bytes[j]) {
        j += 1;
    }
    crate::gcode::parse_float_go(&s[i..j])
}

fn is_num_byte(b: u8) -> bool {
    b == b'+' || b == b'-' || b == b'.' || b.is_ascii_digit()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feature_line_variants() {
        assert_eq!(is_feature_line("; FEATURE: Inner wall"), Some("Inner wall"));
        assert_eq!(is_feature_line(";FEATURE: Outer wall"), Some("Outer wall"));
        assert_eq!(is_feature_line("  ; FEATURE: Brim  "), Some("Brim"));
        assert_eq!(is_feature_line("; TOTALTIMES: 1234"), None);
        assert_eq!(is_feature_line("G1 X1"), None);
        // 非 FEATURE 前缀的切片注释不算
        assert_eq!(is_feature_line("; FEATURE_X: no"), None);
    }

    #[test]
    fn layer_num_line_variants() {
        assert!(is_layer_num_line("; layer num/total_layer_count: 5/37"));
        assert!(is_layer_num_line("  ; layer num/total_layer_count  "));
        assert!(!is_layer_num_line("; layer 5"));
    }

    #[test]
    fn z_height_line_variants() {
        assert_eq!(is_z_height_line("; Z_HEIGHT: 0.2"), Some(0.2));
        // 标记含尾随空格：冒号后无空格的 ";Z_HEIGHT:0.25mm" **不命中**（Go 同）
        assert_eq!(is_z_height_line(";Z_HEIGHT:0.25mm"), None);
        assert_eq!(is_z_height_line("; Z_HEIGHT: 0.25 mm"), Some(0.25));
        assert_eq!(is_z_height_line("  ; Z_HEIGHT: 1.2 mm "), Some(1.2));
        assert_eq!(is_z_height_line("; Z_HEIGHT: abc"), None);
        assert_eq!(is_z_height_line("; FEATURE: Ironing"), None);
        assert_eq!(is_z_height_line("G1 Z0.2"), None);
    }

    #[test]
    fn trim_line_strips_crlf_only() {
        assert_eq!(trim_line("G1 X1\r\n"), "G1 X1");
        assert_eq!(trim_line("G1 X1\n"), "G1 X1");
        assert_eq!(trim_line("G1 X1"), "G1 X1");
        assert_eq!(trim_line(" ; c \r\n"), " ; c ");
    }
}
