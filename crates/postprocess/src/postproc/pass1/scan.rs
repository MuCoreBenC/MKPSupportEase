//! pass1 的行级扫描 helpers（对照 processor/pass1_scan.go / pass1_emit.go 尾部）。

/// `stripEFromG1`：剥掉 G1 行里 " E<数值>" 段（pass1_scan.go:3）。
pub(crate) fn strip_e_from_g1(line: &str) -> String {
    let Some(e_idx) = line.find(" E") else {
        return line.to_string();
    };
    let after_e = &line[e_idx + 2..];
    let mut end_of_e = 0;
    for (ci, ch) in after_e.bytes().enumerate() {
        if ch == b' ' || ch == b';' {
            end_of_e = ci;
            break;
        }
    }
    if end_of_e > 0 {
        format!("{}{}", &line[..e_idx], &after_e[end_of_e..])
    } else {
        line[..e_idx].to_string()
    }
}

/// `parseXYFromG1`：从 "G1 X.. Y.." 行提取 X/Y（pass1_scan.go:41）。
/// 只有一轴时另一轴返回 0 且 ok=true —— 调用方必须自持 prevX/prevY 状态。
pub(crate) fn parse_xy_from_g1(line: &str) -> (f64, f64, bool) {
    let mut x = 0.0;
    let mut y = 0.0;
    let mut has_x = false;
    let mut has_y = false;
    for part in line.split_whitespace() {
        let b = part.as_bytes();
        if b.len() > 1 && b[0] == b'X' {
            if let Ok(v) = part[1..].parse::<f64>() {
                x = v;
                has_x = true;
            }
        } else if b.len() > 1 && b[0] == b'Y' {
            if let Ok(v) = part[1..].parse::<f64>() {
                y = v;
                has_y = true;
            }
        }
    }
    (x, y, has_x || has_y)
}

/// `detectVariableLayerHeight`（pass1_scan.go:74）：层高序列是否可变。
pub(crate) fn detect_variable_layer_height(z_height_values: &[f64]) -> bool {
    if z_height_values.len() < 4 {
        return false;
    }
    let reference_increment = z_height_values[2] - z_height_values[1];
    if reference_increment <= 0.0 {
        return false;
    }
    for i in 3..z_height_values.len() {
        let increment = z_height_values[i] - z_height_values[i - 1];
        if increment <= 0.0 {
            continue;
        }
        let diff = increment - reference_increment;
        if !(-0.01..=0.01).contains(&diff) {
            return true;
        }
    }
    false
}

/// 层厚的**范围**（最小 / 最大）—— 界面「层高」那一格在可变层高时摆 `min – max`。
///
/// 取相邻两条 `; Z_HEIGHT:` 的差（与 pass1 里 `layer_thickness` 同一个算法），**从第 3 条差
/// 起算**：与 [`detect_variable_layer_height`] 同一个窗口。前两条刻意不算 —— 切片器的
/// 头两层厚度常与后面不同（首层 + 过渡层），算进来会让一盘 0.2mm 的件显示成
/// `0.15 – 0.20 mm`（`tests/golden/42274.2.gcode` 实测就是 0.2 / 0.35 / 0.5 …）。
///
/// 层数太少（凑不够 3 条差）⇒ `None`：界面退回切片器标称那个数，不瞎猜范围。
pub(crate) fn layer_height_range(z_height_values: &[f64]) -> Option<(f64, f64)> {
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    let mut diffs = 0usize;
    for i in 1..z_height_values.len() {
        let thickness = z_height_values[i] - z_height_values[i - 1];
        if thickness <= 0.0 {
            continue;
        }
        diffs += 1;
        if diffs < 3 {
            continue;
        }
        min = min.min(thickness);
        max = max.max(thickness);
    }
    (max > 0.0).then_some((min, max))
}

/// `findPrevPositionCmd`（pass1_emit.go:201）：向前找最近的 "G1 X"（不带 E）定位行。
/// 注意 Go 循环条件是 `j > 0`（非 `j >= 0`）——索引 0 刻意不被检查，逐字保留。
pub(crate) fn find_prev_position_cmd(content: &[String], feature_idx: usize) -> String {
    let mut j = feature_idx as isize - 1;
    while j > 0 {
        let trimmed = content[j as usize].trim();
        if trimmed.contains("G1 X") && !trimmed.contains(" E") {
            if let Some(z_idx) = trimmed.find('Z') {
                return trimmed[..z_idx].trim().to_string();
            }
            return trimmed.to_string();
        }
        j -= 1;
    }
    String::new()
}

/// `findSegStartCmd`（pass1_emit.go:214）：段内第一个 G1 X/Y 行。
pub(crate) fn find_seg_start_cmd(seg: &[String]) -> String {
    for line in seg {
        let trimmed = line.trim();
        if trimmed.contains("G1 X") || trimmed.contains("G1 Y") {
            return trimmed.to_string();
        }
    }
    String::new()
}

