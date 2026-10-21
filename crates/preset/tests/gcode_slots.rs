//! 三档换笔指令**真的落在 A1 系列预设里**（2026-10-21）。
//!
//! 这条守的是「数据」，不是「解析」：`build.rs` 的选择逻辑有自己的单测，但配方里那三份
//! 文本被改回全速、或者 `active` 忘了写，导出时又会回到会卡笔的老动作 —— 那种回归
//! **不会让任何一条现有判据变红**（整链字节判据只认槽 1，而槽 1 本来就该是原样）。
//!
//! 三档的口径（作者 2026-10-21 定）：
//!   槽 1 = 现状（原字段，内容一字不改，也是默认）；
//!   槽 2 = 稳（F3000 + 停 0.2s + M204 S1500）；
//!   槽 3 = 标准（F5000 + 停 0.2s + M204 S2000）。
//! 依据：`M204 S10000` ≈ 1g，而磁铁保持力只按「扛住 1g 重力」设计，换向瞬间余量为零。

use std::path::{Path, PathBuf};

fn preset_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("assets/presets")
        .join(name)
}

/// 六个 A1 系列预设（fast / fastv3.3 / standard × A1 / A1_MINI）。
const A1_FAMILY: &[&str] = &[
    "A1-fast.toml",
    "A1-fastv3.3.toml",
    "A1-standard.toml",
    "A1_MINI-fast.toml",
    "A1_MINI-fastv3.3.toml",
    "A1_MINI-standard.toml",
];

#[test]
fn a1_family_carries_three_slots_and_still_defaults_to_the_original() {
    for name in A1_FAMILY {
        let cfg = preset::read_preset(&preset_path(name))
            .unwrap_or_else(|e| panic!("{name} 读不回来：{e}"));

        // 槽 1 = 现状：**原样保留**。把它也改掉等于让「什么都不选」的老用户导出变样。
        assert!(
            !cfg.config.toolhead.custom_mount_gcode.contains("G4 P200"),
            "{name} 的槽 1 被当成调速版改了 —— 槽 1 必须保持现状原样"
        );

        for (slot, code) in [
            ("2（稳）", &cfg.config.toolhead.custom_mount_gcode_2),
            ("3（标准）", &cfg.config.toolhead.custom_mount_gcode_3),
        ] {
            assert!(
                code.contains("G4 P200"),
                "{name} 的装载方案 {slot} 少了到位后那 0.2s 停顿 —— 磁铁把笔吸到位靠它"
            );
            assert!(
                code.contains("F3000") || code.contains("F5000"),
                "{name} 的装载方案 {slot} 没降速，还是碰挡片的全速"
            );
            assert!(
                code.contains("M204 S1500") || code.contains("M204 S2000"),
                "{name} 的装载方案 {slot} 没降加速度"
            );
            assert!(
                code.contains("M204 S10000"),
                "{name} 的装载方案 {slot} 段尾没把加速度恢复成 S10000 —— 后面的打印会跟着变慢"
            );
            // 坐标不许动：碰挡片的位置是按机构量出来的（185 压下去 / 189 拉上来是两块挡片）
            assert!(
                code.contains("X185") || code.contains("X267"),
                "{name} 的装载方案 {slot} 坐标被改了（碰挡片的位置不能动）"
            );
        }

        for (slot, code) in [
            ("2（稳）", &cfg.config.toolhead.custom_unmount_gcode_2),
            ("3（标准）", &cfg.config.toolhead.custom_unmount_gcode_3),
        ] {
            assert!(
                code.contains("G4 P200"),
                "{name} 的卸载方案 {slot} 少了到位后那 0.2s 停顿"
            );
            assert!(
                code.contains("L802"),
                "{name} 的卸载方案 {slot} 把 L802（收笔机构动作）弄丢了"
            );
            assert!(
                code.contains("X189") || code.contains("X271") || code.contains("X273"),
                "{name} 的卸载方案 {slot} 坐标被改了（拉起来的挡片位置不能动）"
            );
        }

        // 默认仍是槽 1：`resolve_gcode_slot` 回退 ⇒ 第一份导出与今天逐字节相同
        assert_eq!(
            cfg.config.toolhead.mount_gcode_active, "1",
            "{name} 的默认装载方案不是槽 1 —— 旧预设/旧行为会变"
        );
        assert_eq!(
            cfg.config.toolhead.unmount_gcode_active, "1",
            "{name} 的默认卸载方案不是槽 1 —— 旧预设/旧行为会变"
        );
    }
}
