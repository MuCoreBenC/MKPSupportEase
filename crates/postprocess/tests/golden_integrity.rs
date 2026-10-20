//! vendored golden 的完整性判据（Task 3.2 的测试化形态）。
//!
//! 复制当刻已用 `shasum -a 256` 对源文件逐份比对（提交信息里有实测记录）；
//! 本测试把那次的摘要钉住 —— 之后任何对 tests/golden/ 的意外改动都会红。

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
        // 2026-10-21 再生（第三次）：换笔段最终形态「小螺旋脱层 → 原位抬到装笔高度 → 平飞」——
        // `M204` + `G17` + `G3 Z{层顶+1.0} I.706 J.991 P1 [E..]`，再
        // `G1 Z{层顶+offset_z+3.0}`（`<3.0` 时 `+4.0`）抬到与 `L801` 同一个高度，最后 `G1 X{wipe_x}`；
        // 装笔后 `G1 Z{层顶+offset_z+lift}` 飞回。
        // 平飞高度与装笔高度同值，不再出现「先抬到 21.1、降到 19.1 装笔、又抬回 21.1」的往返。
        // 人工核过 diff：10 个 swap#1 块，行数不变（532_561 / 572_233），只换了两句的位置与 Z 值。
        "42274.2.expected.gcode",
        "ea01ddfe593aa73e8c1fa8124f1e614b7f6db2f4360eb05abc3c3eef9ca1e51f",
        532_561,
    ),
    (
        // 2026-09-12 再生：同上。
        // 2026-10-21 再生：与 G1 同一次改动（pass1 产物进 pass2，只有换笔块不同）。
        "42274.2.pass2.expected.gcode",
        "ba7008d7abf2083affa012eff0c1e7c4b555210377d09c5c46ab23283b5cb099",
        572_233,
    ),
    (
        // 2026-09-11 再生：`find_closest_point_index` 的起点选择改成确定性的
        // 「平局带内取最小下标」（此前严格 < 会因浮点 FMA 差异在 macOS/Windows 上
        // 选中互为镜像的起点，材料塔 G-code 两台机器对不上）。几何不变，只是接缝换位。
        // 人工逐块审查过：23 条用例仅起点旋转，坐标逐行一致。
        "tower_matrix.golden",
        "dad4db0ec7414985d3e905362314bb2ccba0eae1b51c4be770c9c71ef72d4bbe",
        129_583,
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
