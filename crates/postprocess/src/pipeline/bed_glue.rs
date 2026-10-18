//! **热床预涂胶**（`bed_glue.go`，607 行那套）的 Rust 移植 —— 2026-10-17 补上。
//!
//! # 这一份原来是什么
//!
//! `pipeline/hooks.rs` 里那个 `apply_pre_print_bed_glue` 一直是个**空壳**：
//! 「钩子点保留，本体未移植（默认关 + 无字节参考，见模块文档）」。
//! 于是设置页即便摆出"启用热床预涂胶"，勾了也不会有任何一行 G-code 变化。
//!
//! 这一份把它补上：插入点、温度处理、涂胶路径、范围检查，逐段对照
//! `internal/gcodesvc/bed_glue.go` 搬过来。
//!
//! # 做什么（作者的原始文案）
//!
//! 「在自动调平后、检测条前执行涂胶（降温到30°C → 换笔 → 涂胶 → 收笔）」
//!
//! 落成 G-code 就是：把热床降到 30°C 并等待 → 挂上笔 → 沿擦料塔位置（或所有模型
//! 首层）走一遍涂胶路径 → 抬 Z → 卸笔 → 把热床升回用户原本的目标温度。
//!
//! # 两条安全边界（都有意保留）
//!
//! 1. **校准件这一趟不涂**：校准模式没有擦料塔，涂胶路径会跑到校准区域外面去，
//!    与校准流程也冲突。原始实现就是直接 `return nil`。
//! 2. **目标温度 > 50°C 才降温**：用户要是本来就用 30°C / 1°C 那种低温打印板，
//!    没必要（也不该）把它"降"到 50°C —— 那是**升温**。所以只在真的更高时才降，
//!    并且降过之后结尾要把温度升回去。
//!
//! # 没有字节参考这件事
//!
//! 旧侧这份没有 golden 样本可比对，所以判据只能咬**结构**（该有的段在、不该有的段不在、
//! 温度处理对、范围检查会报），不能咬"逐字节一致"。`tests` 里那些就是这么写的。

use crate::gcode::OffsetMode;
use crate::gcode::format::{format_speed, format_z};
use crate::gcode::postprocess::has_e_param;
use crate::ir::Ir;
use crate::postproc::calibration;
use crate::postproc::offset::process_gcode_offset;
use crate::postproc::tower::bbox::calculate_tower_bbox;
use crate::postproc::tower::sheath::generate_sheath_gcode;

/// 涂胶时热床降到多少度（不阻塞升温）。只在原始目标温度**高于**它时才降。
const PRE_HEAT_TEMP: i64 = 50;

/// 涂胶笔的写 Z 温度 —— 挂上笔之后走路径时用的 Z（`Toolhead.ZOffset + Glue.ZOffset`）。
const GLUE_Z_MARGIN: f64 = 0.0;

/// 涂胶块那三段标记（判据与排查都靠它）。
pub const BLOCK_START: &str = ";===== pre-print bed glue =================";
pub const BLOCK_END: &str = ";===== pre-print bed glue end ============";

/// 主入口：按 `ir.prefs.bed_glue_*` 决定要不要插，插哪儿。
///
/// 与旧侧 `applyPrePrintBedGlueToFile` 的差别只有一处：那边自己读文件、自己写文件；
/// 这里收**行序列**、还行序列 —— 管线是内存里跑到底再一次落盘的，
/// 中间不落盘（`pipeline/mod.rs` 的写步才是唯一写点）。
pub fn apply_bed_glue(
    lines: Vec<String>,
    ir: &Ir,
    calibration: crate::postproc::calibration::Mode,
) -> Vec<String> {
    if !ir.prefs.bed_glue_enabled {
        return lines;
    }

    /* 校准件这一趟不涂：没有擦料塔，路径会跑出校准区域。
    身份由调用方定案传进来（`IrProcessRequest::calibration`）—— 不在这里扫 G-code。 */
    if calibration::is_any(calibration) {
        return lines;
    }

    let Some(insert_idx) = find_insertion_point(&lines) else {
        return lines;
    };

    let target_bed_temp = extract_original_bed_temp(&lines);
    /* 只有真的更高才降；降过之后结尾得升回去 */
    let lowered = target_bed_temp > PRE_HEAT_TEMP;
    let working = if lowered {
        lower_bed_temp_for_glue(&lines, PRE_HEAT_TEMP)
    } else {
        lines
    };

    let block = build_block(
        ir,
        &working,
        ir.prefs.bed_glue_all_models,
        target_bed_temp,
        lowered,
    );
    if block.is_empty() {
        return working;
    }

    let mut out = Vec::with_capacity(working.len() + block.len());
    out.extend_from_slice(&working[..insert_idx]);
    out.extend(block);
    out.extend_from_slice(&working[insert_idx..]);
    out
}

