//! `process_gcode_offset` —— 应用 XYZE 偏移（对照 gcodebiz/offset.go 逐行）。
//!
//! 纯字节操作来自 mkp-gcode（format_xyze_string / parse_xyze / replace_axis_value /
//! format_float）；本函数承载依赖 ir 的领域语义：挤出比例、边界与涂胶范围校验。

use crate::diag::PostprocError;
use crate::diag::messages;
use crate::gcode::{
    OffsetMode, XyzeValues, format_float, format_xyze_string, parse_xyze, replace_axis_value,
};
use crate::ir::Ir;
use crate::postproc::geom::{MoveDirection, ceil_to_01};

/// 速度发射点的 mm/s→mm/min 换算常量。
///
/// 诚实登记：gatecheck `speed_unit_conversion_only_in_ir` 的意图是「换算不许散布」，
/// 但旧 Go 侧的 IR.Machine.TravelSpeed 保持 mm/s，换算发生在各发射点（generator.go
/// 的 `TravelSpeed*60` 等）。字节判据优先于重排，Rust 按 AGENTS §4 的「常量具名」
/// 路径在每个发射点显式引用本常量，不做静默数字。
pub(crate) const MM_PER_MINUTE: f64 = 60.0;

/// 一条越界（对照 calibrationfmt.FormatUserMessage 可见部分）。
///
/// ★ `code` 而不是 `title`（2026-10-11 改）：以前这里存的是中文标题
/// （"模型超出打印边界"），出码的时候**拿标题字符串去比** ——
/// `if be.title == "模型超出打印边界" { "E_…_001" } else { "E_…_002" }`。
/// 那就是"文案即逻辑"：改一个字，错误码就换了一条。现在码是本字段，句子在 `messages.toml`。
struct BoundaryError {
    /// 两个边界码之一：`E_GCODE_BOUNDARY_001`（打印边界）/ `_002`（涂胶范围）。
    code: &'static str,
    axis_x: bool,
    current_x: f64,
    limit_x_min: f64,
    limit_x_max: f64,
    required_move_x: f64,
    axis_y: bool,
    current_y: f64,
    limit_y_min: f64,
    limit_y_max: f64,
    required_move_y: f64,
}

impl BoundaryError {
    fn direction(&self) -> MoveDirection {
        let (mut dx, mut dy) = (0, 0);
        if self.axis_x {
            if self.current_x < self.limit_x_min {
                dx = 1;
            } else if self.current_x > self.limit_x_max {
                dx = -1;
            }
        }
        if self.axis_y {
            if self.current_y < self.limit_y_min {
                dy = 1;
            } else if self.current_y > self.limit_y_max {
                dy = -1;
            }
        }
        MoveDirection::from_delta(dx, dy)
    }

    /// 越界那句的**从句串**（形如 `当前X：185.9mm，允许范围：≤180.0mm`）。
    ///
    /// 一个字都不写死：从句模板、上下限的说法（`≥` / `≤`）、连两段之间的分隔符，
    /// 全在 `messages.toml` 那几条 `boundary.*` 里。本函数只做表做不了的那件事 ——
    /// X 与 Y **同时**越界时要拼两段（模板语言里没有循环）。
    fn sides(&self) -> String {
        /* 一条轴的说法 = 「现在是多少、允许到哪」+「往哪边挪多少」。
         * 后者是 2026-11-19 加的：只报"超了 9mm"用户还得自己算挪哪边。 */
        let clause = |axis: char, current: f64, lo: f64, hi: f64, required_move: f64| {
            // 只剩一种可能：出到界外了（调用点只在越界时才建这条），所以不是下界就是上界
            let below = current < lo;
            let (key, bound) = if below {
                ("boundary.limit.min", lo)
            } else {
                ("boundary.limit.max", hi)
            };
            let limit = messages::text(key, &[("value", &format!("{bound:.1}"))]);
            let current = format!("{current:.1}");
            let axis = axis.to_string();
            let head = messages::text(
                "boundary.clause",
                &[("axis", &axis), ("current", &current), ("limit", &limit)],
            );
            /* 只报超出多少：往哪挪由报告页那张图上的箭头说（2026-11-19） */
            let mm = format!("{required_move:.1}");
            format!("{head}{}", messages::text("boundary.over", &[("mm", &mm)]))
        };
        let mut parts: Vec<String> = Vec::new();
        if self.axis_x {
            parts.push(clause(
                'X',
                self.current_x,
                self.limit_x_min,
                self.limit_x_max,
                self.required_move_x,
            ));
        }
        if self.axis_y {
            parts.push(clause(
                'Y',
                self.current_y,
                self.limit_y_min,
                self.limit_y_max,
                self.required_move_y,
            ));
        }
        parts.join(&messages::text("boundary.separator", &[]))
    }
}

