//! 涂胶块发射（对照 processor/glue_block.go + pass1_emit.go 的 Z 补偿与微抬）。

// 逐字对照 glue_block.go：嵌套 if 与 Go 同形（涂胶块的分支链本身就近百行）。
#![allow(
    clippy::collapsible_if,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    clippy::manual_clamp
)]

use std::collections::HashMap;

use super::ZKey;

use crate::diag::PostprocError;
use crate::gcode::{
    format_e, format_float, format_python_float, format_speed, format_z, get_e_value, has_e_param,
    replace_e_value,
};
use crate::ir::{Ir, num_strip};
use crate::postproc::marks;

use super::LineSink;
use super::model_iface::{IfaceBbox, split_iface_by_model, total_path_length};
use super::scan::{find_seg_start_cmd, parse_xy_from_g1, strip_z_and_e};
use super::stages::{MkpStage, stage_comment_line};

/// Z 补偿包围盒每侧外扩量（pass1_emit.go ZCompExpandMM，±2mm/侧）。
const Z_COMP_EXPAND_MM: f64 = 2.0;

/// `calcGlueZ`：按 4 角双线性插值计算 (x, y) 处的涂胶 Z 补偿量。
pub(crate) fn calc_glue_z(
    base_z: f64,
    x: f64,
    y: f64,
    ir_data: &Ir,
    patch_bbox: Option<IfaceBbox>,
) -> f64 {
    if !ir_data.glue.z_comp_enabled {
        return base_z;
    }
    let (corners, mut bbox) = if ir_data.glue.z_comp_mode == "patch" {
        match patch_bbox {
            Some(b) => (&ir_data.glue.z_comp_patch, b),
            None => (
                &ir_data.glue.z_comp_bed,
                IfaceBbox {
                    min_x: ir_data.machine.min_x,
                    max_x: ir_data.machine.max_x,
                    min_y: ir_data.machine.min_y,
                    max_y: ir_data.machine.max_y,
                },
            ),
        }
    } else {
        (
            &ir_data.glue.z_comp_bed,
            IfaceBbox {
                min_x: ir_data.machine.min_x,
                max_x: ir_data.machine.max_x,
                min_y: ir_data.machine.min_y,
                max_y: ir_data.machine.max_y,
            },
        )
    };
    // 全 0 快速返回（常见「未配置补偿」场景）
    if corners.front_left == 0.0
        && corners.front_right == 0.0
        && corners.back_left == 0.0
        && corners.back_right == 0.0
    {
        return base_z;
    }
    // 外扩 2mm（每侧 +2，总 +4）
    bbox.min_x -= Z_COMP_EXPAND_MM;
    bbox.max_x += Z_COMP_EXPAND_MM;
    bbox.min_y -= Z_COMP_EXPAND_MM;
    bbox.max_y += Z_COMP_EXPAND_MM;
    let dx = bbox.max_x - bbox.min_x;
    let dy = bbox.max_y - bbox.min_y;
    if dx == 0.0 || dy == 0.0 {
        return base_z;
    }
    let mut xn = (x - bbox.min_x) / dx;
    let mut yn = (y - bbox.min_y) / dy;
    if xn < 0.0 {
        xn = 0.0;
    } else if xn > 1.0 {
        xn = 1.0;
    }
    if yn < 0.0 {
        yn = 0.0;
    } else if yn > 1.0 {
        yn = 1.0;
    }
    // 双线性插值（yn=0 → Front，yn=1 → Back）
    let comp = corners.front_left * (1.0 - xn) * (1.0 - yn)
        + corners.front_right * xn * (1.0 - yn)
        + corners.back_left * (1.0 - xn) * yn
        + corners.back_right * xn * yn;
    base_z + comp
}

/// 笔的姿态。**所有** 涂胶高度都必须经过这里，避免同一公式散落成多份。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PenZ {
    /// 段间跳转：笔尖离层顶 `inter_model_lift_height`
    Hop,
    /// 平飞 / 收笔 / 装笔：笔尖离层顶 `z_lift_height`（首层分支用 `_first_layers`，判据是层顶高度）
    Travel,
    /// 落笔：笔尖压进上一层 `glue_z_offset`
    Landed,
}

/// 笔尖相对给定层顶的目标高度。
///
/// `Z_t = Z_n - offset_z`，`Z_n = 层顶 + offset_z + 净空` ⇒ `Z_t = 层顶 + 净空`。
/// 净空与笔伸多长无关；界面上的「涂胶区域间 Z 轴抬升高度」就是这个净空。
fn glue_pen_z(ir_data: &Ir, layer_z: f64, kind: PenZ) -> f64 {
    let clearance = match kind {
        PenZ::Hop => ir_data.glue.inter_model_lift_height,
        PenZ::Travel => {
            if layer_z < 3.0 {
                ir_data.glue.z_lift_height_first_layers
            } else {
                ir_data.glue.z_lift_height
            }
        }
        PenZ::Landed => ir_data.glue.z_offset,
    };
    layer_z + clearance
}

/// 喷嘴（G-code 里的 Z）目标高度 = 笔尖高度 + 笔的机械偏移。
fn glue_nozzle_z(ir_data: &Ir, layer_z: f64, kind: PenZ) -> f64 {
    glue_pen_z(ir_data, layer_z, kind) + ir_data.toolhead.z_offset
}

/// 一段跳转：抬一次 → 走完整段空走 → 只在涂胶行之前落一次（`pass1_emit.go:88`）。
///
/// 切片器在长跳转上会把移动拆成**多条首尾相接的空走**（实测最多两条，都带 Z）。
/// 旧实现只把标记后**紧邻的一条**交出去，后面的空走要么被 `delete_wipe` 丢弃、
/// 要么按涂胶线原样发出 —— 两者都会让笔在落笔高度飞过一段距离。
/// 这里按「连续空走」整段抬着走完，返回第一条未消费的下标。
fn emit_jump_run(
    sink: &mut LineSink,
    lines: &[String],
    from: usize,
    ir_data: &Ir,
    current_layer_height: f64,
    last_layer_height: f64,
    already_up: bool,
) -> Result<(usize, f64, f64, f64), PostprocError> {
    let lift_height = ir_data.glue.inter_model_lift_height;
    let mut cur_z = glue_nozzle_z(ir_data, current_layer_height, PenZ::Hop);
    if already_up {
        cur_z = f64::NAN; // 交给调用方保持它自己记的那个值
    } else {
        sink.push_str(&format!(
            "G1 F{}",
            format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
        ));
        sink.push_str(&format!(
            "G1 Z{}",
            format_z(glue_nozzle_z(ir_data, current_layer_height, PenZ::Hop))
        ));
    }

    let mut i = from;
    let mut moved = 0usize;
    let mut last = (f64::NAN, f64::NAN);
    while i < lines.len() {
        let tl = lines[i].trim();
        if !is_travel_line(tl) {
            break;
        }
        let jump_cmd = strip_z_and_e(tl);
        // 与上一句同点 ⇒ 0mm 移动，发出去只会多一根竖线，跳过
        let (px, py, pok) = parse_xy_from_g1(&jump_cmd);
        if pok && moved > 0 && (px - last.0).abs() < 1e-9 && (py - last.1).abs() < 1e-9 {
            i += 1;
            continue;
        }
        let processed = crate::postproc::offset::process_gcode_offset(
            &jump_cmd,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset,
            ir_data.toolhead.z_offset + lift_height,
            crate::gcode::OffsetMode::Normal,
            ir_data,
        )?;
        sink.push_str(&processed);
        let (lx, ly, ok) = parse_xy_from_g1(&jump_cmd);
        if ok {
            if jump_cmd.contains('X') {
                last.0 = lx;
            }
            if jump_cmd.contains('Y') {
                last.1 = ly;
            }
        }
        moved += 1;
        i += 1;
    }
    if moved == 0 {
        // 标记后面不是空走（少见）：退回「一条」的旧行为，别把涂胶行吞掉
        let jump_cmd = strip_z_and_e(lines[from].trim());
        let processed = crate::postproc::offset::process_gcode_offset(
            &jump_cmd,
            ir_data.toolhead.x_offset,
            ir_data.toolhead.y_offset,
            ir_data.toolhead.z_offset + lift_height,
            crate::gcode::OffsetMode::Normal,
            ir_data,
        )?;
        sink.push_str(&processed);
        i = from + 1;
    }

    /*
     * 后面紧跟的仍然是标记 ⇒ 说明这一跳还没结束（切片器把一次跳转拆成多行，
     * `delete_wipe` 又在中间插了标记）。此时**不许落**：保持抬着，由下一个标记接着走，
     * 直到真正出现涂胶行再落。返回 `false` 表示「还抬着」。
     */
    // 往后看穿空行/注释：中间夹一句注释不代表这次跳转结束了
    let next_is_marker = lines
        .iter()
        .skip(i)
        .find(|l| {
            let t = l.trim();
            !t.is_empty() && !t.starts_with(';')
        })
        .map(|l| {
            let t = l.trim();
            t.contains(crate::gcode::MARKER_ZJUMP_START)
                || t.contains(crate::gcode::MARKER_ZJUMP_INTERNAL)
                || t.contains(crate::gcode::MARKER_MICRO_LIFT)
        })
        .unwrap_or(false);
    if next_is_marker {
        return Ok((i, last.0, last.1, cur_z));
    }

    let landed_z = glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed);
    sink.push_str(&format!("G1 Z{}", format_z(landed_z)));
    sink.push_str(&format!(
        "G1 F{}",
        format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
    ));
    Ok((i, last.0, last.1, landed_z))
}

