//! 擦拭段删除与特征跳转插入（对照 processor/delete_wipe.go / delete_wipe_continuous.go）。

// 结构逐字对照 delete_wipe.go：嵌套 if / 手工索引循环与 Go 同形，不改写。
#![allow(
    clippy::collapsible_if,
    clippy::needless_range_loop,
    clippy::int_plus_one
)]

use crate::gcode::{
    MARKER_MICRO_LIFT, MARKER_ZJUMP_INTERNAL, MARKER_ZJUMP_START, has_slicer_comment_prefix,
    is_glue_continuous_feature, match_slicer_comment,
};

/// `isTransferLine`（delete_wipe.go:8）。
fn is_transfer_line(trimmed: &str) -> bool {
    if trimmed.starts_with("G3 ") {
        return true;
    }
    if trimmed.starts_with("G1 Z") {
        return true;
    }
    if trimmed.starts_with("G1 E") && !trimmed.contains('X') && !trimmed.contains('Y') {
        return true;
    }
    if trimmed.starts_with("G17") {
        return true;
    }
    if trimmed.starts_with("M204") {
        return true;
    }
    false
}

/// `isTransferLineContinuous`（delete_wipe_continuous.go:8）：与 is_transfer_line
/// 相同但**不含 M204**（M204 在连续模式下保留）。
fn is_transfer_line_continuous(trimmed: &str) -> bool {
    if trimmed.starts_with("G3 ") {
        return true;
    }
    if trimmed.starts_with("G1 Z") {
        return true;
    }
    if trimmed.starts_with("G1 E") && !trimmed.contains('X') && !trimmed.contains('Y') {
        return true;
    }
    if trimmed.starts_with("G17") {
        return true;
    }
    false
}

/// `stripZFromLine`（delete_wipe.go:24）：剥掉 " Z<val>" 参数（仅当前导空格存在时）。
fn strip_z_from_line(line: &str) -> String {
    if let Some(z_idx) = line.find('Z') {
        if z_idx > 0 && line.as_bytes()[z_idx - 1] == b' ' {
            if z_idx + 1 < line.len() {
                let rest = &line[z_idx + 1..];
                if let Some(space_idx) = rest.find(' ') {
                    return format!("{}{}", &line[..z_idx], &rest[space_idx..])
                        .trim()
                        .to_string();
                }
            }
            return line[..z_idx].trim().to_string();
        }
    }
    line.to_string()
}

/// `stripSkippableBlocks`（pass1.go:1123）：移除 "; SKIPPABLE_START" 到
/// "; SKIPPABLE_END"（含）之间的行（延时摄影 / head_wrap_detect 等检测段）。
pub(crate) fn strip_skippable_blocks(lines: &[String]) -> Vec<String> {
    let mut result = Vec::new();
    let mut skip = false;
    for line in lines {
        let trimmed = line.trim();
        if trimmed.contains("; SKIPPABLE_START") {
            skip = true;
            continue;
        }
        if trimmed.contains("; SKIPPABLE_END") {
            skip = false;
            continue;
        }
        if !skip {
            result.push(line.clone());
        }
    }
    result
}