/// `ProcessGCodeOffset`：应用 XYZE 偏移并返回修正后的 G-code 行。
pub fn process_gcode_offset(
    gcommand: &str,
    x_offset: f64,
    y_offset: f64,
    z_offset: f64,
    mode: OffsetMode,
    ir_data: &Ir,
) -> Result<String, PostprocError> {
    let (mut cmd_part, comment_part) = match gcommand.find(';') {
        Some(idx) => (&gcommand[..idx], &gcommand[idx..]),
        None => (gcommand, ""),
    };
    if let Some(f_idx) = cmd_part.find('F') {
        cmd_part = &cmd_part[..f_idx];
    }
    let mut gcommand = format!("{cmd_part}{comment_part}");

    gcommand = format_xyze_string(&gcommand);

    let raw_vals = parse_xyze(gcommand.as_bytes());

    let mut values = XyzeValues::default();
    if raw_vals.has_x {
        values.x = ((raw_vals.x + x_offset) * 1000.0).round() / 1000.0;
        values.has_x = true;
    }
    if raw_vals.has_y {
        values.y = ((raw_vals.y + y_offset) * 1000.0).round() / 1000.0;
        values.has_y = true;
    }
    if raw_vals.has_z && mode != OffsetMode::Ironing {
        values.z = ((raw_vals.z + z_offset) * 1000.0).round() / 1000.0;
        values.has_z = true;
    }
    if raw_vals.has_e {
        match mode {
            OffsetMode::Ironing => {
                values.e = (raw_vals.e * ir_data.ironing.extrude_ratio * 1000.0).round() / 1000.0;
                values.has_e = true;
            }
            OffsetMode::Tower => {
                values.e = (raw_vals.e * ir_data.tower.extrude_ratio * 1000.0).round() / 1000.0;
                values.has_e = true;
            }
            OffsetMode::Normal | OffsetMode::Calibration => {
                // E 不存储 —— 稍后由 ReplaceAxisValue 剥离
            }
        }
    }

    if mode == OffsetMode::Normal || mode == OffsetMode::Calibration {
        let mut boundary_err: Option<BoundaryError> = None;

        if values.has_x {
            let x_after = values.x;
            if x_after < ir_data.machine.min_x || x_after > ir_data.machine.max_x {
                let required_move_x = if x_after < ir_data.machine.min_x {
                    ceil_to_01(ir_data.machine.min_x - x_after)
                } else {
                    ceil_to_01(x_after - ir_data.machine.max_x)
                };
                boundary_err = Some(BoundaryError {
                    code: "E_GCODE_BOUNDARY_001",
                    axis_x: true,
                    current_x: x_after,
                    limit_x_min: ir_data.machine.min_x,
                    limit_x_max: ir_data.machine.max_x,
                    required_move_x,
                    axis_y: false,
                    current_y: 0.0,
                    limit_y_min: 0.0,
                    limit_y_max: 0.0,
                    required_move_y: 0.0,
                });
            } else if mode == OffsetMode::Normal && boundary_err.is_none() {
                if ir_data.machine.glue_min_x > ir_data.machine.min_x
                    && x_after < ir_data.machine.glue_min_x
                {
                    boundary_err = Some(BoundaryError {
                        code: "E_GCODE_BOUNDARY_002",
                        axis_x: true,
                        current_x: x_after,
                        limit_x_min: ir_data.machine.glue_min_x,
                        limit_x_max: ir_data.machine.glue_max_x,
                        required_move_x: ceil_to_01(ir_data.machine.glue_min_x - x_after),
                        axis_y: false,
                        current_y: 0.0,
                        limit_y_min: 0.0,
                        limit_y_max: 0.0,
                        required_move_y: 0.0,
                    });
                } else if ir_data.machine.glue_max_x < ir_data.machine.max_x
                    && x_after > ir_data.machine.glue_max_x
                {
                    boundary_err = Some(BoundaryError {
                        code: "E_GCODE_BOUNDARY_002",
                        axis_x: true,
                        current_x: x_after,
                        limit_x_min: ir_data.machine.glue_min_x,
                        limit_x_max: ir_data.machine.glue_max_x,
                        required_move_x: ceil_to_01(x_after - ir_data.machine.glue_max_x),
                        axis_y: false,
                        current_y: 0.0,
                        limit_y_min: 0.0,
                        limit_y_max: 0.0,
                        required_move_y: 0.0,
                    });
                }
            }
        }
        if values.has_y {
            let y_after = values.y;
            if y_after < ir_data.machine.min_y || y_after > ir_data.machine.max_y {
                let required_move_y = if y_after < ir_data.machine.min_y {
                    ceil_to_01(ir_data.machine.min_y - y_after)
                } else {
                    ceil_to_01(y_after - ir_data.machine.max_y)
                };
                boundary_err = match boundary_err {
                    None => Some(BoundaryError {
                        code: "E_GCODE_BOUNDARY_001",
                        axis_x: false,
                        current_x: 0.0,
                        limit_x_min: 0.0,
                        limit_x_max: 0.0,
                        required_move_x: 0.0,
                        axis_y: true,
                        current_y: y_after,
                        limit_y_min: ir_data.machine.min_y,
                        limit_y_max: ir_data.machine.max_y,
                        required_move_y,
                    }),
                    Some(mut be) => {
                        be.axis_y = true;
                        be.current_y = y_after;
                        be.limit_y_min = ir_data.machine.min_y;
                        be.limit_y_max = ir_data.machine.max_y;
                        be.required_move_y = required_move_y;
                        Some(be)
                    }
                };
            } else if mode == OffsetMode::Normal && boundary_err.is_none() {
                if ir_data.machine.glue_min_y > ir_data.machine.min_y
                    && y_after < ir_data.machine.glue_min_y
                {
                    boundary_err = Some(BoundaryError {
                        code: "E_GCODE_BOUNDARY_002",
                        axis_x: false,
                        current_x: 0.0,
                        limit_x_min: 0.0,
                        limit_x_max: 0.0,
                        required_move_x: 0.0,
                        axis_y: true,
                        current_y: y_after,
                        limit_y_min: ir_data.machine.glue_min_y,
                        limit_y_max: ir_data.machine.glue_max_y,
                        required_move_y: ceil_to_01(ir_data.machine.glue_min_y - y_after),
                    });
                } else if ir_data.machine.glue_max_y > 0.0
                    && ir_data.machine.glue_max_y < ir_data.machine.max_y
                    && y_after > ir_data.machine.glue_max_y
                {
                    boundary_err = Some(BoundaryError {
                        code: "E_GCODE_BOUNDARY_002",
                        axis_x: false,
                        current_x: 0.0,
                        limit_x_min: 0.0,
                        limit_x_max: 0.0,
                        required_move_x: 0.0,
                        axis_y: true,
                        current_y: y_after,
                        limit_y_min: ir_data.machine.glue_min_y,
                        limit_y_max: ir_data.machine.glue_max_y,
                        required_move_y: ceil_to_01(y_after - ir_data.machine.glue_max_y),
                    });
                }
            }
        }

        /* 调试那一趟整段产物都要被替换掉（只剩动作），模型越不越界与它无关；
         * 而且调试动作本身就走床外（落笔 267 / 收笔 276），拿模型的边界去卡它
         * 只会把这一趟白挡掉（2026-11-12 作者：调试模式优先，这些都不用管）。
         * `filter` 就是这条豁免：调试在跑时把已经查出来的越界丢掉。 */
        let boundary_err = boundary_err.filter(|_| ir_data.debug.is_none());
        if let Some(be) = boundary_err {
            let _ = (be.direction(), be.required_move_x, be.required_move_y);
            /* 码随 `be` 走，正文（从句）由表渲染 —— 这里不再有字符串比较，也不再有
            `E_…: ` 那一截自我引用（旧形态：`InvalidConfig` 里塞一句带码的正文，
            于是外层码 "配置无效" 在说假话，见 `diag/error.rs` 的 `Boundary` 文档） */
            return Err(PostprocError::Boundary {
                code: be.code,
                sides: be.sides(),
            });
        }
    }

    if values.has_x {
        gcommand = replace_axis_value(&gcommand, b'X', &format!("X{}", format_float(values.x)));
    }
    if values.has_y {
        gcommand = replace_axis_value(&gcommand, b'Y', &format!("Y{}", format_float(values.y)));
    }
    if values.has_z {
        gcommand = replace_axis_value(&gcommand, b'Z', &format!("Z{}", format_float(values.z)));
    }
    if values.has_e {
        gcommand = replace_axis_value(&gcommand, b'E', &format!("E{}", format_float(values.e)));
    }

    if mode == OffsetMode::Normal || mode == OffsetMode::Calibration {
        let (cmd_part, comment_part) = match gcommand.find(';') {
            Some(idx) => (&gcommand[..idx], &gcommand[idx..]),
            None => (gcommand.as_str(), ""),
        };
        let cmd_part = replace_axis_value(cmd_part, b'E', "");
        gcommand = format!("{cmd_part}{comment_part}");
    }

    Ok(gcommand.trim_start_matches([' ', '\t']).to_string())
}

