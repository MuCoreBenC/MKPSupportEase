//! 调试模式：只产出「文件头 → 抬 Z 段 → 落笔/收笔动作 → 文件尾」的骨架 G-code。
//!
//! 不预热、不清嘴、不打印模型、不做 Z 归零 —— 目的是把笔装上/收回去，
//! 让人在真机上试位置。既然不加热也不挤出，回抽与挤出在这里一律不写。
//!
//! # 为什么不能删太狠
//!
//! Bambu Studio 打开文件时会校验这几个块。缺了任何一处，文件会被判成不完整、
//! 拒绝显示（作者原话：「如果你去掉了一些东西，它可能就不显示」）：
//!
//! - `; HEADER_BLOCK_START` / `; HEADER_BLOCK_END`（头部注释与参数快照）
//! - `; CONFIG_BLOCK_START` / `; CONFIG_BLOCK_END`
//! - `; EXECUTABLE_BLOCK_START` / `; EXECUTABLE_BLOCK_END`
//! - `M73`（进度）、`M201`–`M205`（运动上限）、`M106`（风扇）
//!
//! 所以头部**整段保留到原版抬 Z 段收尾**，只从中挑走温度指令；尾部同样整段保留。
//! 中间那一大坨（预热、AMS 换料、模型层）才是被丢掉的。

use crate::ir::types::{DebugIr, Ir};

/// 加热相关指令：调试不加热，全部剔掉。
///
/// 剔掉之后 BBS 里温度显示为 0，这是预期的 —— 比留着温度让它真去加热安全。
const TEMPERATURE: &[&str] = &["M104", "M140", "M109", "M190"];

fn is_temperature(line: &str) -> bool {
    let t = line.trim_start();
    TEMPERATURE.iter().any(|c| t.starts_with(c))
}