/// 插入点：`EXECUTABLE_BLOCK_START` 之后、第一条 `nozzle load line` /
/// `extrude cali test` 之前。
///
/// ★ 只在可执行块里找：`CONFIG_BLOCK` 里的 `machine_start_gcode` 注释内嵌着
/// 一整套 G-code 字符串，在那儿匹配会插进注释里（旧侧注释点明了这件事）。
fn find_insertion_point(lines: &[String]) -> Option<usize> {
    let exec_start = lines
        .iter()
        .position(|l| l.contains("; EXECUTABLE_BLOCK_START"))?
        + 1;
    (exec_start..lines.len()).find(|&i| {
        lines[i].contains(";===== nozzle load line")
            || lines[i].contains(";===== extrude cali test")
    })
}

/// 从可执行块里的 `M190 S<temp>` 取原始热床目标温度。取不到按 70（旧侧同一条兜底）。
fn extract_original_bed_temp(lines: &[String]) -> i64 {
    const FALLBACK: i64 = 70;
    let Some(exec_start) = lines
        .iter()
        .position(|l| l.contains("; EXECUTABLE_BLOCK_START"))
        .map(|i| i + 1)
    else {
        return FALLBACK;
    };
    for line in &lines[exec_start..] {
        let t = line.trim();
        if let Some(rest) = t.strip_prefix("M190 S") {
            let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
            if let Ok(v) = digits.parse::<i64>()
                && v > 0
            {
                return v;
            }
        }
    }
    FALLBACK
}

/// 把可执行块里**第一个** `M140 S…` 与**第一个** `M190 S…` 降到涂胶温度。
///
/// 只改头一个：后面那些是打印过程中按层改温的，动它们就改了工艺。
fn lower_bed_temp_for_glue(lines: &[String], pre_heat: i64) -> Vec<String> {
    let Some(exec_start) = lines
        .iter()
        .position(|l| l.contains("; EXECUTABLE_BLOCK_START"))
        .map(|i| i + 1)
    else {
        return lines.to_vec();
    };

    let mut out = lines.to_vec();
    let (mut m140_done, mut m190_done) = (false, false);
    for line in out.iter_mut().skip(exec_start) {
        if m140_done && m190_done {
            break;
        }
        let t = line.trim();
        if !m140_done && t.starts_with("M140 S") {
            *line = replace_bed_temp(line, "M140", pre_heat);
            m140_done = true;
            continue;
        }
        if !m190_done && t.starts_with("M190 S") {
            *line = replace_bed_temp(line, "M190", pre_heat);
            m190_done = true;
        }
    }
    out
}

/// 换掉 `M140/M190 S<数字>` 里的数字，**保留行尾注释**。
fn replace_bed_temp(line: &str, cmd: &str, new_temp: i64) -> String {
    let needle = format!("{cmd} S");
    let Some(idx) = line.find(&needle) else {
        return line.to_owned();
    };
    let rest = &line[idx + needle.len()..];
    let end = rest
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit())
        .map_or(rest.len(), |(i, _)| i);
    format!("{}{}{}{}", &line[..idx], needle, new_temp, &rest[end..])
}

