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
//! 落成 G-code 就是：把热床降到预设的涂胶温度并等待（本机 50°C，只有原目标更高时才降）
//! → 喷嘴目标压到 30°C（`M104`，**不等待**）→ 挂上笔 → 沿擦料塔位置（或所有模型
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

/// 挂笔/预抬高度：`Toolhead.ZOffset` 之上 4mm。挂笔那行（`L801`）与进块前的
/// 预抬都取这一个值 —— 于是装笔时 Z 不用再动，也不会出现两个数各说各话。
/// 涂胶相关运动的进给（mm/min）。三档：
///
/// - `follow` = 机型的移动速度（`machine.travel_speed`）
/// - `glue`   = 「涂胶速度限制」那一档（`toolhead.max_speed`，已在 `build.rs` 里乘过 60）
/// - `custom` = 参数页填的 mm/s（**这里才**乘 60）
///
/// 空串 / 未知值按 `follow`：老预设根本没有这两格，行为必须与加参数之前一致。
/// 两格（预涂胶速度、段间横移速度）共用这一个出口 —— 不许两处各写一套 if-else，
/// 否则又会出现"参数页一个数、gcode 另一个数"。
fn glue_feed_of(ir: &Ir, mode: &str, custom_mm_s: f64) -> f64 {
    match mode {
        "glue" => ir.toolhead.max_speed,
        "custom" => {
            let v = if custom_mm_s > 0.0 {
                custom_mm_s
            } else {
                ir.toolhead.max_speed / 60.0
            };
            v * 60.0
        }
        // "follow" 与空串 / 未知值都落这里：老预设没有这一格，不许因此改变行为。
        "follow" | "" => ir.machine.travel_speed * 60.0,
        _ => ir.machine.travel_speed * 60.0,
    }
}

/// 涂胶块要抬多高（**不含** Z 偏移，调用处再加 `toolhead.z_offset`）。
///
/// 涂胶发生在第一层之前 —— 也就是"第 0 层"，所以用它的是预设里
/// 「前 3 层抬升高度」（`z_lift_height_first_layers`，本机 6.00mm），
/// 不是普通那档 5.00（那个给涂胶路径内部的 Safe Z 用）。
///
/// **不占用原文的前三层**：原文第一层照样享受它自己那份抬升，两件事互不影响。
///
/// 预设没填（旧配置 / `Ir::default()`）时退回 [`MOUNT_Z_LIFT`]，保持旧行为。
/// 段间（模型之间 / 段与段之间）抬多高。
///
/// 预设值 `glue_inter_model_lift_height`（本机 2.00mm，范围 2~5）。
/// **绝不允许是 0**：0 意味着笔尖贴着刚涂的胶面平移，直接拖出一条胶痕 ——
/// 旧配置 / `Ir::default()` 没填时退回 2.0（预设的下限，也是旧硬编码的那个值）。
fn inter_model_lift(ir: &Ir) -> f64 {
    if ir.glue.inter_model_lift_height > 0.0 {
        ir.glue.inter_model_lift_height
    } else {
        2.0
    }
}

