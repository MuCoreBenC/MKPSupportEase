//! iface 的按模型分组与熨平/支撑面混合（对照 processor/model_z_map.go）。

// 逐字对照 model_z_map.go：索引循环与 Go 同形。
#![allow(clippy::needless_range_loop)]

use crate::gcode::{MARKER_ZJUMP_INTERNAL, MARKER_ZJUMP_START, extract_coord};

/// 私有 bbox（Go 小写 bbox；与 geom::BBox 不同，仅本模块与 glue_block 用）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct IfaceBbox {
    pub(crate) min_x: f64,
    pub(crate) min_y: f64,
    pub(crate) max_x: f64,
    pub(crate) max_y: f64,
}

impl IfaceBbox {
    fn sentinel() -> Self {
        IfaceBbox {
            min_x: 1.0,
            min_y: 1.0,
            max_x: -1.0,
            max_y: -1.0,
        }
    }
}

/// `bboxOfLines`：所有 "G1 " 行的 X/Y 极值（model_z_map.go:14）。
pub(crate) fn bbox_of_lines(lines: &[String]) -> IfaceBbox {
    let mut b = IfaceBbox {
        min_x: f64::MAX,
        min_y: f64::MAX,
        max_x: f64::MIN,
        max_y: f64::MIN,
    };
    let mut found = false;
    for line in lines {
        if !line.starts_with("G1 ") {
            continue;
        }
        if let Some(x_val) = extract_coord(line.as_bytes(), b'X') {
            b.min_x = b.min_x.min(x_val);
            b.max_x = b.max_x.max(x_val);
            found = true;
        }
        if let Some(y_val) = extract_coord(line.as_bytes(), b'Y') {
            b.min_y = b.min_y.min(y_val);
            b.max_y = b.max_y.max(y_val);
            found = true;
        }
    }
    if !found {
        return IfaceBbox::sentinel();
    }
    b
}

/// `calcBBoxIoU`（model_z_map.go:60）。
fn calc_bbox_iou(a: IfaceBbox, b: IfaceBbox) -> f64 {
    if a.min_x > a.max_x || b.min_x > b.max_x {
        return 0.0;
    }
    let inter_min_x = a.min_x.max(b.min_x);
    let inter_min_y = a.min_y.max(b.min_y);
    let inter_max_x = a.max_x.min(b.max_x);
    let inter_max_y = a.max_y.min(b.max_y);
    let mut inter_area = 0.0;
    if inter_max_x > inter_min_x && inter_max_y > inter_min_y {
        inter_area = (inter_max_x - inter_min_x) * (inter_max_y - inter_min_y);
    }
    let area_a = (a.max_x - a.min_x) * (a.max_y - a.min_y);
    let area_b = (b.max_x - b.min_x) * (b.max_y - b.min_y);
    let union_area = area_a + area_b - inter_area;
    if union_area <= 0.0 {
        return 0.0;
    }
    inter_area / union_area
}

fn expand_bbox(b: IfaceBbox, margin: f64) -> IfaceBbox {
    if b.min_x > b.max_x {
        return b;
    }
    IfaceBbox {
        min_x: b.min_x - margin,
        min_y: b.min_y - margin,
        max_x: b.max_x + margin,
        max_y: b.max_y + margin,
    }
}

fn expand_bbox_by_centroid(b: IfaceBbox, radius: f64) -> IfaceBbox {
    if b.min_x > b.max_x {
        return b;
    }
    let cx = (b.min_x + b.max_x) / 2.0;
    let cy = (b.min_y + b.max_y) / 2.0;
    IfaceBbox {
        min_x: cx - radius,
        min_y: cy - radius,
        max_x: cx + radius,
        max_y: cy + radius,
    }
}

fn bbox_distance(a: IfaceBbox, b: IfaceBbox) -> f64 {
    let mut gap_x = (a.min_x - b.max_x).max(b.min_x - a.max_x);
    let mut gap_y = (a.min_y - b.max_y).max(b.min_y - a.max_y);
    gap_x = gap_x.max(0.0);
    gap_y = gap_y.max(0.0);
    gap_x.max(gap_y)
}

fn bbox_overlaps(a: IfaceBbox, b: IfaceBbox) -> bool {
    if a.min_x > a.max_x || b.min_x > b.max_x {
        return false;
    }
    a.min_x <= b.max_x && a.max_x >= b.min_x && a.min_y <= b.max_y && a.max_y >= b.min_y
}

fn merge_bbox(a: IfaceBbox, b: IfaceBbox) -> IfaceBbox {
    IfaceBbox {
        min_x: a.min_x.min(b.min_x),
        min_y: a.min_y.min(b.min_y),
        max_x: a.max_x.max(b.max_x),
        max_y: a.max_y.max(b.max_y),
    }
}