/// 生成整个涂胶块。
fn build_block(
    ir: &Ir,
    content: &[String],
    all_models: bool,
    target_bed_temp: i64,
    lowered: bool,
) -> Vec<String> {
    let mut block: Vec<String> = vec![
        /* 起始标记：判据与排查都靠它认这一块从哪开始 */
        BLOCK_START.to_owned(),
    ];
    /* 给切片器的预览器一个"这里有动作"的锚点：纯虚拟挤出，不改实际耗材 */
    block.push(
        "G1 X100 Y100 E0.001 F3000 ; MKPSE: virtual extrusion for BBS G-code viewer compatibility"
            .to_owned(),
    );
    block.push("M109 S30 ; 降温到30°C并等待".to_owned());

    block.push(";Mounting Toolhead".to_owned());
    block.extend(filter_mount_gcode_lines(
        &ir.toolhead.custom_mount_gcode,
        ir.toolhead.z_offset + 4.0,
    ));
    block.push(";Toolhead Mounted".to_owned());

    block.push(";Bed Glueing Started".to_owned());
    block.extend(prime_tower_glue_path(ir));
    if all_models {
        block.extend(all_models_glue_path(ir, content));
    }
    block.push(";Bed Glueing Finished".to_owned());

    /* 抬 Z 到笔尖离床 5mm（绝对量，不是相对上一行） */
    let lift_z = ir.toolhead.z_offset + 5.0;
    block.push(format!(
        "G1 Z{} F{}",
        format_z(lift_z),
        format_speed(ir.machine.travel_speed * 60.0)
    ));

    block.push(";Unmounting Toolhead".to_owned());
    block.extend(filter_unmount_lines(
        &ir.toolhead.custom_unmount_gcode,
        ir.wiping.fan_speed,
    ));
    block.push(";Toolhead Unmounted".to_owned());

    /* 温度降过才需要升回去；原本就 ≤50 的没降过，就不能在这儿多插一条 M190 */
    if lowered {
        block.push(format!(
            "M190 S{target_bed_temp} ; 涂胶完成，阻塞等待热床升温到目标温度"
        ));
    }
    block.push(BLOCK_END.to_owned());

    block
}

/// 挂笔那一串：去空行、去带 E 的行（挂笔不该挤出），`L801` 那行换成抬 Z。
fn filter_mount_gcode_lines(mount_lines: &[String], z_lift: f64) -> Vec<String> {
    let mut out = Vec::new();
    for raw in mount_lines {
        let l = raw.trim();
        if l.is_empty() {
            continue;
        }
        if has_e_param(l) {
            continue;
        }
        if l.contains("L801") {
            out.push(format!("G1 Z{};L801", format_z(z_lift)));
            continue;
        }
        out.push(l.to_owned());
    }
    out
}

/// 卸笔那一串：同 `hooks::filter_unmount_gcode_lines` 的规则，只是这里收 `&[String]`。
fn filter_unmount_lines(unmount_lines: &[String], fan_speed: f64) -> Vec<String> {
    let mut out = Vec::new();
    for raw in unmount_lines {
        let l = raw.trim();
        if l.is_empty() || l.contains(";Wipe") || l.contains(";Brush") {
            continue;
        }
        if l.contains("M106 S[AUTO]") {
            out.push(format!("M106 S{}", format_speed(fan_speed)));
            continue;
        }
        if l.contains("M106 P1 S[AUTO]") {
            out.push(format!("M106 P1 S{}", format_speed(fan_speed)));
            continue;
        }
        if has_e_param(l) {
            continue;
        }
        out.push(l.to_owned());
    }
    out
}

