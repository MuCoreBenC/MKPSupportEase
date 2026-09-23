//! 擦拭决策（对照 gcodebiz/wiping_decision.go + ir/wiping_decision.go）。
//!
//! 偏离登记：Go 的 ApplyWipingDecision 经 fallback sink 做 L1 放行/留痕，
//! sink 未注入时硬错误；Rust Phase 1 无 GUI 回退通道 —— 语义取 Go 的
//! 「permissive sink」分支（放行 + 不留痕），nil-sink 硬错误路径不存在。
//! 留痕（detail.fallbacks[]）随 sink 系统一并丢弃（GUI 面）。

use crate::gcode::detect_support_fallback;
use crate::ir::Ir;

use crate::postproc::calibration::{Mode, detect_mode};

/// WipingReason（ir/types.go 的字符串形态）。
pub const REASON_NOT_NEEDED: &str = "not_needed";
pub const REASON_NEEDED_CALIBRATION: &str = "needed_calibration";
pub const REASON_NEEDED_SUPPORT: &str = "needed_support";
pub const REASON_SUPPORT_FALLBACK: &str = "support_fallback";

#[derive(Debug, Clone)]
pub struct WipingDecision {
    pub needed: bool,
    pub reason: &'static str,
    pub preferred_mode: String,
    pub fallback_objects: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct WipingApplyResult {
    pub support_fallback: bool,
    pub fallback_objects: Vec<String>,
}

fn has_any_support_feature(content: &[String]) -> bool {
    content.iter().any(|line| line.contains("FEATURE: Support"))
}

/// `AnalyzeNeedWiping`（gcodebiz/wiping_decision.go:14）：纯模型特征分析。
pub fn analyze_need_wiping(content: &[String]) -> (bool, &'static str, Vec<String>) {
    // 1. 校准模式
    if detect_mode(content) != Mode::None {
        return (true, REASON_NEEDED_CALIBRATION, Vec::new());
    }
    // 2. 支撑检测
    if has_any_support_feature(content) {
        let (fb, objs) = detect_support_fallback(content);
        if fb {
            return (true, REASON_SUPPORT_FALLBACK, objs);
        }
        return (true, REASON_NEEDED_SUPPORT, Vec::new());
    }
    // 3. 多色检测（Go 侧 TODO，同现状）
    (false, REASON_NOT_NEEDED, Vec::new())
}

/// `DecideWiping`（gcodebiz/wiping_decision.go:47）。
///
/// **`preferred_mode` 该从哪里取（本项目的抽取结论，已实测确认）**：
///
/// 来源仓库 mkp-sr 的 engine 在 `lib.rs:242` 传的是
/// `preset.config.wiping.have_wiping_components`，也就是**直接读预设**、不经 IR。
/// 本项目没有预设了，只能从 `ir.wiping.preferred_mode` 取。这一步换源**必须先证明两者恒等**，
/// 否则会静默改变擦拭决策（tower / disk / 强制 tower 三条分支）。
///
/// 实测证据：`crates/ir/src/build.rs:74` 是
/// `ir.wiping.preferred_mode = cfg.wiping.have_wiping_components.clone();`
/// —— **纯恒等复制，无归一化、无 trim、无小写化、无默认值兜底**。
/// 所以 `ir.wiping.preferred_mode` 就是来源仓库传给本函数的那个字符串，换源安全。
///
/// 顺带确认：本函数**自己不做归一化**（:61 只 `to_string()`），
/// 非法值走 `apply_wiping_decision` 的 `_` 臂强制 tower（:98-103）。
/// 也就是说「有没有归一化」这件事在整条链上答案一致：没有。
///
/// **这个结论没有被 G1/G2 覆盖**：那两份判据用固定 fixture，`preferred_mode` 只有一个取值，
/// 换成另一个字段也可能照样全绿。钉住它的是 pipeline 侧那条独立判据（见
/// `tests/pipeline_wiping_source.rs`），不是字节 golden。
pub fn decide_wiping(preferred_mode: &str, content: &[String]) -> WipingDecision {
    let (needed, reason, objs) = analyze_need_wiping(content);
    WipingDecision {
        needed,
        reason,
        preferred_mode: preferred_mode.to_string(),
        fallback_objects: objs,
    }
}

fn is_unset_preferred_mode(mode: &str) -> bool {
    mode.is_empty() || mode == "none" || mode == "false"
}

/// `ApplyWipingDecision`（ir/wiping_decision.go:101）。
pub fn apply_wiping_decision(ir_data: &mut Ir, decision: &WipingDecision) -> WipingApplyResult {
    // 分支 2：支撑面回退（唯一先于通用赋值的路径）
    if decision.needed && decision.reason == REASON_SUPPORT_FALLBACK {
        ir_data.wiping.needed = decision.needed;
        ir_data.wiping.mode = "tower".to_string();
        ir_data.tower.use_towers = true;
        return WipingApplyResult {
            support_fallback: true,
            fallback_objects: decision.fallback_objects.clone(),
        };
    }

    ir_data.wiping.needed = decision.needed;
    if !decision.needed {
        ir_data.wiping.mode = "none".to_string();
        ir_data.tower.use_towers = false;
        return WipingApplyResult::default();
    }
    match decision.preferred_mode.as_str() {
        "disk" => {
            ir_data.wiping.mode = "disk".to_string();
            ir_data.tower.use_towers = false;
        }
        "tower" => {
            ir_data.wiping.mode = "tower".to_string();
            ir_data.tower.use_towers = true;
        }
        _ => {
            // PreferredMode = none/false/""（或非法值）但 Needed=true：强制 tower
            let _ = is_unset_preferred_mode(&decision.preferred_mode);
            ir_data.wiping.mode = "tower".to_string();
            ir_data.tower.use_towers = true;
        }
    }
    WipingApplyResult::default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn decide_matrix() {
        // 无支撑无校准 → 不需要
        let d = decide_wiping("tower", &lines(&["G1 X1 Y1"]));
        assert!(!d.needed);
        // 仅有支撑面无支撑体 → SupportFallback（detect_support_fallback 判定）
        let d = decide_wiping(
            "tower",
            &lines(&["; FEATURE: Support interface", "G1 X1 Y1"]),
        );
        assert!(d.needed);
        assert_eq!(d.reason, REASON_SUPPORT_FALLBACK);
        // 有支撑体 → 正常支撑
        let d = decide_wiping(
            "tower",
            &lines(&[
                "; FEATURE: Support interface",
                "G1 X1 Y1",
                "; FEATURE: Support body",
                "G1 X2 Y2",
            ]),
        );
        assert!(d.needed);
        assert_eq!(d.reason, REASON_NEEDED_SUPPORT);
        // 校准文件 → 需要（即使无支撑）
        let d = decide_wiping("none", &lines(&["; ZOffset Calibration"]));
        assert!(d.needed);
        assert_eq!(d.reason, REASON_NEEDED_CALIBRATION);
    }

    #[test]
    fn apply_preferred_none_forces_tower() {
        let mut ir = Ir::default();
        let d = decide_wiping("none", &lines(&["; FEATURE: Support", "G1 X1 Y1"]));
        apply_wiping_decision(&mut ir, &d);
        assert!(ir.tower.use_towers);
        assert_eq!(ir.wiping.mode, "tower");
    }

    #[test]
    fn apply_disk_mode() {
        let mut ir = Ir::default();
        let d = decide_wiping("disk", &lines(&["; FEATURE: Support", "G1 X1 Y1"]));
        apply_wiping_decision(&mut ir, &d);
        assert!(!ir.tower.use_towers);
        assert_eq!(ir.wiping.mode, "disk");
    }

    #[test]
    fn apply_not_needed_clears() {
        let mut ir = Ir::default();
        ir.tower.use_towers = true;
        let d = decide_wiping("tower", &lines(&["G1 X1 Y1"]));
        apply_wiping_decision(&mut ir, &d);
        assert!(!ir.tower.use_towers);
        assert_eq!(ir.wiping.mode, "none");
    }
}