/// 提前粗检的余量（mm）：预扫的样本比逐行那套宽，留这一截防误拦。
pub(crate) const EARLY_BOUNDARY_MARGIN_MM: f64 = 1.0;

/// 哪些行属于**打印机自己的段**（机器起手 / 结束），不是模型。
///
/// 这些段里的坐标是机器动作：A1 的擦嘴位就是 `Y262.5`（比打印范围的 `maxY = 255`
/// 还远 7.5mm），带上笔尖偏移 24.4 就是 286.9。把它们算进"模型边界"会把正常的
/// A1 打印/校准整批拦下来。
///
/// 配对见 [`opens_machine_block`] / [`closes_machine_block`]：起点没等到终点的
/// **整段不跳**（宁可多检查，不许漏）。
fn machine_block_mask(content: &[String]) -> Vec<bool> {
    let mut mask = vec![false; content.len()];
    let mut open: Option<usize> = None;
    for (i, line) in content.iter().enumerate() {
        let head = line.trim_start();
        if opens_machine_block(head) && open.is_none() {
            open = Some(i);
        } else if closes_machine_block(head)
            && let Some(start) = open.take()
        {
            for slot in &mut mask[start..=i] {
                *slot = true;
            }
        }
    }
    mask
}

/// 机器段的起点：Bambu 的 `;===== machine: …`、Orca 的 `; MACHINE_START_GCODE_START`、
/// 以及结束段的 `; MACHINE_END_GCODE_START`。
fn opens_machine_block(head: &str) -> bool {
    head.starts_with(";===== machine:")
        || head.starts_with("; MACHINE_START_GCODE_START")
        || head.starts_with("; MACHINE_END_GCODE_START")
}