/// 擦料塔位置的涂胶路径：拿首层螺旋路径（`GenerateSheathGCode`）当轨迹，
/// 逐行剥掉 Z、只留 XY，再套上笔的 XY 偏移。
fn prime_tower_glue_path(ir: &Ir) -> Vec<String> {
    let bbox = calculate_tower_bbox(
        &ir.wiping.outer_structure,
        ir.sheath.base_expand,
        ir.rib.extra_length,
        ir.rib.width,
        0.0,
        ir.machine.nozzle_diameter,
        ir.machine.first_layer_height,
    );

    let Some(sheath_lines) = generate_sheath_gcode(
        0,
        ir.machine.first_layer_height,
        ir.machine.first_layer_height,
        ir.machine.first_layer_height,
        ir.wiping.wiper_x,
        ir.wiping.wiper_y,
        ir.machine.travel_speed * 60.0,
        ir.tower.sheath_speed * 60.0,
        ir.machine.retract_length,
        ir.machine.nozzle_diameter,
        &ir.wiping.outer_structure,
        ir.sheath.base_expand,
        ir.sheath.enable_height,
        ir.sheath.converge_layers,
        ir.sheath.wall_width,
        ir.machine.first_layer_height,
        ir.tower.first_layer_flow,
        ir.machine.first_layer_height,
        ir.tower.safe_z_offset,
        bbox.min_x,
        bbox.min_y,
    ) else {
        return Vec::new();
    };

    let writing_z = ir.toolhead.z_offset + ir.glue.z_offset + GLUE_Z_MARGIN;
    let travel_speed = format_speed(ir.machine.travel_speed * 60.0);

    let mut out = Vec::new();
    /* 显式把 Z 定到写胶高度：后面每一行的 Z 都被剥掉了，靠这一行定住 */
    out.push(format!(
        "G1 Z{} F{} ;Print Z",
        format_z(writing_z),
        travel_speed
    ));

    for line in &sheath_lines {
        let t = line.trim();
        /* 只要 XY 运动：纯 Z 那些是原轨迹的抬升/换层，涂胶时 Z 恒定 */
        if !t.starts_with("G1 ") || !(t.contains('X') || t.contains('Y')) {
            continue;
        }
        let stripped = strip_z(t);
        if let Ok(p) = process_gcode_offset(
            &stripped,
            ir.toolhead.x_offset,
            ir.toolhead.y_offset,
            0.0,
            OffsetMode::Normal,
            ir,
        ) && !p.is_empty()
        {
            out.push(p);
        }
    }
    out
}

/// 所有模型首层位置也涂一遍（只涂擦料塔时不做）。
fn all_models_glue_path(ir: &Ir, content: &[String]) -> Vec<String> {
    let segments = extract_first_layer_paths(content, ir.machine.first_layer_height);
    if segments.is_empty() {
        return Vec::new();
    }

    let writing_z = ir.toolhead.z_offset + ir.glue.z_offset + GLUE_Z_MARGIN;
    let lift_z = writing_z + 2.0;
    let travel_speed = format_speed(ir.machine.travel_speed * 60.0);

    let mut out = Vec::new();
    for (i, seg) in segments.iter().enumerate() {
        /* 段间走一个"抬起来 → 平移到下一段起点 → 落下去"，不然会拖一条横穿床面的胶痕 */
        if i > 0 {
            let (nx, ny) = extract_first_xy(&seg[0]);
            out.push(format!("G1 Z{} F{}", format_z(lift_z), travel_speed));
            if !nx.is_empty() && !ny.is_empty() {
                out.push(format!("G1 X{nx} Y{ny} F{travel_speed}"));
            }
            out.push(format!("G1 Z{} F{}", format_z(writing_z), travel_speed));
        }
        for line in seg {
            let stripped = strip_z(line);
            if let Ok(p) = process_gcode_offset(
                &stripped,
                ir.toolhead.x_offset,
                ir.toolhead.y_offset,
                0.0,
                OffsetMode::Normal,
                ir,
            ) && !p.is_empty()
            {
                out.push(p);
            }
        }
    }
    out
}

