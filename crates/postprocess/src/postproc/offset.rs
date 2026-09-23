//! `process_gcode_offset` —— 应用 XYZE 偏移（对照 gcodebiz/offset.go 逐行）。
//!
//! 纯字节操作来自 mkp-gcode（format_xyze_string / parse_xyze / replace_axis_value /
//! format_float）；本函数承载依赖 ir 的领域语义：挤出比例、边界与涂胶范围校验。

use crate::diag::PostprocError;
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

/// 边界错误的用户可见消息（对照 calibrationfmt.FormatUserMessage 可见部分）。
struct BoundaryError {
    title: &'static str,
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

    fn user_message(&self) -> String {
        let mut parts: Vec<String> = vec![self.title.to_string()];
        if self.axis_x {
            parts.push(format!("当前X：{:.1}mm", self.current_x));
            let range_str = if self.current_x < self.limit_x_min {
                format!("≥{:.1}mm", self.limit_x_min)
            } else if self.current_x > self.limit_x_max {
                format!("≤{:.1}mm", self.limit_x_max)
            } else {
                String::new()
            };
            parts.push(format!("允许范围：{range_str}"));
        }
        if self.axis_y {
            parts.push(format!("当前Y：{:.1}mm", self.current_y));
            let range_str = if self.current_y < self.limit_y_min {
                format!("≥{:.1}mm", self.limit_y_min)
            } else if self.current_y > self.limit_y_max {
                format!("≤{:.1}mm", self.limit_y_max)
            } else {
                String::new()
            };
            parts.push(format!("允许范围：{range_str}"));
        }
        parts.join("，")
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
                    title: "模型超出打印边界",
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
                        title: "模型超出涂胶范围",
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
                        title: "模型超出涂胶范围",
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
                        title: "模型超出打印边界",
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
                        title: "模型超出涂胶范围",
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
                        title: "模型超出涂胶范围",
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

        if let Some(be) = boundary_err {
            let _ = (be.direction(), be.required_move_x, be.required_move_y);
            let code = if be.title == "模型超出打印边界" {
                "E_GCODE_BOUNDARY_001"
            } else {
                "E_GCODE_BOUNDARY_002"
            };
            return Err(PostprocError::InvalidConfig {
                message: format!("{code}: {}", be.user_message()),
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