fn mount_lift(ir: &Ir) -> f64 {
    /* 三级：前 3 层那档 → 普通那档 → 6.0（预设允许的下限）。
    旧的 `MOUNT_Z_LIFT = 4.0` 常量已删（2026-11-05 作者：「这个不需要了吧」）——
    它既不在预设里、也不是任何一档的合法值，留着只会让人以为 7.9 是"设计值"。 */
    if ir.glue.z_lift_height_first_layers > 0.0 {
        ir.glue.z_lift_height_first_layers
    } else if ir.glue.z_lift_height > 0.0 {
        ir.glue.z_lift_height
    } else {
        6.0
    }
}

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
    /* 涂胶前热床温度：原来是写死的 50，现随预设（`bed_glue_preheat_temp`）。
    预设没填（0）⇒ 退回 50，不会因为老预设缺这一格就把热床关掉。 */
    let pre_heat_temp = if ir.glue.bed_glue_preheat_temp > 0.0 {
        ir.glue.bed_glue_preheat_temp.round() as i64
    } else {
        PRE_HEAT_TEMP
    };
    let lowered = target_bed_temp > pre_heat_temp;
    let working = if lowered {
        lower_bed_temp_for_glue(&lines, pre_heat_temp)
    } else {
        lines
    };

    /* 进块前喷嘴停在哪 —— 卸完笔要送回这里。见 `build_block` 里那段回位注释。 */
    let resume_xy = last_xy_before(&working, insert_idx);
    let block = build_block(
        ir,
        &working,
        ir.prefs.bed_glue_all_models,
        target_bed_temp,
        lowered,
        resume_xy,
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

/// 把一行里已有的 `F<数字>` 换成指定进给；没有 F 就追加在注释之前。
///
/// 用途：涂胶行原本的进给是"塔护套的打印速度"，但胶笔的真实上限是
/// 预设里的「涂胶速度限制」（`toolhead.speed_limit`，本机 70mm/s ⇒ `max_speed = 4200`）。
/// 真机上超出这个上限的部分会被固件截掉，但 G-code 里写着 700mm/s 会让人误判。
fn set_feed(line: &str, feed: f64) -> String {
    let f = format!("F{}", format_speed(feed));
    let head_end = line.find(';').unwrap_or(line.len());
    let (head, tail) = line.split_at(head_end);
    match head.find('F') {
        Some(i) => {
            let after = &head[i + 1..];
            let end = after
                .char_indices()
                .find(|(_, c)| !c.is_ascii_digit() && *c != '.')
                .map_or(after.len(), |(j, _)| j);
            format!("{}{}{}{}", &head[..i], f, &after[end..], tail)
        }
        None => format!("{} {}{}", head.trim_end(), f, tail),
    }
}

/// 把一串 `G1` 移动改写成 `G0`。
///
/// **为什么要改**（2026-11-04）：涂胶块里的移动**没有 E**（不是塑料），但切片器预览
/// 会把 `Custom` 这一段里的 `G1` 逐条画出来 —— 于是塔心那圈涂胶路径、模型轮廓那圈，
/// 全被画成一坨绿色实体压在塔首层和模型首层上（作者：「擦嘴条和擦料塔首层的显示
/// 还是不正常」）。`G0` 在固件里与 `G1` 同一条直线插补（F 已给），只是预览把它归到
/// 「空驶」—— 用户关掉空驶，这些就不画了。**坐标、Z、顺序一个都不动。**
fn as_travel(lines: Vec<String>) -> Vec<String> {
    /* ★ 2026-11-05 定案（照校准模式）：**只在一个点挤出，其余全是空驶**。
     *
     * 三种写法都试过：
     *   · 路径写 `G1`（无 E）落在 `Custom` 里 → 预览画成一坨绿块压在塔首层/模型首层上
     *   · 路径写 `G0`（纯空驶）           → 看不见（段落在 `nozzle load line` 之前，勾空驶也不渲染）
     *   · 路径**每行** `E0.001`           → 连成一条横穿床面的粗绿棒（作者 2026-11-05 实测）
     *
     * 校准模式只有 `G1 X100 Y100 Z10 E0.001` **那一个点**带 E，后面图案全是无 E 移动 ——
     * 于是预览只亮一个点，路径按空驶处理。这里照抄：锚点那一下带 E（在 `build_block` 里），
     * 路径本身一律 `G0`。
     */
    lines
        .into_iter()
        .map(|l| match l.trim_start().strip_prefix("G1") {
            Some(rest) => format!("G0{rest}"),
            None => l,
        })
        .collect()
}

/// 进块前喷嘴停在哪：顺着可执行块走一遍，记最后一个 X、最后一个 Y。
///
/// 必须**逐行累积**（而不是找最后一条同时带 XY 的行）：收尾那几行常是
/// `G1 X108.000 Y-0.500` 之后再来一条只改 Z 的抬升 —— 只看带齐 XY 的行会取错。
fn last_xy_before(lines: &[String], end: usize) -> Option<(f64, f64)> {
    let (mut x, mut y) = (None, None);
    for line in &lines[..end] {
        let t = line.trim();
        if !(t.starts_with("G1") || t.starts_with("G0")) {
            continue;
        }
        if let Some(v) = crate::gcode::coord::extract_coord(t.as_bytes(), b'X') {
            x = Some(v);
        }
        if let Some(v) = crate::gcode::coord::extract_coord(t.as_bytes(), b'Y') {
            y = Some(v);
        }
    }
    match (x, y) {
        (Some(a), Some(b)) => Some((a, b)),
        _ => None,
    }
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
    /* ★ 优先插在 `M104 S… ; prepare to print` **之前**（2026-11-05，作者：
    「预涂胶的，是不是应该去掉他多余的升温」）。
    插在 load line 之前的话，切片器会先 `M104 S245` 把喷嘴烧到 245，胶还没涂喷嘴就是热的
    —— 涂胶过程盯着一个 245°C 的喷嘴，与"涂胶时喷嘴该是冷的"相悖。
    往前挪一行，涂胶从头到尾喷嘴都没被加热，`M104 S245` 等涂完再执行。 */
    if let Some(i) = (exec_start..lines.len()).find(|&i| {
        let t = lines[i].trim();
        t.starts_with("M104 S") && t.contains("prepare to print")
    }) {
        return Some(i);
    }
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
    resume_xy: Option<(f64, f64)>,
) -> Vec<String> {
    let mut block: Vec<String> = vec![
        /* 起始标记：判据与排查都靠它认这一块从哪开始 */
        BLOCK_START.to_owned(),
    ];
    /* ★ 锚点（2026-11-05 定稿，作者在切片器里逐个变体验过）：
     *
     *  **它是一个「假挤出」，只为让切片器预览认到"这里有动作"。** 真机上是胶笔在涂，
     *  塑料一滴都不出 —— 总挤出量 0.001mm（线材），可以当 0 看。
     *
     *  Z 必须等于首层层高（`first_layer_height`）：切片器的「层高」是按相邻两个 Z
     *  反推出来的，写成 0.3 / 0.8 / 1.5 都会让第一层显示成那个高度（实测 0.30）。
     *
     *  XY 只走 0.1mm：带 E 的移动在预览里一定被画成一条挤出线，太长就是一根横跨
     *  床面的棒；0.1mm 短到看不出来，但足够让预览认到「这里有动作」。
     *
     *  `G92 E0` 不能省：换料/回抽之后 E 可能是 -1 这种负值，直接写 `E0.001`
     *  会变成「挤出 1mm」，在原地堆出一坨料。
     *
     *  抬 Z 放在锚点**之后**：真机上只有那 0.1mm 贴着首层层高走，其余仍是高处空驶。
     *
     *  XY 取**进块前喷嘴停的那个点**（不是写死的 100/100）：既不会横穿床面，
     *  也不会因为写死坐标而在别的机型 / 摆位上跑到无关的位置。 */
    let travel = format_speed(ir.machine.travel_speed * 60.0);
    let layer_z = ir.machine.first_layer_height;
    /* 抬升高度（2026-11-05 作者指出）：涂胶块发生在**第一层之前**，也就是"第 0 层"，
    用的是预设里「前 3 层抬升高度」（`z_lift_height_first_layers`，本机 6.00mm），
    不是普通那档 5.00，也不是旧的写死 4.0。
    它也不占用原文的「前三层」—— 原文第一层照样享受它自己的抬升。 */
    let mount_z = ir.toolhead.z_offset + mount_lift(ir);
    let (ax, ay) = resume_xy.unwrap_or((100.0, 100.0));
    block.push(format!("G0 Z{} F{}", format_z(layer_z), travel));
    block.push("G92 E0".to_owned());
    /* 顺序是死的（2026-11-05 作者改法）：
     * 抬到首层层高 → 归零 E → **空驶回到原位** → 从原位往外挤 0.1mm → 再回原位 → 抬 Z。
     * 反过来（先到 ax+0.1、再挤回 ax）也是 0.1mm，但锚点会落在回来的路上，
     * 预览会从上一条挤出终点连一条线头；先到位再往外挤，锚点才落在原位本身。 */
    /* 速度跟机型走（空驶速度），不再写死 F30000 —— 换台机器 / 改速度参数时它也该跟着变。 */
    let anchor_feed = format_speed(ir.machine.travel_speed * 60.0);
    block.push(format!(
        "G1 X{} Y{} F{}",
        format_z(ax),
        format_z(ay),
        anchor_feed
    ));
    block.push(crate::gcode::virtual_extrude::line(
        &format!("X{} Y{}", format_z(ax + 0.1), format_z(ay)),
        &anchor_feed,
    ));
    /* 挤完往回退 0.1mm，把喷嘴放回原位 —— 下一步 `G0 Z{装笔高度}` 才是纯垂直抬升，
     * 不会顺带把喷嘴带偏；块尾的回位也因此能从同一个点出发（首尾一致）。 */
    block.push(format!(
        "G0 X{} Y{} F{}",
        format_z(ax),
        format_z(ay),
        anchor_feed
    ));
    block.push(format!("G0 Z{} F{}", format_z(mount_z), travel));
    /* ★ 2026-11-05 作者定案：**这里不许阻塞**。
     *
     * 原来写的是 `M109 S30` —— M109 会停在原地等喷嘴真的凉到 30°C，而这一块在
     * 第一层之前，胶涂完紧接着就要升温打印首层，"先烧到 245 再等凉到 30" 那几分钟
     * 全是白等（A1 上尤其明显）。`M104` 只设目标、不等待：涂胶期间喷嘴自然降温，
     * 块后切片器自己那句 `M104 S245 ; prepare to print` 再把目标改回来。
     *
     * 温度值本身不重要（作者：「到 30 到多少度都无所谓，到 50 都可以」），
     * 保留 30 只是沿用旧参数；判据只有一条 —— **这一行不许阻塞**。
     * 同一口径见 `postproc/calibration.rs::should_preserve_nozzle_load_line_line`
     * （那边也是 `M109` → `M104`，2026-10-19 真机踩过）。 */
    block.push("M104 S30 ; 降温到30°C（不等待，涂完还要升温）".to_owned());

    block.push(";Mounting Toolhead".to_owned());
    block.extend(filter_mount_gcode_lines(
        &ir.toolhead.custom_mount_gcode,
        mount_z,
    ));
    block.push(";Toolhead Mounted".to_owned());

    block.push(";Bed Glueing Started".to_owned());
    block.extend(as_travel(prime_tower_glue_path(ir)));
    if all_models {
        block.extend(as_travel(all_models_glue_path(ir, content)));
    }
    block.push(";Bed Glueing Finished".to_owned());

    /* 抬 Z 到「笔尖离床 `z_lift_height`」（预设 5.00mm）—— 原来是写死的 5.0，
    作者 2026-11-05：「这个抬升是不是没有按照涂胶 Z 轴抬升高度加 z 偏移高度去算」。 */
    let lift_z = ir.toolhead.z_offset + ir.glue.z_lift_height;
    block.push(format!(
        "G0 Z{} F{}",
        format_z(lift_z),
        format_speed(ir.machine.travel_speed * 60.0)
    ));

    /* 涂胶后静置（2026-11-05）：笔已抬到安全高度，胶也涂完了，再停一会儿让它贴住。
    秒数 0（默认）时**一个字节都不输出** —— `end_to_end` 判据靠这个保证老预设逐字节不变。
    `G4 P<毫秒>`：Bambu 固件的 G4 认毫秒（与仓内 `M400` 那条同一口径）。 */
    let wait_ms = (ir.glue.bed_glue_wait_seconds * 1000.0).round();
    if wait_ms > 0.0 {
        block.push(format!(
            "G4 P{} ; 涂胶后静置 {:?}s",
            wait_ms as i64, ir.glue.bed_glue_wait_seconds
        ));
    }

    block.push(";Unmounting Toolhead".to_owned());
    block.extend(filter_unmount_lines(
        &ir.toolhead.custom_unmount_gcode,
        ir.wiping.fan_speed,
    ));
    block.push(";Toolhead Unmounted".to_owned());

    /* ★ **回位到涂胶前的落点**（2026-11-04，第四次修正，前面三种写法都错）：
     *
     * 紧跟其后的切片器「挤出校准」（`;===== extrude cali test =====`）第一条是
     * `G0 X128 E8 F780` —— 带 E 的 G0，没有 Y。它的起点由**块结束时喷嘴停在哪**决定：
     *
     *   · 停在装配位 (256,51)  → (256,51)→(128,51)，一条 128mm 的斜棒
     *   · 停在 (108,-0.5) Z8.9 → (108,-0.5)→(128,-0.5)，20mm 短棒，但**挂在 8.9mm 高处**
     *   · 停在 (108,-0.5) Z0.2 → 就是切片器本来的样子：贴着床、床沿下方，看不见
     *
     * 所以回位必须**同时**满足三条：回进块前那个 XY、Z 压回首层层高、并且放在 M190 之后
     * （M190 要阻塞等热床升温，那段时间喷嘴抬着更安全；抬这一下用 G0，不画形体）。
     */
    if let Some((rx, ry)) = resume_xy {
        let travel = format_speed(ir.machine.travel_speed * 60.0);
        if lowered {
            block.push(format!(
                "M190 S{target_bed_temp} ; 涂胶完成，阻塞等待热床升温到目标温度"
            ));
        }
        block.push(format!(
            "G0 X{} Y{} Z{} F{} ; 回到涂胶前的位置（校准时喷嘴就在这儿起笔）",
            format_z(rx),
            format_z(ry),
            format_z(lift_z),
            travel
        ));
        block.push(format!(
            "G0 Z{} F{} ; 落回首层层高，校准线贴床画",
            format_z(ir.machine.first_layer_height),
            travel
        ));
        block.push(";===== pre-print bed glue end ============".to_owned());
        return block;
    }

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
    let lift_z = ir.toolhead.z_offset + mount_lift(ir);
    let travel_speed = format_speed(ir.machine.travel_speed * 60.0);
    /* 笔的速度：预涂胶速度那一格（`follow` / `glue` / `custom` 三档） */
    let glue_feed = glue_feed_of(ir, &ir.glue.bed_glue_speed_mode, ir.glue.bed_glue_speed);

    let mut xy: Vec<String> = Vec::new();
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
            /* 笔的速度上限 = 预设「涂胶速度限制」（`toolhead.speed_limit`），
            不是护套那档打印速度 —— 作者：「我的 MKP 笔涂胶就是用这个速度的」。 */
            xy.push(set_feed(&p, glue_feed));
        }
    }
    let Some(first) = xy.first() else {
        return Vec::new();
    };

    /* 顺序是死的：**先在抬高处平飞到起点，再落到写胶高度**。
    Z 全被剥掉了，落笔那一行就是块里最早的那个 Z —— 要是把它摆在平移之前，
    刚装好的笔会带着板面从挂笔位（X≈260）一路拖到塔心，拖出一整条胶痕。
    原轨迹（`GenerateSheathGCode`）本来就是「Safe Z → 飞到塔心 → Print Z」。 */
    let mut out = vec![
        format!("G1 Z{} F{} ;Safe Z", format_z(lift_z), travel_speed),
        first.clone(),
        format!("G1 Z{} F{} ;Print Z", format_z(writing_z), travel_speed),
    ];
    out.extend(xy.into_iter().skip(1));
    out
}

