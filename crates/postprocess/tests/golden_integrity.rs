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
        "42274.2.expected.gcode",
        "a159482fd96289065c54e38ae4256dc1673b6ff6ab6a6f54462162c9a57f4064",
        532_131,
    ),
    (
        // 2026-09-12 再生：同上。
        "42274.2.pass2.expected.gcode",
        "5deaafe4e0b0bb759a5f9c0b5388be6b1f2b53ffad5f36c8cb53ee9155a4a13a",
        571_803,
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