/// 这一段连续空走是否真的把笔挪了位置（零长度重复句不算）。
///
/// **看每一个点，不只看终点**：`A → B → A` 这种来回，终点等于起点，但中间
/// 确实飞出去过一段——只看终点会把它当成「没动」，于是这段就落在落笔高度上。
fn travel_run_moves(lines: &[String], from: usize, cur_x: f64, cur_y: f64) -> bool {
    let (mut x, mut y) = (cur_x, cur_y);
    let mut i = from;
    while i < lines.len() {
        let tl = lines[i].trim();
        if !is_travel_line(tl) {
            break;
        }
        let (nx, ny, _) = parse_xy_from_g1(&strip_z_and_e(tl));
        if tl.contains('X') {
            x = nx;
        }
        if tl.contains('Y') {
            y = ny;
        }
        if (x - cur_x).abs() > 1e-6 || (y - cur_y).abs() > 1e-6 {
            return true;
        }
        i += 1;
    }
    false
}

/// 只移动不挤出的一行（空走）。判据是语法：`G1` + 有 X/Y + 没有 `E`。
fn is_travel_line(line: &str) -> bool {
    let code = line.split(';').next().unwrap_or("").trim();
    code.starts_with("G1 ") && (code.contains('X') || code.contains('Y')) && !code.contains(" E")
}

/// 段首的**前导空走**：连续若干条不带 `E` 的移动。返回 (最后一条命令, 跳过几条)。
///
/// 这些点是切片器从**上一个特征**出发时的路径（它会先飞到特征前的位置、再飞到涂胶
/// 起点）。我们的笔是从换笔位飞过来的，逐点复刻只会多绕一段 —— 作者 2026-11-04 指出的
/// 「为什么先去 X137.456 而不直接到 X155.812」就是这一段。回位直接取这段的终点。
fn leading_travel_run(seg: &[String]) -> Option<(String, usize)> {
    let is_marker = |t: &str| {
        t.contains(crate::gcode::MARKER_ZJUMP_START)
            || t.contains(crate::gcode::MARKER_ZJUMP_INTERNAL)
            || t.contains(crate::gcode::MARKER_MICRO_LIFT)
    };
    let mut i = 0;
    while i < seg.len() {
        let t = seg[i].trim();
        if t.is_empty() || t.starts_with(';') {
            i += 1;
            continue;
        }
        break;
    }
    if i >= seg.len() {
        return None;
    }
    // 第一条必须是空走；第一条就是涂胶行 ⇒ 没有前导段
    let first = &seg[i];
    if !(first.contains("G1 X") || first.contains("G1 Y")) || first.contains(" E") {
        return None;
    }
    let mut end = i;
    let mut j = i;
    while j < seg.len() {
        let t = seg[j].trim();
        if t.is_empty() || is_marker(t) || t.starts_with("; FEATURE") || t.starts_with(";FEATURE") {
            j += 1;
            continue;
        }
        if (t.contains("G1 X") || t.contains("G1 Y")) && !t.contains(" E") {
            end = j;
            j += 1;
            continue;
        }
        break;
    }
    Some((seg[end].trim().to_string(), end + 1))
}

