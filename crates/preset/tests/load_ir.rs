//! `load_ir` 的判据（Task 5）。
//!
//! 三层，从强到弱：
//!
//! 1. **整链字节判据**：`load_ir(预设 A1.toml)` → `process_with_ir` 的输出与
//!    `crates/core/tests/golden/42274.2.e2e-A1.reference.gcode`（668,687 B）
//!    **除 MKP 标记行外逐字节相等**。这条把「预设 → IR → 12 步 → 输出」整条链一次钉住
//!    —— 参考物正是 `mkp-sr` 的 CLI 用**同一份预设**生成的（命令记在
//!    `crates/core/tests/end_to_end.rs` 头部）。
//!    注意它比 `end_to_end.rs` 更强一档：那条走的是**已经转好的 IR TOML**，
//!    这条走的是**真正的预设文件**，中间那 2,242 行映射也在覆盖面里。
//!
//!    2026-09-12 起判据里剔除 `;MKP_*` 标记行：本仓开始按 MKP 标记协议 v1 往产物里插
//!    `MKP_BEGIN/END/INFO/STATS`，参考物里没有这些行。参考物由**来源仓库的 CLI** 生成、
//!    在本仓无法复现，拿本侧输出覆盖它等于把判据变成自比自（彻底空转）。
//!    收窄成「除新增标记行外逐字节相同」比原判据更强 —— 它同时证明了标记是纯增量，
//!    没有顺带移动或改写任何一行原有输出。处置与 `end_to_end.rs` / `process_with_ir.rs` 一致。
//! 2. **硬错误判据**：机型别名不认识 / 预设头缺 `# machine:` 各自报什么码。
//! 3. **真实预设**：用户机器上的 `~/Documents/MKPSupportSSR/presets/mkp/A1MF.toml`
//!    能过 `load_ir`。它在仓库外 ⇒ 不存在时**响亮跳过**（打印路径与原因，不静默 return）。
//!
//! 伪随机序列是**进程级**状态（见 `crates/core/tests/process_with_ir.rs` 的文件头），
//! 所以整链那条在跑之前 reset，且用一把锁串行。

use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use mkp_pp::diag::CancelToken;
use mkp_pp::pipeline::{self, IrProcessRequest, NoProgress};

static SEQUENCE: Mutex<()> = Mutex::new(());

fn lock_sequence() -> MutexGuard<'static, ()> {
    match SEQUENCE.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    }
}

fn core_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../core/{rel}"))
}

/// 用户机器上的真实预设（仓库外资产）。
const REAL_PRESET: &str = "/Users/wzy/Documents/MKPSupportSSR/presets/mkp/A1MF.toml";

/// 剔除 MKP 标记协议 v1 的标记行 —— 判据侧只有
/// `mkp_pp::postproc::marks::strip_mark_lines` 这一份实现（见那边的注释）。
fn strip_mkp_marks(bytes: &[u8]) -> Vec<u8> {
    mkp_pp::postproc::marks::strip_mark_lines(bytes)
}

#[test]
fn preset_to_ir_to_output_is_byte_identical_to_the_mkp_sr_reference() {
    let _seq = lock_sequence();
    mkp_pp::gcode::reset_pseudo_random();

    let dir = tempfile::tempdir().expect("临时目录");
    let work = dir.path().join("input.gcode");
    std::fs::copy(core_path("tests/golden/42274.2.gcode"), &work).expect("复制输入");

    let raw = std::fs::read_to_string(&work).expect("读输入");
    let ir = mkp_preset::load_ir(&core_path("tests/fixtures/presets/A1.toml"), Some(&raw))
        .expect("预设 → IR 必须成功");

    // 顺手把「第 7、8 步真的发生了」钉住：文件名带扩展名、机型已归一。
    assert_eq!(
        ir.meta.preset_name, "A1.toml",
        "preset_name 必须是带扩展名的文件名"
    );
    assert_eq!(ir.machine.machine_type, "A1", "机型必须归一成 Canonical ID");

    pipeline::process_with_ir(
        IrProcessRequest {
            gcode_path: work.clone(),
            ir,
            output_path: None,
        },
        &mut NoProgress,
        &CancelToken::new(),
    )
    .expect("整链必须成功");

    let got = strip_mkp_marks(&std::fs::read(&work).expect("读回原地输出"));
    let want = std::fs::read(core_path("tests/golden/42274.2.e2e-A1.reference.gcode"))
        .expect("读整链参考物");
    // 反空转哨兵（字面量，不插值）。
    assert!(
        want.len() == 668_687,
        "整链参考物字节数必须是 668687，实测 {}",
        want.len()
    );
    // 大对象不整体 assert_eq!。
    assert!(
        got.len() == want.len(),
        "剔除 MKP 标记行后长度不同 本侧={} 参考={}",
        got.len(),
        want.len()
    );
    if let Some(i) = got.iter().zip(want.iter()).position(|(a, b)| a != b) {
        panic!(
            "首个不同字节在 offset {i}（本侧=0x{:02x} 参考=0x{:02x}）",
            got[i], want[i]
        );
    }
}