/// `totalPathLength`（model_z_map.go:110）：带 E 的 G1 移动总长（跳转标记重置基点）。
pub(crate) fn total_path_length(lines: &[String]) -> f64 {
    let mut last_x = 0.0;
    let mut last_y = 0.0;
    let mut has_last = false;
    let mut total = 0.0;
    for line in lines {
        if line.contains(MARKER_ZJUMP_START) || line.contains(MARKER_ZJUMP_INTERNAL) {
            has_last = false;
            continue;
        }
        if !line.starts_with("G1 ") {
            continue;
        }
        if !line.contains(" E") {
            has_last = false;
            continue;
        }
        let x_val = extract_coord(line.as_bytes(), b'X');
        let y_val = extract_coord(line.as_bytes(), b'Y');
        let mut cur_x = 0.0;
        let mut cur_y = 0.0;
        let mut has_xy = false;
        if let Some(xv) = x_val {
            cur_x = xv;
            has_xy = true;
        } else if has_last {
            cur_x = last_x;
            has_xy = true;
        }
        if let Some(yv) = y_val {
            cur_y = yv;
            has_xy = true;
        } else if has_last {
            cur_y = last_y;
        }
        if has_xy && has_last {
            let dx = cur_x - last_x;
            let dy = cur_y - last_y;
            total += (dx * dx + dy * dy).sqrt();
        }
        if has_xy {
            last_x = cur_x;
            last_y = cur_y;
            has_last = true;
        }
    }
    total
}

/// `splitIfaceByModel`（model_z_map.go:148）：按 ZJUMP_START 切段。
pub(crate) fn split_iface_by_model(iface: &[String]) -> Vec<Vec<String>> {
    let mut segments: Vec<Vec<String>> = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in iface {
        if line.contains(MARKER_ZJUMP_START) {
            if !current.is_empty() {
                segments.push(current);
            }
            current = Vec::new();
            continue;
        }
        current.push(line.clone());
    }
    if !current.is_empty() {
        segments.push(current);
    }
    if segments.is_empty() {
        segments.push(iface.to_vec());
    }
    segments
}

fn merge_group_segments(segs: Vec<Vec<String>>) -> Vec<String> {
    let mut result: Vec<String> = Vec::new();
    for (i, seg) in segs.into_iter().enumerate() {
        if i > 0 {
            result.push(MARKER_ZJUMP_INTERNAL.to_string());
        }
        result.extend(seg);
    }
    result
}

/// `groupConsecutiveSegments`（model_z_map.go:186）：IoU>0 或间距<2 的相邻段合并。
fn group_consecutive_segments(segments: Vec<Vec<String>>) -> Vec<Vec<String>> {
    if segments.is_empty() {
        return Vec::new();
    }
    let mut groups: Vec<Vec<String>> = Vec::new();
    let mut group_segs: Vec<Vec<String>> = Vec::new();
    let mut group_bbox = IfaceBbox::sentinel();
    for seg in segments {
        let seg_bbox = bbox_of_lines(&seg);
        if group_segs.is_empty() {
            group_bbox = seg_bbox;
            group_segs.push(seg);
            continue;
        }
        if calc_bbox_iou(seg_bbox, group_bbox) > 0.0 || bbox_distance(seg_bbox, group_bbox) < 2.0 {
            group_bbox = merge_bbox(group_bbox, seg_bbox);
            group_segs.push(seg);
        } else {
            groups.push(merge_group_segments(std::mem::take(&mut group_segs)));
            group_bbox = seg_bbox;
            group_segs.push(seg);
        }
    }
    if !group_segs.is_empty() {
        groups.push(merge_group_segments(group_segs));
    }
    groups
}

/// `convertInternalJumps`（model_z_map.go:171）：把 INTERNAL 跳转按分组升级为 START。
pub(crate) fn convert_internal_jumps(iface: &[String]) -> Vec<String> {
    let segments = split_iface_by_model(iface);
    let groups = group_consecutive_segments(segments);
    if groups.is_empty() {
        return iface.to_vec();
    }
    let mut result: Vec<String> = Vec::new();
    for (gi, grp) in groups.into_iter().enumerate() {
        if gi > 0 {
            result.push(MARKER_ZJUMP_START.to_string());
        }
        result.extend(grp);
    }
    result
}