/// `writeGlueLines`（pass1_emit.go:120）：发射一个段的涂胶移动。
/// 注意：稀疏化分支会**修改** `lines`（Go 同名行为，调用方负责传拷贝）。
fn write_glue_lines(
    sink: &mut LineSink,
    lines: &mut Vec<String>,
    ir_data: &Ir,
    current_layer_height: f64,
    last_layer_height: f64,
    start_z: f64,
    skip_leading: usize,
) -> Result<(), PostprocError> {
    use crate::gcode::MARKER_MICRO_LIFT;
    use crate::gcode::{MARKER_ZJUMP_INTERNAL, MARKER_ZJUMP_START};

    if ir_data.machine.nozzle_diameter <= 0.3 && ir_data.glue.sparse_ratio > 0.0 {
        let mut move_indices: Vec<usize> = Vec::new();
        for (i, l) in lines.iter().enumerate() {
            let tl = l.trim();
            if tl.starts_with("G1 ")
                && !tl.contains("G1 F")
                && (tl.contains('X') || tl.contains('Y'))
            {
                move_indices.push(i);
            }
        }
        if move_indices.len() > 2 {
            let step = 100.0 / ir_data.glue.sparse_ratio;
            let mut keep_set = std::collections::HashSet::new();
            let mut k = 0.0f64;
            while k < move_indices.len() as f64 {
                let mut idx = k as usize;
                if idx >= move_indices.len() {
                    idx = move_indices.len() - 1;
                }
                keep_set.insert(move_indices[idx]);
                k += step;
            }
            keep_set.insert(move_indices[0]);
            let last = *move_indices.last().unwrap();
            keep_set.insert(last);
            for &mi in move_indices.iter().rev() {
                if !keep_set.contains(&mi) {
                    lines.remove(mi);
                }
            }
        }
    }

    // Patch 模式 Z 补偿 bbox 预计算（每层一个总 bbox 的降级策略）
    let mut layer_bbox: Option<IfaceBbox> = None;
    if ir_data.glue.z_comp_enabled && ir_data.glue.z_comp_mode == "patch" {
        let mut bx = IfaceBbox {
            min_x: f64::MAX,
            max_x: f64::MIN,
            min_y: f64::MAX,
            max_y: f64::MIN,
        };
        for l in lines.iter() {
            let tl = l.trim();
            if !tl.starts_with("G1 ") {
                continue;
            }
            let (x, y, ok) = parse_xy_from_g1(tl);
            if !ok {
                continue;
            }
            if tl.contains('X') {
                bx.min_x = bx.min_x.min(x);
                bx.max_x = bx.max_x.max(x);
            }
            if tl.contains('Y') {
                bx.min_y = bx.min_y.min(y);
                bx.max_y = bx.max_y.max(y);
            }
        }
        if bx.max_x > bx.min_x && bx.max_y > bx.min_y {
            layer_bbox = Some(bx);
        }
    }

    // prevX/prevY 维护笔当前所在点（G1 可能只含单轴）。段首由 `emit_glue_block`
    // 回位并落笔到段起点，所以起点取 iface 里第一处 XY 的落点 —— 否则那句与回位
    // 重复的零长度空走会被误判成「需要抬一次」。
    let mut prev_x = 0.0;
    let mut prev_y = 0.0;
    if skip_leading > 0 {
        if let Some(l) = lines.get(skip_leading - 1) {
            let (x, y, ok) = parse_xy_from_g1(l.trim());
            if ok {
                prev_x = x;
                prev_y = y;
            }
        }
    }
    for l in lines.iter().skip(skip_leading) {
        let t = l.trim();
        if is_travel_line(t) || (t.starts_with("G1 ") && t.contains(" E")) {
            let (x, y, ok) = parse_xy_from_g1(t);
            if ok {
                prev_x = x;
                prev_y = y;
            }
            break;
        }
    }

    /*
     * 笔的**实际喷嘴 Z**。所有「要不要抬 / 要不要落」都从这一格推出来，
     * 不用布尔假设 —— 之前用「以为抬着」的标志，会在笔其实已经落到落笔高度时
     * 仍然跳过抬升，直接把那段空走放到贴面高度飞（实测 R2 从 24 反弹到 113）。
     */
    let land_z = glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed);
    let mut pen_z = start_z;
    let pen_is_up = |z: f64| z > land_z + 1e-6;

    /*
     * 段尾的多余空走：最后一条涂胶行之后的空走没有任何用处（收笔由 `;Glueing Finished`
     * 前的那次抬升负责），发出去只会多一段空驶 —— 作者 2026-11-04 截图里的 310078。
     */
    let last_glue = lines.iter().rposition(|l| {
        let t = l.trim();
        (t.starts_with("G1 ") || t.starts_with("G2 ") || t.starts_with("G3 ")) && t.contains(" E")
    });
    let mut emitted_move = skip_leading > 0;
    let mut idx = skip_leading;
    while idx < lines.len() {
        let il = lines[idx].trim().to_string();
        // 只丢「段尾的空走」，其余（`M204` 之类的状态行）照旧发出去
        if last_glue.is_some_and(|i| idx > i) && is_travel_line(&il) {
            idx += 1;
            continue;
        }
        if il.contains(MARKER_ZJUMP_START) {
            let mut next_start_idx: isize = -1;
            for ni in idx + 1..lines.len() {
                if lines[ni].contains("G1 X") || lines[ni].contains("G1 Y") {
                    next_start_idx = ni as isize;
                    break;
                }
            }
            if next_start_idx < 0 {
                idx += 1;
                continue;
            }
            // 空走没有实际位移（段首那句与回位重复的零长度移动）⇒ 不抬不落，
            // 原样交给下面的普通分支发出。上游多插一个标记也不会多一次抬落。
            if !travel_run_moves(lines, next_start_idx as usize, prev_x, prev_y) {
                idx = next_start_idx as usize;
                continue;
            }
            let (next, lx, ly, z_after) = emit_jump_run(
                sink,
                lines,
                next_start_idx as usize,
                ir_data,
                current_layer_height,
                last_layer_height,
                pen_is_up(pen_z),
            )?;
            prev_x = lx;
            prev_y = ly;
            if !z_after.is_nan() {
                pen_z = z_after;
            }
            idx = next;
            continue;
        }
        if il.contains(MARKER_ZJUMP_INTERNAL) {
            let feature_idx = il.find("; FEATURE:").or_else(|| il.find(";FEATURE:"));
            if let Some(fi) = feature_idx {
                sink.push_str(&format!(";Feature transition: {}", &il[fi..]));
            }
            let mut next_start_idx: isize = -1;
            for ni in idx + 1..lines.len() {
                if lines[ni].contains("G1 X") || lines[ni].contains("G1 Y") {
                    next_start_idx = ni as isize;
                    break;
                }
            }
            if next_start_idx < 0 {
                idx += 1;
                continue;
            }
            // 空走没有实际位移（段首那句与回位重复的零长度移动）⇒ 不抬不落，
            // 原样交给下面的普通分支发出。上游多插一个标记也不会多一次抬落。
            if !travel_run_moves(lines, next_start_idx as usize, prev_x, prev_y) {
                idx = next_start_idx as usize;
                continue;
            }
            let (next, lx, ly, z_after) = emit_jump_run(
                sink,
                lines,
                next_start_idx as usize,
                ir_data,
                current_layer_height,
                last_layer_height,
                pen_is_up(pen_z),
            )?;
            prev_x = lx;
            prev_y = ly;
            if !z_after.is_nan() {
                pen_z = z_after;
            }
            idx = next;
            continue;
        }
        if il.contains(MARKER_MICRO_LIFT) {
            let mut next_start_idx: isize = -1;
            for ni in idx + 1..lines.len() {
                if lines[ni].contains("G1 X") || lines[ni].contains("G1 Y") {
                    next_start_idx = ni as isize;
                    break;
                }
            }
            if next_start_idx < 0 {
                idx += 1;
                continue;
            }
            // 空走没有实际位移（段首那句与回位重复的零长度移动）⇒ 不抬不落，
            // 原样交给下面的普通分支发出。上游多插一个标记也不会多一次抬落。
            if !travel_run_moves(lines, next_start_idx as usize, prev_x, prev_y) {
                idx = next_start_idx as usize;
                continue;
            }
            let (next, lx, ly, z_after) = emit_jump_run(
                sink,
                lines,
                next_start_idx as usize,
                ir_data,
                current_layer_height,
                last_layer_height,
                pen_is_up(pen_z),
            )?;
            prev_x = lx;
            prev_y = ly;
            if !z_after.is_nan() {
                pen_z = z_after;
            }
            idx = next;
            continue;
        }
        // 段首那句与回位同点的定位行：只在「本段还没发出任何移动」时丢，
        // 0mm 空操作，发出去只会在预览里多一根竖线。
        if !emitted_move && is_travel_line(&il) {
            let (tx, ty, ok) = parse_xy_from_g1(&il);
            if ok && (tx - prev_x).abs() < 1e-9 && (ty - prev_y).abs() < 1e-9 {
                idx += 1;
                continue;
            }
        }
        if is_travel_line(&il) && travel_run_moves(lines, idx, prev_x, prev_y) {
            // 笔还在落笔状态而下一段是空走 —— 先抬起来再走（同标记分支的抬法）
            let (next, lx, ly, z_after) = emit_jump_run(
                sink,
                lines,
                idx,
                ir_data,
                current_layer_height,
                last_layer_height,
                pen_is_up(pen_z),
            )?;
            prev_x = lx;
            prev_y = ly;
            if !z_after.is_nan() {
                pen_z = z_after;
            }
            idx = next;
            continue;
        }
        // `G2/G3` 是圆弧挤出移动，必须和 `G1` 一样发出去（I/J 是相对量，偏移不影响）。
        let is_move_cmd = il.starts_with("G1 ") || il.starts_with("G2 ") || il.starts_with("G3 ");
        if is_move_cmd && !il.contains("G1 F") {
            emitted_move = true;
            // 抬着走过来、终于遇到涂胶行 ⇒ 先落再涂
            if pen_is_up(pen_z) && il.contains(" E") {
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed))
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
                pen_z = land_z;
            }
            let mut glue_line = il.clone();
            if glue_line.contains(" E") {
                let e_idx = glue_line.find(" E").unwrap();
                let after_e = &glue_line[e_idx + 2..];
                let mut end_of_e = 0;
                for (ci, ch) in after_e.bytes().enumerate() {
                    if ch == b' ' || ch == b';' {
                        end_of_e = ci;
                        break;
                    }
                }
                if end_of_e > 0 {
                    glue_line = format!("{}{}", &glue_line[..e_idx], &after_e[end_of_e..]);
                } else {
                    glue_line = glue_line[..e_idx].to_string();
                }
            }
            if glue_line.contains('X') || glue_line.contains('Y') {
                let (x, y, ok) = parse_xy_from_g1(&glue_line);
                if ok {
                    if glue_line.contains('X') {
                        prev_x = x;
                    }
                    if glue_line.contains('Y') {
                        prev_y = y;
                    }
                }
                let comp_z = calc_glue_z(0.0, prev_x, prev_y, ir_data, layer_bbox);
                let processed = crate::postproc::offset::process_gcode_offset(
                    &glue_line,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.z_offset + comp_z,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&processed);
            }
        } else if il.contains("M204") {
            sink.push_str(&il);
        }
        idx += 1;
    }
    Ok(())
}

/// `customMountGcodeHasRetract`（glue_block.go:471）：custom_mount_gcode 含负 E 指令。
pub(crate) fn custom_mount_gcode_has_retract(ir_data: &Ir) -> bool {
    ir_data
        .toolhead
        .custom_mount_gcode
        .iter()
        .any(|line| get_e_value(line).is_some_and(|v| v < 0.0))
}