/// 数字按 G-code 习惯输出：整数不带 `.0`。
fn num(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// 原版抬 Z 段（`G380 … G90`）收尾的下一位。找不到就 `None`。
fn z_lift_end(lines: &[String]) -> Option<usize> {
    let first = lines.iter().position(|l| l.trim().starts_with("G380"))?;
    lines[first..]
        .iter()
        .position(|l| l.trim() == "G90")
        .map(|offset| first + offset + 1)
}

/// 文件尾的起点：`; EXECUTABLE_BLOCK_END` 那一行（含）。
fn tail_start(lines: &[String]) -> Option<usize> {
    lines
        .iter()
        .position(|l| l.contains("; EXECUTABLE_BLOCK_END"))
}

/// 动作块：驻点 → 落笔位 → 驻点 → 收笔位 → 驻点。
///
/// 两个动作各自可勾选；都不勾时只走一遍驻点，用来确认 XY 归零是否正常。
fn action_block(d: &DebugIr) -> Vec<String> {
    let speed = num(d.speed);
    let wait_x = num(d.wait_x);
    let has_y = d.wait_y.abs() > 1e-9;
    let wait_y = num(d.wait_y);
    let to_wait = if has_y {
        format!("G1 X{wait_x} Y{wait_y} F{speed}")
    } else {
        format!("G1 X{wait_x} F{speed}")
    };

    let mut out = vec![";===== debug actions ==================".to_string()];
    /* 只归 X/Y：Z 由原版 `G380` 段抬到床上方，这里不碰。 */
    out.push("G28 X".to_string());
    if has_y {
        out.push("G28 Y".to_string());
    }
    out.push(to_wait.clone());
    if d.test_mount {
        out.push(format!("G1 X{} F{speed}", num(d.mount_x)));
        out.push(to_wait.clone());
    }
    if d.test_unmount {
        out.push(format!("G1 X{} F{speed}", num(d.unmount_x)));
        out.push(to_wait);
    }
    out.push(";===== debug actions end ==============".to_string());
    out
}

/// 拼出调试产物。`lines` 是切片器给的原始 G-code。
pub fn build_debug_gcode(lines: &[String], d: &DebugIr) -> Vec<String> {
    let head_end = z_lift_end(lines).unwrap_or_else(|| {
        /* 认不出抬 Z 段（机型/切片器不同）：退到 EXECUTABLE_BLOCK_START 的后一行，
         * 再不行就只留注释头 —— 仍然不能把模型层带出来。 */
        lines
            .iter()
            .position(|l| l.contains("; EXECUTABLE_BLOCK_START"))
            .map(|i| i + 1)
            .unwrap_or(0)
    });
    let tail = tail_start(lines).unwrap_or(lines.len());

    let mut out: Vec<String> = Vec::with_capacity(head_end + 32);
    for line in &lines[..head_end] {
        if is_temperature(line) {
            continue;
        }
        out.push(line.clone());
    }
    out.extend(action_block(d));
    /* 进度推到 100：BBS 的进度条与缩略图都读它，缺了会显示成没打完。 */
    out.push("M73 P100 R0".to_string());
    for line in &lines[tail..] {
        if is_temperature(line) {
            continue;
        }
        out.push(line.clone());
    }
    out
}

/// 只有在 `ir.debug` 有值时才有意义；没有就返回 `None`（正常后处理）。
pub fn debug_params(ir: &Ir) -> Option<&DebugIr> {
    ir.debug.as_ref()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn params() -> DebugIr {
        DebugIr {
            wait_x: 256.0,
            wait_y: 0.0,
            mount_x: 267.0,
            unmount_x: 276.0,
            speed: 10000.0,
            test_mount: true,
            test_unmount: true,
        }
    }

    fn startup() -> Vec<String> {
        lines(&[
            "; HEADER_BLOCK_START",
            "M73 P0 R35",
            "M201 X20000 Y20000",
            "M104 S220",
            "; EXECUTABLE_BLOCK_START",
            ";=====avoid end stop =================",
            "G91",
            "G380 S2 Z40 F1200",
            "G380 S3 Z-15 F1200",
            "G90",
            "M109 S220",
            "G1 X100 Y100",
            "; layer 1",
            "G1 X10 Y10 E1.2",
            "; EXECUTABLE_BLOCK_END",
            "M400",
            "M18",
        ])
    }

    /// 头部保留 BBS 需要的那几行，温度与模型层一个都不留。
    #[test]
    fn keeps_bbs_blocks_and_drops_model_and_heat() {
        let out = build_debug_gcode(&startup(), &params());
        let has = |n: &str| out.iter().any(|l| l.contains(n));
        assert!(has("; HEADER_BLOCK_START"));
        assert!(has("; EXECUTABLE_BLOCK_START"));
        assert!(has("; EXECUTABLE_BLOCK_END"));
        assert!(has("M73 P0 R35"));
        assert!(has("M201 X20000"));
        // 温度、模型层必须消失
        assert!(!out.iter().any(|l| l.trim_start().starts_with("M104")));
        assert!(!out.iter().any(|l| l.trim_start().starts_with("M109")));
        assert!(!out.iter().any(|l| l.contains("E1.2")), "{out:?}");
        assert!(!has("layer 1"));
    }

    /// 动作块：驻点 → 落笔位 → 驻点 → 收笔位 → 驻点。
    #[test]
    fn actions_go_wait_mount_wait_unmount_wait() {
        let out = build_debug_gcode(&startup(), &params());
        let seq: Vec<&String> = out
            .iter()
            .filter(|l| l.starts_with("G1 X") && !l.contains('Y'))
            .collect();
        let want = vec![
            "G1 X256 F10000",
            "G1 X267 F10000",
            "G1 X256 F10000",
            "G1 X276 F10000",
            "G1 X256 F10000",
        ];
        assert_eq!(
            seq.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            want,
            "{out:?}"
        );
    }

    /// 只勾一个动作时另一个不出现；两个都不勾只走驻点。
    #[test]
    fn action_switches_are_independent() {
        let mut only_mount = params();
        only_mount.test_unmount = false;
        let out = build_debug_gcode(&startup(), &only_mount);
        assert!(!out.iter().any(|l| l == "G1 X276 F10000"));

        let mut none = params();
        none.test_mount = false;
        none.test_unmount = false;
        let out = build_debug_gcode(&startup(), &none);
        assert_eq!(out.iter().filter(|l| l.starts_with("G1 X")).count(), 1);
    }

    /// 带 Y 的机型（P1/X1）：驻点走两轴，两个动作之间也回两轴。
    #[test]
    fn wait_y_is_used_when_non_zero() {
        let mut d = params();
        d.wait_y = 7.0;
        let out = build_debug_gcode(&startup(), &d);
        assert!(out.iter().any(|l| l == "G28 Y"));
        assert_eq!(
            out.iter()
                .filter(|l| l.as_str() == "G1 X256 Y7 F10000")
                .count(),
            3
        );
    }

    /// 进度推到 100，否则 BBS 显示成没打完。
    #[test]
    fn progress_is_pushed_to_done() {
        let out = build_debug_gcode(&startup(), &params());
        assert!(out.iter().any(|l| l == "M73 P100 R0"));
    }

    /// 认不出抬 Z 段时退到 EXECUTABLE_BLOCK_START 之后，不把模型层带出来。
    #[test]
    fn unknown_startup_falls_back_to_executable_start() {
        let content = lines(&[
            "; HEADER_BLOCK_START",
            "; EXECUTABLE_BLOCK_START",
            "G1 X10 Y10 E1.2",
            "; EXECUTABLE_BLOCK_END",
        ]);
        let out = build_debug_gcode(&content, &params());
        assert!(!out.iter().any(|l| l.contains("E1.2")), "{out:?}");
        assert!(out.iter().any(|l| l.contains("debug actions")));
    }
}