/// `BuildPerModelIface`（model_z_map.go:210）：按区域混合熨烫面 / 支撑面。
/// threshold=0 时重叠即判覆盖；否则按路径长度覆盖率判。
pub(crate) fn build_per_model_iface(
    iface_ironing: &[String],
    iface_moisture: &[String],
    threshold: f64,
    suppress_expand: f64,
    suppress_expand_mode: &str,
) -> Vec<String> {
    let ironing_groups = group_consecutive_segments(split_iface_by_model(iface_ironing));
    let moisture_groups = group_consecutive_segments(split_iface_by_model(iface_moisture));

    if ironing_groups.is_empty() {
        return convert_internal_jumps(iface_moisture);
    }

    let effective_threshold = threshold;

    let ironing_bboxes: Vec<IfaceBbox> = ironing_groups.iter().map(|g| bbox_of_lines(g)).collect();
    let ironing_path_lens: Vec<f64> = ironing_groups
        .iter()
        .map(|g| total_path_length(g))
        .collect();
    let moisture_bboxes: Vec<IfaceBbox> =
        moisture_groups.iter().map(|g| bbox_of_lines(g)).collect();
    let moisture_path_lens: Vec<f64> = moisture_groups
        .iter()
        .map(|g| total_path_length(g))
        .collect();

    let mut moisture_covered = vec![false; moisture_groups.len()];
    let mut ironing_used = vec![false; ironing_groups.len()];

    // Phase 1：判定覆盖关系
    for mi in 0..moisture_groups.len() {
        if suppress_expand > 0.0 {
            let mut skip = false;
            for ii in 0..ironing_groups.len() {
                let expanded = if suppress_expand_mode == "centroid" {
                    expand_bbox_by_centroid(ironing_bboxes[ii], suppress_expand)
                } else {
                    expand_bbox(ironing_bboxes[ii], suppress_expand)
                };
                if bbox_overlaps(moisture_bboxes[mi], expanded) {
                    skip = true;
                    break;
                }
            }
            if skip {
                moisture_covered[mi] = true;
                continue;
            }
        }

        let mut best_ii: isize = -1;
        let mut best_iou = -1.0;
        for ii in 0..ironing_groups.len() {
            if ironing_used[ii] {
                continue;
            }
            if !bbox_overlaps(moisture_bboxes[mi], ironing_bboxes[ii]) {
                continue;
            }
            let iou = calc_bbox_iou(moisture_bboxes[mi], ironing_bboxes[ii]);
            if iou > best_iou {
                best_iou = iou;
                best_ii = ii as isize;
            }
        }

        if best_ii >= 0 {
            let best = best_ii as usize;
            if threshold == 0.0 {
                moisture_covered[mi] = true;
                ironing_used[best] = true;
                continue;
            }
            let cov = if moisture_path_lens[mi] > 0.0 {
                ironing_path_lens[best] / moisture_path_lens[mi] * 100.0
            } else {
                100.0
            };
            if cov >= effective_threshold {
                moisture_covered[mi] = true;
                ironing_used[best] = true;
                continue;
            }
        }
    }

    // Phase 1.5：被已使用熨烫面分组覆盖的支撑面分组一并标记
    for ii in 0..ironing_groups.len() {
        if !ironing_used[ii] {
            continue;
        }
        for mi in 0..moisture_groups.len() {
            if moisture_covered[mi] {
                continue;
            }
            if bbox_overlaps(moisture_bboxes[mi], ironing_bboxes[ii]) {
                moisture_covered[mi] = true;
            }
        }
    }

    // Phase 2：冲突检测
    let mut ironing_skip = vec![false; ironing_groups.len()];
    for ii in 0..ironing_groups.len() {
        if ironing_used[ii] {
            continue;
        }
        for mi in 0..moisture_groups.len() {
            if moisture_covered[mi] {
                continue;
            }
            if bbox_overlaps(moisture_bboxes[mi], ironing_bboxes[ii]) {
                if threshold == 0.0 {
                    moisture_covered[mi] = true;
                } else {
                    ironing_skip[ii] = true;
                }
                break;
            }
        }
    }

    // Phase 3：构建结果 —— 先保留的支撑面，后未跳过的熨烫面
    let mut result: Vec<String> = Vec::new();
    for (mi, m_grp) in moisture_groups.into_iter().enumerate() {
        if moisture_covered[mi] {
            continue;
        }
        if !result.is_empty() {
            result.push(MARKER_ZJUMP_START.to_string());
        }
        result.extend(m_grp);
    }
    for (ii, i_grp) in ironing_groups.into_iter().enumerate() {
        if ironing_skip[ii] {
            continue;
        }
        if !result.is_empty() {
            result.push(MARKER_ZJUMP_START.to_string());
        }
        result.extend(i_grp);
    }

    result
}

// parseFloat（model_z_map.go:140）在 Go 侧无消费者，未移植。

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn split_by_zjump_start() {
        let input = lines(&["G1 X1 Y1 E0.5", MARKER_ZJUMP_START, "G1 X9 Y9 E0.5"]);
        let segs = split_iface_by_model(&input);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[1][0], "G1 X9 Y9 E0.5");
    }

    #[test]
    fn total_path_length_accumulates() {
        let input = lines(&["G1 X0 Y0 E0.5", "G1 X3 Y4 E0.6"]);
        assert_eq!(total_path_length(&input), 5.0);
    }

    #[test]
    fn empty_iface_splits_to_single_empty_segment() {
        let segs = split_iface_by_model(&[]);
        assert_eq!(segs.len(), 1);
        assert!(segs[0].is_empty());
    }
}
