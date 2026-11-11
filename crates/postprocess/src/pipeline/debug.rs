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
    /*
     * 动不动 Y 是**用户勾的那一格**（2026-11-11 作者）：
     *   勾了 → **每一条移动**都带 Y（`G1 X267 Y0 F…`），包括 Y 恰好是 0 的时候；
     *   没勾 → 一条都不带（`G1 X267 F…`）。
     *
     * ★ 不能拿「Y 坐标是不是 0」当判据：作者要的正是「显式把 Y 移到 0」，
     *   0 是一个合法的目标位置，不是「没填」。
     */
    /* 落笔 / 收笔的 Y 没填就**跟随驻点**（作者：不填写就用驻点的） */
    let mount_y = d.mount_y.unwrap_or(d.wait_y);
    let unmount_y = d.unmount_y.unwrap_or(d.wait_y);
    /* 落笔 / 收笔那一档（`F{speed}`） */
    let mv = |x: f64, y: f64| {
        if d.move_y {
            format!("G1 X{} Y{} F{}", num(x), num(y), speed)
        } else {
            format!("G1 X{} F{}", num(x), speed)
        }
    };
    /* 到驻点 / 回驻点那一档：开了单独的驻点速度才换，否则与落笔同速 */
    let travel = if d.use_travel_speed {
        num(d.travel_speed)
    } else {
        speed.clone()
    };
    let walk = |x: f64, y: f64| {
        if d.move_y {
            format!("G1 X{} Y{} F{}", num(x), num(y), travel)
        } else {
            format!("G1 X{} F{}", num(x), travel)
        }
    };
    let to_wait = walk(d.wait_x, d.wait_y);

    let mut out = vec![";===== debug actions ==================".to_string()];
    /*
     * 归零 X 和 Y **总是都做**（2026-11-11 作者：「就算关 Y 也要加上 G28 Y」）。
     *
     * 理由：`G28` 是定基准，不是"移动"。不归 Y 的话，动作行里虽然不写 Y，
     * 但 Y 的物理位置是切片前的残留 —— 那才是真正不可控的。
     * 归完 Y，不写 Y 的移动就停在 Y=0，位置是确定的。
     *
     * Z 不归：由原版 `G380` 段抬到床上方，这里碰了会撞。
     */
    out.push("G28 X".to_string());
    out.push("G28 Y".to_string());
    out.push(to_wait.clone());
    /* 落笔 → 回驻点 → 收笔 → 回驻点，整段按 `repeat` 跑几遍 */
    for _ in 0..d.repeat.max(1) {
        if d.test_mount {
            out.push(mv(d.mount_x, mount_y));
            out.push(walk(d.wait_x, d.wait_y));
        }
        if d.test_unmount {
            out.push(mv(d.unmount_x, unmount_y));
            out.push(walk(d.wait_x, d.wait_y));
        }
    }
    /* 末尾那条回驻点只留一条：重复时每遍都已经回到驻点，最后这条只是收口 */
    if out.last() == Some(&to_wait) {
        out.pop();
    }
    out.push(to_wait);
    out.push(";===== debug actions end ==============".to_string());
    out
}

/// 这一段动作去哪儿来：高级模式下用户写的那段，否则由字段生成。
///
/// 全空行 / 只有空白 ⇒ 回落到生成那一条，不会产出一段空的动作块。
fn actions_of(d: &DebugIr) -> Vec<String> {
    if let Some(txt) = &d.custom_gcode {
        let lines: Vec<String> = txt
            .lines()
            .map(|l| l.trim_end().to_string())
            .filter(|l| !l.trim().is_empty())
            .collect();
        if !lines.is_empty() {
            return lines;
        }
    }
    action_block(d)
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
    out.extend(actions_of(d));
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
            mount_y: None,
            unmount_y: None,
            repeat: 1,
            custom_gcode: None,
            travel_speed: 42000.0,
            use_travel_speed: false,
            move_y: false,
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
    fn move_y_appends_y_to_every_move() {
        let mut d = params();
        d.move_y = true;
        d.wait_y = 7.0;
        let out = build_debug_gcode(&startup(), &d);
        assert!(out.iter().any(|l| l == "G28 Y"));
        /* 驻点、落笔、收笔——**每一条移动**都带 Y */
        assert_eq!(
            out.iter()
                .filter(|l| l.as_str() == "G1 X256 Y7 F10000")
                .count(),
            3
        );
        assert!(out.iter().any(|l| l == "G1 X267 Y7 F10000"));
        assert!(out.iter().any(|l| l == "G1 X276 Y7 F10000"));
    }

    /// Y 是 0 也要显式写出来：0 是合法目标位置，不是「没填」。
    #[test]
    fn move_y_writes_zero_y_too() {
        let mut d = params();
        d.move_y = true;
        d.wait_y = 0.0;
        let out = build_debug_gcode(&startup(), &d);
        assert!(out.iter().any(|l| l == "G1 X256 Y0 F10000"));
        assert!(out.iter().any(|l| l == "G28 Y"));
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