/// 机器段的终点。Bambu 的结束段没有 `MACHINE_END_GCODE_END`，只能靠
/// `; EXECUTABLE_BLOCK_END` 收尾。
fn closes_machine_block(head: &str) -> bool {
    head.starts_with("; MACHINE_START_GCODE_END")
        || head.starts_with("; MACHINE_END_GCODE_END")
        || head.starts_with("; EXECUTABLE_BLOCK_END")
}

/// 模型路径的 X/Y 极值（**G-code 原坐标**，机器起手 / 结束段不算）。
///
/// 某一轴一个点都没有 ⇒ `None`（不拿 0 顶替）。
#[derive(Debug, Clone, Copy, Default)]
struct XyLimits {
    x: Option<(f64, f64)>,
    y: Option<(f64, f64)>,
}

fn scan_xy_limits(content: &[String]) -> XyLimits {
    let skip = machine_block_mask(content);
    let mut limits = XyLimits::default();
    let (mut min_x, mut max_x) = (f64::INFINITY, f64::NEG_INFINITY);
    let (mut min_y, mut max_y) = (f64::INFINITY, f64::NEG_INFINITY);
    for (i, line) in content.iter().enumerate() {
        if skip[i] {
            continue;
        }
        let head = line.trim_start();
        /* 切片器出来的坐标都在 G0 / G1 上，别的行不看 */
        if !(head.starts_with("G1") || head.starts_with("G0")) {
            continue;
        }
        let v = parse_xyze(head.as_bytes());
        if v.has_x {
            min_x = min_x.min(v.x);
            max_x = max_x.max(v.x);
        }
        if v.has_y {
            min_y = min_y.min(v.y);
            max_y = max_y.max(v.y);
        }
    }
    if min_x.is_finite() {
        limits.x = Some((min_x, max_x));
    }
    if min_y.is_finite() {
        limits.y = Some((min_y, max_y));
    }
    limits
}

