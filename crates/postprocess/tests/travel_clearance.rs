//! 换笔段移动净空判据（防退化）。
//!
//! 背景见 `docs/GLUE-TRAVEL-Z-SAFETY-2026-10-20.md`：2026-10-20 高层打印时模型被撞倒，
//! 两个病灶都出在**换笔前后的长距离横移**上：
//!
//! - 「打印完 → 切笔位」原来写成 `G1 X{wipe_x} Z{层顶+1.0}` 的 XY+Z 斜插，
//!   起点净空为 0，喷嘴贴着刚打完的顶面滑行 120mm；
//! - 「装好笔 → 飞回涂胶点」原来落到 `last + offset_z + 3.0`，笔尖只比层顶高 2.8mm。
//!
//! 本测试用真实 fixture 跑完整 pass1 + pass2，逐行跟踪 Z，断言这四条：
//!
//! 0. `swap#1`（`to=pen`）在 `;Mounting Toolhead` 之前必须先出现 `G3` 小螺旋（z-hop 1mm）——
//!    禁止把脱层动作写成原地 `G1 Z`（会把料拉成丝），也不允许全程斜插（起点净空为 0）；
//!    螺旋之后才允许长距离横移；
//! 1. `swap#1` 在 `;Mounting Toolhead` 之前的任何 >20mm 横移，当前 Z 必须 ≥ `层顶 + 1.0`
//!    （此时笔收着，管的对象是喷嘴）；飞回涂胶点那段则必须 ≥ `层顶 + offset_z + lift`
//!    （笔已伸出，管的对象是笔尖）；
//! 2. `;Toolhead Mounted` 之后的第一条 Z 必须满足同一不等式；
//! 3. `paint` 块里**落笔那条 Z 之前**的 >20mm 横移（即飞回涂胶点那段）同样必须满足。
//!
//! 装笔机构动作（`;Mounting Toolhead` ~ `;Toolhead Mounted`，含 `L801` 与
//! `X256/261/267`）豁免：那是机械工作高度，不由这里的净空模型决定。
//!
//! 反空转：至少要检查到 3 个 swap#1 块和 3 次长距离横移，否则判据视为没接上。

mod golden_common;

use golden_common as gc;

/// 超过这个位移就算"长距离横移"（涂胶路径里的短段不受影响）。
const LONG_TRAVEL_MM: f64 = 20.0;
const EPS: f64 = 1e-6;

/// pass1 + pass2 全链（与 `marks_protocol.rs` 同一条路径）。
fn run_pass2_chain(input: &str) -> Vec<String> {
    let content: Vec<String> = gc::split_lines(input);
    let ir_json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    let mut ir_data: postprocess::ir::Ir =
        serde_json::from_str(&ir_json).expect("IR JSON 反序列化失败");
    let toml_machine = ir_data.machine.machine_type.clone();
    let mut progress = |_pct: f64, _msg: String| {};
    let p1 = postprocess::postproc::pass1::first_pass(
        &content,
        &mut ir_data,
        &toml_machine,
        &mut progress,
        &postprocess::diag::CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("first_pass 失败");
    let final_tower_height = p1.stats.max_z_height;
    let mut pass2_stats = Default::default();
    let p2 = postprocess::postproc::pass2::second_pass(
        p1,
        &mut ir_data,
        final_tower_height,
        &toml_machine,
        &mut progress,
        &mut pass2_stats,
        &postprocess::diag::CancelToken::new(),
        postprocess::postproc::cancel::DEFAULT_CANCEL_CHECK_INTERVAL,
    )
    .expect("second_pass 失败");
    postprocess::postproc::marks::finalize(p2.lines, &toml_machine)
}

struct LiftParams {
    z_offset: f64,
    lift: f64,
    lift_first_layers: f64,
}

impl LiftParams {
    /// 笔伸出态的安全高度：笔尖离层顶 = lift（`< 3.0` 走 first_layers 分支）。
    fn clearance(&self, layer_z: f64) -> f64 {
        let lift = if layer_z < 3.0 {
            self.lift_first_layers
        } else {
            self.lift
        };
        layer_z + self.z_offset + lift
    }
}

fn load_params() -> LiftParams {
    let ir_json = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/ir/golden_42274_2.json"),
    )
    .expect("读不到 IR fixture");
    let ir: postprocess::ir::Ir = serde_json::from_str(&ir_json).expect("IR JSON 反序列化失败");
    assert!(
        ir.glue.z_lift_height > 0.0,
        "反空转：IR 里 z_lift_height = {}，判据会退化成永真",
        ir.glue.z_lift_height
    );
    LiftParams {
        z_offset: ir.toolhead.z_offset,
        lift: ir.glue.z_lift_height,
        lift_first_layers: ir.glue.z_lift_height_first_layers,
    }
}