/// `stripZAndE`（pass1_emit.go:224）：从命令剥掉 Z 段与 " E" 段。
pub(crate) fn strip_z_and_e(cmd: &str) -> String {
    let mut cmd = cmd.to_string();
    if let Some(z_idx) = cmd.find('Z') {
        cmd.truncate(z_idx);
    }
    if let Some(e_idx) = cmd.find(" E") {
        cmd.truncate(e_idx);
    }
    cmd
}

/// `trimEndIdxBeforeWipe`（pass1_emit.go:233）：段末若有 WIPE_START 则截到它，
/// 否则截到最后一条挤出移动之后；都不命中则原样返回 end_idx。
pub(crate) fn trim_end_idx_before_wipe(
    content: &[String],
    start_idx: usize,
    end_idx: usize,
) -> usize {
    let mut wipe_idx: Option<usize> = None;
    for j in (start_idx..end_idx).rev() {
        if content[j].contains("WIPE_START") {
            wipe_idx = Some(j);
            break;
        }
    }
    if let Some(w) = wipe_idx {
        return w;
    }
    let mut last_print_idx: Option<usize> = None;
    for j in (start_idx..end_idx).rev() {
        let trimmed = content[j].trim();
        if trimmed.starts_with("G1 ")
            && trimmed.contains(" E")
            && (trimmed.contains('X') || trimmed.contains('Y'))
        {
            last_print_idx = Some(j);
            break;
        }
    }
    if let Some(p) = last_print_idx {
        return p + 1;
    }
    end_idx
}

/// `isSupportRelatedFeature` 的直通（pass1_emit.go 同名私有包装）。
pub(crate) fn is_support_related_feature(trimmed: &str) -> bool {
    crate::gcode::is_support_related_feature(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_e_variants() {
        assert_eq!(strip_e_from_g1("G1 X1 Y2 E0.5"), "G1 X1 Y2");
        assert_eq!(strip_e_from_g1("G1 X1 Y2 E0.5 F1800"), "G1 X1 Y2 F1800");
        assert_eq!(strip_e_from_g1("G1 X1 Y2 E.5;comment"), "G1 X1 Y2;comment");
        assert_eq!(strip_e_from_g1("G1 X1 Y2"), "G1 X1 Y2");
        // Go：endOfE==0（token 跑到行尾）时截到 " E" 之前，保留尾随空格语义
        assert_eq!(strip_e_from_g1("G1 X1 E5"), "G1 X1");
    }

    #[test]
    fn strip_z_and_e_combo() {
        // Go 语义：截到 Z 前的尾随空格保留（后续 ProcessGCodeOffset 自行 trim）
        assert_eq!(strip_z_and_e("G1 X10 Y20 Z0.5 E1.2"), "G1 X10 Y20 ");
        assert_eq!(strip_z_and_e("G1 X10 E1.2"), "G1 X10");
        assert_eq!(strip_z_and_e("G1 X10"), "G1 X10");
    }

    #[test]
    fn find_prev_position_cmd_skips_index_zero() {
        let content: Vec<String> = vec![
            "G1 X0 Y0".into(), // 索引 0：Go 的 j>0 循环永不检查它
            "G1 X5 Y5".into(),
            "; FEATURE: Support interface".into(),
        ];
        assert_eq!(find_prev_position_cmd(&content, 2), "G1 X5 Y5");
        // 唯一候选在索引 0 时返回空（Go 行为，有状态测试钉住）
        let content2: Vec<String> = vec!["G1 X0 Y0".into(), "; FEATURE: X".into()];
        assert_eq!(find_prev_position_cmd(&content2, 1), "");
    }

    #[test]
    fn variable_layer_height() {
        assert!(!detect_variable_layer_height(&[0.2, 0.4, 0.6, 0.8]));
        assert!(detect_variable_layer_height(&[0.2, 0.4, 0.6, 0.9]));
        assert!(!detect_variable_layer_height(&[0.2, 0.4]));
    }

    /// 层高范围：**前两条差不参与**（真实件 `42274.2.gcode` 的头两层是 0.15，
    /// 算进来一盘 0.2mm 的件会显示成 `0.15 – 0.20 mm`）。
    #[test]
    fn layer_height_range_skips_the_first_two_layers() {
        /* 0.2 / 0.35 / 0.5 之后一路 0.2：真实那份的样子 —— 范围应当只有一个 0.2
        （浮点减法带 1e-16 的噪声，比容差，不比字面量） */
        let (min, max) = layer_height_range(&[0.2, 0.35, 0.5, 0.7, 0.9, 1.1]).unwrap();
        assert!((min - 0.2).abs() < 1e-9, "min={min}");
        assert!((max - 0.2).abs() < 1e-9, "max={max}");
        /* 自适应层高：后面确实在变，范围要抓得到 */
        let (min, max) = layer_height_range(&[0.2, 0.28, 0.48, 0.56, 0.70, 0.78]).unwrap();
        assert!((min - 0.08).abs() < 1e-9, "min={min}");
        assert!((max - 0.14).abs() < 1e-9, "max={max}");
        /* 层数太少 ⇒ 没有范围（界面退回标称值） */
        assert_eq!(layer_height_range(&[0.2, 0.4]), None);
        assert_eq!(layer_height_range(&[]), None);
    }
}
