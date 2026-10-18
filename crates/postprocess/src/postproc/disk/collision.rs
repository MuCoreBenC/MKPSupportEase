//! 碰撞检测（对照 disk/collision.go 逐行）。诊断（logInfo）在旧侧是旁路输出，
//! Rust 侧结果结构体已经携带全部字段，调用方自取。

use crate::ir::Ir;
use crate::postproc::geom::{self, BBox as GBBox, MoveDirection, Polygon};
use crate::postproc::tower::calculate_tower_bbox;
use crate::postproc::tower::constants::{TOWER_INNER_BOUND, TOWER_OUTER_BOUND};
use serde::{Deserialize, Serialize};

/// **给记录用的冲突点上限**（2026-10-18）。
///
/// 检测本身按文件长短取 200 / 500，那份是"算建议"用的；写进 `_meta.json` 的只留
/// 前 200 个 —— 报告页要的是"撞在哪一片"，不是逐点复刻（一份记录不该被几百个
/// 坐标撑大，而 200 个点足够画出分布）。
const CONFLICT_POINT_CAP: usize = 200;

/// 轴对齐框（mm）—— 事实里的几何一律用它（camelCase 进 `_meta.json`）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Box4 {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl Box4 {
    fn of(b: &GBBox) -> Self {
        Self {
            x_min: b.x_min,
            x_max: b.x_max,
            y_min: b.y_min,
            y_max: b.y_max,
        }
    }
}

/// **画塔用的那几个参数**（2026-10-18）—— 不是句子，是几何。
///
/// 报告页拿它按参数页同一套算法画出塔的真实形状（塔身 / 斜肋 / 护套），
/// 于是"图上这块"与"判据按这块算"是同一件事实的两个说法。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TowerShape {
    /// `brim` / `rib` / `sheath`（`ir.wiping.outer_structure`）
    pub outer_structure: String,
    /// 塔身边长 mm（`TOWER_OUTER_BOUND - TOWER_INNER_BOUND`，与参数页同一个数）
    pub body_mm: f64,
    pub rib_width: f64,
    pub rib_extra_length: f64,
    pub rib_fillet_wall: bool,
    pub sheath_base_expand: f64,
}

/// **碰撞事实**（2026-10-18）：撞的是谁、建议往哪挪多少、撞在哪、几何长什么样。
///
/// 只装**数据**与**已经渲染好的那句建议**（建议正文从 `messages.toml` 来，码在
/// `advice_code`）—— 内核不认识"界面"，也不拼别的句子。上层（`src-tauri`）原样
/// 落进 `_meta.json` 的 `collision` 一格。
/*
 * ★ `Deserialize` 是**后来才要的**（2026-10-18 挪塔那一问要把这一整份几何随问句
 * 发给界面、界面那份契约也要能读回来）；所以 `kind` / `target` 用 `String` 而不是
 * `&'static str`（借来的字符串借不进反序列化）。**JSON 一字不变** —— 它们本来就是
 * 写出去的字符串。
 */
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollisionFacts {
    /// 撞的是谁：`model` / `prime-tower` / `both` / `forbidden` / `wipe-tower-needed`
    pub kind: String,
    /// 建议挪谁：`model` / `wipe-tower`
    pub target: String,
    /// 方向（中文；算不出方向时是空串 —— 那时看 `target` 走"挪塔"那句）
    pub direction: String,
    pub required_move_x: f64,
    pub required_move_y: f64,
    pub conflict_count: i64,
    /// 冲突点采样（mm，机器坐标；封顶 [`CONFLICT_POINT_CAP`]）
    pub conflict_points: Vec<[f64; 2]>,
    /// **判据用的塔包围盒**（含外围结构扩展；左下角 = `wiper_x` / `wiper_y`）
    pub tower_bbox: Box4,
    /// 模型挤出点的整体包围盒（禁区那一档没有 ⇒ 全 0 框，界面按 `kind` 判）。
    /// **"挪模型"的建议方向按它算**；画图用下面的 `model_boxes`。
    pub model_bbox: Box4,
    /// **每个模型各自的包围盒**（2026-10-18）：按占用格连通块切出来的，图上画成
    /// 一个个框。空数组 = 旧记录 / 这一档没算（界面退回 `model_bbox`）。
    pub model_boxes: Vec<Box4>,
    /// 可打印区外接框（mm，机器坐标）：**gcode 里切片器写下的 `; printable_area` 优先**
    /// （[`printable_area_of`]），gcode 没写才退回机型 toml 的 `min_x…max_y`
    pub printable: Box4,
    pub tower: Option<TowerShape>,
    /// 机型额外禁区（通常为空）
    pub forbidden_zones: Vec<Vec<[f64; 2]>>,
    /// 建议那句的**码**（`E_GCODE_COLLISION_002/003/004`）
    pub advice_code: String,
    /// 建议那句的**正文**（表里渲染好的，一个字都不在这里拼）
    pub advice_text: String,
    /// **建议挪动的箭头**（俯视图画"往哪边、挪多少"）：起点 = 挪谁的中心（机器坐标），
    /// 位移 = **带符号** mm（沿 `MoveDirection::delta()` × 距离；禁区档的 MTV 本来就带符号）。
    /// 位移为零 = 没有方向可画（"挪不开"那一档）。
    pub arrow_from: [f64; 2],
    pub arrow_vec: [f64; 2],
}

/// 第 6 步撞了之后"要不要问人"的那条缝 —— **内核只认这一条**。
///
/// 内核不知道界面、通道、归档这些东西的存在：谁想在 `Step::Collision` 停下来问人，
/// 就实现这个 trait，从 `pipeline::process_with_ir_with_tower_ask` 递进来。
/// **没人递 = 原行为一字不差**（直接 `Err(Collision)`）。
pub trait TowerAsk {
    /// 问一次。
    ///
    /// - `facts`：这一趟的碰撞事实（撞了多少、建议往哪、几何）；
    /// - `candidate`：内核自动摆出来的位置（`None` = 板上没有不撞的地方）；
    /// - `current`：现在的塔框左下角（机器坐标）；
    /// - `tower_height_mm`：这一趟的塔高（登记用，内核不复算）。
    fn ask(
        &mut self,
        facts: &CollisionFacts,
        candidate: Option<[f64; 2]>,
        current: [f64; 2],
        tower_height_mm: f64,
    ) -> TowerAskOutcome;

    /// **复检回告**：内核拿改完之后的 IR 又跑了一遍 `detect_tower_collision`。
    ///
    /// `after = None` = 干净（可以继续）；`Some(facts)` = 仍然撞（连"撞了多少"一起给）。
    /// ★ 判据由内核算，回调说了不算。默认空实现 —— 只想知道"要不要挪"的实现方不必覆写。
    fn verified(&mut self, _after: Option<&CollisionFacts>) {}
}

