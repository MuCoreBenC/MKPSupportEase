//! G-code 元数据单行提取（对照 gcodebiz/metadata.go 的 FilamentTypeFromLine）。

/// `FilamentTypeFromLine`：从 "; filament_type = PLA;PETG" 类注释提取第一个
/// 非空 token 并大写；兼容 "= PLA" / "=PLA" / "= \"PLA\"" 写法。不匹配返回 ""。
pub(crate) fn filament_type_from_line(line: &str) -> String {
    let Some(idx) = line.find("; filament_type") else {
        return String::new();
    };
    let mut rest = line[idx + "; filament_type".len()..].trim();
    if !rest.starts_with('=') {
        return String::new();
    }
    rest = rest[1..].trim();
    let rest = rest.trim_matches('"');
    // Bambu / Orca 多挤出机位：取 ';' 或 ',' 前的第一个 token
    let rest = match rest.find([';', ',']) {
        Some(sep) => &rest[..sep],
        None => rest,
    };
    let rest = rest.trim();
    if rest.is_empty() {
        return String::new();
    }
    rest.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filament_type_formats() {
        assert_eq!(filament_type_from_line("; filament_type = PLA"), "PLA");
        assert_eq!(filament_type_from_line("; filament_type=PLA"), "PLA");
        assert_eq!(filament_type_from_line("; filament_type = \"PLA\""), "PLA");
        assert_eq!(
            filament_type_from_line("; filament_type = PLA;PETG;ABS"),
            "PLA"
        );
        assert_eq!(filament_type_from_line("; filament_type = pla"), "PLA");
        assert_eq!(filament_type_from_line("; other = PLA"), "");
        assert_eq!(filament_type_from_line("; filament_type ="), "");
    }
}
