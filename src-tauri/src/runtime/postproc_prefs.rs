//! **用户偏好 → IR**（2026-10-17）：设置页里那几档开关真正生效的唯一写入点。
//!
//! # 为什么需要这一层
//!
//! 2026-10-17 之前，后处理管线里有几个 `DEFAULT_*` 常量被**写死**在调用点：
//! 跳过震动检测恒为 `true`、收笔恒为 `true`、停留时间恒为 2/3 秒、
//! `state.glue_bottom_enabled` **只被读、从没被写过**（于是"支撑面底面涂胶"
//! 永远是关的）。也就是说：设置页即便摆出这些开关，改了也不会有任何效果。
//!
//! 这一层把"用户在应用状态里点出来的那几档"翻译成 `Ir::prefs`，
//! 再由管线去读 `ir.prefs.*` —— **一个偏差只在一处修正**。
//!
//! # 边界
//!
//! 只翻"真会改变切出来 G-code"的那几项。界面形态（静默 / GUI）与系统通知
//! **不进 IR**：它们不影响产物，只归客户端自己管（`hook_ui`）。
//!
//! # 与预设的关系
//!
//! 预设（`presets-local/*.toml`）说的是"这台机器的工艺"，用户偏好说的是
//! "我想额外怎么做"。前者随预设走、后者跟着人走 —— 所以这一层是**覆盖**，
//! 不写回预设文件。同一台机器换一份预设，用户偏好不该跟着变。
//!
//! ★ `glue_bottom_enabled` 是个特例：它一直住在 `ir.state`（那是管线的工作区，
//!   由 pass1 读写）而不是一个新字段。所以这里要**同时**写两处：
//!   `ir.state.glue_bottom_enabled`（给 pass1 看）与 `ir.prefs.glue_bottom_enabled`
//!   （给"这一趟的偏好是什么"留个账）。两处不写成一致就会有一半生效一半不生效。

use postprocess::ir::{Ir, TowerRelocateModeIr};

use super::app_state::{ResolvedPostprocPrefs, TowerRelocateMode};

/// 客户端那三档 → 内核那三档。
///
/// ★ 一一对应、**不许有默认档互相顶替**（`Off` 对 `Off`，不是"认不出就当关"）：
///   两端枚举拆开写是因为内核不许反向依赖客户端，代价就是这里必须逐档列出 ——
///   只写 `_ => Off` 的话，哪天新增一档会被静默当成"关"。
fn tower_mode_to_ir(mode: TowerRelocateMode) -> TowerRelocateModeIr {
    match mode {
        TowerRelocateMode::Off => TowerRelocateModeIr::Off,
        TowerRelocateMode::Auto => TowerRelocateModeIr::Auto,
        TowerRelocateMode::Ask => TowerRelocateModeIr::Ask,
    }
}