/// `customUnmountGcodeHasExtrude`（glue_block.go:483；pass2 的 prepare-next-tower 消费）。
pub(crate) fn custom_unmount_gcode_has_extrude(ir_data: &Ir) -> bool {
    ir_data
        .toolhead
        .custom_unmount_gcode
        .iter()
        .any(|line| get_e_value(line).is_some_and(|v| v > 0.0))
}

/// 换笔相关长距离横移的目标 Z（笔伸出态）。
///
/// 笔尖 = 喷嘴 − `toolhead.z_offset`，所以"笔尖离层顶的净空"恒等于 `lift_height`，
/// 与笔伸多长无关；与 RiseNozzle / GlueEnd 同公式、同分支，避免出现两套基准。
/// `< 3.0` 的判据是**层顶高度**，不是层序号。
fn extended_travel_z(ir_data: &Ir, layer_z: f64) -> f64 {
    glue_nozzle_z(ir_data, layer_z, PenZ::Travel)
}

/// 一次换笔的回抽：抽多少（`amount`，负数）与抽多快（`speed_mm_per_s`）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct SwapRetract {
    pub amount: f64,
    pub speed_mm_per_s: f64,
}

impl SwapRetract {
    /// 独立一行 `G1 E{量} F{速度}`（面板是 mm/s，G-code 要 mm/min）。
    pub(crate) fn line(&self) -> String {
        format!(
            "G1 E{} F{}",
            format_e(self.amount),
            format_speed(self.speed_mm_per_s * super::MM_PER_MINUTE)
        )
    }
}

/// 回抽插在换笔的哪一刻。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RetractAt {
    /// 脱层小螺旋之前（模型处，起飞前）
    Model,
    /// 挂载块里第一条带 X 的移动之后（到达等待位、推入之前）
    Wait,
    /// 挂载块末尾（换笔动作之后）
    AfterDock,
}

/// 面板那一格 `MKP_retract_at` → 枚举。认不出的值由 `build` 在更早一步报错，
/// 直接吃 IR JSON 的那条路按 `model` 处理并留一条 warn（不静默换成别的档）。
pub(crate) fn retract_at(ir_data: &Ir) -> RetractAt {
    match ir_data.gcode.mkp_retract_at.as_str() {
        "wait" => RetractAt::Wait,
        "after_dock" => RetractAt::AfterDock,
        "model" | "" => RetractAt::Model,
        other => {
            tracing::warn!(value = other, "MKP_retract_at 认不出，按 model 处理");
            RetractAt::Model
        }
    }
}

/// 一次换笔该发多少回抽：`Some(..)` = 内核发这一笔，`None` = 不发（模板自带，或量填 0）。
///
/// **整段 swap 只有一笔回抽。** 两个发起 swap 的地方各发一次：涂胶块与首笔活化，
/// 插在哪由 [`retract_at`] 决定。`mount_toolhead` 只在 `wait` / `after_dock` 两档动手 ——
/// 谁在里面无条件补一笔，就等于每次换笔抽两遍（0.0.11 就是这么把 E 抽成了两笔）。
pub(crate) fn swap_retract(ir_data: &Ir) -> Option<SwapRetract> {
    if custom_mount_gcode_has_retract(ir_data) || ir_data.gcode.mkp_retract == 0.0 {
        return None;
    }
    Some(SwapRetract {
        amount: ir_data.gcode.mkp_retract,
        speed_mm_per_s: ir_data.gcode.mkp_retract_speed,
    })
}

/// 这一行是不是「带 X 的 G1 移动」（判断是否已经到等待位）。
fn is_x_move(line: &str) -> bool {
    let code = line.split(';').next().unwrap_or("").trim();
    code.starts_with("G1") && code.contains('X')
}

/// `mountToolhead`（glue_block.go:376）：custom_mount_gcode 切换下笔。
#[allow(clippy::too_many_arguments)]
pub(crate) fn mount_toolhead(
    sink: &mut LineSink,
    ir_data: &Ir,
    current_layer_height: f64,
    skip_final_z: bool,
    retract: Option<SwapRetract>,
    at: RetractAt,
) {
    sink.push_str(";Mounting Toolhead");
    sink.push_str(&stage_comment_line(MkpStage::MountToolhead, ""));

    /* `model` 那一档由调用方在起飞前发过，这里只管另外两档：
     * `wait` 插在「到达等待位」那一行之后（推入之前），`after_dock` 落在块尾。 */
    let mut pending = if at == RetractAt::Model {
        None
    } else {
        retract
    };
    let mut injected = false;

    for mount_line in &ir_data.toolhead.custom_mount_gcode {
        let ml = mount_line.trim();
        if ml.is_empty() {
            continue;
        }
        if ml.contains("L801") {
            // 与平飞/飞回同一个高度：装笔是机构动作，垂直位置不影响能否装上，
            // 让它跟着 `glue_z_lift_height` 走，整段只有一个可调参数。
            let l801_z = extended_travel_z(ir_data, current_layer_height);
            sink.push_str(&format!("G1 Z{};L801", format_z(l801_z)));
        } else if !ir_data.gcode.custom_gcode_e_ownership && has_e_param(ml) {
            sink.push_str(&replace_e_value(ml, ir_data.gcode.mkp_retract));
        } else {
            sink.push_str(ml);
        }
        /* 「到达等待位」= 挂载块里第一条带 X 移动的 G1。模板里已经没有 E，
         * 也不许再塞占位符，所以只能按这个形状认。 */
        if at == RetractAt::Wait && !injected && is_x_move(ml) {
            if let Some(r) = pending.take() {
                sink.push_str(&r.line());
            }
            injected = true;
        }
    }
    if let Some(r) = pending.take() {
        if at == RetractAt::Wait {
            tracing::warn!(
                "挂载模板里找不到带 X 的移动，回抽落到换笔之后（本该插在到达等待位那一刻）"
            );
        }
        sink.push_str(&r.line());
    }
    sink.push_str(";Toolhead Mounted");
    if !skip_final_z {
        // 装好笔后的飞回段用 `current`（原来误用 `last`，凭空少 0.2），
        // 并且抬到与收笔同口径的净空，笔尖余量 2.8 → lift_height。
        sink.push_str(&format!(
            "G1 Z{}",
            format_z(extended_travel_z(ir_data, current_layer_height))
        ));
    }
}

/// `unmountToolhead`（glue_block.go:413）：;Wipe/;Brush 跳过 + [AUTO] 风扇替换。
pub(crate) fn unmount_toolhead(sink: &mut LineSink, ir_data: &Ir) {
    sink.push_str(";Unmounting Toolhead");
    for unmount_line in &ir_data.toolhead.custom_unmount_gcode {
        let ul = unmount_line.trim();
        if ul.is_empty() {
            continue;
        }
        if ul.contains(";Wipe") {
            // 跳过
        } else if ul.contains(";Brush") {
            // 跳过
        } else if ul.contains("M106 S[AUTO]") {
            sink.push_str(&format!(
                "M106 S{}",
                format_python_float(ir_data.wiping.fan_speed)
            ));
        } else if ul.contains("M106 P1 S[AUTO]") {
            sink.push_str(&format!(
                "M106 P1 S{}",
                format_python_float(ir_data.wiping.fan_speed)
            ));
        } else if !ir_data.gcode.custom_gcode_e_ownership && has_e_param(ul) {
            sink.push_str(&replace_e_value(ul, ir_data.gcode.mkp_extrude));
        } else {
            sink.push_str(ul);
        }
    }
    sink.push_str(";Toolhead Unmounted");
}

/// `restoreFanState`（glue_block.go:455）：恢复 MKP 插入前的原始风扇状态。
pub(crate) fn restore_fan_state(sink: &mut LineSink, ir_data: &Ir, machine_type: &str) {
    if !ir_data.state.current_fan_speed_set {
        return;
    }
    sink.push_str(";Restore fan state");
    let fan_speed = format_python_float(ir_data.state.current_fan_speed);
    if crate::postproc::machine_dims::get_machine_dimensions(machine_type)
        .flags
        .has_second_fan
    {
        sink.push_str(&format!("M106 P1 S{fan_speed}"));
    } else {
        sink.push_str(&format!("M106 S{fan_speed}"));
    }
}