/// 那一问的答复 —— **三条路必须分开**（作者 2026-10-18：`Option<[f64; 2]>` 分不清
/// "用户取消 / 没有可用位置 / 交互失败"，混成一个 `None` 会让取消被当成碰撞、被记成成功）。
#[derive(Debug, Clone, PartialEq)]
pub enum TowerAskOutcome {
    /// 挪到这儿（塔框左下角，机器坐标）—— 内核会复检，通过才继续
    Moved([f64; 2]),
    /// 用户不挪（自己去挪模型）/ 超时没人答 / 界面上没有这一问 ⇒ 这一趟按"撞了"收场
    Declined,
    /// 用户按了停止 ⇒ **取消**（不是碰撞失败；外层的归档不得记成成功）
    Cancelled,
    /// 交互本身坏了（通道断了 / 界面退回）⇒ 明确失败，不假装没事
    Failed { message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CollisionType {
    #[default]
    None,
    Model,
    PrimeTower,
    Both,
    /// 擦料塔落在机型额外禁区内
    Forbidden,
    /// 模型无足够空间，建议调整擦料塔
    WipeTowerNeeded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AdjustmentTarget {
    #[default]
    Model,
    WipeTower,
}

#[derive(Debug, Clone, Copy)]
pub struct MoveAttempt {
    pub direction: MoveDirection,
    pub move_x: f64,
    pub move_y: f64,
    pub collision: bool,
    pub out_of_bounds: bool,
}

#[derive(Debug, Clone, Default)]
pub struct CollisionResult {
    pub has_collision: bool,
    pub conflict_count: i64,
    pub direction: MoveDirection,
    pub direction_cn: String,
    pub required_move_x: f64,
    pub required_move_y: f64,
    pub collision_type: CollisionType,
    pub adjustment_target: AdjustmentTarget,
    pub attempts: Vec<MoveAttempt>,
    /// 实际冲突点采样（调试/模拟用）
    pub conflict_points: Vec<[f64; 2]>,
    /// **判据用的塔包围盒**（2026-10-18：要带出门给报告页画图，所以存下来）
    pub tower_bbox: Option<GBBox>,
    /// 模型挤出点的整体包围盒（禁区那一档没有）
    pub model_bbox: Option<GBBox>,
    /// **每个模型各自的包围盒**（按占用格的连通块算；报告页画成一个个框）。
    /// `model_bbox` 仍然保留：它是"整体一块"，给"挪模型"的建议方向用。
    pub model_boxes: Vec<GBBox>,
    /// 避让用的**占用格**（板内挤出点，2mm 一格）—— 自动摆塔按它判"会不会压到东西"。
    /// 不进 `facts`（那是对外形状），只在这一层用。
    pub occupancy: Occupancy,
    /// 这一趟的**塔高**（= gcode 里最高的 Z，mm）。斜肋外墙的"最小底深随高度涨"
    /// 就靠它（见 `MIN_DEPTH_PER_HEIGHT`）—— 少了它，高塔的底座会被算小（2026-10-18 真机。
    /// 作者：「挪动后的还是碰撞了，因为斜肋外墙随着高度会变大，下面会比较大」）。
    pub tower_height: f64,
    /// **热床按 gcode 判**（2026-10-18 作者定的口径）：切片器在 CONFIG_BLOCK 里写下的
    /// `; printable_area` 多边形的外接框。`None` = 这份 gcode 没写（老文件 / 其它切片器）
    /// ⇒ `facts` 退回机型 toml 的维度。
    pub printable: Option<Box4>,
}

/// 塔避让用的**占用格**（2026-10-18）。
///
/// 为什么不是包围盒：真机上「撞没撞」是**逐个挤出点**判的，而自动摆塔过去只避
/// `model_bbox`（板内点的 AABB）—— 两套样本，于是"挪完还撞"（作者真机，
/// `.5926.1.gcode`：冲突 149351 个点、建议向右挪 40.4mm，挪了还撞）。
///
/// 现在把**同一批挤出点**（板内那些，塔框本来也不许出板）打进 2mm 的格子里，
/// 自动摆塔按格子判"这个位置会不会压到东西"：与 detect 同一份样本、比 AABB 精确，
/// 而且原擦料塔 / 支撑 / 清料段的点天然都在里面（它们也是挤出点）。
#[derive(Debug, Clone)]
pub struct Occupancy {
    cell: f64,
    origin_x: f64,
    origin_y: f64,
    cols: usize,
    rows: usize,
    bits: Vec<bool>,
}

impl Default for Occupancy {
    fn default() -> Self {
        Self {
            cell: 2.0,
            origin_x: 0.0,
            origin_y: 0.0,
            cols: 0,
            rows: 0,
            bits: Vec::new(),
        }
    }
}

impl Occupancy {
    /// 按打印范围（板子）开一张格网；`cell` 是格边长（mm，<=0 时取 2.0）。
    pub fn new(bed: &Box4, cell: f64) -> Self {
        let cell = if cell > 0.0 { cell } else { 2.0 };
        let cols = (((bed.x_max - bed.x_min) / cell).ceil() as usize).max(1);
        let rows = (((bed.y_max - bed.y_min) / cell).ceil() as usize).max(1);
        Self {
            cell,
            origin_x: bed.x_min,
            origin_y: bed.y_min,
            cols,
            rows,
            bits: vec![false; cols * rows],
        }
    }

    /// 打一个点（板外的点直接忽略 —— 塔框也不许出板，永远碰不到它们）。
    pub fn mark(&mut self, x: f64, y: f64) {
        if self.cols == 0 || self.rows == 0 {
            return;
        }
        let cx = ((x - self.origin_x) / self.cell).floor();
        let cy = ((y - self.origin_y) / self.cell).floor();
        if cx < 0.0 || cy < 0.0 {
            return;
        }
        let (cx, cy) = (cx as usize, cy as usize);
        if cx >= self.cols || cy >= self.rows {
            return;
        }
        self.bits[cy * self.cols + cx] = true;
    }

    /// 这个框碰到任何一个被占的格子 ⇒ `true`。
    /// 上界按"框盖到的最后一个格子"取（`floor`），下界同理 —— 与打点用的同一套取整，
    /// 不会出现"点打进去了、框却查不到"。
    pub fn overlaps(&self, b: &GBBox) -> bool {
        if self.bits.is_empty() {
            return false;
        }
        let clamp = |v: f64, hi: usize| -> usize {
            if v <= 0.0 {
                0
            } else if v >= hi as f64 {
                hi - 1
            } else {
                v as usize
            }
        };
        let x0 = clamp(((b.x_min - self.origin_x) / self.cell).floor(), self.cols);
        let x1 = clamp(((b.x_max - self.origin_x) / self.cell).floor(), self.cols);
        let y0 = clamp(((b.y_min - self.origin_y) / self.cell).floor(), self.rows);
        let y1 = clamp(((b.y_max - self.origin_y) / self.cell).floor(), self.rows);
        for y in y0..=y1 {
            let row = y * self.cols;
            for x in x0..=x1 {
                if self.bits[row + x] {
                    return true;
                }
            }
        }
        false
    }

    /// **每个模型各自的包围盒**（2026-10-18 作者：「看起来确实不是大包围盒，那你需要
    /// 做出来，分开的独立的包围盒」）。
    ///
    /// 按占用格的**四连通块**算：一个模型哪怕分成很多层，它的挤出点在 2mm 格上连成
    /// 一片 ⇒ 一块；两个模型中间隔着空 ⇒ 两块。于是图上画出来的是一台台模型，
    /// 而不是把它们兜在一起的大方框。
    ///
    /// 单格也算一块（一个孤立的挤出点仍是板上的一件东西；真实切片里基本见不到 ——
    /// 一份 gcode 的点密到"每个模型一大片连通"，这里不额外过滤，宁可多画一个 2mm 小框，
    /// 也不许把"稀疏但真实"的样例切成零个框）。
    pub fn boxes(&self) -> Vec<GBBox> {
        let mut out = Vec::new();
        if self.bits.is_empty() {
            return out;
        }
        let mut seen = vec![false; self.bits.len()];
        let mut stack: Vec<(usize, usize)> = Vec::new();
        for cy in 0..self.rows {
            for cx in 0..self.cols {
                let idx = cy * self.cols + cx;
                if !self.bits[idx] || seen[idx] {
                    continue;
                }
                seen[idx] = true;
                stack.push((cx, cy));
                let (mut x0, mut x1, mut y0, mut y1) = (cx, cx, cy, cy);
                while let Some((x, y)) = stack.pop() {
                    x0 = x0.min(x);
                    x1 = x1.max(x);
                    y0 = y0.min(y);
                    y1 = y1.max(y);
                    let neighbours = [
                        (x.wrapping_sub(1), y),
                        (x + 1, y),
                        (x, y.wrapping_sub(1)),
                        (x, y + 1),
                    ];
                    for (nx, ny) in neighbours {
                        if nx >= self.cols || ny >= self.rows {
                            continue;
                        }
                        let i = ny * self.cols + nx;
                        if self.bits[i] && !seen[i] {
                            seen[i] = true;
                            stack.push((nx, ny));
                        }
                    }
                }
                out.push(GBBox {
                    x_min: self.origin_x + x0 as f64 * self.cell,
                    x_max: self.origin_x + (x1 as f64 + 1.0) * self.cell,
                    y_min: self.origin_y + y0 as f64 * self.cell,
                    y_max: self.origin_y + (y1 as f64 + 1.0) * self.cell,
                });
            }
        }
        out
    }
}

/// `DetectTowerCollision`：检测擦料塔包围盒与模型/原塔挤出点的冲突。
/// `UseTowers=false` 时直接返回空结果（无 MKP 擦料塔，跳过检测）。
pub fn detect_tower_collision(content: &[String], ir_data: &Ir) -> CollisionResult {
    let mut result = CollisionResult::default();

    if !ir_data.tower.use_towers {
        return result;
    }

    /* 热床按 gcode 判（见 [`printable_area_of`]）—— 放在所有早退之前，
    禁区那一档的事实同样要带得出门 */
    result.printable = printable_area_of(content);

    /*
     * ★★ **塔高 = 这份 gcode 最高的 Z**（2026-10-18）。斜肋外墙的底座随高度变大
     * （`MIN_DEPTH_PER_HEIGHT` 那张表），少了这个数，碰撞框会按"最矮的塔"算 ——
     * 于是自动摆出来的位置看着不撞、真印起来底座压模型。作者真机踩到过。
     */
    result.tower_height = max_z_of(content);
    /*
     * 避让用的占用格：**与"撞没撞"同一批挤出点**（板内），见 [`Occupancy`]。
     * 单独扫一遍（几十毫秒）—— 比把它塞进下面那条多层循环清楚得多。
     */
    let bed = bed_of(ir_data, result.printable);
    result.occupancy = occupancy_of(content, &bed);
    let tower_bbox = calculate_tower_collision_bbox(ir_data, result.tower_height);

    result.tower_bbox = Some(tower_bbox);
    let tower_poly: Polygon = vec![
        vec![tower_bbox.x_min, tower_bbox.y_min],
        vec![tower_bbox.x_max, tower_bbox.y_min],
        vec![tower_bbox.x_max, tower_bbox.y_max],
        vec![tower_bbox.x_min, tower_bbox.y_max],
    ];

    for zone in &ir_data.machine.forbidden_zones {
        if geom::polygons_intersect(&tower_poly, zone) {
            result.has_collision = true;
            result.collision_type = CollisionType::Forbidden;
            result.conflict_count = 1;
            result.direction_cn = "擦料塔位置落在机型额外禁区内".to_string();
            let (dx, dy) = geom::min_translation_vector(&tower_poly, zone);
            result.required_move_x = dx;
            result.required_move_y = dy;
            return result;
        }
    }

    /*
     * ★★★ **边缘带**（机型表 `edgeZone`，A1 = 10mm）：塔整块要落在"往里收一圈"的
     * 可摆放区里。与 [`auto_tower_position`] 共用 [`placement_bed`] —— 判的和摆的
     * 是同一条边。（2026-10-18 作者真机：「y 怎么可能为 0」—— 以前只有摆的算法，
     * 而且只按打印范围。）
     *
     * 归到 `Forbidden` 那一档：它和"机型额外禁区"是同一类事（板上一片不能放东西的
     * 区域），建议句与自动挪塔都走现成那条路。
     */
    let place = placement_bed(ir_data, &bed);
    let push = |min: f64, max: f64, lo: f64, hi: f64| -> f64 {
        if min < lo {
            lo - min
        } else if max > hi {
            hi - max
        } else {
            0.0
        }
    };
    let push_x = push(tower_bbox.x_min, tower_bbox.x_max, place.x_min, place.x_max);
    let push_y = push(tower_bbox.y_min, tower_bbox.y_max, place.y_min, place.y_max);
    if push_x != 0.0 || push_y != 0.0 {
        result.has_collision = true;
        result.collision_type = CollisionType::Forbidden;
        result.conflict_count = 1;
        result.direction_cn = "擦料塔压在板子边缘带上".to_string();
        result.required_move_x = push_x;
        result.required_move_y = push_y;
        return result;
    }

    detect_tower_collision_all_layers(
        content,
        &tower_bbox,
        ir_data,
        result.printable,
        result.tower_height,
        &result.occupancy,
    )
}

impl CollisionResult {
    /// **把检测结论变成"能带出门"的一份事实**（2026-10-18）。
    ///
    /// 建议那句按 `collision_type` / `adjustment_target` 挑码，正文由 `messages.toml`
    /// 渲染 —— 这里只给参数（方向 / X / Y），一个字都不拼。
    pub fn facts(&self, ir: &Ir) -> CollisionFacts {
        let kind = match self.collision_type {
            CollisionType::None => "none",
            CollisionType::Model => "model",
            CollisionType::PrimeTower => "prime-tower",
            CollisionType::Both => "both",
            CollisionType::Forbidden => "forbidden",
            CollisionType::WipeTowerNeeded => "wipe-tower-needed",
        };
        let target = match self.adjustment_target {
            AdjustmentTarget::Model => "model",
            AdjustmentTarget::WipeTower => "wipe-tower",
        };
        let (advice_code, advice_text) = self.advice();
        /*
         * **箭头**（2026-10-18 作者：「图片能不能做上引导移动的箭头呢」）：
         * 起点画在"挪谁"的中心 —— 挪模型 = 模型框中心，挪塔（禁区 / 挪不开）= 塔框中心；
         * 位移沿 `MoveDirection::delta()` 的单位分量 × 距离（禁区档的 MTV 本来就带符号）。
         * 位移为零 = 没有方向（挪不开那一档），前端就不画。
         */
        let center_of = |b: Option<GBBox>| match b {
            Some(b) => [(b.x_min + b.x_max) / 2.0, (b.y_min + b.y_max) / 2.0],
            None => [0.0, 0.0],
        };
        let (arrow_from, arrow_vec) = match self.collision_type {
            CollisionType::Forbidden => (
                center_of(self.tower_bbox),
                [self.required_move_x, self.required_move_y],
            ),
            CollisionType::None | CollisionType::WipeTowerNeeded => {
                (center_of(self.model_bbox), [0.0, 0.0])
            }
            _ => {
                let (dx, dy) = self.direction.delta();
                (
                    center_of(self.model_bbox),
                    [dx * self.required_move_x, dy * self.required_move_y],
                )
            }
        };
        CollisionFacts {
            kind: kind.to_owned(),
            target: target.to_owned(),
            direction: self.direction_cn.clone(),
            required_move_x: self.required_move_x,
            required_move_y: self.required_move_y,
            conflict_count: self.conflict_count,
            conflict_points: self
                .conflict_points
                .iter()
                .take(CONFLICT_POINT_CAP)
                .copied()
                .collect(),
            tower_bbox: self.tower_bbox.map(|b| Box4::of(&b)).unwrap_or_default(),
            model_bbox: self.model_bbox.map(|b| Box4::of(&b)).unwrap_or_default(),
            model_boxes: self.model_boxes.iter().map(Box4::of).collect(),
            printable: self.printable.unwrap_or(Box4 {
                x_min: ir.machine.min_x,
                x_max: ir.machine.max_x,
                y_min: ir.machine.min_y,
                y_max: ir.machine.max_y,
            }),
            tower: Some(TowerShape {
                outer_structure: ir.wiping.outer_structure.clone(),
                body_mm: TOWER_OUTER_BOUND - TOWER_INNER_BOUND,
                rib_width: ir.rib.width,
                rib_extra_length: ir.rib.extra_length,
                rib_fillet_wall: ir.rib.fillet_wall,
                sheath_base_expand: ir.sheath.base_expand,
            }),
            forbidden_zones: ir
                .machine
                .forbidden_zones
                .iter()
                .map(|zone| zone.iter().map(|p| [p[0], p[1]]).collect())
                .collect(),
            advice_code,
            advice_text,
            arrow_from,
            arrow_vec,
        }
    }

    /// 建议那句：码 + 正文（正文由 `messages.toml` 渲染）。
    ///
    /// 三条，按 `collision_type` 分：禁区 ⇒ 挪塔（_004）；算得出方向 ⇒ 挪模型（_002）；
    /// 三个候选方向都挪不开 ⇒ 改塔位置（_003）。
    fn advice(&self) -> (String, String) {
        /* 数字进模板要字符串（表里 `params` 那一档），一位小数够用 ——
        建议里的"至少几毫米"本来就是取整过的 `ceil_to_01`。 */
        let one_dp = |v: f64| format!("{:.1}", v.abs());
        /* 只列**真的要动**的那个轴（2026-10-18）：单轴方向下另一个轴是 0，
        旧模板固定写 `X 至少 …，Y 至少 …`，屏幕上就是 "Y 至少 0.0mm" 的噪音。
        分量表放 `amounts` 一个参数里，句子仍只在 `messages.toml` 有一份。 */
        let amounts = |x: f64, y: f64| -> String {
            let xs = if x.abs() > 0.0 {
                Some(format!("X 至少 {}mm", one_dp(x)))
            } else {
                None
            };
            let ys = if y.abs() > 0.0 {
                Some(format!("Y 至少 {}mm", one_dp(y)))
            } else {
                None
            };
            match (xs, ys) {
                (Some(x), Some(y)) => format!("{x}，{y}"),
                (Some(x), None) => x,
                (None, Some(y)) => y,
                (None, None) => "0mm".to_string(),
            }
        };
        let (code, params): (&str, Vec<(&str, String)>) = match self.collision_type {
            CollisionType::Forbidden => (
                "E_GCODE_COLLISION_004",
                vec![(
                    "amounts",
                    amounts(self.required_move_x, self.required_move_y),
                )],
            ),
            CollisionType::WipeTowerNeeded => ("E_GCODE_COLLISION_003", Vec::new()),
            _ => {
                if self.adjustment_target == AdjustmentTarget::Model
                    && !self.direction_cn.is_empty()
                {
                    (
                        "E_GCODE_COLLISION_002",
                        vec![
                            ("direction", self.direction_cn.clone()),
                            (
                                "amounts",
                                amounts(self.required_move_x, self.required_move_y),
                            ),
                        ],
                    )
                } else {
                    ("E_GCODE_COLLISION_003", Vec::new())
                }
            }
        };
        let refs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
        (code.to_string(), crate::diag::messages::text(code, &refs))
    }
}

/// 打印范围（板子）：gcode 的 `; printable_area` 优先，没写退回机型维度。
/// （`detect` 与自动摆塔必须用同一份 —— 两处各算一次就是"两套床"。）
fn bed_of(ir_data: &Ir, printable: Option<Box4>) -> Box4 {
    printable.unwrap_or(Box4 {
        x_min: ir_data.machine.min_x,
        x_max: ir_data.machine.max_x,
        y_min: ir_data.machine.min_y,
        y_max: ir_data.machine.max_y,
    })
}

/// 把所有**板内挤出点**打进占用格（[`Occupancy`]）。
/// 与 `detect_tower_collision_all_layers` 里"算不算一个点"同一套判据：
/// G0/G1 + 带 `E` + 有 X/Y。
pub fn occupancy_of(content: &[String], bed: &Box4) -> Occupancy {
    let mut occ = Occupancy::new(bed, 2.0);
    for line in content {
        let mut it = line.split_whitespace();
        let Some(cmd) = it.next() else { continue };
        if cmd != "G0" && cmd != "G1" {
            continue;
        }
        let (mut x, mut y, mut has_e, mut has_x, mut has_y) = (0.0f64, 0.0f64, false, false, false);
        for tok in it {
            if tok == ";" {
                break;
            }
            let mut chars = tok.chars();
            let Some(axis) = chars.next() else { continue };
            let rest = chars.as_str();
            match axis {
                'X' => {
                    if let Ok(v) = rest.parse::<f64>() {
                        x = v;
                        has_x = true;
                    }
                }
                'Y' => {
                    if let Ok(v) = rest.parse::<f64>() {
                        y = v;
                        has_y = true;
                    }
                }
                'E' => has_e = true,
                _ => {}
            }
        }
        if !(has_e && has_x && has_y) {
            continue;
        }
        if x < bed.x_min || x > bed.x_max || y < bed.y_min || y > bed.y_max {
            continue;
        }
        occ.mark(x, y);
    }
    occ
}

/// 这份 gcode 里**最高的 Z**（mm）= 这一趟的塔高（塔跟模型一样高）。
/// 只认 G0/G1 行里以 `Z` 开头的那个数；读不到 ⇒ 0（退回"最矮的塔"，与旧行为一致）。
pub fn max_z_of(content: &[String]) -> f64 {
    /*
     * ★ 先认切片器自报的那一行（`; max_z_height: 245.60`）—— 它是**模型的真实最高层**，
     * 比扫 G0/G1 稳：弧线（G2/G3）抬 Z、只写 `G1 F…` 换层的写法都会漏。
     * 读不到才退回逐行扫。
     */
    for line in content {
        let Some(rest) = line.trim().strip_prefix("; max_z_height:") else {
            continue;
        };
        if let Ok(z) = rest.trim().parse::<f64>() {
            return z;
        }
    }
    let mut top = 0.0f64;
    for line in content {
        let mut it = line.split_whitespace();
        let Some(cmd) = it.next() else { continue };
        if cmd != "G0" && cmd != "G1" {
            continue;
        }
        for tok in it {
            if tok == ";" {
                break;
            }
            /* MSRV 1.77 没有 let-chains ⇒ 拆成 `let … else` + `max`（也别招 clippy） */
            let Some(rest) = tok.strip_prefix('Z') else {
                continue;
            };
            if let Ok(z) = rest.parse::<f64>() {
                top = top.max(z);
            }
        }
    }
    top
}

/// **热床（可打印区）按 gcode 判**（2026-10-18 作者定的口径）。
///
/// 切片器在 CONFIG_BLOCK 里亲手写下了热床多边形（A1 mini：
/// `; printable_area = 0x0,180x0,180x180,0x180`）。它是"这一趟 gcode 到底认为床多大"
/// 的**当事人**，比机型 toml 更贴这一份文件 —— 取它的外接框。gcode 没写才退回 toml。
///
/// ★ 只读声明、不改判据：检测里"挪塔出界"的床框仍取 `ir.machine`（两者都来自同一个
/// 机型定义，通常恒等；改判据是另一件事，不趁这一刀）。
pub fn printable_area_of(content: &[String]) -> Option<Box4> {
    for line in content {
        let Some(rest) = line.trim().strip_prefix("; printable_area") else {
            continue;
        };
        let Some(value) = rest.trim_start().strip_prefix('=') else {
            continue;
        };
        let mut bbox: Option<Box4> = None;
        for pt in value.trim().split(',') {
            let (x, y) = pt.trim().split_once('x')?;
            let x: f64 = x.trim().parse().ok()?;
            let y: f64 = y.trim().parse().ok()?;
            bbox = Some(match bbox {
                Some(mut b) => {
                    b.x_min = b.x_min.min(x);
                    b.x_max = b.x_max.max(x);
                    b.y_min = b.y_min.min(y);
                    b.y_max = b.y_max.max(y);
                    b
                }
                None => Box4 {
                    x_min: x,
                    x_max: x,
                    y_min: y,
                    y_max: y,
                },
            });
        }
        /* 只有一个 CONFIG_BLOCK：读到（无论解析成没成）就收手 */
        return bbox;
    }
    None
}

/// `isExtrusionMove`：G0/G1/G2/G3 行含 " E" 且 rest 有 X/Y，NumStrip 取 [1]/[2]。
fn is_extrusion_move(line: &str) -> Option<(f64, f64)> {
    let trimmed = line.trim();
    if !trimmed.contains(" E") {
        return None;
    }
    let is_motion = [("G0", 3), ("G1", 3), ("G2", 3), ("G3", 3)]
        .iter()
        .any(|(pfx, _)| {
            // 零分配等价形态（Task 18.2）：原 starts_with(&format!("{pfx} "))×2
            // 是每行两次堆分配；语义 = pfx 后紧跟空格或制表符
            trimmed
                .strip_prefix(pfx)
                .is_some_and(|r| r.starts_with(' ') || r.starts_with('\t'))
        });
    if !is_motion {
        return None;
    }
    let code_end = 2;
    let rest = &trimmed[code_end..];
    if !rest.contains('X') || !rest.contains('Y') {
        return None;
    }
    let nums = num_strip(trimmed);
    if nums.len() >= 3 {
        Some((nums[1], nums[2]))
    } else {
        None
    }
}

/// ir.NumStrip 的本模块内联（与 ir::build 的实现同一语义，独立副本避免跨层引用）。
fn num_strip(line: &str) -> Vec<f64> {
    let mut result = Vec::with_capacity(8);
    let mut buf = String::with_capacity(64);
    let mut has_dot = false;
    fn flush(buf: &mut String, has_dot: &mut bool, out: &mut Vec<f64>) {
        if !buf.is_empty()
            && buf.as_str() != "-"
            && buf.as_str() != "."
            && let Ok(v) = buf.parse::<f64>()
        {
            out.push(v);
        }
        buf.clear();
        *has_dot = false;
    }
    for c in line.bytes() {
        if c == b'-' && buf.is_empty() {
            buf.push('-');
        } else if c.is_ascii_digit() {
            buf.push(c as char);
        } else if c == b'.' && !has_dot && !buf.is_empty() {
            buf.push('.');
            has_dot = true;
        } else {
            flush(&mut buf, &mut has_dot, &mut result);
        }
    }
    flush(&mut buf, &mut has_dot, &mut result);
    result
}

/* ---------- 自动摆塔（2026-10-18 作者定的口径） ---------- */

/// `wiper_x` / `wiper_y` 的**内核量程**（照 `preset/validate.rs` 那一栏：WiperX
/// `-40..260`、WiperY `0..265`）。按机型的注册表更严，但自动摆放那一步还要与
/// **打印范围**（板子）取交集，交集通常比注册表还紧 ⇒ 用这一档就够，也不必在
/// 内核这一层回头去读注册表。
const WIPER_X_MIN: f64 = -40.0;
const WIPER_X_MAX: f64 = 260.0;
const WIPER_Y_MIN: f64 = 0.0;
const WIPER_Y_MAX: f64 = 265.0;

/// 机型表的 `[dimensions] edgeZone`：板子最外面这一圈**不放擦料塔**。
///
/// 为什么读机型表而不是 IR 上的字段：IR 只抄了运动范围 / 涂胶区那几格，`edgeZone`
/// 没有对应字段；机型表是这一格的**唯一真值**（与 `fill_machine_dims_from_toml`
/// 同一个来源）。机型名为空 / 表里没有这台机器 ⇒ 0（= 不做这一道）。
fn edge_zone_of(ir_data: &Ir) -> f64 {
    if ir_data.machine.machine_type.is_empty() {
        return 0.0;
    }
    crate::postproc::machine_dims::get_machine_dimensions(&ir_data.machine.machine_type).edge_zone
}

/// **可摆放区** = 打印范围往里收一圈 `edgeZone`。
///
/// ★★★ 这是"塔能放哪"的**唯一**出口：自动摆塔（挑位置）与碰撞检测（判要不要挪）
/// 都走它。以前只有前者、而且只按打印范围 —— 于是自动摆出来的位置能贴在板边上
/// （作者真机看到「Y 怎么可能为 0」）。两个入口共用一条边，才不会出现
/// "判的是一套、摆的是另一套"。
fn placement_bed(ir_data: &Ir, bed: &Box4) -> Box4 {
    let e = edge_zone_of(ir_data);
    /* `is_nan` 单独写：`e <= 0.0` 对 NaN 是 false，不能拿它当"没有边缘带" */
    if e.is_nan() || e <= 0.0 {
        return *bed;
    }
    let mut b = *bed;
    /* 板子比两圈还小就整块不动（那种机器配的 edgeZone 本身就不成立） */
    if b.x_max - b.x_min > 2.0 * e {
        b.x_min += e;
        b.x_max -= e;
    }
    if b.y_max - b.y_min > 2.0 * e {
        b.y_min += e;
        b.y_max -= e;
    }
    b
}

/// **自动摆塔**（2026-10-18 作者定的口径：模型与塔相撞时，自动挑一个不撞的塔位置，
/// 画出来给用户确认；**只改这一趟的 `wiper_x` / `wiper_y`，不落盘**）。
///
/// 算法（作者选的"区间内网格搜索"）：在允许的 `wiper_x` / `wiper_y` 区间里按网格扫，
/// 挑**离原位最近**、且 ① 塔框整个落在**可摆放区**（打印范围扣掉 `edgeZone`）内
/// ② 不与占用格（模型 / 支撑 / 原塔的挤出点）相交 ③ 不与机型禁区相交 的那一个。
/// `None` = 整块板都摆不下 —— 那就只剩"自己挪模型 / 改工艺"两条路，由上层说。
pub fn auto_tower_position(
    ir_data: &Ir,
    bed: &Box4,
    occupancy: &Occupancy,
    forbidden: &[Vec<Vec<f64>>],
    tower_height: f64,
) -> Option<[f64; 2]> {
    /* 塔框按**这一趟的塔高**算（斜肋底座会随高度变大，见 `tower_height` 的注释） */
    let cur = calculate_tower_collision_bbox(ir_data, tower_height);
    let w = cur.x_max - cur.x_min;
    let h = cur.y_max - cur.y_min;
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    /* 边缘带先扣掉（检测那边用的是同一个函数 —— 摆出来的位置必然过得了复检） */
    let bed = &placement_bed(ir_data, bed);
    let lo_x = bed.x_min.max(WIPER_X_MIN);
    let hi_x = bed.x_max.min(WIPER_X_MAX) - w;
    let lo_y = bed.y_min.max(WIPER_Y_MIN);
    let hi_y = bed.y_max.min(WIPER_Y_MAX) - h;
    if hi_x < lo_x || hi_y < lo_y {
        return None;
    }
    /* 步长取塔短边的四分之一：细到不会漏掉一条缝，粗到不至于把整块板扫成几万次 */
    let step = (w.min(h) / 4.0).max(0.5);
    let orig = [ir_data.wiping.wiper_x, ir_data.wiping.wiper_y];
    let mut best: Option<(f64, [f64; 2])> = None;
    let mut x = lo_x;
    while x <= hi_x + 1e-9 {
        let mut y = lo_y;
        while y <= hi_y + 1e-9 {
            if tower_fits(x, y, w, h, bed, occupancy, forbidden) {
                let d = (x - orig[0]).powi(2) + (y - orig[1]).powi(2);
                /* 显式 match：MSRV 1.77 没有 `Option::is_none_or`，
                `map_or(true, …)` 又要挨 clippy 那一刀 */
                let nearer = match best {
                    Some((bd, _)) => d < bd - 1e-9,
                    None => true,
                };
                if nearer {
                    best = Some((d, [x, y]))
                }
            }
            y += step;
        }
        x += step;
    }
    best.map(|(_, p)| p)
}

/// [`auto_tower_position`] 的"照事实那一档"（钩子那一侧用这个）：床取 `facts.printable`
/// （就是俯视图画的那块板 —— 与图上看到的是同一件事实），禁区取 IR 上那份，
/// 模型框取 `model`（`None` = 这一趟没有模型包围盒，比如禁区那一档 ⇒ 只避禁区）。
pub fn auto_tower_position_for_facts(
    ir_data: &Ir,
    facts: &CollisionFacts,
    occupancy: &Occupancy,
    tower_height: f64,
) -> Option<[f64; 2]> {
    auto_tower_position(
        ir_data,
        &facts.printable,
        occupancy,
        &ir_data.machine.forbidden_zones,
        tower_height,
    )
}

/// 塔摆在 `(x, y)`（**左下角**）行不行：整个在打印范围内、不碰模型、不在禁区。
fn tower_fits(
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    bed: &Box4,
    occupancy: &Occupancy,
    forbidden: &[Vec<Vec<f64>>],
) -> bool {
    let b = GBBox {
        x_min: x,
        x_max: x + w,
        y_min: y,
        y_max: y + h,
    };
    if b.x_min < bed.x_min || b.x_max > bed.x_max || b.y_min < bed.y_min || b.y_max > bed.y_max {
        return false;
    }
    /*
     * 与 detect **同一批点**：占一格就算撞 —— 不再拿"板内点的 AABB"当替身
     * （包围盒看不见模型轮廓、原擦料塔、支撑；挪完照撞就是这么来的）。
     */
    if occupancy.overlaps(&b) {
        return false;
    }
    let poly = vec![
        vec![b.x_min, b.y_min],
        vec![b.x_max, b.y_min],
        vec![b.x_max, b.y_max],
        vec![b.x_min, b.y_max],
    ];
    !forbidden
        .iter()
        .any(|zone| geom::polygons_intersect(&poly, zone))
}

fn detect_tower_collision_all_layers(
    content: &[String],
    tower_bbox: &GBBox,
    ir_data: &Ir,
    printable: Option<Box4>,
    tower_height: f64,
    occupancy: &Occupancy,
) -> CollisionResult {
    /* 判据用的塔框跟着结论一起出门（报告页画俯视图要它）；热床框同样带上
    （gcode 里写了 `; printable_area` 就听 gcode 的，见 [`printable_area_of`]） */
    let mut result = CollisionResult {
        tower_bbox: Some(*tower_bbox),
        printable,
        tower_height,
        occupancy: occupancy.clone(),
        ..CollisionResult::default()
    };

    let mut processed_count: i64 = 0;
    let mut model_conflict_count: i64 = 0;
    let mut prime_tower_conflict_count: i64 = 0;
    let mut in_prime_tower = false;
    let mut conflict_points: Vec<[f64; 2]> = Vec::new();

    let mut model_bbox_tracking = false;
    let (mut model_min_x, mut model_max_x, mut model_min_y, mut model_max_y) = (0.0, 0.0, 0.0, 0.0);
    let mut prime_tower_bbox_tracking = false;
    let (mut pt_min_x, mut pt_max_x, mut pt_min_y, mut pt_max_y) = (0.0, 0.0, 0.0, 0.0);

    /*
     * ★★★ **"床"有两个，别混**（2026-10-18 作者定的口径）：
     * **运动范围** = 机型 toml 的维度（A1 是 300×255）—— 那是喷头能到哪；
     * **打印范围** = 板子（gcode 的 `; printable_area`，见 [`printable_area_of`]，
     * 没写才退回机型维度）—— 那是模型能待哪。
     * ★ 模型包围盒的统计闸和"挪出去出界"的判据**都按打印范围**：喷头能跑到板外
     * 不代表模型可以摆到板外（作者：「再往左上岂不是都到板子外面了吗？你要判断
     * 移动后的模型，会不会超过板子的范围」）。
     */
    let bed = printable.unwrap_or(Box4 {
        x_min: ir_data.machine.min_x,
        x_max: ir_data.machine.max_x,
        y_min: ir_data.machine.min_y,
        y_max: ir_data.machine.max_y,
    });

    /* 只装**模型**的占用格（不含切片器的 prime tower）—— 扫完按连通块切成一个个框 */
    let mut model_occ = Occupancy::new(&bed, 2.0);

    let total_lines = content.len();
    let (sample_interval, max_conflict_points) = if total_lines > 10000 {
        (500i64, 500usize)
    } else if total_lines > 5000 {
        (150i64, 200usize)
    } else {
        (50i64, 200usize)
    };
    let max_cal_points = if total_lines > 10000 {
        500usize
    } else if total_lines > 5000 {
        150usize
    } else {
        100usize
    };
    let mut cal_points: Vec<[f64; 2]> = Vec::new();
    let _ = &mut cal_points;
    cal_points.reserve(max_cal_points);

    for line in content {
        let trimmed = line.trim();

        if crate::gcode::has_slicer_comment_prefix(trimmed, "FEATURE") {
            in_prime_tower =
                crate::gcode::has_slicer_comment_prefix(trimmed, "FEATURE: Prime tower");
            continue;
        }

        let Some((x_coord, y_coord)) = is_extrusion_move(line) else {
            continue;
        };

        processed_count += 1;

        if processed_count % sample_interval == 0 && cal_points.len() < max_cal_points {
            cal_points.push([x_coord, y_coord]);
        }

        let in_collision = x_coord >= tower_bbox.x_min
            && x_coord <= tower_bbox.x_max
            && y_coord >= tower_bbox.y_min
            && y_coord <= tower_bbox.y_max;

        if in_collision && conflict_points.len() < max_conflict_points {
            conflict_points.push([x_coord, y_coord]);
        }

        /*
         * ★ **板外的挤出不算"模型"**（2026-10-18 真机：A1 的清料 / 擦拭在 X<0 那段
         * 也带挤出 ⇒ 包围盒被拽出板外，俯视图上模型框伸到板子外面，用户看不懂）。
         * 包围盒只统计**打印范围之内**的挤出点（板子，见上面的 `bed` —— 不是运动
         * 范围）；冲突判定（上面那段）不动 —— 塔在板内，板外的点本来也进不了塔框。
         */
        let in_bed = x_coord >= bed.x_min
            && x_coord <= bed.x_max
            && y_coord >= bed.y_min
            && y_coord <= bed.y_max;
        if !in_bed {
            continue;
        }

        if in_prime_tower {
            if !prime_tower_bbox_tracking {
                prime_tower_bbox_tracking = true;
                pt_min_x = x_coord;
                pt_max_x = x_coord;
                pt_min_y = y_coord;
                pt_max_y = y_coord;
            } else {
                pt_min_x = pt_min_x.min(x_coord);
                pt_max_x = pt_max_x.max(x_coord);
                pt_min_y = pt_min_y.min(y_coord);
                pt_max_y = pt_max_y.max(y_coord);
            }
            if in_collision {
                prime_tower_conflict_count += 1;
            }
        } else {
            /* 一个个模型的框靠这片点云切（见 `Occupancy::boxes`） */
            model_occ.mark(x_coord, y_coord);
            if !model_bbox_tracking {
                model_bbox_tracking = true;
                model_min_x = x_coord;
                model_max_x = x_coord;
                model_min_y = y_coord;
                model_max_y = y_coord;
            } else {
                model_min_x = model_min_x.min(x_coord);
                model_max_x = model_max_x.max(x_coord);
                model_min_y = model_min_y.min(y_coord);
                model_max_y = model_max_y.max(y_coord);
            }
            if in_collision {
                model_conflict_count += 1;
            }
        }
    }

    result.conflict_count = model_conflict_count + prime_tower_conflict_count;

    let has_model_collision = model_conflict_count > 0 && model_bbox_tracking;
    let has_prime_tower_collision = prime_tower_conflict_count > 0 && prime_tower_bbox_tracking;

    if has_model_collision || has_prime_tower_collision {
        result.has_collision = true;
        if model_bbox_tracking {
            result.model_bbox = Some(GBBox {
                x_min: model_min_x,
                x_max: model_max_x,
                y_min: model_min_y,
                y_max: model_max_y,
            });
        }

        let mut ref_bbox = GBBox::default();
        if has_model_collision {
            ref_bbox = GBBox {
                x_min: model_min_x,
                x_max: model_max_x,
                y_min: model_min_y,
                y_max: model_max_y,
            };
            result.collision_type = CollisionType::Model;
        }
        if has_prime_tower_collision {
            let pt_bbox = GBBox {
                x_min: pt_min_x,
                x_max: pt_max_x,
                y_min: pt_min_y,
                y_max: pt_max_y,
            };
            if has_model_collision {
                ref_bbox.x_min = ref_bbox.x_min.min(pt_bbox.x_min);
                ref_bbox.x_max = ref_bbox.x_max.max(pt_bbox.x_max);
                ref_bbox.y_min = ref_bbox.y_min.min(pt_bbox.y_min);
                ref_bbox.y_max = ref_bbox.y_max.max(pt_bbox.y_max);
                result.collision_type = CollisionType::Both;
            } else {
                ref_bbox = pt_bbox;
                result.collision_type = CollisionType::PrimeTower;
            }
        }

        /* ★ 出界判据按**打印范围**（板子）判，不是运动范围 —— 见上面的 `bed` */
        let bed_bbox = GBBox {
            x_min: bed.x_min,
            x_max: bed.x_max,
            y_min: bed.y_min,
            y_max: bed.y_max,
        };
        result.conflict_points = conflict_points.clone();
        result =
            resolve_model_adjustment(result, &ref_bbox, tower_bbox, &bed_bbox, &conflict_points);
    }

    /* 每个模型各自的框（2026-10-18 作者：「做出来，分开的独立的包围盒」）——
    与"有没有撞"无关：只要板上有模型点，就切出来，报告页照它画。 */
    result.model_boxes = model_occ.boxes();
    result
}

/// `resolveModelAdjustment`：基于实际冲突点算各方向最小退出距离。
fn resolve_model_adjustment(
    mut result: CollisionResult,
    ref_bbox: &GBBox,
    tower_bbox: &GBBox,
    bed_bbox: &GBBox,
    conflict_points: &[[f64; 2]],
) -> CollisionResult {
    let directions = build_adjustment_directions(ref_bbox, tower_bbox);

    let mut scored: Vec<(MoveDirection, f64, f64, f64)> = Vec::new();
    for dir_ in directions {
        let (dx, dy) = dir_.delta();
        let t = min_exit_distance(conflict_points, tower_bbox, dx, dy);
        if t <= 0.0 || t.is_nan() || t.is_infinite() {
            continue;
        }
        let moved = ref_bbox.translated(dx * t, dy * t);
        if !bed_bbox.contains(&moved) {
            continue;
        }
        scored.push((dir_, dx, dy, t));
    }

    scored.sort_by(|a, b| a.3.partial_cmp(&b.3).unwrap_or(std::cmp::Ordering::Equal));

    const EXIT_EPSILON: f64 = 0.05; // mm，确保冲突点严格离开 tower 边界
    let mut attempts: Vec<MoveAttempt> = Vec::with_capacity(3);
    for (i, s) in scored.iter().enumerate() {
        if i >= 3 {
            break;
        }
        let (_, dx, dy, t) = *s;
        let t_with_epsilon = t + EXIT_EPSILON;
        let move_x = geom::ceil_to_01(t_with_epsilon * dx.abs());
        let move_y = geom::ceil_to_01(t_with_epsilon * dy.abs());
        let moved = ref_bbox.translated(dx * t_with_epsilon, dy * t_with_epsilon);
        let out_of_bounds = !bed_bbox.contains(&moved);
        let collision = has_conflicts_after_move(
            conflict_points,
            tower_bbox,
            dx * t_with_epsilon,
            dy * t_with_epsilon,
        );
        attempts.push(MoveAttempt {
            direction: scored[i].0,
            move_x,
            move_y,
            collision,
            out_of_bounds,
        });
        if !out_of_bounds && !collision {
            result.direction = scored[i].0;
            result.direction_cn = scored[i].0.as_str_cn().to_string();
            result.required_move_x = move_x;
            result.required_move_y = move_y;
            result.adjustment_target = AdjustmentTarget::Model;
            result.attempts = attempts;
            return result;
        }
    }

    result.direction = MoveDirection::None;
    result.direction_cn = String::new();
    result.required_move_x = 0.0;
    result.required_move_y = 0.0;
    result.collision_type = CollisionType::WipeTowerNeeded;
    result.adjustment_target = AdjustmentTarget::WipeTower;
    result.attempts = attempts;
    result
}

/// 候选方向 = **背离塔的那几条轴**（2026-10-18 晚重写）。
///
/// 旧实现是"bbox 相交就给四个对角"，真机上出了两张票（作者的原话：
/// 「怎么这个箭头总是右上角呢」「而且这个方向也不对」）：
///
/// · 模型明明在塔的右边，却能建议「往左上挪」—— 那要整个越过塔到另一侧，
///   虽然算出来"不撞"，但没人会这么搬模型；
/// · 对角方向要求两个轴一起动，`X 至少 32.7mm，Y 至少 32.7mm` 每次都一样，
///   读数看着像常数 —— 而实际只需要沿一个轴走。
///
/// 现在的口径：
/// 1. 只在**确实重叠的轴**上给方向（不重叠的轴本来就不用动）；
/// 2. 方向必须**背离塔**：模型框中心在塔中心右边就只给「向右」，左边只给
///    「向左」—— 不允许把模型推到塔的另一侧；中心重合（模型横跨塔）时两侧
///    都给，让退出距离自己挑；
/// 3. **不再给对角**：模型框与塔框都是轴对齐的，单轴总能脱离；对角只会
///    多挪另一个轴的那一段，永远更远。
fn build_adjustment_directions(ref_bbox: &GBBox, tower_bbox: &GBBox) -> Vec<MoveDirection> {
    let x_overlap = ref_bbox.x_min < tower_bbox.x_max && ref_bbox.x_max > tower_bbox.x_min;
    let y_overlap = ref_bbox.y_min < tower_bbox.y_max && ref_bbox.y_max > tower_bbox.y_min;

    let ref_cx = (ref_bbox.x_min + ref_bbox.x_max) / 2.0;
    let ref_cy = (ref_bbox.y_min + ref_bbox.y_max) / 2.0;
    let tower_cx = (tower_bbox.x_min + tower_bbox.x_max) / 2.0;
    let tower_cy = (tower_bbox.y_min + tower_bbox.y_max) / 2.0;

    let mut dirs = Vec::with_capacity(2);
    if x_overlap {
        if (ref_cx - tower_cx).abs() < 1e-9 {
            dirs.push(MoveDirection::Left);
            dirs.push(MoveDirection::Right);
        } else if ref_cx > tower_cx {
            dirs.push(MoveDirection::Right);
        } else {
            dirs.push(MoveDirection::Left);
        }
    }
    if y_overlap {
        if (ref_cy - tower_cy).abs() < 1e-9 {
            dirs.push(MoveDirection::Back);
            dirs.push(MoveDirection::Front);
        } else if ref_cy > tower_cy {
            dirs.push(MoveDirection::Front);
        } else {
            dirs.push(MoveDirection::Back);
        }
    }
    dirs
}

fn has_conflicts_after_move(
    conflict_points: &[[f64; 2]],
    tower_bbox: &GBBox,
    move_x: f64,
    move_y: f64,
) -> bool {
    for p in conflict_points {
        let x = p[0] + move_x;
        let y = p[1] + move_y;
        if x >= tower_bbox.x_min
            && x <= tower_bbox.x_max
            && y >= tower_bbox.y_min
            && y <= tower_bbox.y_max
        {
            return true;
        }
    }
    false
}

fn min_exit_distance(conflict_points: &[[f64; 2]], tower_bbox: &GBBox, dx: f64, dy: f64) -> f64 {
    let mut t = 0.0;
    for p in conflict_points {
        let mut required = 0.0;
        let has_x = dx != 0.0;
        let has_y = dy != 0.0;
        if has_x && has_y {
            let mut tx = (tower_bbox.x_max - p[0]) / dx;
            if dx < 0.0 {
                tx = (tower_bbox.x_min - p[0]) / dx;
            }
            let mut ty = (tower_bbox.y_max - p[1]) / dy;
            if dy < 0.0 {
                ty = (tower_bbox.y_min - p[1]) / dy;
            }
            required = tx.max(ty);
        } else if has_x {
            required = (tower_bbox.x_max - p[0]) / dx;
            if dx < 0.0 {
                required = (tower_bbox.x_min - p[0]) / dx;
            }
        } else if has_y {
            required = (tower_bbox.y_max - p[1]) / dy;
            if dy < 0.0 {
                required = (tower_bbox.y_min - p[1]) / dy;
            }
        }
        if required > t {
            t = required;
        }
    }
    t
}

/// `calculateTowerCollisionBBox`：塔碰撞 BBox（wiperXY 为左下角）。
/// towerHeight=0 跳过 minDepth 限制（碰撞用保守估计）。
fn calculate_tower_collision_bbox(ir_data: &Ir, tower_height: f64) -> GBBox {
    let local = calculate_tower_bbox(
        &ir_data.wiping.outer_structure,
        ir_data.sheath.base_expand,
        ir_data.rib.extra_length,
        ir_data.rib.width,
        tower_height,
        ir_data.machine.nozzle_diameter,
        ir_data.machine.first_layer_height,
    );
    GBBox {
        x_min: ir_data.wiping.wiper_x,
        x_max: ir_data.wiping.wiper_x + local.width(),
        y_min: ir_data.wiping.wiper_y,
        y_max: ir_data.wiping.wiper_y + local.height(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Ir;

    fn tower_ir() -> Ir {
        // wiper (20,20) + brim bbox：local [2.96, 37.04]² → machine [22.96, 57.04]²
        let mut ir = Ir::default();
        ir.tower.use_towers = true;
        ir.wiping.outer_structure = "brim".to_string();
        ir.wiping.wiper_x = 20.0;
        ir.wiping.wiper_y = 20.0;
        ir.sheath.base_expand = 5.0;
        ir.machine.nozzle_diameter = 0.4;
        ir.machine.first_layer_height = 0.2;
        ir.machine.min_x = 0.0;
        ir.machine.max_x = 256.0;
        ir.machine.min_y = 0.0;
        ir.machine.max_y = 256.0;
        ir
    }

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    /// 9.6 命中侧：挤出点落进塔包围盒 → has_collision + Model 类型 + 建议位移。
    #[test]
    fn collision_detected_when_model_extrudes_into_tower() {
        let ir = tower_ir();
        let content = lines(&[
            "G1 X100 Y100 E0.5 ; 远离塔",
            "G1 X30 Y30 E0.6 ; 塔内冲突点",
            "G1 X120 Y120 E0.7",
        ]);
        let r = detect_tower_collision(&content, &ir);
        assert!(r.has_collision);
        assert_eq!(r.collision_type, CollisionType::Model);
        assert!(r.conflict_count >= 1);
        assert!(!r.conflict_points.is_empty());
    }

    /// 9.6 不命中侧：模型全部远离塔 → 无冲突；且 UseTowers=false 时整体跳过。
    #[test]
    fn no_collision_when_model_away_or_towers_disabled() {
        let ir = tower_ir();
        let content = lines(&["G1 X100 Y100 E0.5", "G1 X150 Y150 E0.6"]);
        let r = detect_tower_collision(&content, &ir);
        assert!(!r.has_collision);
        assert_eq!(r.collision_type, CollisionType::None);

        let mut disabled = tower_ir();
        disabled.tower.use_towers = false;
        let r2 = detect_tower_collision(&content, &disabled);
        assert!(!r2.has_collision, "UseTowers=false 必须整体跳过检测");
    }

    /// **建议是跟着事实一起出门的**（2026-10-18，作者：「碰撞检测…移动的建议，
    /// 我的旧版是有移动的建议的」）：命中那一趟必须带得出"往哪挪、至少几毫米"。
    #[test]
    fn a_collision_carries_the_move_advice() {
        let ir = tower_ir();
        let content = lines(&[
            "G1 X100 Y100 E0.5 ; 远离塔",
            "G1 X30 Y30 E0.6 ; 塔内冲突点",
            "G1 X120 Y120 E0.7",
        ]);
        let r = detect_tower_collision(&content, &ir);
        assert!(r.has_collision);
        assert!(!r.direction_cn.is_empty(), "得给出方向：{:?}", r);
        assert!(
            r.required_move_x > 0.0 || r.required_move_y > 0.0,
            "得给出至少挪多少：{:?}",
            r
        );

        let facts = r.facts(&ir);
        assert_eq!(facts.kind, "model");
        assert_eq!(facts.target, "model");
        assert_eq!(facts.direction, r.direction_cn);
        assert_eq!(facts.advice_code, "E_GCODE_COLLISION_002");
        assert!(
            facts.advice_text.contains(&facts.direction) && !facts.advice_text.contains('{'),
            "建议那句要渲染好（不能有没填的占位符）：{}",
            facts.advice_text
        );
        /* 俯视图要的几何：塔框（含外围扩展）、模型框、可打印区、冲突点 */
        assert!(facts.tower_bbox.x_max > facts.tower_bbox.x_min);
        assert!(facts.model_bbox.x_max > facts.model_bbox.x_min);
        assert_eq!(facts.printable.x_max, 256.0);
        assert!(!facts.conflict_points.is_empty());
        assert!(
            facts.conflict_points.len() <= 200,
            "写进记录的冲突点要封顶：{}",
            facts.conflict_points.len()
        );
        let tower = facts.tower.as_ref().expect("画塔要的参数");
        assert_eq!(tower.outer_structure, "brim");
        assert!(
            (tower.body_mm - 19.58).abs() < 1e-6,
            "塔身边长：{}",
            tower.body_mm
        );

        /* **键名钉死**（2026-10-18 真机踩的）：序列化后是 `towerBbox`（`tower_bbox`
        的 camelCase），前端契约照这个写 —— 写成 `towerBBox` 大小写对不上，
        那边按"没给就不摆"整块退掉，屏上就是空的。 */
        let wire = serde_json::to_value(&facts).expect("事实可序列化");
        assert!(
            wire.get("towerBbox").is_some(),
            "键名必须是 towerBbox：{wire:?}"
        );
        assert!(
            wire.get("modelBbox").is_some(),
            "键名必须是 modelBbox：{wire:?}"
        );
        assert!(wire.get("conflictPoints").is_some());
    }

    /// **建议只往背离塔的那一侧挪，且只动一个轴**（2026-10-18 真机两张票）：
    ///
    /// · 模型主体在塔右边时，建议只能是「向右」—— 旧实现会给「向左上」
    ///   （越过整座塔到另一侧），箭头指回去，作者一眼看出不对；
    /// · 单轴就够脱离，`Y` 不许再跟着一起动（旧实现给出 `X 32.7 / Y 32.7`
    ///   那种看着像常数的读数）。
    #[test]
    fn the_advice_never_moves_the_model_across_the_tower() {
        let ir = tower_ir();

        /* 模型主体在塔右侧（x 40..120），只有左缘压进塔 —— 只许往右 */
        let right = detect_tower_collision(
            &lines(&[
                "G1 X40 Y30 E0.5 ; 塔内（塔 22.96..57.04）",
                "G1 X120 Y30 E0.6",
                "G1 X40 Y120 E0.7",
                "G1 X120 Y120 E0.8",
            ]),
            &ir,
        );
        assert!(right.has_collision);
        assert_eq!(
            right.direction,
            MoveDirection::Right,
            "模型在塔右边，不许建议往左：{right:?}"
        );
        assert_eq!(right.required_move_y, 0.0, "单轴就够，Y 不许跟着动");
        assert!(
            right.required_move_x > 0.0 && right.required_move_x < 34.1,
            "向右只需走到塔右缘（不到一个塔宽），不是对角那种两轴一起动：{}",
            right.required_move_x
        );

        /* 模型主体在塔左侧（x 22..40）—— 只许往左 */
        let left = detect_tower_collision(
            &lines(&[
                "G1 X22 Y30 E0.5",
                "G1 X40 Y30 E0.6 ; 塔内",
                "G1 X22 Y120 E0.7",
                "G1 X40 Y120 E0.8",
            ]),
            &ir,
        );
        assert!(left.has_collision);
        assert_eq!(
            left.direction,
            MoveDirection::Left,
            "模型在塔左边，不许建议往右：{left:?}"
        );
        assert_eq!(left.required_move_y, 0.0);
    }

    /// **热床按 gcode 判**（2026-10-18）：gcode 写了 `; printable_area` 就听 gcode 的，
    /// 没写才退回机型 toml 的维度。
    #[test]
    fn printable_bed_comes_from_the_gcode_when_declared() {
        let ir = tower_ir();
        let mut content = lines(&[
            "; CONFIG_BLOCK_START",
            "; printable_area = 0x0,180x0,180x180,0x180",
            "; printable_height = 180",
            "; printer_model = Bambu Lab A1 mini",
            "; CONFIG_BLOCK_END",
        ]);
        content.extend(lines(&["G1 X30 Y30 E0.6", "G1 X120 Y120 E0.7"]));
        let facts = detect_tower_collision(&content, &ir).facts(&ir);
        assert_eq!(
            facts.printable,
            Box4 {
                x_min: 0.0,
                x_max: 180.0,
                y_min: 0.0,
                y_max: 180.0
            },
            "gcode 写了热床 ⇒ 不是 toml 那个 256"
        );

        /* 没写 ⇒ 退回机型 toml（tower_ir 里是 0..256） */
        let fallback =
            detect_tower_collision(&lines(&["G1 X30 Y30 E0.6", "G1 X120 Y120 E0.7"]), &ir)
                .facts(&ir);
        assert_eq!(fallback.printable.x_max, 256.0);
        assert_eq!(fallback.printable.x_min, 0.0);
    }

    /// **板外的挤出不算"模型"**（2026-10-18 真机：A1 清料在 X<0 也带挤出 ⇒
    /// 模型框被拽出板外，俯视图上模型伸到板子外面）。
    #[test]
    fn the_model_bbox_stays_inside_the_bed() {
        let ir = tower_ir();
        let content = lines(&[
            "G1 X-13.5 Y10 E0.5 ; 板外的清料挤出",
            "G1 X30 Y30 E0.6",
            "G1 X120 Y120 E0.7",
        ]);
        let facts = detect_tower_collision(&content, &ir).facts(&ir);
        assert!(
            facts.model_bbox.x_min >= ir.machine.min_x
                && facts.model_bbox.y_min >= ir.machine.min_y,
            "模型框不许伸出板外：{:?}",
            facts.model_bbox
        );
        assert_eq!(facts.model_bbox.x_min, 30.0, "板外那一段不计入包围盒");
    }

    /// **建议挪动带得出箭头**（2026-10-18 作者：「图片能不能做上引导移动的箭头呢」）：
    /// 起点在模型框中心，位移 = 方向单位分量 × 距离（模长与建议挪的距离一致）。
    #[test]
    fn the_move_advice_carries_an_arrow() {
        let ir = tower_ir();
        let content = lines(&[
            "G1 X100 Y100 E0.5 ; 远离塔",
            "G1 X30 Y30 E0.6 ; 塔内冲突点",
            "G1 X120 Y120 E0.7",
        ]);
        let r = detect_tower_collision(&content, &ir);
        let facts = r.facts(&ir);
        assert_eq!(facts.arrow_from, [75.0, 75.0], "起点 = 模型框中心");
        let vec_len = (facts.arrow_vec[0] * facts.arrow_vec[0]
            + facts.arrow_vec[1] * facts.arrow_vec[1])
            .sqrt();
        let move_len =
            (r.required_move_x * r.required_move_x + r.required_move_y * r.required_move_y).sqrt();
        assert!(
            move_len > 0.0 && (vec_len - move_len).abs() < 1e-6,
            "箭头长度 = 建议挪的距离：vec {vec_len} vs move {move_len}"
        );
    }

    /// **挪出去的判据 = 板子（打印范围），不是运动范围**（2026-10-18 作者：
    /// 「再往左上岂不是都到板子外面了吗？你要判断移动后的模型，会不会超过板子的范围」）。
    ///
    /// 场景：运动范围给得很宽（A1 的 300×255 那种，喷头能跑到板外），板子只有 180；
    /// 模型贴着板缘 —— 往左 / 往上挪都在**运动范围**之内，但挪完模型出板 ⇒ 不许选；
    /// 四个对角方向全出板 ⇒ 建议换成"挪不开，调塔"（不是硬指一个出板的方向）。
    #[test]
    fn the_advised_move_judges_the_plate_not_the_motion_range() {
        let mut ir = tower_ir();
        /* 运动范围故意给宽（A1：喷头能出板） */
        ir.machine.min_x = -40.0;
        ir.machine.max_x = 280.0;
        ir.machine.min_y = -40.0;
        ir.machine.max_y = 255.0;

        let content = lines(&[
            "; printable_area = 0x0,180x0,180x180,0x180", // 板子只有 180
            "G1 X30 Y30 E0.5 ; 与塔相撞",
            "G1 X50 Y30 E0.6 ; 与塔相撞",
            "G1 X179 Y0.5 E0.7 ; 模型伸到板右下角",
            "G1 X25 Y60 E0.8",
        ]);
        let facts = detect_tower_collision(&content, &ir).facts(&ir);

        /* 四个对角方向挪完模型都会出板（x<0 或 x>180）⇒ 没有一个方向可选 ⇒
        建议换成"调塔"，而不是硬指一个出板的方向 */
        assert_eq!(
            facts.kind, "wipe-tower-needed",
            "挪不出板就该让用户调塔，方向：{}",
            facts.direction
        );
        assert_eq!(facts.arrow_vec, [0.0, 0.0], "调塔没有方向可画");
    }

    /// 同一条口径的另一面：**有能留在板内的方向时，建议挪完的模型必须还在板内**
    /// （贴板左缘的模型，向左出板的候选被否，选仍然在板内的那个）。
    #[test]
    fn the_advised_move_never_pushes_the_model_off_the_plate() {
        let mut ir = tower_ir();
        ir.machine.min_x = -40.0;
        ir.machine.max_x = 280.0;
        ir.machine.min_y = -40.0;
        ir.machine.max_y = 255.0;

        let content = lines(&[
            "; printable_area = 0x0,180x0,180x180,0x180",
            "G1 X30 Y30 E0.5",
            "G1 X50 Y30 E0.6",
            "G1 X30 Y55 E0.7",
            "G1 X50 Y55 E0.8",
        ]);
        let facts = detect_tower_collision(&content, &ir).facts(&ir);
        assert!(facts.kind == "model" || facts.kind == "both");

        /* 建议挪完的模型框必须还在板内（本场景：向左挪 2.91 就出板，被否） */
        let dx = facts.arrow_vec[0];
        let dy = facts.arrow_vec[1];
        let moved = (
            facts.model_bbox.x_min + dx.min(0.0),
            facts.model_bbox.x_max + dx.max(0.0),
            facts.model_bbox.y_min + dy.min(0.0),
            facts.model_bbox.y_max + dy.max(0.0),
        );
        assert!(
            moved.0 >= facts.printable.x_min
                && moved.1 <= facts.printable.x_max
                && moved.2 >= facts.printable.y_min
                && moved.3 <= facts.printable.y_max,
            "建议挪完的模型必须还在板内：{moved:?}（板 {:?}）",
            facts.printable
        );
    }

    /// 禁区那一档：挪的是**塔**不是模型（建议句也不同 ⇒ 码不同）。
    #[test]
    fn a_forbidden_tower_advises_moving_the_tower() {
        let mut ir = tower_ir();
        ir.machine.forbidden_zones = vec![vec![
            vec![22.0, 22.0],
            vec![60.0, 22.0],
            vec![60.0, 60.0],
            vec![22.0, 60.0],
        ]];
        let r = detect_tower_collision(&lines(&["G1 X200 Y200 E0.5"]), &ir);
        let facts = r.facts(&ir);
        assert_eq!(facts.kind, "forbidden");
        assert_eq!(facts.advice_code, "E_GCODE_COLLISION_004");
        assert!(
            facts.advice_text.contains("禁区"),
            "禁区那句要说清是禁区：{}",
            facts.advice_text
        );
        assert_eq!(facts.forbidden_zones.len(), 1);
    }

    /// 把一块矩形区域按 2mm 格打成"有东西"（模拟模型 / 支撑的挤出点云）
    fn solid(bed: &Box4, x0: f64, x1: f64, y0: f64, y1: f64) -> Occupancy {
        let mut occ = Occupancy::new(bed, 2.0);
        let mut y = y0;
        while y <= y1 {
            let mut x = x0;
            while x <= x1 {
                occ.mark(x, y);
                x += 2.0;
            }
            y += 2.0;
        }
        occ
    }

    /// **自动摆塔**（2026-10-18 作者定的口径）：挑一个不撞的塔位置，离原位最近。
    #[test]
    fn the_tower_is_relocated_to_the_nearest_free_place() {
        let ir = tower_ir();
        let bed = Box4 {
            x_min: 0.0,
            x_max: 256.0,
            y_min: 0.0,
            y_max: 256.0,
        };
        /*
         * 模型堵住左下那一带（塔原位 20..57 完全被压住）—— 现在避让对象是**占用格**
         * （与 detect 同一批挤出点），这里把那一片按 2mm 打成"有东西"。
         */
        let occ = solid(&bed, 0.0, 120.0, 0.0, 120.0);
        let at = auto_tower_position(&ir, &bed, &occ, &[], 0.0).expect("板这么大，摆得下");
        let w = 57.04 - 22.96;
        assert!(
            at[0] >= 0.0 && at[0] + w <= 256.0 && at[1] >= 0.0 && at[1] + w <= 256.0,
            "新位置必须整个在板内：{at:?}"
        );
        assert!(at[0] >= 120.0 || at[1] >= 120.0, "不许压在模型上：{at:?}");
        /*
         * 离原位最近：原位 (20,20) 在模型左下角里 —— 往右挪要动 ~108mm，往上挪只动
         * ~108mm 但**X 几乎不用动**（17.04 ≈ 原位那条网格线）。所以挑出来的是
         * "沿 Y 挪出去"，不是"沿 X 挪出去"（两个的平方距离差就在这条 X 上）。
         */
        assert!(
            at[1] >= 120.0 && at[1] <= 120.0 + w && at[0] < 60.0,
            "挑的是离原位最近的那一个（沿 Y 出去，X 几乎不动）：{at:?}"
        );
    }

    /// **真实样例**（`tests/fixtures/real/rib-tall-min.gcode`，从作者的
    /// `.5926.1.gcode` 裁出来的最小复现）：斜肋外墙 + 245.6mm 塔高 ——
    /// 塔高要从 header 的 `; max_z_height` 认，自动摆位不许压到模型点上。
    #[test]
    fn the_real_rib_sample_moves_the_tower_off_the_points() {
        let raw = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tests/fixtures/real/rib-tall-min.gcode"
        ))
        .expect("样例文件应当在库里");
        let content = lines(&raw.lines().collect::<Vec<_>>());
        let mut ir = tower_ir();
        ir.wiping.outer_structure = "rib".to_owned();

        let r = detect_tower_collision(&content, &ir);
        assert!(r.has_collision, "样例应当撞（否则这条判据没有意义）");
        assert!(
            (r.tower_height - 245.6).abs() < 0.01,
            "塔高要认 header 的 ; max_z_height：{}",
            r.tower_height
        );
        assert!(
            !r.model_boxes.is_empty(),
            "真实样例应当按连通块切出模型框（作者要的\"分开的\"那几个框）"
        );

        let facts = r.facts(&ir);
        let at = auto_tower_position_for_facts(&ir, &facts, &r.occupancy, r.tower_height)
            .expect("板上还有地方");
        let w = {
            let b = calculate_tower_collision_bbox(&ir, r.tower_height);
            b.x_max - b.x_min
        };
        let boxed = GBBox {
            x_min: at[0],
            x_max: at[0] + w,
            y_min: at[1],
            y_max: at[1] + w,
        };
        assert!(
            !r.occupancy.overlaps(&boxed),
            "自动摆出来的位置仍然压着模型点：{at:?}"
        );
    }

    /// **挪完还撞的真凶**：过去避让用的是 `model_bbox`（板内点的 AABB），
    /// 而"撞没撞"是逐点判的 —— 只有原擦料塔撞上时 `model_bbox` 根本没被算出来，
    /// 于是自动摆塔"避开一个空框"，挪完照撞。占用格装的是**全部挤出点**，
    /// 原塔那片点也在里面 ⇒ 它必须被避开。
    #[test]
    fn relocation_avoids_points_even_when_the_model_box_is_empty() {
        let ir = tower_ir();
        let bed = Box4 {
            x_min: 0.0,
            x_max: 256.0,
            y_min: 0.0,
            y_max: 256.0,
        };
        /* 只有"原擦料塔"那一小片（模型框压根没有），正好压在当前的塔位上 */
        let occ = solid(&bed, 30.0, 45.0, 30.0, 45.0);
        let at = auto_tower_position(&ir, &bed, &occ, &[], 0.0).expect("板上还有地方");
        let w = 57.04 - 22.96;
        let (x0, x1, y0, y1) = (at[0], at[0] + w, at[1], at[1] + w);
        assert!(
            !(x0 < 45.0 && x1 > 30.0 && y0 < 45.0 && y1 > 30.0),
            "新塔框不许压到那片挤出点：{at:?}"
        );
    }

    /// 分开的独立包围盒（2026-10-18 作者：「做出来，分开的独立的包围盒」）：
    /// 两坨点中间隔着空 ⇒ 两个框；一个孤点自己一个小框。
    #[test]
    fn models_get_their_own_boxes() {
        let bed = Box4 {
            x_min: 0.0,
            x_max: 64.0,
            y_min: 0.0,
            y_max: 64.0,
        };
        let mut occ = Occupancy::new(&bed, 2.0);
        for x in [10.0, 12.0, 14.0] {
            for y in [10.0, 12.0] {
                occ.mark(x, y);
            }
        }
        for x in [40.0, 42.0] {
            for y in [40.0, 42.0, 44.0] {
                occ.mark(x, y);
            }
        }
        occ.mark(60.0, 4.0);
        let boxes = occ.boxes();
        assert_eq!(boxes.len(), 3, "两坨点 + 一个孤点 ⇒ 三个框：{boxes:?}");
        let has = |x0: f64, x1: f64, y0: f64, y1: f64| {
            boxes.iter().any(|b| {
                (b.x_min - x0).abs() < 1e-9
                    && (b.x_max - x1).abs() < 1e-9
                    && (b.y_min - y0).abs() < 1e-9
                    && (b.y_max - y1).abs() < 1e-9
            })
        };
        assert!(has(10.0, 16.0, 10.0, 14.0), "第一坨：{boxes:?}");
        assert!(has(40.0, 44.0, 40.0, 46.0), "第二坨：{boxes:?}");
        assert!(
            has(60.0, 62.0, 4.0, 6.0),
            "孤点自己一个 2mm 小框：{boxes:?}"
        );
    }

    /// **边缘带不许放塔**（2026-10-18 作者真机：自动摆出来的位置 `Y = 0.0` —— 贴在板边）。
    ///
    /// 机型表的 `edgeZone`（A1 = 10mm）是"最外面这一圈不放东西"。自动摆塔与碰撞检测
    /// 共用 `placement_bed`：摆出来的位置整个在圈内，复检也按同一条边判。
    #[test]
    fn the_edge_band_keeps_the_tower_off_the_border() {
        let mut ir = tower_ir();
        ir.machine.machine_type = "A1".to_owned();
        let bed = Box4 {
            x_min: 0.0,
            x_max: 256.0,
            y_min: 0.0,
            y_max: 256.0,
        };
        let w = 57.04 - 22.96;
        let at = auto_tower_position(&ir, &bed, &Occupancy::new(&bed, 2.0), &[], 0.0)
            .expect("空板摆得下");
        assert!(
            at[0] >= 10.0 - 1e-9 && at[1] >= 10.0 - 1e-9,
            "塔框不许压进边缘带（A1 edgeZone=10）：{at:?}"
        );
        assert!(
            at[0] + w <= 246.0 + 1e-9 && at[1] + w <= 246.0 + 1e-9,
            "另一侧同样不许贴边：{at:?}"
        );
    }

    /// 塔已经压在边缘带上 ⇒ 报 `forbidden`（= 改塔位置那一档），并给出往里推多少。
    #[test]
    fn a_tower_on_the_edge_band_is_pushed_inward() {
        let mut ir = tower_ir();
        ir.machine.machine_type = "A1".to_owned();
        /* 原位 (0,0) ⇒ 塔框从 2.96 起，压进 10mm 边缘带 */
        ir.wiping.wiper_x = 0.0;
        ir.wiping.wiper_y = 0.0;
        let r = detect_tower_collision(&lines(&["G1 X200 Y200 E0.5"]), &ir);
        assert!(r.has_collision, "贴边的塔要报出来");
        assert_eq!(r.conflict_count, 1);
        let facts = r.facts(&ir);
        assert_eq!(facts.kind, "forbidden");
        assert!(
            facts.required_move_x > 0.0 && facts.required_move_y > 0.0,
            "要往里推：x={} y={}",
            facts.required_move_x,
            facts.required_move_y
        );
    }

    /// 板子被模型占满 ⇒ 摆不下 ⇒ 明确说"没有"（由上层改口让用户自己动手）。
    #[test]
    fn relocation_gives_up_when_the_plate_has_no_room() {
        let ir = tower_ir();
        let bed = Box4 {
            x_min: 0.0,
            x_max: 256.0,
            y_min: 0.0,
            y_max: 256.0,
        };
        let occ = solid(&bed, 0.0, 256.0, 0.0, 256.0);
        assert!(auto_tower_position(&ir, &bed, &occ, &[], 0.0).is_none());
    }

    /// 禁区里不许摆 —— 自动摆出来的位置若落在禁区，等于把"塔在禁区"这个错换了个地方犯。
    #[test]
    fn relocation_skips_forbidden_zones() {
        let ir = tower_ir();
        let bed = Box4 {
            x_min: 0.0,
            x_max: 256.0,
            y_min: 0.0,
            y_max: 256.0,
        };
        let occ = solid(&bed, 0.0, 60.0, 0.0, 60.0);
        /* 下半张板全是禁区 */
        let zone = vec![
            vec![0.0, 0.0],
            vec![256.0, 0.0],
            vec![256.0, 60.0],
            vec![0.0, 60.0],
        ];
        let at = auto_tower_position(&ir, &bed, &occ, std::slice::from_ref(&zone), 0.0)
            .expect("上半区还有地方");
        assert!(at[1] >= 60.0, "禁区里不能摆：{at:?}");
    }

    /// **斜肋外墙的底座随高度变大**（2026-10-18 作者真机：「挪动后的还是碰撞了，
    /// 因为斜肋外墙随着高度会变大，下面会比较大」）。
    ///
    /// 碰撞框（也就自动摆塔的候选框）必须按**这一趟的塔高**算：高塔比矮塔宽。
    #[test]
    fn a_tall_rib_tower_gets_a_wider_footprint() {
        let mut ir = tower_ir();
        ir.wiping.outer_structure = "rib".to_owned();
        let low = calculate_tower_collision_bbox(&ir, 0.0);
        let tall = calculate_tower_collision_bbox(&ir, 240.0);
        assert!(
            tall.x_max - tall.x_min > (low.x_max - low.x_min) + 0.5,
            "高塔的框必须更宽：矮 {:?} / 高 {:?}",
            low.x_max - low.x_min,
            tall.x_max - tall.x_min
        );
    }

    /// 塔高从 gcode 里最高的那个 Z 读出来（读不到 ⇒ 0，退回旧行为）。
    #[test]
    fn the_tower_height_is_the_tallest_z() {
        let content = lines(&[
            "G1 X10 Y10 E0.2",
            "G1 Z0.2",
            "G1 Z12.4 ; 半截",
            "G0 Z48.6 ; 跳空",
            "G1 Z3.0",
        ]);
        assert_eq!(max_z_of(&content), 48.6);
        assert_eq!(max_z_of(&lines(&["G1 X1 Y1 E0.1"])), 0.0);
    }

    /// 禁区命中：塔多边形与 ForbiddenZones 相交 → Forbidden 类型 + MTV 位移。
    #[test]
    fn forbidden_zone_collision() {
        let mut ir = tower_ir();
        ir.machine.forbidden_zones = vec![vec![
            vec![22.0, 22.0],
            vec![60.0, 22.0],
            vec![60.0, 60.0],
            vec![22.0, 60.0],
        ]];
        let content = lines(&["G1 X200 Y200 E0.5"]);
        let r = detect_tower_collision(&content, &ir);
        assert!(r.has_collision);
        assert_eq!(r.collision_type, CollisionType::Forbidden);
        assert_eq!(r.direction_cn, "擦料塔位置落在机型额外禁区内");
    }
}