/// `DeleteWipe`（delete_wipe.go:38）。
pub(crate) fn delete_wipe(iface: &[String]) -> Vec<String> {
    let iface = strip_skippable_blocks(iface);

    let mut result: Vec<String> = Vec::new();
    let mut wipe_active = false;
    let mut after_wipe = false;
    let mut after_travel = false;

    let mut i = 0;
    while i < iface.len() {
        let trimmed = iface[i].trim().to_string();

        if match_slicer_comment(&trimmed, "WIPE_START") {
            wipe_active = true;
            i += 1;
            continue;
        }
        if match_slicer_comment(&trimmed, "WIPE_END") {
            wipe_active = false;
            after_wipe = true;
            i += 1;
            continue;
        }
        if wipe_active {
            i += 1;
            continue;
        }

        if after_travel {
            if trimmed.starts_with("G1 ")
                && (trimmed.contains('X') || trimmed.contains('Y'))
                && trimmed.contains(" E")
            {
                after_travel = false;
                // Go：此分支不 continue，落到后续通用处理
            } else if has_slicer_comment_prefix(&trimmed, "FEATURE:") {
                after_travel = false;
                result.push(iface[i].clone());
                i += 1;
                continue;
            } else {
                i += 1;
                continue;
            }
        }

        if after_wipe {
            if is_transfer_line(&trimmed) {
                i += 1;
                continue;
            }
            result.push(MARKER_ZJUMP_START.to_string());
            after_wipe = false;
            if (trimmed.contains("G1 X") || trimmed.contains("G1 Y"))
                && trimmed.contains('Z')
                && !trimmed.contains('E')
            {
                result.push(strip_z_from_line(&trimmed));
                after_travel = true;
                i += 1;
                continue;
            }
            result.push(iface[i].clone());
            i += 1;
            continue;
        }

        if (trimmed.contains("G1 X") || trimmed.contains("G1 Y"))
            && trimmed.contains('Z')
            && !trimmed.contains('E')
        {
            result.push(strip_z_from_line(&trimmed));
            i += 1;
            continue;
        }
        if trimmed.starts_with("G1 Z") {
            result.push(trimmed);
            i += 1;
            continue;
        }
        result.push(iface[i].clone());
        i += 1;
    }

    let mut iface = insert_feature_transition_jumps(result);

    if let Some(last) = iface.last() {
        if (last.contains("G1 X") || last.contains("G1 Y")) && !last.contains('E') {
            iface.pop();
        }
    }
    match iface.last() {
        Some(last) if last.contains(MARKER_ZJUMP_INTERNAL) || last.contains(MARKER_ZJUMP_START) => {
        }
        _ => iface.push(MARKER_ZJUMP_START.to_string()),
    }

    iface
}

/// `DeleteWipeContinuous`（delete_wipe_continuous.go:25）：保留 M204、
/// 插入 MICRO_LIFT 标记的连续变体。
pub(crate) fn delete_wipe_continuous(iface: &[String]) -> Vec<String> {
    let iface = strip_skippable_blocks(iface);

    let mut result: Vec<String> = Vec::new();
    let mut wipe_active = false;
    let mut after_wipe = false;
    let mut after_travel = false;

    let mut i = 0;
    while i < iface.len() {
        let trimmed = iface[i].trim().to_string();

        if match_slicer_comment(&trimmed, "WIPE_START") {
            wipe_active = true;
            i += 1;
            continue;
        }
        if match_slicer_comment(&trimmed, "WIPE_END") {
            wipe_active = false;
            after_wipe = true;
            i += 1;
            continue;
        }
        if wipe_active {
            i += 1;
            continue;
        }

        if after_travel {
            if trimmed.starts_with("G1 ")
                && (trimmed.contains('X') || trimmed.contains('Y'))
                && trimmed.contains(" E")
            {
                after_travel = false;
            } else if has_slicer_comment_prefix(&trimmed, "FEATURE:") {
                after_travel = false;
                result.push(iface[i].clone());
                i += 1;
                continue;
            } else if trimmed.starts_with("M204") {
                result.push(iface[i].clone());
                i += 1;
                continue;
            } else {
                i += 1;
                continue;
            }
        }

        if after_wipe {
            if is_transfer_line_continuous(&trimmed) {
                i += 1;
                continue;
            }
            if trimmed.starts_with("M204") {
                result.push(iface[i].clone());
                i += 1;
                continue;
            }
            after_wipe = false;
            if (trimmed.contains("G1 X") || trimmed.contains("G1 Y"))
                && trimmed.contains('Z')
                && !trimmed.contains('E')
            {
                result.push(strip_z_from_line(&trimmed));
                after_travel = true;
                i += 1;
                continue;
            }
            result.push(iface[i].clone());
            i += 1;
            continue;
        }

        if (trimmed.contains("G1 X") || trimmed.contains("G1 Y"))
            && trimmed.contains('Z')
            && !trimmed.contains('E')
        {
            result.push(strip_z_from_line(&trimmed));
            i += 1;
            continue;
        }
        if trimmed.starts_with("G1 Z") {
            result.push(trimmed);
            i += 1;
            continue;
        }
        result.push(iface[i].clone());
        i += 1;
    }

    let iface = insert_micro_lift_markers(result);
    let mut iface = insert_feature_transition_jumps(iface);

    if let Some(last) = iface.last() {
        if (last.contains("G1 X") || last.contains("G1 Y")) && !last.contains('E') {
            iface.pop();
        }
    }
    match iface.last() {
        Some(last) if last.contains(MARKER_ZJUMP_INTERNAL) || last.contains(MARKER_ZJUMP_START) => {
        }
        _ => iface.push(MARKER_ZJUMP_START.to_string()),
    }

    iface
}

