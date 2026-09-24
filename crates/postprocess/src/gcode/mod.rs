//! `gcode` —— G-code 纯算法原语（本项目最底层、最稳定的一层）。
//!
//! 硬边界（`tests/boundaries.rs` 的 `gcode_module_stays_pure` 守）：
//! 不引用本 crate 的其他任何模块（`crate::ir` / `crate::postproc` / `crate::pipeline` /
//! `crate::config`）、不依赖 tracing。
//! 它不知道 TOML / MKP / machine / wiping / tower / calibration / 编排层的存在。
//! 需要诊断就把信息返回给上层（`check_xy_range` 返回 `Vec<RangeViolation>` 就是这个形状）。
//!
//! **判据形态的降级要说清**：来源仓库这条边界由 gatecheck 读 Cargo.toml 实现
//! （反向依赖会被 Cargo 自己拦成 cyclic package dependency）。改成模块后 Cargo 不再兜底，
//! 换成文本扫描 —— 能力更弱（绕得过：换个写法引用），登记在 spec doc.md §3.1。
//!
//! 本模块内部的 `crate::…` 路径在搬家时被改写成 `crate::gcode::…`（16 行，
//! 因为 `crate::` 的含义从「gcode crate 根」变成了「postprocess crate 根」）；除此之外零改动。
//!
//! 移植对照（行为规格 = 旧仓库 `mkp-core/gcode`，不做文件级翻译，按语义重组）：
//! - `format`      ← offset_fmt.go（Format* / 伪随机 / MathRound）
//! - `line_builder`← line_builder.go（LineBuilder，字节级等价的追加式格式化）
//! - `parser`      ← parser.go（行级识别：FEATURE / layer num / Z_HEIGHT）
//! - `postprocess` ← postprocess.go（E 值读写 / filter_e_moves / remove_final_lower_z）
//! - `coord`       ← coord_extract.go / offset_byte.go / offset_replace.go
//! - `markers`     ← markers.go / mkp_markers.go
//! - `range_check` ← range_check.go
//! - `support_fallback` ← support_fallback.go
//!
//! 刻意**不移植**（属 audit/诊断面，后处理管线零消费，实测 grep 确认）：
//! invariants.go / state_audit.go / explain.go / risk.go / segment_explain.go /
//! stage_extract.go / operation_replay.go / transition_diff.go。

pub mod coord;
pub mod format;
pub mod header;
pub mod line_builder;
pub mod markers;
pub mod parser;
pub mod postprocess;
pub mod range_check;
pub mod support_fallback;

pub use coord::{XyzeValues, extract_coord, format_xyze_string, parse_xyze, replace_axis_value};
pub use format::{
    OffsetMode, format_e, format_e_value, format_float, format_python_float, format_speed,
    format_speed_int, format_z, get_pseudo_random, math_round, reset_pseudo_random,
};
pub use header::{
    machine_preset_name, process_preset_name, slicer_ironing_enabled, three_mf_file_name,
};
pub use line_builder::LineBuilder;
pub use markers::*;
pub use parser::{is_feature_line, is_layer_num_line, is_z_height_line, trim_line};
pub use postprocess::{
    filter_e_moves, get_e_value, has_e_param, remove_final_lower_z, remove_param_from_line,
    replace_e_value,
};
pub use range_check::{MachineMovementRange, RangeViolation, check_xy_range};
pub use support_fallback::detect_support_fallback;

/// Go `strconv.ParseFloat` 的接受面镜像。
///
/// 为什么不直接用 `str::parse::<f64>()`：两者在真实 G-code 里可分叉——
/// Go 接受尾部小数点（"5."、"+5.e2"），Rust 拒绝。`Z.` / `E5.` 这类写法
/// 在旧实现的扫描语义下是合法命中，直接 parse 会把「找到」变成「没找到」。
pub(crate) fn parse_float_go(s: &str) -> Option<f64> {
    if let Ok(v) = s.parse::<f64>() {
        return Some(v);
    }
    // 尾随小数点：在小数点后补一个 '0' 再试（覆盖 "5." 与 "5.e2" 两形态）。
    // 前提：点前必须有至少一个数字——孤点 "." / "-." 在 Go 里同样非法。
    if let Some(dot) = s.rfind('.') {
        let after = &s[dot + 1..];
        let boundary_ok = after.is_empty() || after.starts_with('e') || after.starts_with('E');
        let has_digit_before = s[..dot].bytes().any(|b| b.is_ascii_digit());
        if boundary_ok && has_digit_before {
            let mut fixed = String::with_capacity(s.len() + 1);
            fixed.push_str(&s[..dot + 1]);
            fixed.push('0');
            fixed.push_str(after);
            if let Ok(v) = fixed.parse::<f64>() {
                return Some(v);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Go/Rust 接受面差异的锁定用例：尾部小数点 Go 接受、Rust 原生拒绝。
    #[test]
    fn parse_float_go_accepts_go_forms() {
        assert_eq!(parse_float_go("5."), Some(5.0));
        assert_eq!(parse_float_go("+5."), Some(5.0));
        assert_eq!(parse_float_go("5.e2"), Some(500.0));
        assert_eq!(parse_float_go(".5"), Some(0.5));
        assert_eq!(parse_float_go("-.5"), Some(-0.5));
        assert_eq!(parse_float_go("1.2.3"), None);
        assert_eq!(parse_float_go(""), None);
        assert_eq!(parse_float_go("."), None);
    }
}