/// `emitGlueBlock`（glue_block.go:27）：生成完整涂胶块（换笔→涂胶→擦料→准备下一塔）。
///
/// 副作用：修改 ir_data.state.this_layer_revitalization 与 stats 计数字段；
/// write_glue_lines 可能修改传入的 iface（稀疏化），调用方按 Go 语义传拷贝。
#[allow(clippy::too_many_arguments)]
pub(crate) fn emit_glue_block(
    sink: &mut LineSink,
    iface: &[String],
    ir_data: &mut Ir,
    current_layer_height: f64,
    last_layer_height: f64,
    machine_type: &str,
    _layer_feature_map: &HashMap<ZKey, bool>,
    stats: &mut super::ProcessStats,
) -> Result<(), PostprocError> {
    // MKP 标记协议 v1：本块的事件序号。glue_cutter_changes 在下面 +1，这里先读，
    // 三个区间共享同一个值。最终 ev 会在 marks::finalize 里按落盘行序重编号。
    let ev = stats.glue_cutter_changes + 1;

    sink.push_str(";Pre-glue preparation");
    // swap#1（取笔）。END 必须在 ;Toolhead Mounted 附近闭合：pass2 的
    // skipping_mount_sequence 会丢弃这一整段，标记靠 marks::is_mark_line 穿透。
    sink.push_str(&marks::swap_begin(ev, "pen", current_layer_height));

    if ir_data.filament.filament_type.to_uppercase() != "ABS" {
        let fan_speed = format_python_float(ir_data.wiping.fan_speed);
        if crate::postproc::machine_dims::get_machine_dimensions(machine_type)
            .flags
            .has_second_fan
        {
            sink.push_str(&format!("M106 P1 S{fan_speed}"));
        } else {
            sink.push_str(&format!("M106 S{fan_speed}"));
        }
    }

    let wipe_x = crate::postproc::machine_dims::get_machine_dimensions(machine_type)
        .glue_area
        .wipe_x;
    if ir_data.wiping.nozzle_cooling {
        sink.push_str(";Pervent Leakage");
        sink.push_str(&format!(
            "M104 S{:.0}",
            ir_data.toolhead.nozzle_switch_temperature - 30.0
        ));
    }

    // 1) 小螺旋只做 z-hop：抬到「层顶 + swap_hop_height」脱离层顶（默认 1.0，面板可调）。
    //    尺度与切片器自己的 `G3 Z.8 I.706 J.991 P1` 一致（只脱层 0.2~1.0）；
    //    I/J 分量照抄切片器（`sqrt(.706² + .991²) = 1.217`），不用 `I{半径} J0` 那种写法。
    sink.push_str("M204 S10000");
    sink.push_str("G17");
    /* 回抽单独一行，位置由 `MKP_retract_at` 决定（默认就是这里：起飞前）。
     * 小螺旋只做脱层 —— 它那句的 F 是空走速度（700 mm/s），带着 E 就等于按空走速度抽。 */
    let retract = swap_retract(ir_data);
    let at = retract_at(ir_data);
    if at == RetractAt::Model {
        if let Some(r) = retract {
            sink.push_str(&r.line());
        }
    }
    sink.push_str(&format!(
        "G3 Z{} I.706 J.991 P1 F{}",
        format_z(current_layer_height + ir_data.glue.swap_hop_height),
        format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
    ));

    sink.push_str(crate::gcode::MARKER_RISING_NOZZLE);
    sink.push_str(&stage_comment_line(MkpStage::RiseNozzle, ""));
    // 2) 脱层之后**先**原位抬到净空高度（`层顶 + offset_z + lift`，也就是
    //    `glue_z_lift_height` —— 面板上那一个值说了算），
    //    3) **再**平飞到切笔位 —— 平飞 / 装笔 / 飞回同一个高度，全程不升降往返。
    //    只抬 1mm 就平飞会贴着同层的其它模型走，这里必须抬到位再走。
    sink.push_str(&format!(
        "G1 Z{}",
        format_z(extended_travel_z(ir_data, current_layer_height))
    ));
    sink.push_str(&format!(
        "G1 X{} F{}",
        format_float(wipe_x),
        format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
    ));

    mount_toolhead(sink, ir_data, current_layer_height, false, retract, at);
    sink.push_str(&marks::end("swap"));

    sink.push_str(";Glueing Started");
    sink.push_str(&stage_comment_line(MkpStage::GlueStart, ""));
    sink.push_str(&marks::paint_begin(ev, current_layer_height));
    let segments = split_iface_by_model(iface);

    stats.glue_cutter_changes += 1;
    stats.glue_layer_count += 1;
    for seg in &segments {
        stats.glue_total_length += total_path_length(seg) * ir_data.glue.pass_count as f64;
    }

    for (seg_idx, seg) in segments.iter().enumerate() {
        // Go 侧此处先累计 segESum，但该值从未被消费（死存储），未移植。

        /*
         * 整段没有一条挤出移动 ⇒ 它只是段尾剩下的空走（切段时被 `;ZJUMP_START` 分出来的）。
         * 发出去就是「涂完了还空驶一段」（作者 2026-11-04 截图 310078）。
         */
        if !seg.iter().any(|l| l.contains(" E")) {
            continue;
        }

        // 段首若是「先飞到特征前、再飞到起点」的两段空走，回位直接取后者（见 leading_travel_run）
        let (seg_start_cmd, skip_leading) = match leading_travel_run(seg) {
            Some((cmd, n)) => (cmd, n),
            None => (find_seg_start_cmd(seg), 0),
        };

        // 回位结束时笔**实际**在哪一格（喷嘴 Z）。发射层据此判断要不要再抬。
        let land_z = glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed);
        let mut start_z = land_z;
        if !seg_start_cmd.is_empty() {
            let repos_cmd = strip_z_and_e(&seg_start_cmd);
            /*
             * 段首那一句如果是**空走**，说明这一段的第一个动作是「跳到某处」而不是
             * 「就在起点涂」。此时回位不该先落一次笔：落了又立刻为同一次跳转抬起，
             * 就是成品里「点一下又抬」的来源（一段一处，实测 135 处）。
             * 保持抬着交给 `write_glue_lines`，由它在真正的涂胶行之前落。
             */
            let starts_with_jump = is_travel_line(seg_start_cmd.trim());
            if seg_idx == 0 {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                if starts_with_jump {
                    start_z = glue_nozzle_z(ir_data, current_layer_height, PenZ::Travel);
                } else {
                    sink.push_str(&format!(
                        "G1 Z{}",
                        format_z(glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed))
                    ));
                    sink.push_str(&format!(
                        "G1 F{}",
                        format_speed(
                            ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor
                        )
                    ));
                }
            } else if ir_data.glue.inter_model_lift_height > 0.0 {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(glue_nozzle_z(ir_data, current_layer_height, PenZ::Hop))
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.inter_model_lift_height,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                if starts_with_jump {
                    start_z = glue_nozzle_z(ir_data, current_layer_height, PenZ::Hop);
                } else {
                    sink.push_str(&format!(
                        "G1 Z{}",
                        format_z(glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed))
                    ));
                    sink.push_str(&format!(
                        "G1 F{}",
                        format_speed(
                            ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor
                        )
                    ));
                }
            } else {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            }
        }

        if ir_data.prime.enabled
            && ir_data.state.this_layer_revitalization
            && (ir_data.prime.min_path_length == 0.0
                || total_path_length(seg) >= ir_data.prime.min_path_length)
        {
            sink.push_str(";Gluepen Revitalization Start");
            sink.push_str(&stage_comment_line(MkpStage::RevitalizationStart, ""));
            let revitalization_speed =
                ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor;
            sink.push_str(&format!("G1 F{}", format_speed(revitalization_speed)));
            let mut target_length = ir_data.prime.length;
            if target_length < 20.0 {
                target_length = 100.0;
            }
            let prime_iface = seg;
            let mut accumulated_dist = 0.0;
            let mut last_rx = f64::NAN;
            let mut last_ry = f64::NAN;
            for iface_line in prime_iface {
                let il = iface_line.trim();
                if il.contains(crate::gcode::MARKER_ZJUMP_START) {
                    break;
                }
                if !il.starts_with("G1 ") || il.contains("G1 F") {
                    continue;
                }
                let mut revit_line = il.to_string();
                if revit_line.contains(" E") {
                    let e_idx = revit_line.find(" E").unwrap();
                    let after_e = &revit_line[e_idx + 2..];
                    let mut end_of_e = 0;
                    for (ci, ch) in after_e.bytes().enumerate() {
                        if ch == b' ' || ch == b';' {
                            end_of_e = ci;
                            break;
                        }
                    }
                    if end_of_e > 0 {
                        revit_line = format!("{}{}", &revit_line[..e_idx], &after_e[end_of_e..]);
                    } else {
                        revit_line = revit_line[..e_idx].to_string();
                    }
                }
                if !revit_line.contains('X') && !revit_line.contains('Y') {
                    continue;
                }
                let mut rx = 0.0;
                let mut ry = 0.0;
                let mut has_x = false;
                let mut has_y = false;
                if let Some(x_idx) = revit_line.find('X') {
                    let mut x_end = x_idx + 1;
                    let rb = revit_line.as_bytes();
                    while x_end < rb.len() && rb[x_end] != b' ' && rb[x_end] != b';' {
                        x_end += 1;
                    }
                    let mut x_part = revit_line[x_idx..x_end].to_string();
                    if x_part.len() > 1 && x_part.as_bytes()[1] == b'.' {
                        x_part = format!("{}0{}", &x_part[..1], &x_part[1..]);
                    }
                    let x_nums = num_strip(&x_part);
                    if let Some(&n) = x_nums.first() {
                        rx = n;
                        has_x = true;
                    }
                }
                if let Some(y_idx) = revit_line.find('Y') {
                    let mut y_end = y_idx + 1;
                    let rb = revit_line.as_bytes();
                    while y_end < rb.len() && rb[y_end] != b' ' && rb[y_end] != b';' {
                        y_end += 1;
                    }
                    let mut y_part = revit_line[y_idx..y_end].to_string();
                    if y_part.len() > 1 && y_part.as_bytes()[1] == b'.' {
                        y_part = format!("{}0{}", &y_part[..1], &y_part[1..]);
                    }
                    let y_nums = num_strip(&y_part);
                    if let Some(&n) = y_nums.first() {
                        ry = n;
                        has_y = true;
                    }
                }
                if has_x && has_y {
                    if !last_rx.is_nan() && !last_ry.is_nan() {
                        let dx = rx - last_rx;
                        let dy = ry - last_ry;
                        accumulated_dist += (dx * dx + dy * dy).sqrt();
                    }
                    last_rx = rx;
                    last_ry = ry;
                    if accumulated_dist >= target_length {
                        break;
                    }
                    let processed = crate::postproc::offset::process_gcode_offset(
                        &revit_line,
                        ir_data.toolhead.x_offset,
                        ir_data.toolhead.y_offset,
                        ir_data.toolhead.z_offset + ir_data.glue.z_offset,
                        crate::gcode::OffsetMode::Normal,
                        ir_data,
                    )?;
                    sink.push_str(&processed);
                }
            }
            sink.push_str(";Gluepen Revitalization End");
            sink.push_str(&stage_comment_line(MkpStage::RevitalizationEnd, ""));
            stats.prime_total_length += accumulated_dist;
            if !ir_data.prime.per_model {
                ir_data.state.this_layer_revitalization = false;
            }
            let revit_repos_cmd = strip_z_and_e(&seg_start_cmd);
            if ir_data.glue.inter_model_lift_height > 0.0 {
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(glue_nozzle_z(ir_data, current_layer_height, PenZ::Hop))
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &revit_repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.inter_model_lift_height,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed))
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            } else {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &revit_repos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            }
        }

        let mut seg_owned = seg.clone();
        write_glue_lines(
            sink,
            &mut seg_owned,
            ir_data,
            current_layer_height,
            last_layer_height,
            start_z,
            skip_leading,
        )?;

        for pass_idx in 1..ir_data.glue.pass_count {
            sink.push_str(&format!(";Glue Pass {}", pass_idx + 1));
            sink.push_str(&stage_comment_line(
                MkpStage::GluePass,
                &format!("{}", pass_idx + 1),
            ));
            let inpos_cmd = strip_z_and_e(&seg_start_cmd);
            if ir_data.glue.inter_model_lift_height > 0.0 {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(glue_nozzle_z(ir_data, current_layer_height, PenZ::Hop))
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &inpos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset + ir_data.glue.inter_model_lift_height,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 Z{}",
                    format_z(glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed))
                ));
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            } else {
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
                ));
                let offset_result = crate::postproc::offset::process_gcode_offset(
                    &inpos_cmd,
                    ir_data.toolhead.x_offset,
                    ir_data.toolhead.y_offset,
                    ir_data.toolhead.z_offset,
                    crate::gcode::OffsetMode::Normal,
                    ir_data,
                )?;
                sink.push_str(&offset_result);
                sink.push_str(&format!(
                    "G1 F{}",
                    format_speed(ir_data.toolhead.max_speed * ir_data.wiping.small_feature_factor)
                ));
            }
            // 多遍涂胶：每遍都从落笔开始（Q7：第二阶段之前不改这个语义）
            write_glue_lines(
                sink,
                &mut seg_owned,
                ir_data,
                current_layer_height,
                last_layer_height,
                glue_nozzle_z(ir_data, last_layer_height, PenZ::Landed),
                0,
            )?;
        }

        if !ir_data.prime.per_model {
            ir_data.state.this_layer_revitalization = false;
        }
    }

    sink.push_str(";Glueing Finished");
    sink.push_str(&stage_comment_line(MkpStage::GlueEnd, ""));
    sink.push_str(&format!(
        "G1 F{}",
        format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
    ));
    let mut glue_finished_z =
        current_layer_height + ir_data.toolhead.z_offset + ir_data.glue.z_lift_height;
    if current_layer_height < 3.0 {
        glue_finished_z = current_layer_height
            + ir_data.toolhead.z_offset
            + ir_data.glue.z_lift_height_first_layers;
    }
    sink.push_str(&format!("G1 Z{}", format_z(glue_finished_z)));
    sink.push_str(&format!(";Lift-z:{}", format_z(glue_finished_z)));
    sink.push_str(&marks::end("paint"));

    // swap#2（放笔）。END 必须在 ;Prepare for next tower 之前 —— pass2 会在那之后
    // 灌入延迟的圆盘擦拭段（;DISK_WIPE_START/END 是别名区间），否则会嵌套。
    sink.push_str(&marks::swap_begin(ev, "print", current_layer_height));
    unmount_toolhead(sink, ir_data);
    restore_fan_state(sink, ir_data, machine_type);
    sink.push_str(&marks::end("swap"));

    if ir_data.tower.use_towers {
        let tower_bbox = crate::postproc::tower::calculate_tower_bbox(
            &ir_data.wiping.outer_structure,
            ir_data.sheath.base_expand,
            ir_data.rib.extra_length,
            ir_data.rib.width,
            stats.max_z_height,
            ir_data.machine.nozzle_diameter,
            ir_data.machine.first_layer_height,
        );
        let offset_result = crate::postproc::offset::process_gcode_offset(
            "G1 X20 Y10.19",
            ir_data.wiping.wiper_x - tower_bbox.min_x,
            ir_data.wiping.wiper_y - tower_bbox.min_y,
            current_layer_height + 3.0,
            crate::gcode::OffsetMode::Normal,
            ir_data,
        )?;
        sink.push_str(&offset_result);
        sink.push_str(";Prepare for next tower");
        if ir_data.wiping.nozzle_cooling {
            if ir_data.wiping.user_dry_time != 0 {
                sink.push_str(&format!(
                    "M104 S{:.0}",
                    ir_data.toolhead.nozzle_switch_temperature
                ));
            } else {
                sink.push_str(&format!(
                    "M109 S{:.0}",
                    ir_data.toolhead.nozzle_switch_temperature
                ));
            }
        }
    } else {
        sink.push_str(";Prepare for next tower");
        if !ir_data.gcode.refill_extrude_command.is_empty() {
            let mut cmd = ir_data.gcode.refill_extrude_command.clone();
            if !ir_data.gcode.custom_gcode_e_ownership {
                cmd = replace_e_value(&cmd, ir_data.gcode.mkp_extrude);
            }
            sink.push_str(&cmd);
            sink.push_str(&format!(
                "G1 F{}",
                format_speed(ir_data.machine.travel_speed * super::MM_PER_MINUTE)
            ));
        }
    }

    Ok(())
}

