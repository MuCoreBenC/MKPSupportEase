//! 塔几何常量表（对照 tower/constants.go，逐字一致）。

pub const TOWER_OFFSET: f64 = 5.0;
pub const TOWER_INNER_BOUND: f64 = 10.21;
pub const TOWER_OUTER_BOUND: f64 = 29.79;
pub const TOWER_CENTER_X: f64 = 20.0;
pub const TOWER_CENTER_Y: f64 = 20.0;

pub const TOWER_LAYER_ENTRY_X: f64 = 20.0;
pub const TOWER_LAYER_ENTRY_Y: f64 = 10.19;
pub const TOWER_LAYER_LEFT_X: f64 = 10.19;
pub const TOWER_LAYER_RIGHT_X: f64 = 29.81;
pub const TOWER_LAYER_TOP_Y: f64 = 29.81;
pub const TOWER_LAYER_MID_Y: f64 = 20.0;

pub const WIPE_CENTER_X: f64 = 25.0;
pub const WIPE_CENTER_Y: f64 = 25.0;
pub const WIPE_VAR_X: f64 = 15.0;
pub const WIPE_VAR_Y_BASE: f64 = 2.0;
pub const WIPE_ALT_X: f64 = 15.0;
pub const WIPE_ALT_Y: f64 = 15.0;
pub const WIPE_FINAL_X: f64 = 20.0;
pub const WIPE_FINAL_Y_BASE: f64 = 1.0;
pub const LEAVE_TOWER_X: f64 = 33.0;
pub const LEAVE_TOWER_Y: f64 = 33.0;

pub const FILAMENT_DIAMETER: f64 = 1.75;
pub const FILAMENT_RADIUS: f64 = FILAMENT_DIAMETER / 2.0;
pub const LINE_WIDTH_MULTIPLIER: f64 = 1.25;
pub const RETRACT_SPEED: i64 = 5400;
pub const WIPE_EXIT_OFFSET: f64 = 3.0;
pub const TOWER_WIPE_DISTANCE: f64 = 1.0;
pub const TOWER_WIPE_SPEED: i64 = 4800;
pub const TOWER_WIPE_ACCEL: i64 = 6000;
pub const TOWER_FINAL_RETRACT: f64 = 0.02;
pub const TOWER_FINAL_RETRACT_SPEED: i64 = 1800;
pub const SPIRAL_LIFT_Z_HEIGHT: f64 = 0.6;
pub const SPIRAL_LIFT_RADIUS: f64 = 1.217;
pub const SPIRAL_LIFT_SPEED: i64 = 42000;

pub const RIB_BRIM_WIDTH: f64 = 3.0;
pub const RIB_MAX_CHAMFER_WIDTH: f64 = 3.0;
pub const RIB_FIRST_LAYER_SHEATH_SIZE: f64 = 20.0;
pub const TOWER_SPEED_LIMIT: f64 = 35.0;
pub const TOWER_SPEED_LIMIT_LAYERS: i64 = 3;

/// 拐角补偿（仅作用于拐角离开段）。
pub const TOWER_CORNER_SPEED_RATIO: f64 = 0.4;
pub const TOWER_CORNER_EXTRA_FLOW: f64 = 0.10;
pub const TOWER_EXIT_EXTRA_PATH: f64 = 1.0;

/// 小螺旋（第二层 4 象限不挤出润笔）参数。
pub const MINI_SPIRAL_QUADRANT_SIZE: f64 = 7.0;
pub const MINI_SPIRAL_PRE_RETRACT: f64 = 0.5;
pub const MINI_SPIRAL_PRE_RETRACT_SPEED: i64 = 1800;
pub const MINI_SPIRAL_POST_RECOVER: f64 = 0.5;
pub const MINI_SPIRAL_POST_RECOVER_SPEED: i64 = 1800;
pub const MINI_SPIRAL_CENTER_ADJUST: f64 = 1.5;

/// MinDepthPerHeight 插值表：塔越高，斜肋最小深度越大。
pub const MIN_DEPTH_PER_HEIGHT: [(f64, f64); 4] =
    [(5.0, 5.0), (100.0, 20.0), (250.0, 40.0), (350.0, 60.0)];