/// 把偏好写进 `ir`。**管线开跑之前调一次**（`ipc::postprocess` 与钩子两条入口都走它）。
///
/// ★ 必须在 `preset::load_ir` **之后**：那一份会把预设里的值铺满整个 IR，
///   先写的话会被它盖掉。
pub fn apply_to_ir(ir: &mut Ir, prefs: &ResolvedPostprocPrefs) {
    ir.prefs.glue_bottom_enabled = prefs.glue_bottom_enabled;
    ir.prefs.skip_vibration_calibration = prefs.skip_vibration_calibration;
    ir.prefs.begin_stage_retract_enabled = prefs.begin_stage_retract_enabled;
    ir.prefs.begin_stage_retract_z_lift_mm = prefs.begin_stage_retract_z_lift_mm;
    ir.prefs.z_calibration_dwell_sec = prefs.z_calibration_dwell_sec;
    ir.prefs.xy_calibration_dwell_sec = prefs.xy_calibration_dwell_sec;
    ir.prefs.bed_glue_enabled = prefs.bed_glue_enabled;
    ir.prefs.bed_glue_all_models = prefs.bed_glue_all_models;
    ir.prefs.auto_tower_position = tower_mode_to_ir(prefs.auto_tower_position);

    /* 底面涂胶实际生效的那一格在 `state`（见模块头那条"特例"） */
    ir.state.glue_bottom_enabled = prefs.glue_bottom_enabled;

    /* 热床预涂胶同理：`bed_glue.go` 那一套的开关也住在 `state` 这一带，
    这里先只写 `prefs`（生成器读的是它）。等 `state` 那边加了字段再一并写。 */

    /* 调试模式：开关打开时把这一趟的动作参数塞进 IR。
     * `None` 才是正常后处理 —— 关掉开关必须把它清回 `None`，否则上一次调试的参数会留在
     * 复用同一个 IR 的下一趟里。 */
    ir.debug = if prefs.debug_mode_enabled {
        Some(postprocess::ir::types::DebugIr {
            wait_x: prefs.debug_wait_x,
            wait_y: 0.0,
            mount_x: prefs.debug_mount_x,
            unmount_x: prefs.debug_unmount_x,
            speed: prefs.debug_speed,
            test_mount: true,
            test_unmount: true,
        })
    } else {
        None
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::app_state::{
        PostprocPrefs, BEGIN_STAGE_RETRACT_Z_LIFT_DEFAULT, BEGIN_STAGE_RETRACT_Z_LIFT_MAX,
        BEGIN_STAGE_RETRACT_Z_LIFT_MIN, XY_DWELL_DEFAULT, Z_DWELL_DEFAULT,
    };

    /// ★★★ **两份出厂档必须一致**：`PrefsIr::default()`（内核那份）与
    /// `ResolvedPostprocPrefs`（客户端那份）是两个 crate 各写一遍的（内核不许反向依赖
    /// 客户端），所以只能靠这条判据咬住。谁哪天只改了一边，这里立刻红。
    #[test]
    fn the_two_default_sets_agree() {
        let client = PostprocPrefs::default().resolved();
        let kernel = postprocess::ir::PrefsIr::default();

        assert_eq!(kernel.glue_bottom_enabled, client.glue_bottom_enabled);
        assert_eq!(
            kernel.skip_vibration_calibration,
            client.skip_vibration_calibration
        );
        assert_eq!(
            kernel.begin_stage_retract_enabled,
            client.begin_stage_retract_enabled
        );
        assert_eq!(
            kernel.begin_stage_retract_z_lift_mm,
            client.begin_stage_retract_z_lift_mm
        );
        assert_eq!(
            kernel.z_calibration_dwell_sec,
            client.z_calibration_dwell_sec
        );
        assert_eq!(
            kernel.xy_calibration_dwell_sec,
            client.xy_calibration_dwell_sec
        );
        assert_eq!(kernel.bed_glue_enabled, client.bed_glue_enabled);
        assert_eq!(kernel.bed_glue_all_models, client.bed_glue_all_models);
        assert_eq!(
            kernel.auto_tower_position,
            tower_mode_to_ir(client.auto_tower_position)
        );

        /* 顺带把常量也咬住：改了默认值却忘了改 `PrefsIr::default()` 同样是红的 */
        assert_eq!(
            kernel.z_calibration_dwell_sec, Z_DWELL_DEFAULT,
            "Z 停留出厂值两端不一致"
        );
        assert_eq!(
            kernel.xy_calibration_dwell_sec, XY_DWELL_DEFAULT,
            "XY 停留出厂值两端不一致"
        );
        assert_eq!(
            kernel.begin_stage_retract_z_lift_mm, BEGIN_STAGE_RETRACT_Z_LIFT_DEFAULT,
            "收笔抬升出厂值两端不一致"
        );
    }

    /// ★★★ **收笔抬升的量程也必须两端一致**（2026-10-21 加）：管线对越界值是
    /// **回落到 20**（原厂口径），而设置页画数字框用的是客户端这一份。
    /// 两边一旦不同，就会出现"界面允许 300、填 150 切出来是 20"那种静默偏差。
    #[test]
    fn the_z_lift_range_agrees_with_the_kernel() {
        use postprocess::pipeline::hooks::{
            BEGIN_STAGE_RETRACT_Z_LIFT_MAX_MM, BEGIN_STAGE_RETRACT_Z_LIFT_MIN_MM,
        };

        assert_eq!(
            BEGIN_STAGE_RETRACT_Z_LIFT_MIN_MM, BEGIN_STAGE_RETRACT_Z_LIFT_MIN,
            "收笔抬升下限两端不一致"
        );
        assert_eq!(
            BEGIN_STAGE_RETRACT_Z_LIFT_MAX_MM, BEGIN_STAGE_RETRACT_Z_LIFT_MAX,
            "收笔抬升上限两端不一致"
        );
        assert!(
            (BEGIN_STAGE_RETRACT_Z_LIFT_MIN_MM..=BEGIN_STAGE_RETRACT_Z_LIFT_MAX_MM)
                .contains(&BEGIN_STAGE_RETRACT_Z_LIFT_DEFAULT),
            "出厂值必须落在量程里"
        );
    }

    /// 写进去的每一格都真的到了 `ir` 上（含 `state` 那一格特例）。
    #[test]
    fn every_pref_lands_on_the_ir() {
        let prefs = crate::runtime::app_state::ResolvedPostprocPrefs {
            notify_on_finish: false,
            ui_mode: crate::runtime::app_state::PostprocUiMode::Silent,
            glue_bottom_enabled: true,
            skip_vibration_calibration: true,
            begin_stage_retract_enabled: false,
            begin_stage_retract_z_lift_mm: 42,
            z_calibration_dwell_sec: 7,
            xy_calibration_dwell_sec: 9,
            bed_glue_enabled: true,
            bed_glue_all_models: true,
            auto_tower_position: TowerRelocateMode::Auto,
        };
        let mut ir = Ir::default();
        apply_to_ir(&mut ir, &prefs);

        assert!(ir.prefs.glue_bottom_enabled);
        assert!(
            ir.state.glue_bottom_enabled,
            "底面涂胶的实际开关在 `state` 这一格 —— 只写 `prefs` 等于没开"
        );
        assert!(ir.prefs.skip_vibration_calibration);
        assert!(!ir.prefs.begin_stage_retract_enabled);
        assert_eq!(ir.prefs.begin_stage_retract_z_lift_mm, 42);
        assert_eq!(ir.prefs.z_calibration_dwell_sec, 7);
        assert_eq!(ir.prefs.xy_calibration_dwell_sec, 9);
        assert!(ir.prefs.bed_glue_enabled);
        assert!(ir.prefs.bed_glue_all_models);
        assert_eq!(
            ir.prefs.auto_tower_position,
            TowerRelocateModeIr::Auto,
            "智能塔位置那一档必须真的落到 IR —— 管线读的就是它"
        );
    }

    /// **三档逐档落地**（2026-11-04）：`Off` 不许被写成 `Auto`，`Ask` 也不许被当成"关"。
    ///
    /// 作者的原话是「关就是关，开就是开」—— 反过来说，中间那档（问一句）也必须是
    /// 它自己，不能被相邻两档吞掉。这条判据比"开了就行"严一档。
    #[test]
    fn every_tower_mode_lands_as_itself() {
        for (mode, want) in [
            (TowerRelocateMode::Off, TowerRelocateModeIr::Off),
            (TowerRelocateMode::Auto, TowerRelocateModeIr::Auto),
            (TowerRelocateMode::Ask, TowerRelocateModeIr::Ask),
        ] {
            let prefs = ResolvedPostprocPrefs {
                auto_tower_position: mode,
                ..PostprocPrefs::default().resolved()
            };
            let mut ir = Ir::default();
            apply_to_ir(&mut ir, &prefs);
            assert_eq!(ir.prefs.auto_tower_position, want, "{mode:?} 落地不对");
        }
    }

    /// 线上字符串是契约：盘上 / DTO / 前端共用这三个字面量。
    #[test]
    fn tower_mode_wire_names_are_stable() {
        assert_eq!(TowerRelocateMode::Off.wire(), "off");
        assert_eq!(TowerRelocateMode::Auto.wire(), "auto");
        assert_eq!(TowerRelocateMode::Ask.wire(), "ask");
        assert_eq!(TowerRelocateMode::default(), TowerRelocateMode::Ask);
    }

    /// **出厂就是"问"**（2026-11-04 作者：「软件安装的时候默认就是问」）。
    ///
    /// 这一条钉两件事：① 盘上没这一格 ⇒ `resolved()` 给 `ask`；② 内核出厂也是 `ask`
    /// （两边一致见上一条判据）。
    #[test]
    fn the_factory_mode_is_ask() {
        let fresh = PostprocPrefs::default().resolved();
        assert_eq!(fresh.auto_tower_position, TowerRelocateMode::Ask);
        assert_eq!(
            postprocess::ir::PrefsIr::default().auto_tower_position,
            TowerRelocateModeIr::Ask,
            "内核出厂档也要是 ask —— 管线读的是它"
        );

        /* 盘上没这一格（老档案 / 新装）⇒ 出厂 `ask` */
        let empty: PostprocPrefs = serde_json::from_str("{}").expect("空对象也要读得动");
        assert_eq!(empty.auto_tower_position, None);
        assert_eq!(empty.resolved().auto_tower_position, TowerRelocateMode::Ask);
    }

    /// 量程夹住：界面上万一传了越界值，管线拿到的一定是合法值。
    #[test]
    fn out_of_range_values_are_clamped() {
        let p = PostprocPrefs {
            z_calibration_dwell_sec: Some(999),
            xy_calibration_dwell_sec: Some(-5),
            begin_stage_retract_z_lift_mm: Some(-1),
            ..PostprocPrefs::default()
        }
        .resolved();
        assert_eq!(p.z_calibration_dwell_sec, 20);
        assert_eq!(p.xy_calibration_dwell_sec, 1);
        assert_eq!(
            p.begin_stage_retract_z_lift_mm,
            BEGIN_STAGE_RETRACT_Z_LIFT_MIN
        );

        let high = PostprocPrefs {
            begin_stage_retract_z_lift_mm: Some(999),
            ..PostprocPrefs::default()
        }
        .resolved();
        assert_eq!(
            high.begin_stage_retract_z_lift_mm,
            BEGIN_STAGE_RETRACT_Z_LIFT_MAX
        );
    }
}