/// 涂胶路径连续性回归（2026-11-03）。
///
/// 病灶：切片器在两段涂胶之间写的是「空走1（带 Z）+ 空走2（带 Z）+ 一条微小
/// 涂胶行（离空走2 终点 0.15~0.4mm）」。`delete_wipe` 只保留 `WIPE_END` 之后
/// 第一条空走，第二条被静默丢弃；发射层又只把标记后**紧邻的一条**交给抬升。
/// 于是涂胶行从空走1 的终点直飞自己的终点 —— 这段位移在落笔高度完成，笔尖
/// 压着模型表面走（真实导出件里最长 65mm，打印时撞件）。
///
/// 判据不看距离：只要求「两条空走都在抬升高度上走完，之后才落笔」。
#[cfg(test)]
mod continuity_tests {
    use super::*;
    use crate::postproc::pass1::delete_wipe::{delete_wipe, delete_wipe_continuous};

    const LAYER_Z: f64 = 40.84;
    const LAST_LAYER_Z: f64 = 40.68;

    fn test_ir() -> Ir {
        let mut ir = Ir::default();
        ir.machine.nozzle_diameter = 0.4;
        ir.machine.travel_speed = 700.0;
        ir.machine.max_x = 256.0;
        ir.machine.max_y = 256.0;
        ir.machine.glue_max_x = 256.0;
        ir.machine.glue_max_y = 256.0;
        ir.machine.min_support_interface_enabled = true;
        ir.toolhead.x_offset = -0.7;
        ir.toolhead.y_offset = 24.4;
        ir.toolhead.z_offset = 3.95;
        ir.toolhead.glue_z_offset = -0.05;
        ir.glue.z_offset = -0.05;
        ir.glue.z_lift_height = 5.0;
        ir.glue.z_lift_height_first_layers = 6.0;
        ir.glue.inter_model_lift_height = 2.0;
        ir.machine.support_interface_speed = 70.0;
        ir.machine.wall_print_speed = 200.0;
        ir
    }

