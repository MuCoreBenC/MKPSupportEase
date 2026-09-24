//! 从来源仓库 `crates/ir/src/build.rs`（1,186 行）里**只摘两个函数**。
//!
//! **没搬的是什么**：那 1,186 行里约 1,130 行是 `TomlConfig → Ir` 的映射
//! （`build()` 从 :44 到 :354，加 `new_default_ir()` 等辅助）。它的唯一输入类型是
//! `mkp_preset::TomlConfig` —— 本项目**不要预设文件**，所以那段连同整个 `preset` crate
//! 一起消失。配置从此直接反序列化成 [`crate::ir::Ir`]（spec doc.md §4 方案 C）。
//!
//! **搬进来的两个是 postproc 对 ir 的全部函数依赖**（实测 grep：postproc 只用
//! `Ir` 类型 + 这两个函数，共 3 样）：
//! - [`fill_defaults`]（来源 build.rs:359-382，24 行）
//! - [`num_strip`]（来源 build.rs:915-944，30 行）
//!
//! 两个函数的**函数体逐字未改**。改的只有文档注释里的行号引用。

use crate::ir::Ir;

/// `FillDefaults`（Go build.go:526）：`build()` 刻意**不含** FillDefaults ——
/// 它要在 G-code 元数据提取之后同一时机调用（保持与 Go `ConfigToParams` 的等价边界）。
///
/// 本项目里没有 `build()` 了，所以这条时序约束变成：**配置反序列化之后、pass1 之前**
/// 由 pipeline 的 config 步调用一次。调用点还有一个在
/// `postproc::calibration::insert_calibration_gcode` 首行（来源仓库同样如此）。
///
/// 一共 8 条规则，逐条核对过：6 个「== 0.0 才填」的零值兜底、1 个 tree_support 强制、
/// 1 个**无条件**覆写 `filament.slicer`。最后那条不是兜底 —— 它无论原值是什么都会被冲掉。
pub fn fill_defaults(ir: &mut Ir) {
    if ir.machine.travel_speed == 0.0 {
        ir.machine.travel_speed = 150.0;
    }
    if ir.machine.nozzle_diameter == 0.0 {
        ir.machine.nozzle_diameter = 0.4;
    }
    if ir.machine.first_layer_height == 0.0 {
        ir.machine.first_layer_height = 0.2;
    }
    if ir.machine.typical_layer_height == 0.0 {
        ir.machine.typical_layer_height = 0.2;
    }
    if ir.machine.retract_length == 0.0 {
        ir.machine.retract_length = 0.4;
    }
    if ir.machine.wall_print_speed == 0.0 {
        ir.machine.wall_print_speed = 60.0;
    }
    if ir.machine.tree_support && ir.wiping.support_extrusion_multiplier != 1.27 {
        ir.wiping.support_extrusion_multiplier = 1.27;
    }
    ir.filament.slicer = "BambuStudio".to_string();
}