/// 写一份最小可解析的预设到临时目录，`machine` 头由调用方给。
fn minimal_preset(dir: &Path, machine_header: Option<&str>) -> PathBuf {
    let src =
        std::fs::read_to_string(core_path("tests/fixtures/presets/A1.toml")).expect("读基准预设");
    // 去掉原有的 `# machine:` 行，再按需要加回一条 —— 这样其余字段全是真实值，
    // 判据测的是机型这一个变量。
    let body: String = src
        .lines()
        .filter(|l| !l.trim_start().starts_with("# machine:"))
        .map(|l| format!("{l}\n"))
        .collect();
    let text = match machine_header {
        Some(m) => format!("# machine: {m}\n{body}"),
        None => body,
    };
    let path = dir.join("probe.toml");
    std::fs::write(&path, text).expect("写临时预设");
    path
}

#[test]
fn an_unknown_machine_alias_is_a_hard_error() {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = minimal_preset(dir.path(), Some("NOT_A_PRINTER"));
    // **不用 expect_err**：它在 Ok 分支会把整个 Ir 用 Debug 打出来（NV-5.5 实测刷了 5KB，
    // 真因埋在里面 —— AGENTS.md §7⑦ 同款）。只打一句人话 + 两个关键字段。
    let err = match mkp_preset::load_ir(&path, None) {
        Err(e) => e,
        Ok(ir) => panic!(
            "未知机型必须失败，实测却成功了：machine_type={:?} max_x={} —— \
             这正是「0×0 的机器安静跑完」那个失败形态",
            ir.machine.machine_type, ir.machine.max_x
        ),
    };
    let msg = err.to_string();
    assert!(
        msg.contains("NOT_A_PRINTER"),
        "错误必须点名那个写错的机型，实测：{msg}"
    );
}

#[test]
fn a_missing_machine_header_is_a_hard_error() {
    let dir = tempfile::tempdir().expect("临时目录");
    let path = minimal_preset(dir.path(), None);
    let err = mkp_preset::load_ir(&path, None).expect_err("缺 `# machine:` 必须失败");
    assert_eq!(
        err.code(),
        "E_CFG_PARSE_001",
        "缺头注释的错误码是对外契约，实测 {}：{err}",
        err.code()
    );
}

#[test]
fn the_real_user_preset_loads() {
    let path = Path::new(REAL_PRESET);
    if !path.is_file() {
        // 响亮跳过：点名缺什么，不静默通过（AGENTS.md §6.1）。
        eprintln!(
            "SKIP the_real_user_preset_loads：仓库外资产不存在 {REAL_PRESET}\n\
             这条判据需要用户机器上的真实预设目录；它缺失时本用例**没有验证任何东西**。"
        );
        return;
    }
    let ir = mkp_preset::load_ir(path, None).expect("真实预设必须能过 load_ir");
    assert_eq!(ir.meta.preset_name, "A1MF.toml");
    assert!(
        !ir.machine.machine_type.is_empty() && ir.machine.max_x > 0.0,
        "真实预设的机型与运动范围必须被填上，实测 machine_type={:?} max_x={}",
        ir.machine.machine_type,
        ir.machine.max_x
    );

    // **这条钉的是一处刻意的不对称，不是 bug**：`load_ir` 不调 `fill_defaults`
    // （时机是「G-code 元数据提取之后」，由 pass1 调 —— `pass1/mod.rs:1275`），
    // 所以刚出炉的 IR 里层高/喷嘴等零值兜底字段**还是 0**。
    // 而 CLI 那条路（`config::load`，`config.rs:75`）**当场就跑了** fill_defaults。
    // 后果：GUI 想展示「生效层高」不能直接读这里的值，得自己先 fill 一份副本。
    // 这条判据存在的意义就是让下一个人别把 0 当成搬运错误。
    assert!(
        ir.machine.first_layer_height == 0.0 && ir.machine.typical_layer_height == 0.0,
        "load_ir 之后层高应仍为 0（fill_defaults 未跑），实测 {}/{}",
        ir.machine.first_layer_height,
        ir.machine.typical_layer_height
    );
    let mut filled = ir.clone();
    mkp_pp::ir::fill_defaults(&mut filled);
    assert!(
        filled.machine.first_layer_height == 0.2 && filled.machine.typical_layer_height == 0.2,
        "fill_defaults 之后层高应是 0.2/0.2，实测 {}/{}",
        filled.machine.first_layer_height,
        filled.machine.typical_layer_height
    );

    eprintln!(
        "真实预设 {}：机型 {} / X {:.1}..{:.1} / Y {:.1}..{:.1} / 禁区 {} 处 / \
         层高（fill 之后）{:.3}/{:.3}",
        ir.meta.preset_name,
        ir.machine.machine_type,
        ir.machine.min_x,
        ir.machine.max_x,
        ir.machine.min_y,
        ir.machine.max_y,
        ir.machine.forbidden_zones.len(),
        filled.machine.first_layer_height,
        filled.machine.typical_layer_height
    );
}