/// 跑管线之前的**粗检**：拿模型路径的 X/Y 极值 + 笔偏移对一遍板范围，明显越界就立刻报。
///
/// # 它不替掉逐行那道闸
///
/// 真正的判据仍是 [`process_gcode_offset`]：它只看"这一行会不会被写到"，精确到单点。
/// 这里拿的是全文极值，样本更宽（有些行 Pass1 不会过偏移），所以**只拦明显越界**
/// （留 [`EARLY_BOUNDARY_MARGIN_MM`] 的余量）；边缘那一档仍交给逐行。
///
/// 收益：大文件不必等 Pass1 跑到出事那一行（实测 9 秒）才失败。
///
/// # 三条与逐行一致的规矩
///
/// - 机器起手 / 结束段不查（[`machine_block_mask`]）：那不是模型；
/// - 调试那一趟不查：那一趟的产物只跑笔的动作，正式坐标本来就走出床外；
/// - 只报 `E_GCODE_BOUNDARY_001`（打印板）：涂胶范围 `_002` 要按涂胶路径判，
///   极值不够用，留给逐行。
pub(crate) fn check_xy_bounds_early(
    content: &[String],
    ir_data: &Ir,
    margin: f64,
) -> Result<(), PostprocError> {
    if ir_data.debug.is_some() {
        return Ok(());
    }

    let limits = scan_xy_limits(content);
    let machine = &ir_data.machine;
    let (xl, xh) = (machine.min_x - margin, machine.max_x + margin);
    let (yl, yh) = (machine.min_y - margin, machine.max_y + margin);

    let mut err: Option<BoundaryError> = None;
    if let Some((min_x, max_x)) = limits.x {
        let (lo, hi) = (
            min_x + ir_data.toolhead.x_offset,
            max_x + ir_data.toolhead.x_offset,
        );
        let hit = if lo < xl {
            Some((lo, ceil_to_01(machine.min_x - lo)))
        } else if hi > xh {
            Some((hi, ceil_to_01(hi - machine.max_x)))
        } else {
            None
        };
        if let Some((current, required_move)) = hit {
            err = Some(BoundaryError {
                code: "E_GCODE_BOUNDARY_001",
                axis_x: true,
                current_x: current,
                limit_x_min: machine.min_x,
                limit_x_max: machine.max_x,
                required_move_x: required_move,
                axis_y: false,
                current_y: 0.0,
                limit_y_min: 0.0,
                limit_y_max: 0.0,
                required_move_y: 0.0,
            });
        }
    }
    if let Some((min_y, max_y)) = limits.y {
        let (lo, hi) = (
            min_y + ir_data.toolhead.y_offset,
            max_y + ir_data.toolhead.y_offset,
        );
        let hit = if lo < yl {
            Some((lo, ceil_to_01(machine.min_y - lo)))
        } else if hi > yh {
            Some((hi, ceil_to_01(hi - machine.max_y)))
        } else {
            None
        };
        if let Some((current, required_move)) = hit {
            err = match err {
                None => Some(BoundaryError {
                    code: "E_GCODE_BOUNDARY_001",
                    axis_x: false,
                    current_x: 0.0,
                    limit_x_min: 0.0,
                    limit_x_max: 0.0,
                    required_move_x: 0.0,
                    axis_y: true,
                    current_y: current,
                    limit_y_min: machine.min_y,
                    limit_y_max: machine.max_y,
                    required_move_y: required_move,
                }),
                /* 两轴都越界：并成一句（与逐行那条一致） */
                Some(mut be) => {
                    be.axis_y = true;
                    be.current_y = current;
                    be.limit_y_min = machine.min_y;
                    be.limit_y_max = machine.max_y;
                    be.required_move_y = required_move;
                    Some(be)
                }
            };
        }
    }

    match err {
        Some(be) => Err(PostprocError::Boundary {
            code: be.code,
            sides: be.sides(),
        }),
        None => Ok(()),
    }
}