/// `insertFeatureTransitionJumps`（delete_wipe.go:118）：在「非打印移动的间隙」
/// 处插入 `;ZJUMP_INTERNAL[ ; FEATURE: ...]`，压缩特征过渡段。
fn insert_feature_transition_jumps(lines: Vec<String>) -> Vec<String> {
    let is_print_move = |line: &str| -> bool {
        let trimmed = line.trim();
        trimmed.starts_with("G1 ")
            && (trimmed.contains('X') || trimmed.contains('Y'))
            && trimmed.contains(" E")
    };

    let is_marker = |line: &str| -> bool {
        let trimmed = line.trim();
        trimmed.contains(MARKER_ZJUMP_START)
            || trimmed.contains(MARKER_ZJUMP_INTERNAL)
            || (trimmed.starts_with("G1 ")
                && (trimmed.contains('X') || trimmed.contains('Y'))
                && !trimmed.contains(" E"))
    };

    let is_feature_transition = |line: &str| -> bool {
        let trimmed = line.trim();
        if has_slicer_comment_prefix(trimmed, "FEATURE:") && !is_glue_continuous_feature(trimmed) {
            return true;
        }
        if trimmed.starts_with("G1 Z") || trimmed.starts_with("G3 ") {
            return true;
        }
        false
    };

    let is_feature_comment = |line: &str| -> bool {
        let trimmed = line.trim();
        has_slicer_comment_prefix(trimmed, "FEATURE:") && !is_glue_continuous_feature(trimmed)
    };

    struct Gap {
        start: usize,
        end: usize,
        has_transition: bool,
        feature_comments: Vec<String>,
    }

    let mut gaps: Vec<Gap> = Vec::new();
    let mut gap_start: isize = -1;

    for (i, line) in lines.iter().enumerate() {
        if is_print_move(line) || is_marker(line) {
            if gap_start >= 0 && gap_start <= i as isize - 1 {
                let start = gap_start as usize;
                let end = i - 1;
                let mut g = Gap {
                    start,
                    end,
                    has_transition: false,
                    feature_comments: Vec::new(),
                };
                for j in start..=end {
                    if is_feature_transition(&lines[j]) {
                        g.has_transition = true;
                    }
                    if is_feature_comment(&lines[j]) {
                        g.feature_comments.push(lines[j].trim().to_string());
                    }
                }
                gaps.push(g);
            }
            gap_start = -1;
        } else if gap_start < 0 {
            gap_start = i as isize;
        }
    }

    if gap_start >= 0 && gap_start <= lines.len() as isize - 1 {
        let start = gap_start as usize;
        let end = lines.len() - 1;
        let mut g = Gap {
            start,
            end,
            has_transition: false,
            feature_comments: Vec::new(),
        };
        for j in start..=end {
            if is_feature_transition(&lines[j]) {
                g.has_transition = true;
            }
            if is_feature_comment(&lines[j]) {
                g.feature_comments.push(lines[j].trim().to_string());
            }
        }
        gaps.push(g);
    }

    if gaps.is_empty() {
        return lines;
    }

    let mut result: Vec<String> = Vec::new();
    let mut prev_end: isize = -1;
    for g in &gaps {
        for i in (prev_end + 1) as usize..g.start {
            result.push(lines[i].clone());
        }
        if g.has_transition {
            let mut marker = MARKER_ZJUMP_INTERNAL.to_string();
            for fc in &g.feature_comments {
                marker.push_str(&format!(" {fc}"));
            }
            result.push(marker);
            let mut skip_to = g.end;
            if g.end + 1 < lines.len() && lines[g.end + 1].contains(MARKER_ZJUMP_START) {
                skip_to = g.end + 1;
            }
            prev_end = skip_to as isize;
        } else {
            for i in g.start..=g.end {
                result.push(lines[i].clone());
            }
            prev_end = g.end as isize;
        }
    }
    for i in (prev_end + 1) as usize..lines.len() {
        result.push(lines[i].clone());
    }

    result
}