/// 所有模型首层位置也涂一遍（只涂擦料塔时不做）。
fn all_models_glue_path(ir: &Ir, content: &[String]) -> Vec<String> {
    let segments = extract_first_layer_paths(content, ir.machine.first_layer_height);
    if segments.is_empty() {
        return Vec::new();
    }

    let writing_z = ir.toolhead.z_offset + ir.glue.z_offset + GLUE_Z_MARGIN;
    /* 段间抬升用预设那一档（`glue_inter_model_lift_height`，本机 2.00mm）——
    原来这里是写死的 `+ 2.0`，用户在参数页把这一格改成 3 / 4 时根本不生效。 */
    let lift_z = writing_z + inter_model_lift(ir);
    let travel_speed = format_speed(ir.machine.travel_speed * 60.0);
    let glue_feed = glue_feed_of(ir, &ir.glue.bed_glue_speed_mode, ir.glue.bed_glue_speed);
    /* 段与段之间的横移：单独一格（默认 follow = 机型空驶速度） */
    let travel_feed = glue_feed_of(
        ir,
        &ir.glue.glue_travel_speed_mode,
        ir.glue.glue_travel_speed,
    );

    let mut out = Vec::new();
    for seg in &segments {
        /* 每一段（**含第一段**）都走「抬起来 → 平移到起点 → 落下去」：
        第一段之前笔还停在塔心那个写胶高度上，不抬就是一条横穿床面的胶痕。 */
        let (nx, ny) = extract_first_xy(&seg[0]);
        out.push(format!("G1 Z{} F{}", format_z(lift_z), travel_speed));
        if !nx.is_empty() && !ny.is_empty() {
            /* 这一段是笔在胶面之间横移（不是抬起来飞）：用「段间横移速度」那一格。 */
            out.push(format!("G1 X{nx} Y{ny} F{}", format_speed(travel_feed)));
        }
        out.push(format!("G1 Z{} F{}", format_z(writing_z), travel_speed));
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
                /* 同塔那条：预涂胶速度那一格 */
                out.push(set_feed(&p, glue_feed));
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

    /// 平移之前必须先抬 Z：虚拟挤出那行只有 XY，写在低 Z 上就是喷嘴贴板拖过去
    /// （校准块同一个坑，见 `calibration::CALIBRATION_PRELIFT`）。
    #[test]
    fn lifts_z_before_the_virtual_travel() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M190 S30",
            ";===== nozzle load line",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;
        ir.toolhead.z_offset = 4.0;
        let out = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);

        /* 2026-11-05 起顺序改成「贴首层层高走 0.1mm → 再抬到装笔高度」：
        锚点的 Z 必须等于首层层高，否则切片器会把第一层的层高显示成那个值
        （作者实测 0.3/0.8/1.5 都会写进层高面板）。 */
        let anchor = out
            .iter()
            .position(|l| l.contains("virtual extrusion"))
            .expect("虚拟挤出那行还在");
        let lift = out
            .iter()
            .position(|l| l == "G0 Z10.0 F0.0")
            .expect("锚点之后要抬到装笔高度（默认兜底 6 + z_offset 4）");
        assert!(
            anchor < lift,
            "先贴首层走完 0.1mm，再抬到 Toolhead.ZOffset + 4"
        );
        assert_eq!(
            out[anchor - 3],
            "G0 Z0.0 F0.0",
            "锚点的 Z 用首层层高（默认 0 就是 0）"
        );
    }

    /// 擦料塔那条涂胶路径：**先在高处飞到塔心，再落笔** —— 顺序反了就是从挂笔位
    /// （X≈260）贴着板面拖一条胶痕到塔心。
    #[test]
    fn tower_path_flies_high_before_descending() {
        let mut ir = Ir::default();
        ir.sheath.base_expand = 1.0;
        ir.machine.travel_speed = 300.0;
        ir.machine.nozzle_diameter = 0.4;
        ir.machine.first_layer_height = 0.2;
        ir.machine.retract_length = 0.8;
        ir.tower.sheath_speed = 100.0;
        ir.tower.first_layer_flow = 1.0;
        ir.toolhead.z_offset = 4.0;
        ir.glue.z_offset = -0.05;

        let path = prime_tower_glue_path(&ir);
        assert!(path.len() > 3, "路径没生成：{path:?}");
        assert_eq!(path[0], "G1 Z10.0 F18000.0 ;Safe Z", "先抬到装笔高度");
        assert!(
            path[1].contains('X') && path[1].contains('Y'),
            "第二行才是平飞：{}",
            path[1]
        );
        assert_eq!(path[2], "G1 Z3.95 F18000.0 ;Print Z", "到了起点才落笔");
    }

    /// 「所有模型首层也涂」的第一段同样要先抬再飞：它前面笔还停在塔心的写胶高度上。
    #[test]
    fn all_models_path_lifts_before_the_first_segment() {
        let content = lines(&[
            ";LAYER:0",
            "G1 X10 Y10 E0.5",
            "G1 X20 Y10 E0.5",
            ";LAYER:1",
            "G1 X30 Y30 E0.5",
        ]);
        let mut ir = Ir::default();
        ir.machine.travel_speed = 300.0;
        ir.toolhead.z_offset = 4.0;
        ir.glue.z_offset = -0.05;

        let out = all_models_glue_path(&ir, &content);
        /* 段内那几行会被 `process_gcode_offset` 过一遍（空机的默认参数下会被剔掉），
        这里只咬这一段的开门三步。 */
        assert!(out.len() >= 3, "路径没生成：{out:?}");
        assert_eq!(out[0], "G1 Z5.95 F18000.0");
        assert!(
            out[1].contains("X10") && out[1].contains("Y10"),
            "{}",
            out[1]
        );
        assert_eq!(out[2], "G1 Z3.95 F18000.0");
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

    /// 三档速度：follow=机型移动速度、glue=涂胶速度限制、custom=填的那个数。
    /// 空串也落 follow —— 老预设没有这两格，行为必须和加参数之前一致。
    #[test]
    fn glue_feed_resolves_the_three_modes() {
        let mut ir = Ir::default();
        ir.machine.travel_speed = 200.0;
        ir.toolhead.max_speed = 4200.0;

        assert_eq!(
            glue_feed_of(&ir, "follow", 70.0),
            12000.0,
            "机型移动 200mm/s"
        );
        assert_eq!(glue_feed_of(&ir, "", 70.0), 12000.0, "空串按 follow");
        assert_eq!(glue_feed_of(&ir, "glue", 10.0), 4200.0, "涂胶速度 70mm/s");
        assert_eq!(glue_feed_of(&ir, "custom", 25.0), 1500.0, "自定义 25mm/s");
        assert_eq!(
            glue_feed_of(&ir, "custom", 0.0),
            4200.0,
            "自定义填 0 ⇒ 退回涂胶速度，不许输出 F0"
        );
    }

    /// 等待秒数 0（默认）时**一个字节都不输出**；填了才出 `G4 P<毫秒>`。
    #[test]
    fn wait_seconds_only_emit_when_positive() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M190 S30",
            ";===== nozzle load line",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;

        let quiet = apply_bed_glue(src.clone(), &ir, crate::postproc::calibration::Mode::None);
        assert!(
            !quiet.iter().any(|l| l.starts_with("G4 P")),
            "0 秒不该有等待行"
        );

        ir.glue.bed_glue_wait_seconds = 1.5;
        let waited = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);
        assert!(
            waited.iter().any(|l| l.starts_with("G4 P1500")),
            "1.5 秒 ⇒ G4 P1500，实际：{:?}",
            waited
                .iter()
                .filter(|l| l.starts_with("G4"))
                .collect::<Vec<_>>()
        );
    }

    /// 涂胶块里**不许有阻塞式 `M109`**（2026-11-05 作者：胶涂完紧接着要升温，
    /// 停在那儿等喷嘴凉到 30 是白等）。喷嘴目标降温只能写 `M104`。
    #[test]
    fn nozzle_cooling_is_not_blocking() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M190 S70",
            ";===== nozzle load line",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;
        let out = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);

        let start = out
            .iter()
            .position(|l| l == BLOCK_START)
            .expect("有起始标记");
        let end = out.iter().position(|l| l == BLOCK_END).expect("有结束标记");
        assert!(
            !out[start..end]
                .iter()
                .any(|l| l.trim_start().starts_with("M109")),
            "涂胶块里出现了阻塞式 M109：{:?}",
            out[start..end]
                .iter()
                .filter(|l| l.contains("M10"))
                .collect::<Vec<_>>()
        );
        assert!(
            out[start..end]
                .iter()
                .any(|l| l.trim_start().starts_with("M104 S30")),
            "喷嘴降温该用不等待的 M104：{:?}",
            out[start..end]
                .iter()
                .filter(|l| l.contains("M10"))
                .collect::<Vec<_>>()
        );
    }

    /// 预热温度改成读预设：30 度时 `M190` 跟着变成 30，不再是写死的 50。
    #[test]
    fn preheat_temp_comes_from_the_preset() {
        let src = lines(&[
            "; EXECUTABLE_BLOCK_START",
            "M190 S70",
            ";===== nozzle load line",
        ]);
        let mut ir = Ir::default();
        ir.prefs.bed_glue_enabled = true;
        ir.glue.bed_glue_preheat_temp = 35.0;

        let out = apply_bed_glue(src, &ir, crate::postproc::calibration::Mode::None);
        assert!(
            out.iter().any(|l| l.starts_with("M190 S35")),
            "热床降到预设的 35 度，实际：{:?}",
            out.iter()
                .filter(|l| l.starts_with("M190"))
                .collect::<Vec<_>>()
        );
    }
}