#[cfg(test)]
mod early_boundary_tests {
    use super::*;

    /// 板子 0..256 × 0..180（A 系列的 Y 上限）；笔偏移由参数给。
    fn lines(text: &str) -> Vec<String> {
        text.lines().map(str::to_owned).collect()
    }

    fn ir_with_pen_y_offset(y_offset: f64) -> Ir {
        let mut ir = Ir::default();
        ir.machine.min_x = 0.0;
        ir.machine.max_x = 256.0;
        ir.machine.min_y = 0.0;
        ir.machine.max_y = 180.0;
        ir.toolhead.x_offset = 0.0;
        ir.toolhead.y_offset = y_offset;
        ir
    }

    /// 模型本身在板内（Y 179），加笔偏移到 181 ⇒ 粗检立刻报，口径与逐行那条一致。
    #[test]
    fn early_check_reports_the_same_boundary_before_pass1() {
        let ir_data = ir_with_pen_y_offset(2.0);
        let content = lines("G1 X10 Y179.5 F3000\nG1 X20 Y179.5 F3000\n");
        let err = check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM)
            .expect_err("179.5 + 2 = 181.5，越过 180（超出 1.0mm 余量）");
        match err {
            PostprocError::Boundary { code, sides } => {
                assert_eq!(code, "E_GCODE_BOUNDARY_001");
                assert!(sides.contains("181.5"), "要报偏移后的那个数：{sides}");
                assert!(sides.contains("180.0"), "要报允许范围：{sides}");
            }
            other => panic!("该报边界错，实际 {other:?}"),
        }
    }

    /// 只越 0.4mm（在余量内）⇒ 粗检不拦，留给逐行那道闸。
    #[test]
    fn early_check_leaves_edge_cases_to_the_per_line_gate() {
        let ir_data = ir_with_pen_y_offset(0.4);
        let content = lines("G1 X10 Y179.6 F3000\n");
        assert!(
            check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM).is_ok(),
            "余量内的越界不该由粗检拦下"
        );
    }

    /// 越界那句只报**超出多少**（2026-11-19）：方向交给报告页那张图上的箭头，
    /// 句子里不再出现 `Y-` 这种写法。
    #[test]
    fn the_boundary_reports_only_how_far_it_spills() {
        let ir_data = ir_with_pen_y_offset(2.0);
        let content = lines("G1 X10.0 Y179.5 F3000\n");
        let err = check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM)
            .expect_err("181.5 越过 180");
        match err {
            PostprocError::Boundary { sides, .. } => {
                assert!(sides.contains("1.5"), "超出量要报出来：{sides}");
                assert!(!sides.contains("Y-"), "不再写方向：{sides}");
                assert!(sides.contains("超出"), "要有那一截：{sides}");
            }
            other => panic!("该报边界错，实际 {other:?}"),
        }
    }

    /// 调试那一趟：正式坐标本来就走出床外（落笔 267 / 收笔 276），与逐行同一条豁免。
    #[test]
    fn early_check_skips_the_debug_run() {
        let mut ir_data = ir_with_pen_y_offset(0.0);
        ir_data.debug = Some(crate::ir::types::DebugIr::default());
        let content = lines("G1 X20 Y276 F10000\n");
        assert!(
            check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM).is_ok(),
            "调试那一趟不许被模型的边界挡住"
        );
    }

    /// A1 的机器起手段真的会走到 `Y262.5`（擦嘴），加笔尖偏移 24.4 就是 286.9 ——
    /// 那不是模型，粗检不许拿它判越界（2026-11-18 实机：校准件就是这么被拦下的）。
    #[test]
    fn early_check_skips_the_machine_start_block() {
        let mut ir_data = ir_with_pen_y_offset(24.4);
        ir_data.machine.max_y = 255.0;
        let content = lines(concat!(
            ";===== machine: A1 =========================\n",
            "G1 Y250 F30000\n",
            "G1 Y262.5 F6000\n",
            "; MACHINE_START_GCODE_END\n",
            "G1 X10 Y100 F3000\n",
        ));
        assert!(
            check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM).is_ok(),
            "机器起手段的擦嘴位不是模型"
        );
    }

    /// 机器结束段同理（AMS 拉丝 / 归位都在那段里，坐标同样走出板外）。
    #[test]
    fn early_check_skips_the_machine_end_block() {
        let ir_data = ir_with_pen_y_offset(0.0);
        let content = lines(concat!(
            "G1 X10 Y100 F3000\n",
            "; MACHINE_END_GCODE_START\n",
            "G1 X267 F15000\n",
            "; EXECUTABLE_BLOCK_END\n",
        ));
        assert!(
            check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM).is_ok(),
            "机器结束段不是模型"
        );
    }

    /// 段外同样的坐标仍要拦：跳过只看段边界，不碰判据本身。
    #[test]
    fn early_check_still_reports_outside_the_machine_block() {
        let mut ir_data = ir_with_pen_y_offset(24.4);
        ir_data.machine.max_y = 255.0;
        let content = lines(concat!(
            ";===== machine: A1 =====\n",
            "; MACHINE_START_GCODE_END\n",
            "G1 Y262.5 F6000\n",
        ));
        let err = check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM)
            .expect_err("起手段之后的 262.5 是模型坐标，仍要报");
        match err {
            PostprocError::Boundary { code, .. } => assert_eq!(code, "E_GCODE_BOUNDARY_001"),
            other => panic!("该报边界错，实际 {other:?}"),
        }
    }

    /// 起点没等到终点 ⇒ **整段不跳**（宁可多检查，不许把整个文件漏过去）。
    #[test]
    fn an_unclosed_machine_block_is_not_skipped() {
        let mut ir_data = ir_with_pen_y_offset(24.4);
        ir_data.machine.max_y = 255.0;
        let content = lines(";===== machine: A1 =====\nG1 Y262.5 F6000\n");
        assert!(
            check_xy_bounds_early(&content, &ir_data, EARLY_BOUNDARY_MARGIN_MM).is_err(),
            "没有收尾标记就照常检查"
        );
    }
}