/// `insertMicroLiftMarkers`（delete_wipe_continuous.go:112）：M204 S10000 之后
/// 若下一条加速度是 S6000（且中间无挤出移动），插入 MICRO_LIFT 标记。
fn insert_micro_lift_markers(iface: Vec<String>) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for i in 0..iface.len() {
        let trimmed = iface[i].trim().to_string();
        result.push(iface[i].clone());
        if trimmed.starts_with("M204") && trimmed.contains("S10000") {
            let mut has_next_s6000 = false;
            for j in i + 1..iface.len() {
                let next_trimmed = iface[j].trim();
                if next_trimmed.starts_with("M204") && next_trimmed.contains("S6000") {
                    has_next_s6000 = true;
                    break;
                }
                if next_trimmed.starts_with("G1 ")
                    && (next_trimmed.contains('X') || next_trimmed.contains('Y'))
                    && next_trimmed.contains(" E")
                {
                    break;
                }
                if next_trimmed.starts_with(';') && !next_trimmed.starts_with(";M204") {
                    continue;
                }
            }
            if has_next_s6000 {
                result.push(MARKER_MICRO_LIFT.to_string());
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn delete_wipe_removes_wipe_section() {
        let input = lines(&[
            "G1 X1 Y1 E0.5",
            ";WIPE_START",
            "G1 X2 Y2",
            ";WIPE_END",
            "G1 X3 Y3 E0.6",
        ]);
        let out = delete_wipe(&input);
        // WIPE 段被删，WIPE_END 后接 ZJUMP_START
        assert!(!out.iter().any(|l| l.contains("WIPE")));
        assert!(out.iter().any(|l| l.contains(MARKER_ZJUMP_START)));
        assert!(out.iter().any(|l| l.contains("G1 X1 Y1 E0.5")));
        assert!(out.iter().any(|l| l.contains("G1 X3 Y3 E0.6")));
        // 末尾必为 ZJUMP 标记
        let last = out.last().unwrap();
        assert!(last.contains(MARKER_ZJUMP_START) || last.contains(MARKER_ZJUMP_INTERNAL));
    }

    #[test]
    fn delete_wipe_empty_input() {
        let out = delete_wipe(&[]);
        assert_eq!(out, vec![MARKER_ZJUMP_START.to_string()]);
    }

    #[test]
    fn skippable_blocks_stripped() {
        let input = lines(&[
            "G1 X1 Y1 E0.5",
            "; SKIPPABLE_START",
            "M971 S11 C11",
            "; SKIPPABLE_END",
            "G1 X2 Y2 E0.6",
        ]);
        let out = strip_skippable_blocks(&input);
        assert_eq!(out.len(), 2);
        assert!(!out.iter().any(|l| l.contains("M971")));
    }

    /// KNOWN BUG RISK-011：SKIPPABLE_END 缺失 → skip 残留到函数结束（逐字复刻）。
    #[test]
    fn skippable_blocks_missing_end_skip_residual() {
        let input = lines(&[
            "G1 X1.0 Y1.0 E0.1",
            "; SKIPPABLE_START",
            "G1 X2.0 Y2.0 E0.2",
            "G1 X3.0 Y3.0 E0.3",
            "G1 X4.0 Y4.0 E0.4",
        ]);
        let out = strip_skippable_blocks(&input);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], "G1 X1.0 Y1.0 E0.1");
    }

    #[test]
    fn micro_lift_marker_inserted() {
        let input = lines(&["M204 S10000", "M204 S6000", "G1 X1 Y1 E0.5"]);
        let out = insert_micro_lift_markers(input);
        assert_eq!(out[1], MARKER_MICRO_LIFT);
    }

    #[test]
    fn strip_z_from_line_variants() {
        // Go 语义：line[:zIdx] 的尾随空格 + rest[spaceIdx:] 的前导空格 = 双空格
        assert_eq!(strip_z_from_line("G1 X10 Y20 Z0.5 E1"), "G1 X10 Y20  E1");
        assert_eq!(strip_z_from_line("G1 X10 Z0.5"), "G1 X10");
        // Z 前无空格（词中 Z）不剥 —— Go 原样
        assert_eq!(strip_z_from_line("G1 X10Y20Z5"), "G1 X10Y20Z5");
    }
}
