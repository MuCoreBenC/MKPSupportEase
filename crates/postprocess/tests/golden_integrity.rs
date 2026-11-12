//! vendored golden 的完整性判据（Task 3.2 的测试化形态）。
//!
//! 每份摘要用 `shasum -a 256` 实测登记（每次再生的依据写在各条注释里）。
//! 有意再生 golden 时必须同步改这里并人工审查 diff —— 之后任何**未登记**的
//! 改动都会红。

mod golden_common;

use golden_common as gc;
use sha2::{Digest, Sha256};

/// (文件名, vendor 当刻实测 sha256, 字节数)
const VENDORED: &[(&str, &str, u64)] = &[
    (
        "42274.2.gcode",
        "c55fd74f4bb9e44a48710349c5b51b0bd7ead3ffa2646c9eb003d5e17c93c956",
        561_363,
    ),
    (
        // 2026-09-12 再生：MKP 标记协议 v1 发射（`;MKP_BEGIN`/`;MKP_END` 的 swap/paint 区间）。
        // Go 实现已废弃，这份不再是「与 Go 逐字节一致」的对等性 oracle，改为我们自己的回归基线。
        // 人工核过 diff：新增行全是 MKP 标记，无任何原有行位移或改写（另有末尾一个空行被
        // 归一化掉 —— write_golden 只写一个结尾换行，此前那个空行本来就读不进判据）。
        // 2026-10-21 再生（第四次）：换笔段最终形态「小螺旋脱层 → 原位抬到净空 → 平飞」——
        // `M204` + `G17` + `G3 Z{层顶+1.0} I.706 J.991 P1 [E..]`，再
        // `G1 Z{层顶+offset_z+lift}`（`glue_z_lift_height` 说了算）抬到位，最后 `G1 X{wipe_x}`；
        // `L801` 与 `;Toolhead Mounted` 之后同高。平飞 / 装笔 / 飞回一个高度，全程无升降往返。
        // 人工核过 diff：10 个 swap#1 块，行数不变（当时 532_561 / 572_233），只换了位置与 Z 值。
        // 2026-11-03 再生（第五次）：6a3f7aa「涂胶抬升改为记账真实 Z」—— 每份 34 删/34 加，
        // 同一对 Z/F 从回位后挪到首条涂胶行前；1038c7b 再删掉段首与回位同点的 0mm 定位行
        // （各 -17 行）。两次都有人工 diff 审查（docs/GLUE-PATH-CONTINUITY-2026-11-03.md §14），
        // 复验：golden_g1 / golden_g2 / end_to_end 与当前管线逐行一致。
        // 2026-11-12 再生（第六次）：落笔补回抽 —— `glue_block::mount_toolhead` 在
        // `;Mounting Toolhead` 与 `;Toolhead Mounted` 之间补 `G1 E{mkp_retract} F1500`。
        // 此前只有收笔侧补 E，落笔不补，撞笔时笔尖带着料。10 次挂载 ↔ 新增 10 行，
        // 字节 +150 = 10 × 15，无其他改动。
        "42274.2.expected.gcode",
        "276255b8b8da9089e7606159dee7d8c7d90938a70986fdcfedd471e01a085831",
        532_383,
    ),
    (
        // 2026-09-12 再生：同上。
        // 2026-10-21 再生：与 G1 同一次改动（pass1 产物进 pass2，只有换笔块不同）。
        // 2026-11-03 再生：与 G1 同两笔改动，diff 逐行同源。
        // 2026-11-12 再生：与 G1 同一次改动（落笔补回抽 10 行，字节 +150）。
        "42274.2.pass2.expected.gcode",
        "c53f6c585efde98ee1413ebbbc25e35921451eefa8d556862699bb7649c7ce63",
        572_055,
    ),
    (
        // 2026-09-11 再生：`find_closest_point_index` 的起点选择改成确定性的
        // 「平局带内取最小下标」（此前严格 < 会因浮点 FMA 差异在 macOS/Windows 上
        // 选中互为镜像的起点，材料塔 G-code 两台机器对不上）。几何不变，只是接缝换位。
        // 人工逐块审查过：23 条用例仅起点旋转，坐标逐行一致。
        "tower_matrix.golden",
        "b728eb05722761fd896df31d567b11d48a7615b77de358ee428670dfadf0eca6",
        129_693,
    ),
    (
        // 2026-11-12：整链自有基线 —— `tests/golden/42274.2.gcode` +
        // `tests/fixtures/config/A1.toml` 原样进管线，出来的字节。判据纯逐字节，
        // 不做任何剔除。再生入口在 `tests/end_to_end.rs` 头部。
        "42274.2.e2e-A1.config.gcode",
        "58178b52ac71d5585ccb0c90df3c61c8be3b5d757b0c3032d89bfb53baf83b1a",
        672_282,
    ),
];

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn vendored_goldens_are_byte_identical_to_vendor_moment() {
    for (name, want_sha, want_len) in VENDORED {
        let bytes = std::fs::read(gc::golden_path(name))
            .unwrap_or_else(|e| panic!("读不到 vendored golden {name}: {e}"));
        assert_eq!(
            bytes.len() as u64,
            *want_len,
            "{name} 字节数变了 —— vendored 副本被改动？"
        );
        let got = hex(&Sha256::digest(&bytes));
        assert_eq!(
            &got, want_sha,
            "{name} 的 sha256 与 vendor 当刻不一致 —— 要么被改动，要么需要走 UPDATE_GOLDEN 并人工审查"
        );
    }
}