    /// 真实切片器（BambuStudio 02.08）在这类边界上的原样写法。
    fn raw_iface() -> Vec<String> {
        [
            "; WIPE_END",
            "G1 E-.04 F1800",
            "M204 S10000",
            "G1 X173.56 Y126.782 Z41.08 F42000",
            "G1 X108.902 Y124.207 Z41.08",
            "G1 Z40.68",
            "G1 E.8 F1800",
            "G1 X108.532 Y124.075 E.00154",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect()
    }

    fn hop_z() -> f64 {
        LAYER_Z + 3.95 + 2.0
    }

    fn land_z() -> f64 {
        LAST_LAYER_Z + 3.95 + -0.05
    }

    fn emit(iface: &[String]) -> Vec<String> {
        let mut lines = iface.to_vec();
        let mut sink = LineSink { lines: Vec::new() };
        let ir = test_ir();
        write_glue_lines(
            &mut sink,
            &mut lines,
            &ir,
            LAYER_Z,
            LAST_LAYER_Z,
            glue_nozzle_z(&ir, LAST_LAYER_Z, PenZ::Landed),
            0,
        )
        .expect("write_glue_lines 失败");
        sink.lines
    }

    fn z_moves(out: &[String]) -> Vec<f64> {
        out.iter()
            .filter_map(|l| {
                let (_, _, z) = parse_xy_from_g1(l.trim());
                let _ = z;
                if l.trim().starts_with("G1 Z") || l.trim().contains(" Z") {
                    l.split_whitespace()
                        .find_map(|t| t.strip_prefix('Z'))
                        .and_then(|v| v.parse::<f64>().ok())
                } else {
                    None
                }
            })
            .collect()
    }

    fn has_xy(out: &[String], x: f64, y: f64) -> bool {
        out.iter().any(|l| {
            let line = l.trim();
            line.contains(&format!("X{x}")) && line.contains(&format!("Y{y}"))
        })
    }

    /// 抬升 / 落笔各一次，且两条空走都在抬升高度上。
    fn assert_single_lift_and_both_travels(iface: &[String]) {
        let out = emit(iface);

        /*
         * 段首那句「与回位同点」的空走会被去重丢掉（0mm，发出去也是白走），
         * 所以这里不再要求它出现；要保证的是**真正移动的那一段**在，且只抬落一次。
         */
        assert!(
            has_xy(&out, 108.202, 148.607),
            "空走2 被吞掉了 —— 涂胶行会从空走1 的终点直飞，落笔高度横移 \
             （输出：\n{}\n）",
            out.join("\n")
        );

        let zs = z_moves(&out);
        let ups = zs.iter().filter(|z| (**z - hop_z()).abs() < 1e-6).count();
        let downs = zs.iter().filter(|z| (**z - land_z()).abs() < 1e-6).count();
        assert_eq!(
            ups,
            1,
            "抬升应恰好 1 次，实际 Z 序列 {zs:?}（hop={}）",
            hop_z()
        );
        assert_eq!(
            downs,
            1,
            "落笔应恰好 1 次，实际 Z 序列 {zs:?}（land={}）",
            land_z()
        );

        // 两条空走之间不许落笔：落笔只能发生在最后一条涂胶行之前
        let land_at = out
            .iter()
            .position(|l| {
                l.split_whitespace()
                    .find_map(|t| t.strip_prefix('Z'))
                    .and_then(|v| v.parse::<f64>().ok())
                    .is_some_and(|z| (z - land_z()).abs() < 1e-6)
            })
            .expect("找不到落笔指令");
        let second_travel = out
            .iter()
            .position(|l| l.contains("108.202"))
            .expect("找不到空走2");
        assert!(
            second_travel < land_at,
            "落笔早于空走2 —— 空走2 是在落笔高度走的，输出：\n{}",
            out.join("\n")
        );
    }

    /// 段首那句与回位重复的零长度空走，不许再抬一次（成品里 382 处「落→抬→落」的来源）。
    #[test]
    fn zero_length_travel_after_marker_never_lifts() {
        let iface: Vec<String> = [
            ";ZJUMP_START",
            "G1 X173.56 Y126.782",
            "G1 X173.56 Y126.782",
            "G1 X173.1 Y126.4 E.00154",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = emit(&iface);
        let zs = z_moves(&out);
        assert!(
            zs.is_empty(),
            "零长度空走不该产生任何 Z 动作，实际 {zs:?}，输出：\n{}",
            out.join("\n")
        );
        assert!(
            out.iter().any(|l| l.contains("172.4")),
            "涂胶行丢了，输出：\n{}",
            out.join("\n")
        );
    }

    /// 两个标记背靠背（段首重复一次）时，只许为真正的那次跳转抬落一次。
    #[test]
    fn duplicated_marker_lifts_once() {
        let iface: Vec<String> = [
            ";ZJUMP_START",
            "G1 X173.56 Y126.782",
            ";ZJUMP_START",
            "G1 X173.56 Y126.782",
            "G1 X108.902 Y124.207",
            "G1 X108.532 Y124.075 E.00154",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = emit(&iface);
        let zs = z_moves(&out);
        assert_eq!(
            zs.iter().filter(|z| (**z - hop_z()).abs() < 1e-6).count(),
            1,
            "抬升应恰好 1 次，实际 {zs:?}，输出：\n{}",
            out.join("\n")
        );
        assert_eq!(
            zs.iter().filter(|z| (**z - land_z()).abs() < 1e-6).count(),
            1,
            "落笔应恰好 1 次，实际 {zs:?}，输出：\n{}",
            out.join("\n")
        );
    }

    #[test]
    fn two_consecutive_travels_lift_once() {
        assert_single_lift_and_both_travels(&delete_wipe(&raw_iface()));
    }

    #[test]
    fn two_consecutive_travels_lift_once_continuous() {
        assert_single_lift_and_both_travels(&delete_wipe_continuous(&raw_iface()));
    }

    /// 只有一条空走时，输出必须与修复前逐字相同（防止改动波及正常涂胶）。
    /// 段中段的真实空走（不靠标记）：落笔态下仍必须先抬后走。
    ///
    /// 旧的 `single_travel_keeps_previous_behaviour` 要求「只有一条空走也抬一次」，
    /// 但那一条其实是**段首与回位重复的零长度移动**——抬起来走 0mm 再落下，
    /// 正是成品里 382 处「落→抬→落」的形状。段首那条不抬，中段的真实位移照抬。
    #[test]
    fn mid_segment_travel_still_lifts_once() {
        let iface: Vec<String> = [
            "G1 X173.56 Y126.782",
            "G1 X173.1 Y126.4 E.00154",
            "G1 X150.0 Y120.0",
            "G1 X149.5 Y119.5 E.00154",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        let out = emit(&iface);
        let zs = z_moves(&out);
        assert_eq!(
            zs.iter().filter(|z| (**z - hop_z()).abs() < 1e-6).count(),
            1,
            "段中段空走应抬一次，实际 {zs:?}，输出：\n{}",
            out.join("\n")
        );
        assert_eq!(
            zs.iter().filter(|z| (**z - land_z()).abs() < 1e-6).count(),
            1,
            "段中段空走应落一次，实际 {zs:?}，输出：\n{}",
            out.join("\n")
        );
    }

    /// 行里的负 E（回抽）。
    fn negative_e(line: &str) -> Option<f64> {
        line.split_whitespace()
            .find_map(|t| t.strip_prefix('E'))
            .and_then(|v| v.parse::<f64>().ok())
            .filter(|v| *v < 0.0)
    }

    /// 回抽归谁发：模板带 E 就归模板（内核一条都不发），量填 0 也不发。
    #[test]
    fn swap_retract_follows_template_ownership() {
        let mut ir = test_ir();
        ir.gcode.mkp_retract = -5.0;
        ir.gcode.mkp_retract_speed = 18.0;
        ir.toolhead.custom_mount_gcode = ["G92 E0", "L801"].iter().map(|s| s.to_string()).collect();
        let r = swap_retract(&ir).expect("模板没有 E 时该由内核发");
        assert_eq!(r.amount, -5.0);
        assert_eq!(r.speed_mm_per_s, 18.0);
        assert_eq!(r.line(), "G1 E-5.0 F1080.0");

        ir.toolhead.custom_mount_gcode = ["G92 E0", "G1 E-5 F1800", "L801"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        assert!(
            swap_retract(&ir).is_none(),
            "模板自带 E 时内核不许再发第二笔"
        );

        ir.toolhead.custom_mount_gcode = ["G92 E0", "L801"].iter().map(|s| s.to_string()).collect();
        ir.gcode.mkp_retract = 0.0;
        assert!(swap_retract(&ir).is_none(), "回抽填 0 就一行都不发");
    }

    /// 默认那一档：一笔 `G1 E..` 落在小螺旋之前，挂载段里干净。
    ///
    /// 0.0.11 在 `mount_toolhead` 末尾又补了一笔，小螺旋那笔没让位 —— 每次换笔抽 −10，
    /// 收笔只补 +5.5，净账每层 −4.5mm。模板骨架里不含 E 时，这条判据必须恰好是 1 笔。
    #[test]
    fn swap_block_retracts_exactly_once() {
        let out = emit_swap_block(RetractAt::Model);
        let retracts: Vec<&String> = out.iter().filter(|l| negative_e(l).is_some()).collect();
        assert_eq!(
            retracts.len(),
            1,
            "换笔回抽应恰好 1 笔，实际 {} 笔：{retracts:?}\n输出：\n{}",
            retracts.len(),
            out.join("\n")
        );
        assert_eq!(
            retracts[0], "G1 E-5.0 F1500.0",
            "回抽应是一句独立移动（速度 25 mm/s = F1500）"
        );

        let hop_at = out
            .iter()
            .position(|l| l.starts_with("G3 "))
            .expect("没有脱层小螺旋");
        assert!(
            !out[hop_at].contains(" E"),
            "小螺旋只做脱层：{}",
            out[hop_at]
        );
        let mount_at = out
            .iter()
            .position(|l| l.contains(";Mounting Toolhead"))
            .expect("没有 ;Mounting Toolhead");
        let mounted_at = out
            .iter()
            .position(|l| l.contains(";Toolhead Mounted"))
            .expect("没有 ;Toolhead Mounted");
        assert!(
            out[mount_at..mounted_at]
                .iter()
                .all(|l| negative_e(l).is_none()),
            "挂载段里不许再有第二笔回抽：\n{}",
            out[mount_at..mounted_at].join("\n")
        );

        fn emit_swap_block(at: RetractAt) -> Vec<String> {
            let mut ir = swap_ir();
            ir.gcode.mkp_retract_at = match at {
                RetractAt::Model => "model",
                RetractAt::Wait => "wait",
                RetractAt::AfterDock => "after_dock",
            }
            .to_string();
            let iface: Vec<String> = ["G1 X108.532 Y124.075 E.00154", "G1 X108.9 Y124.2 E.00154"]
                .iter()
                .map(|s| s.to_string())
                .collect();
            let mut sink = LineSink { lines: Vec::new() };
            let mut stats = crate::postproc::pass1::ProcessStats::default();
            let features = HashMap::new();
            emit_glue_block(
                &mut sink,
                &iface,
                &mut ir,
                LAYER_Z,
                LAST_LAYER_Z,
                "A1",
                &features,
                &mut stats,
            )
            .expect("emit_glue_block 失败");
            sink.lines
        }
    }

    /// `wait`：回抽插在「到达等待位」那一行之后、推入之前。
    #[test]
    fn retract_at_wait_sits_between_wait_and_dock() {
        let out = emit_with_position("wait");
        let retract_at = out
            .iter()
            .position(|l| negative_e(l).is_some())
            .expect("没有回抽");
        assert_eq!(out[retract_at], "G1 E-5.0 F1500.0");
        assert!(
            out[retract_at - 1].starts_with("G1 X168"),
            "回抽前一句该是到达等待位，实际：{}",
            out[retract_at - 1]
        );
        assert!(
            out[retract_at + 1].starts_with("G1 X182"),
            "回抽后一句该是推入动作位，实际：{}",
            out[retract_at + 1]
        );
        assert_eq!(
            out.iter().filter(|l| negative_e(l).is_some()).count(),
            1,
            "仍然只许一笔"
        );
    }

    /// `after_dock`：回抽落在挂载块末尾（`;Toolhead Mounted` 之前）。
    #[test]
    fn retract_after_dock_sits_at_the_end_of_the_mount_block() {
        let out = emit_with_position("after_dock");
        let retract_at = out
            .iter()
            .position(|l| negative_e(l).is_some())
            .expect("没有回抽");
        assert_eq!(out[retract_at + 1].trim(), ";Toolhead Mounted");
        assert!(
            out[retract_at - 1].starts_with("G1 F30000"),
            "回抽前一句该是挂载块的最后一句，实际：{}",
            out[retract_at - 1]
        );
    }

    fn emit_with_position(position: &str) -> Vec<String> {
        let mut ir = swap_ir();
        ir.gcode.mkp_retract_at = position.to_string();
        let iface: Vec<String> = ["G1 X108.532 Y124.075 E.00154", "G1 X108.9 Y124.2 E.00154"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let mut sink = LineSink { lines: Vec::new() };
        let mut stats = crate::postproc::pass1::ProcessStats::default();
        let features = HashMap::new();
        emit_glue_block(
            &mut sink,
            &iface,
            &mut ir,
            LAYER_Z,
            LAST_LAYER_Z,
            "A1",
            &features,
            &mut stats,
        )
        .expect("emit_glue_block 失败");
        sink.lines
    }

    /// 换笔块的固定底盘：骨架模板（不含 E）、无塔、收笔只留一句。
    fn swap_ir() -> Ir {
        let mut ir = test_ir();
        ir.gcode.mkp_retract = -5.0;
        ir.gcode.mkp_extrude = 5.5;
        ir.gcode.mkp_retract_speed = 25.0;
        ir.tower.use_towers = false;
        ir.toolhead.custom_mount_gcode = [
            "G92 E0",
            "L801",
            "M204 S10000",
            "G1 X168 F30000",
            "G1 X182 F10000",
            "G1 X168 F30000",
            "G1 F30000",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        ir.toolhead.custom_unmount_gcode = ["G92 E0", "G1 F30000"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        ir
    }
}
