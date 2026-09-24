//! 支撑面接口有效性判定（对照 processor/validity.go）。

/// `CheckValidityInterfaceSet`：iface 里存在正挤出量（E > 0.001）的 G1 行即有效。
/// 负 E（回抽）跳过；E 后紧跟 '-' 时直接 continue（Go 原样，含 '-' 分支不置位）。
pub(crate) fn check_validity_interface_set(iface: &[String]) -> bool {
    for line in iface {
        let trimmed = line.trim();
        if trimmed.starts_with("G1 ") && trimmed.contains('E') {
            let Some(e_idx) = trimmed.find('E') else {
                continue;
            };
            if e_idx + 1 >= trimmed.len() {
                continue;
            }
            let rest = &trimmed[e_idx + 1..];
            let first = rest.as_bytes()[0];
            let e_num_str: String = if first.is_ascii_digit() {
                rest.to_string()
            } else if first == b'.' {
                format!("0{rest}")
            } else if first == b'-' {
                continue;
            } else {
                String::new()
            };
            if !e_num_str.is_empty() {
                let nums = crate::ir::num_strip(&e_num_str);
                if let Some(&n) = nums.first() {
                    if n > 0.001 {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iface(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn validity_both_sides() {
        assert!(check_validity_interface_set(&iface(&[
            "; FEATURE: Support interface",
            "G1 X1 Y2 E0.5",
        ])));
        // 只有回抽 → 无效
        assert!(!check_validity_interface_set(&iface(&["G1 X1 Y2 E-0.4"])));
        // E 太小 → 无效（阈值 0.001）
        assert!(!check_validity_interface_set(&iface(&["G1 X1 Y2 E0.0005"])));
        // ".5" 形态补 0 后有效
        assert!(check_validity_interface_set(&iface(&["G1 X1 Y2 E.5"])));
        // 空 / 纯注释 → 无效
        assert!(!check_validity_interface_set(&iface(&["; comment"])));
        assert!(!check_validity_interface_set(&[]));
    }
}