/// 从 G-code 里抠出**首层**的挤出路径，按空驶（不带 E 的 G1）切成一段段。
fn extract_first_layer_paths(content: &[String], first_layer_height: f64) -> Vec<Vec<String>> {
    let (mut start, mut end) = (None, None);

    for (i, line) in content.iter().enumerate() {
        if line.contains(";LAYER:0") {
            start = Some(i + 1);
        }
        if line.contains(";LAYER:1") && start.is_some() {
            end = Some(i);
            break;
        }
        if line.contains(";LAYER_CHANGE") {
            if start.is_none() {
                start = Some(i + 1);
            } else {
                end = Some(i);
                break;
            }
        }
    }

    /* 没有层标记（发布版 G-code 常常剥掉了）：先找第一条挤出运动当起点，
    再按 Z 高度判首层结束 */
    if start.is_none() {
        start = content.iter().position(|l| {
            let t = l.trim();
            t.starts_with("G1 ") && t.contains(" E") && (t.contains('X') || t.contains('Y'))
        });
        let Some(s) = start else {
            return Vec::new();
        };
        for (i, line) in content.iter().enumerate().skip(s + 1) {
            if line.contains(";LAYER:1") || line.contains(";LAYER_CHANGE") {
                end = Some(i);
                break;
            }
        }
        if end.is_none() {
            end = find_first_layer_end_by_z(content, s + 1, first_layer_height);
        }
    }

    let (Some(s), Some(e)) = (start, end) else {
        return Vec::new();
    };
    if s >= e {
        return Vec::new();
    }

    let mut segments: Vec<Vec<String>> = Vec::new();
    let mut cur: Vec<String> = Vec::new();
    for line in &content[s..e] {
        let t = line.trim();
        if !t.starts_with("G1 ") || t.starts_with("G1 F") {
            continue;
        }
        if !t.contains('X') && !t.contains('Y') {
            continue;
        }
        let is_travel = !t.contains(" E");
        if is_travel && !cur.is_empty() {
            segments.push(std::mem::take(&mut cur));
        }
        cur.push(t.to_owned());
    }
    if !cur.is_empty() {
        segments.push(cur);
    }
    segments
}

/// 没有 `;LAYER` 标记时的兜底：跟着 `G1 Z…` 记当前 Z，
/// 遇到"挤出运动且 Z 已经超过首层 + 容差"那一行，就是首层结束。
fn find_first_layer_end_by_z(
    content: &[String],
    start: usize,
    first_layer_height: f64,
) -> Option<usize> {
    const Z_TOLERANCE: f64 = 0.05;
    let mut current_z = first_layer_height;

    for (i, line) in content.iter().enumerate().skip(start) {
        let t = line.trim();
        if !t.starts_with("G1 ") {
            continue;
        }
        if let Some(z) = parse_z_param(t) {
            current_z = z;
        }
        let has_e = t.contains(" E");
        let has_xy = t.contains('X') || t.contains('Y');
        if has_e && has_xy && current_z > first_layer_height + Z_TOLERANCE {
            return Some(i);
        }
    }
    None
}

/// 取 `G1` 行里的 Z 值（`Z` 前面必须是空格，避免撞上 `Z` 出现在别的词里）。
fn parse_z_param(line: &str) -> Option<f64> {
    let bytes = line.as_bytes();
    for (i, _) in line.char_indices() {
        if bytes[i] != b'Z' || i == 0 || bytes[i - 1] != b' ' {
            continue;
        }
        let rest = &line[i + 1..];
        let end = rest
            .char_indices()
            .find(|(_, c)| *c == ' ' || *c == ';')
            .map_or(rest.len(), |(j, _)| j);
        return rest[..end].parse::<f64>().ok();
    }
    None
}

/// 剥掉 `G1` 行里的 Z 参数（连它前面那些空格一起）。
///
/// 例：`G1 X10 Y10 Z0.4 F300` → `G1 X10 Y10 F300`
fn strip_z(line: &str) -> String {
    let bytes = line.as_bytes();
    let Some(pos) = line
        .char_indices()
        .find(|(i, c)| *c == 'Z' && (*i == 0 || bytes[i - 1] == b' '))
    else {
        return line.to_owned();
    };
    let idx = pos.0;
    let mut start = idx;
    while start > 0 && bytes[start - 1] == b' ' {
        start -= 1;
    }
    let rest = &line[idx + 1..];
    let end = rest
        .char_indices()
        .find(|(_, c)| !(*c == '.' || *c == '-' || c.is_ascii_digit()))
        .map_or(rest.len(), |(j, _)| j);
    format!("{}{}", &line[..start], &rest[end..])
}