/// NumStrip：从字符串提取全部数字（可负、可小数；孤立 `-` / `.` 不算）。
///
/// pub 的理由（来源仓库原话）：Go 侧 `ir.NumStrip` 是导出符号，pass1 的元数据扫描直接
/// 调它（`pass1.go:215` 等 12 处）。本项目里消费方是 `postproc::pass1` 与 `postproc::pass2`。
pub fn num_strip(line: &str) -> Vec<f64> {
    let mut result = Vec::with_capacity(8);
    let mut buf = String::with_capacity(64);
    let mut has_dot = false;
    fn flush(buf: &mut String, has_dot: &mut bool, out: &mut Vec<f64>) {
        if !buf.is_empty()
            && buf.as_str() != "-"
            && buf.as_str() != "."
            && let Ok(v) = buf.parse::<f64>()
        {
            out.push(v);
        }
        buf.clear();
        *has_dot = false;
    }
    for c in line.bytes() {
        if c == b'-' && buf.is_empty() {
            buf.push('-');
        } else if c.is_ascii_digit() {
            buf.push(c as char);
        } else if c == b'.' && !has_dot && !buf.is_empty() {
            buf.push('.');
            has_dot = true;
        } else {
            flush(&mut buf, &mut has_dot, &mut result);
        }
    }
    flush(&mut buf, &mut has_dot, &mut result);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 6 条零值兜底：**只在原值为 0.0 时**生效，非零值不许被冲掉。
    ///
    /// 为什么这条判据值得存在（而其余搬家代码不逐个加测）：这两个函数是**手工从 1,186 行里
    /// 摘出来的**，摘漏一条 `if` 或把 `==` 抄成 `!=` 不会有任何编译错误，而 G1/G2 用的
    /// fixture 里这些字段全是非零值 ⇒ 字节判据**盖不住零值分支**。
    #[test]
    fn zero_valued_machine_fields_get_defaults() {
        let mut ir = Ir::default();
        fill_defaults(&mut ir);
        assert_eq!(ir.machine.travel_speed, 150.0);
        assert_eq!(ir.machine.nozzle_diameter, 0.4);
        assert_eq!(ir.machine.first_layer_height, 0.2);
        assert_eq!(ir.machine.typical_layer_height, 0.2);
        assert_eq!(ir.machine.retract_length, 0.4);
        assert_eq!(ir.machine.wall_print_speed, 60.0);
    }

    /// 反向：非零值必须原样保留（证明上面那条不是「无条件写死」）。
    #[test]
    fn nonzero_machine_fields_are_preserved() {
        let mut ir = Ir::default();
        ir.machine.travel_speed = 111.0;
        ir.machine.nozzle_diameter = 0.6;
        ir.machine.first_layer_height = 0.3;
        ir.machine.typical_layer_height = 0.15;
        ir.machine.retract_length = 0.8;
        ir.machine.wall_print_speed = 42.0;
        fill_defaults(&mut ir);
        assert_eq!(ir.machine.travel_speed, 111.0);
        assert_eq!(ir.machine.nozzle_diameter, 0.6);
        assert_eq!(ir.machine.first_layer_height, 0.3);
        assert_eq!(ir.machine.typical_layer_height, 0.15);
        assert_eq!(ir.machine.retract_length, 0.8);
        assert_eq!(ir.machine.wall_print_speed, 42.0);
    }

    /// 第 7 条：tree_support 为真时**强制** 1.27，用户填的值会被冲掉。
    #[test]
    fn tree_support_forces_support_extrusion_multiplier() {
        let mut ir = Ir::default();
        ir.machine.tree_support = true;
        ir.wiping.support_extrusion_multiplier = 1.0;
        fill_defaults(&mut ir);
        assert_eq!(ir.wiping.support_extrusion_multiplier, 1.27);
    }

    /// 第 7 条的反向：tree_support 为假时不碰这个字段。
    #[test]
    fn without_tree_support_multiplier_is_untouched() {
        let mut ir = Ir::default();
        ir.machine.tree_support = false;
        ir.wiping.support_extrusion_multiplier = 1.0;
        fill_defaults(&mut ir);
        assert_eq!(ir.wiping.support_extrusion_multiplier, 1.0);
    }

    /// 第 8 条：`slicer` 是**无条件**覆写，不是兜底 —— 这个区别写在这里，
    /// 因为它是唯一一条「用户在配置里填了也没用」的规则。
    #[test]
    fn slicer_is_overwritten_unconditionally() {
        let mut ir = Ir::default();
        ir.filament.slicer = "OrcaSlicer".to_string();
        fill_defaults(&mut ir);
        assert_eq!(ir.filament.slicer, "BambuStudio");
    }

    /// `num_strip`：负数、小数、孤立 `-` / `.` 不算。
    #[test]
    fn num_strip_extracts_signed_and_fractional_numbers() {
        assert_eq!(num_strip("G1 X12.5 Y-3 E0.04"), vec![1.0, 12.5, -3.0, 0.04]);
    }

    /// `num_strip`：没有数字就给空向量，不 panic。
    #[test]
    fn num_strip_on_bare_punctuation_is_empty() {
        assert_eq!(num_strip("- . -. ;"), Vec::<f64>::new());
    }
}