#[derive(Default, Clone, Copy)]
struct Pos {
    x: f64,
    y: f64,
    z: f64,
}

/// 解析 G0/G1/G2/G3 的 X/Y/Z（`G2/G3` 只取 Z，圆弧端点不参与距离判断）。
fn parse_coords(line: &str) -> Option<(Option<f64>, Option<f64>, Option<f64>)> {
    let code = line.split(';').next()?.trim();
    let first = code.split_whitespace().next()?;
    if !matches!(first, "G0" | "G1" | "G2" | "G3") {
        return None;
    }
    let mut x = None;
    let mut y = None;
    let mut z = None;
    for tok in code.split_whitespace().skip(1) {
        let axis = tok.chars().next();
        let value = tok.get(1..).and_then(|v| v.parse::<f64>().ok());
        match (axis, value) {
            (Some('X'), Some(n)) => x = Some(n),
            (Some('Y'), Some(n)) => y = Some(n),
            (Some('Z'), Some(n)) => z = Some(n),
            _ => {}
        }
    }
    if x.is_none() && y.is_none() && z.is_none() {
        return None;
    }
    Some((x, y, z))
}

#[test]
fn swap_travel_keeps_pen_clearance() {
    let params = load_params();
    let lines = run_pass2_chain(&gc::read_golden("42274.2.gcode"));

    let mut layer_z: f64 = 0.0;
    let mut pos = Pos::default();

    let mut in_swap_pen = false; // `;MKP_BEGIN swap ... to=pen` 区间
    let mut pre_mount = false; // 其中「到 `;Mounting Toolhead` 为止」那一段
    let mut pending_mounted_z = false; // 刚见 `;Toolhead Mounted`，等第一条 Z
    let mut in_paint = false;
    let mut paint_landed = false; // paint 块里已经落笔（之后是涂胶路径，不再检查）

    let mut swap_blocks = 0usize;
    let mut checked_travels = 0usize;
    let mut spiral_seen = false;
    let mut spiral_blocks = 0usize;

    for (idx, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        let ln = idx + 1;

        // 切片器注释有 `;Z_HEIGHT:` 与 `; Z_HEIGHT:` 两种写法，都要认
        if let Some(v) = line
            .strip_prefix(';')
            .map(str::trim_start)
            .and_then(|c| c.strip_prefix("Z_HEIGHT:"))
        {
            if let Ok(z) = v.trim().parse::<f64>() {
                layer_z = z;
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix(";MKP_BEGIN swap") {
            // 标记本身带层顶（`z=`），比依赖切片器注释更可靠
            if let Some(v) = rest
                .rsplit(" z=")
                .next()
                .and_then(|s| s.trim().parse::<f64>().ok())
            {
                layer_z = v;
            }
            if rest.contains("to=pen") {
                in_swap_pen = true;
                pre_mount = true;
                spiral_seen = false;
                swap_blocks += 1;
            }
            continue;
        }
        if line == ";MKP_END swap" {
            in_swap_pen = false;
            pre_mount = false;
            continue;
        }
        if line.starts_with(";MKP_BEGIN paint") {
            in_paint = true;
            paint_landed = false;
            continue;
        }
        if line == ";MKP_END paint" {
            in_paint = false;
            continue;
        }
        if line == ";Mounting Toolhead" {
            assert!(
                spiral_seen,
                "第 {ln} 行：换笔块在装笔前没有出现 G3 小螺旋 —— \
                 抬升必须用螺旋，不能原地 G1 Z（会拉丝）"
            );
            pre_mount = false;
            continue;
        }
        if line == ";Toolhead Mounted" {
            pending_mounted_z = true;
            continue;
        }

        let Some((x, y, z)) = parse_coords(line) else {
            continue;
        };

        // ⓪ 去切笔位那一段：抬升必须是 G3 小螺旋，不能是原地 G1 Z
        //    （原地垂直抬升会把熔融料拉成丝柱；全程斜插又会让起点净空为 0）
        if pre_mount {
            let is_spiral = line.starts_with("G3 ") && z.is_some();
            let is_vertical_lift =
                line.starts_with("G1 ") && z.is_some() && x.is_none() && y.is_none();
            if is_spiral {
                // 小螺旋只做 z-hop：此时笔收着，管的对象是喷嘴，不用抬到笔尖净空高度
                let need = layer_z + 1.0;
                let got = z.unwrap_or_default();
                assert!(
                    (got - need).abs() < EPS,
                    "第 {ln} 行 {line:?}：螺旋 z-hop 高度 Z={got} 不等于 {need}（层顶 + 1.0）"
                );
                spiral_seen = true;
                spiral_blocks += 1;
            } else if is_vertical_lift && !spiral_seen {
                panic!(
                    "第 {ln} 行 {line:?}：换笔块在脱离层顶前出现原地 G1 Z 抬升 —— \
                     会拉丝，必须先用 G3 小螺旋（I.706 J.991 P1）脱层"
                );
            }
        }

        // ② 装好笔之后的第一条 Z：飞回涂胶点的高度基准
        if let (true, Some(zv)) = (pending_mounted_z, z) {
            let need = params.clearance(layer_z);
            assert!(
                zv + EPS >= need,
                "第 {ln} 行 {line:?}：装笔后的移动高度 Z={zv} 低于安全高度 {need}\
                 （层顶 {layer_z} + offset_z {} + lift）",
                params.z_offset
            );
            pending_mounted_z = false;
        }

        let dxy = if x.is_none() && y.is_none() {
            0.0
        } else {
            let nx = x.unwrap_or(pos.x);
            let ny = y.unwrap_or(pos.y);
            ((nx - pos.x).powi(2) + (ny - pos.y).powi(2)).sqrt()
        };

        // ① 去切笔位 / ③ 飞回涂胶点：长距离横移必须在各自的高度基准上
        let go_to_swap = in_swap_pen && pre_mount;
        let go_to_glue = in_paint && !paint_landed;
        if (go_to_swap || go_to_glue) && dxy > LONG_TRAVEL_MM {
            // 去切笔位：笔还收着 → 约束对象是喷嘴，脱离层顶 1.0 即可；
            // 飞回涂胶点：笔已伸出 → 约束对象是笔尖，必须到 层顶 + offset_z + lift。
            let need = if go_to_swap {
                layer_z + 1.0
            } else {
                params.clearance(layer_z)
            };
            assert!(
                pos.z + EPS >= need,
                "第 {ln} 行 {line:?}：横移 {dxy:.1}mm 时 Z={} 低于安全高度 {need}；\
                 换笔横移必须「先抬后走」",
                pos.z
            );
            checked_travels += 1;
        }

        if in_paint && !paint_landed && z.is_some() {
            paint_landed = true;
        }

        if let Some(v) = x {
            pos.x = v;
        }
        if let Some(v) = y {
            pos.y = v;
        }
        if let Some(v) = z {
            pos.z = v;
        }
    }

    println!(
        "swap#1 块 {swap_blocks} 个（{spiral_blocks} 个走 G3 螺旋），\
         检查长距离横移 {checked_travels} 次"
    );
    assert!(
        swap_blocks >= 3,
        "反空转：只扫到 {swap_blocks} 个 swap#1 块，判据没接上"
    );
    assert_eq!(
        spiral_blocks,
        swap_blocks,
        "有 {} 个换笔块没有用 G3 螺旋抬升",
        swap_blocks - spiral_blocks
    );
    assert!(
        checked_travels >= 3,
        "反空转：只检查到 {checked_travels} 次长距离横移，判据没接上"
    );
}