/// 取一行里的 X、Y 数值原文（给段间平移用）。
fn extract_first_xy(line: &str) -> (String, String) {
    fn grab(line: &str, axis: char) -> String {
        let Some(idx) = line.find(axis) else {
            return String::new();
        };
        let rest = &line[idx + 1..];
        let end = rest
            .char_indices()
            .find(|(_, c)| !(*c == '.' || *c == '-' || c.is_ascii_digit()))
            .map_or(rest.len(), |(j, _)| j);
        rest[..end].to_owned()
    }
    (grab(line, 'X'), grab(line, 'Y'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// 出厂（关）时**一行都不动** —— 这是"默认不影响任何人"的那条保证。
    #[test]
    fn disabled_changes_nothing() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M190 S70",
            ";===== nozzle load line",
            "G1 X1 Y1",
        ]);
        let ir = Ir::default();
        assert_eq!(
            apply_bed_glue(src.clone(), &ir, crate::postproc::calibration::Mode::None),
            src
        );
    }

    /// 开着时插在 `nozzle load line` **之前**，且首尾标记都在。
    #[test]
    fn enabled_inserts_before_the_nozzle_load_line() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M190 S30",
            ";===== nozzle load line",
            "G1 X1 Y1",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;
        let out = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);

        let start = out
            .iter()
            .position(|l| l == BLOCK_START)
            .expect("有起始标记");
        let end = out.iter().position(|l| l == BLOCK_END).expect("有结束标记");
        let nozzle = out
            .iter()
            .position(|l| l.contains(";===== nozzle load line"))
            .expect("原点还在");
        assert!(start < end, "标记顺序反了");
        assert!(end < nozzle, "涂胶块必须插在检测条之前");
    }

    /// `CONFIG_BLOCK` 里那句 `machine_start_gcode = "..."` 内嵌着
    /// `;===== nozzle load line`，**不许**插到那儿去。
    #[test]
    fn the_config_block_copy_is_ignored() {
        let src = lines(&[
            "; CONFIG_BLOCK_START",
            "; machine_start_gcode = \";===== nozzle load line\\nG1 X1\"",
            "; CONFIG_BLOCK_END",
            "; EXECUTABLE_BLOCK_START",
            "M190 S30",
            ";===== nozzle load line",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;
        let out = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);
        let block_at = out.iter().position(|l| l == BLOCK_START).expect("插了");
        let exec_at = out
            .iter()
            .position(|l| l.contains("; EXECUTABLE_BLOCK_START"))
            .expect("可执行块在");
        assert!(block_at > exec_at, "插进了配置块那段注释里");
    }

    /// 目标温度 > 50 才降；降了结尾就要升回去。
    #[test]
    fn hot_bed_is_lowered_and_restored() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M140 S70",
            "M190 S70",
            ";===== nozzle load line",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;
        let out = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);

        assert!(
            out.iter().any(|l| l.starts_with("M140 S50")),
            "M140 该降到 50"
        );
        assert!(
            out.iter().any(|l| l.starts_with("M190 S50")),
            "M190 该降到 50"
        );
        assert!(
            out.iter().any(|l| l.starts_with("M190 S70 ;")),
            "结尾该升回 70"
        );
    }

    /// 本来就是 30 度（低温板）：**不许**升到 50，也不许插"升回去"那一行。
    #[test]
    fn cold_bed_is_left_alone() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M140 S30",
            "M190 S30",
            ";===== nozzle load line",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;
        let out = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);

        assert!(out.iter().any(|l| l.starts_with("M140 S30")), "不许升温");
        assert!(out.iter().any(|l| l.starts_with("M190 S30")));
        assert!(
            !out.iter().any(|l| l.contains("涂胶完成，阻塞等待")),
            "没降温就不该有升回去那一行"
        );
    }

    #[test]
    fn strip_z_takes_the_param_with_its_space() {
        assert_eq!(strip_z("G1 X10 Y10 Z0.4 F300"), "G1 X10 Y10 F300");
        assert_eq!(strip_z("G1 X10 Y10"), "G1 X10 Y10");
        assert_eq!(strip_z("G1 Z1.2 F300"), "G1 F300");
    }

    #[test]
    fn bed_temp_replacement_keeps_the_comment() {
        assert_eq!(
            replace_bed_temp("M190 S70 ; heat", "M190", 50),
            "M190 S50 ; heat"
        );
        assert_eq!(replace_bed_temp("M190 S70", "M190", 50), "M190 S50");
    }

    #[test]
    fn xy_extraction_stops_at_the_next_param() {
        assert_eq!(
            extract_first_xy("G1 X34.5 Y58.4 E0.1 F300"),
            ("34.5".to_owned(), "58.4".to_owned())
        );
    }
}
