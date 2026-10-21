//! `load_ir` 的判据：预设文件读不进来时，错误要说话。
//!
//! 这里只留**硬错误**那两条（机型别名不认识 / 预设头缺 `# machine:`）：
//! 这两条守的是「读错了要立刻停下」，与产物内容无关，所以不会随生成逻辑演进变红。
//!
//! 2026-10-21：原来的「整链逐字节等于参考物」那条已删除 —— 本仓不再对照来源实现
//! 的产物，判据只认我们自己的基准（见 `crates/postprocess/tests/golden_*.rs`）。
//! 拿一份在我们这边无法复现的产物当锚，等于把判据变成自比自。

// 测试写临时文件、造夹具是正当的：写盘纪律管的是**生产代码**
// （与源码扫描断言只看 `#[cfg(test)]` 之前那部分同一口径）。
#![allow(clippy::disallowed_methods)]

use std::path::{Path, PathBuf};

fn core_path(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../postprocess/{rel}"))
}

/// 写一份最小可解析的预设到临时目录，`machine` 头由调用方给。
fn minimal_preset(dir: &Path, machine_header: Option<&str>) -> PathBuf {
    let src = std::fs::read_to_string(core_path("tests/fixtures/presets/A1-standard.toml"))
        .expect("读基准预设");
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
    let err = match preset::load_ir(&path, None) {
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
    let err = preset::load_ir(&path, None).expect_err("缺 `# machine:` 必须失败");
    assert_eq!(
        err.code(),
        "E_CFG_PARSE_001",
        "缺头注释的错误码是对外契约，实测 {}：{err}",
        err.code()
    );
}
