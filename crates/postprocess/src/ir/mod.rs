//! `ir` —— 唯一业务真相源。
//!
//! 从配置反序列化出来的那一刻起，[`Ir`] 就是**已合法、已归一化、已完成单位换算**的数据；
//! 后续算法层不得再做「偷偷转换」。同一个换算不许在两处发生
//! （例：`Toolhead.MaxSpeed` 是 mm/min = 用户填的 mm/s × 60，全项目只此一处含义）。
//!
//! 与来源仓库 `crates/ir` 的差异：那边这一层的核心是 `build(cfg: &TomlConfig, …) -> Ir`
//! （预设 TOML 到 IR 的唯一翻译点）。本项目**没有预设文件**，配置文件的结构就是 `Ir` 本身，
//! 于是那 1,130 行映射整段不存在 —— 详见 [`defaults`] 的模块头与 spec doc.md §1.2/§4。

pub mod defaults;
pub mod types;

pub use defaults::{fill_defaults, num_strip};
pub use types::*;

#[cfg(test)]
mod gate_tests {
    use super::*;

    fn fixture() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/ir/golden_42274_2.json"),
        )
        .expect("读不到 IR fixture（tests/fixtures/ir/golden_42274_2.json）")
    }

    /// 闸门：Go 侧导出的 IR JSON 必须能被本结构**完整**反序列化。
    /// 字段缺一个即失败 —— 这是 IR 结构完整性的机械判据。
    #[test]
    fn golden_ir_json_deserializes_fully() {
        let ir: Ir = serde_json::from_str(&fixture()).expect("IR JSON 反序列化失败");
        // 抽查 7 项覆写确实到位（来源 golden_test.go:38-46）
        assert_eq!(ir.schema_version, 1);
        assert_eq!(ir.machine.machine_type, "A1_MINI");
        assert_eq!(ir.filament.filament_type, "PETG");
        assert_eq!(ir.filament.slicer, "BambuStudio");
        assert!((ir.ironing.e_sum_threshold - 7.9).abs() < f64::EPSILON);
        assert!(!ir.ironing.use_path);
        assert!((ir.machine.nozzle_diameter - 0.4).abs() < f64::EPSILON);
        assert!((ir.wiping.fan_speed - 255.0).abs() < f64::EPSILON);
        // MaxSpeed = 200 * 60（newTestIR 直设 12000）
        assert_eq!(ir.toolhead.max_speed, 12000.0);
    }

    /// 闸门的负向验证：从 JSON 里删掉任意一个字段，反序列化必须失败。
    /// （证明上面的通过不是「serde default 静默放过」。）
    ///
    /// **这条在本项目里比在来源仓库更重要**：配置文件的形态就是这个结构，
    /// 「缺键即报错」是 spec doc.md §4 方案 C 成立的前提 ——
    /// 一旦有人给某个子结构加上 `#[serde(default)]`，用户把键名拼错就会被静默吞掉，
    /// 表现是「我明明写了这个参数怎么没生效」，最难查的一类。
    #[test]
    fn removing_any_field_must_fail() {
        for field in [
            "SafeZOffset",
            "BaseExpand",
            "MKPExtrude",
            "GcodeMachineType",
        ] {
            let raw = fixture().replace(&format!("\"{field}\":"), "\"__removed__\":");
            assert!(
                serde_json::from_str::<Ir>(&raw).is_err(),
                "字段 {field} 缺失时仍能反序列化 ⇒ 闸门已空转"
            );
        }
    }
}
