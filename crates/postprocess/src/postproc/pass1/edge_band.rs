//! **涂胶边缘带**（2026-11-16）：把贴近涂胶区域边界的那几段标出来，交给发射端抬高落笔。
//!
//! # 为什么不是「内缩」（这一块换掉了什么）
//!
//! 2026-11-16 之前这里是 `inset`：取**整层涂胶点的一个外接矩形**，四边各向内收 N mm，
//! 把落在矩形外的移动**裁掉**。作者实测判定它不能用：
//!
//! - 切的是**矩形的边**，不是形状的边 —— 小面整块消失、大面被切掉一整条；
//! - 凹形的内沿、环的内圈、斜边根本不在矩形边上 ⇒ 一点没动（「还有很多没缩到」）；
//! - 收空时整层退回原路径，用户以为开着、其实没生效。
//!
//! 作者要的是两件事：**只有边缘那几段要处理，里面的照旧**；处理方式也**不是删路径**，
//! 而是「照涂，把那段落笔 Z 抬高」—— 防的是 XY/Z 校准有偏差时笔跑出支撑面再回来刮到模型。
//!
//! # 这里怎么判「边缘」
//!
//! 涂胶路径只有一串 `G1 X Y`，没有多边形。做法是把这一层的涂胶点铺成一张**细格子**
//! （0.25mm，带一点画笔半径，让并排的涂胶线连成一片），然后算每个格子**离"没涂胶"的地方
//! 多远**（3-4 倒角距离）。离边界不足阈值的移动就是边缘带 —— 凹形内沿、环的内圈、
//! 斜边都会自然落进来，因为"没涂胶的地方"就在它们旁边。
//!
//! **阈值用百分比**：`宽度 = 百分比 × 这块面自己的短边`（同一层里每块面各算各的）。
//! 用 mm 的话，1mm 在 20mm 的小面上等于吃掉半个面，在 200mm 的大面上又几乎看不见。
//!
//! # 输出
//!
//! 只做两件事：在边缘/里面的交界处**插两个标记**（[`MARKER_EDGE_RAISE`] /
//! [`MARKER_EDGE_NORMAL`]），并**在交界处把移动切开**（一段移动可能一半在带内、一半在外）。
//! 路径一段都不删 —— 抬高/回落由 `glue_block::write_glue_lines` 认标记后做。
//!
//! `edge_lift <= 0` 或 `edge_percent <= 0` ⇒ **原样返回**（关闭时输出一个字节都不变）。

use crate::gcode::{
    MARKER_EDGE_NORMAL, MARKER_EDGE_RAISE, MARKER_ZJUMP_INTERNAL, MARKER_ZJUMP_START,
    extract_coord, format_float,
};

/// 格子的边长（mm）。涂胶线间距约一个喷嘴宽（0.4mm），这个粒度足够把并排的线连成一片。
const CELL_MM: f64 = 0.25;
/// 格子总数上限：超过就把格子放大，避免一份大件把内存吃爆。
const MAX_CELLS: f64 = 2_000_000.0;
/// 沿一条移动取样的步长（mm）——用于找边缘/里面的交界。
const SAMPLE_MM: f64 = 0.1;

/// 一条带坐标的 `G1` 移动，`line` 是它在输入里的行号。
struct Move {
    line: usize,
    from: (f64, f64),
    to: (f64, f64),
    /// 原行的挤出量。切开后按长度比例分给各子段 —— 发射端靠「段内有没有带 E 的移动」
    /// 判断这一段是不是空走。
    e: f64,
}

/// 这一层涂胶点的格子图 + 距离场。
struct EdgeGrid {
    x0: f64,
    y0: f64,
    cell: f64,
    nx: usize,
    ny: usize,
    /// 每个格子的连通块编号（0 = 没涂胶）
    comp: Vec<u32>,
    /// 每个连通块的**短边**（mm）—— 百分比的分母
    comp_short: Vec<f64>,
    /// 到最近「没涂胶格子」的距离（单位：1/3 格，3-4 倒角的整数权重）
    dist: Vec<u32>,
}

impl EdgeGrid {
    fn build(moves: &[Move]) -> Option<Self> {
        let (mut min_x, mut min_y) = (f64::MAX, f64::MAX);
        let (mut max_x, mut max_y) = (f64::MIN, f64::MIN);
        for m in moves {
            for p in [m.from, m.to] {
                min_x = min_x.min(p.0);
                min_y = min_y.min(p.1);
                max_x = max_x.max(p.0);
                max_y = max_y.max(p.1);
            }
        }
        if !min_x.is_finite() || max_x <= min_x || max_y <= min_y {
            return None;
        }
        /* 外圈留两格：距离场要有"外面"可量 */
        let pad = 2.0 * CELL_MM;
        let (x0, y0) = (min_x - pad, min_y - pad);
        let (w, h) = (max_x - min_x + 2.0 * pad, max_y - min_y + 2.0 * pad);
        let mut cell = CELL_MM;
        if (w / cell) * (h / cell) > MAX_CELLS {
            cell = (w * h / MAX_CELLS).sqrt().max(CELL_MM);
        }
        let nx = ((w / cell).ceil() as usize).max(1) + 1;
        let ny = ((h / cell).ceil() as usize).max(1) + 1;

        let mut covered = vec![false; nx * ny];
        let at = |x: f64, y: f64| -> usize {
            let ix = (((x - x0) / cell).round() as isize).clamp(0, nx as isize - 1) as usize;
            let iy = (((y - y0) / cell).round() as isize).clamp(0, ny as isize - 1) as usize;
            iy * nx + ix
        };
        /* 画笔：每个采样点连同 3×3 邻格一起标脏，并排的涂胶线才会连成一片 */
        for m in moves {
            let len = dist(m.from, m.to);
            let steps = ((len / (cell * 0.5)).ceil() as usize).clamp(1, 200_000);
            for k in 0..=steps {
                let p = lerp(m.from, m.to, k as f64 / steps as f64);
                let i = at(p.0, p.1);
                let (ix, iy) = (i % nx, i / nx);
                for dy in -1isize..=1 {
                    for dx in -1isize..=1 {
                        let (xx, yy) = (ix as isize + dx, iy as isize + dy);
                        if xx >= 0 && yy >= 0 && (xx as usize) < nx && (yy as usize) < ny {
                            covered[yy as usize * nx + xx as usize] = true;
                        }
                    }
                }
            }
        }

        /* 连通块标号 + 每块的短边（涂胶面可能就是好几张互不相连的面） */
        let mut comp = vec![0u32; nx * ny];
        let mut comp_short: Vec<f64> = vec![0.0];
        let mut next_id = 1u32;
        let mut stack: Vec<usize> = Vec::new();
        for i in 0..covered.len() {
            if !covered[i] || comp[i] != 0 {
                continue;
            }
            let id = next_id;
            next_id += 1;
            let (mut bx0, mut by0, mut bx1, mut by1) = (nx, ny, 1usize, 1usize);
            stack.push(i);
            comp[i] = id;
            while let Some(k) = stack.pop() {
                let (x, y) = (k % nx, k / nx);
                bx0 = bx0.min(x);
                by0 = by0.min(y);
                bx1 = bx1.max(x + 1);
                by1 = by1.max(y + 1);
                for (dx, dy) in [(1isize, 0isize), (-1, 0), (0, 1), (0, -1)] {
                    let (xx, yy) = (x as isize + dx, y as isize + dy);
                    if xx < 0 || yy < 0 || xx as usize >= nx || yy as usize >= ny {
                        continue;
                    }
                    let j = yy as usize * nx + xx as usize;
                    if covered[j] && comp[j] == 0 {
                        comp[j] = id;
                        stack.push(j);
                    }
                }
            }
            let w_mm = (bx1 - bx0) as f64 * cell;
            let h_mm = (by1 - by0) as f64 * cell;
            comp_short.push(w_mm.min(h_mm));
        }

        /* 3-4 倒角距离：每个涂胶格子到最近非涂胶格子的距离 */
        let mut dist: Vec<u32> = covered
            .iter()
            .map(|c| if *c { u32::MAX / 4 } else { 0 })
            .collect();
        for y in 0..ny {
            for x in 0..nx {
                let i = y * nx + x;
                if dist[i] == 0 {
                    continue;
                }
                let mut d = dist[i];
                if x > 0 {
                    d = d.min(dist[i - 1] + 3);
                }
                if y > 0 {
                    d = d.min(dist[i - nx] + 3);
                    if x > 0 {
                        d = d.min(dist[i - nx - 1] + 4);
                    }
                    if x + 1 < nx {
                        d = d.min(dist[i - nx + 1] + 4);
                    }
                }
                dist[i] = d;
            }
        }
        for y in (0..ny).rev() {
            for x in (0..nx).rev() {
                let i = y * nx + x;
                if dist[i] == 0 {
                    continue;
                }
                let mut d = dist[i];
                if x + 1 < nx {
                    d = d.min(dist[i + 1] + 3);
                }
                if y + 1 < ny {
                    d = d.min(dist[i + nx] + 3);
                    if x + 1 < nx {
                        d = d.min(dist[i + nx + 1] + 4);
                    }
                    if x > 0 {
                        d = d.min(dist[i + nx - 1] + 4);
                    }
                }
                dist[i] = d;
            }
        }

        Some(EdgeGrid {
            x0,
            y0,
            cell,
            nx,
            ny,
            comp,
            comp_short,
            dist,
        })
    }

    /// 找这个点**最近的那个涂胶格子**（点自己不在格子上时，比如正好落在两条线之间）。
    fn cell_of(&self, p: (f64, f64)) -> Option<usize> {
        let cx = (((p.0 - self.x0) / self.cell).round() as isize).clamp(0, self.nx as isize - 1);
        let cy = (((p.1 - self.y0) / self.cell).round() as isize).clamp(0, self.ny as isize - 1);
        let mut best: Option<(usize, i64)> = None;
        for dy in -2isize..=2 {
            for dx in -2isize..=2 {
                let (x, y) = (cx + dx, cy + dy);
                if x < 0 || y < 0 || x as usize >= self.nx || y as usize >= self.ny {
                    continue;
                }
                let i = y as usize * self.nx + x as usize;
                if self.comp[i] == 0 {
                    continue;
                }
                let d = (dx * dx + dy * dy) as i64;
                if best.is_none_or(|(_, bd)| d < bd) {
                    best = Some((i, d));
                }
            }
        }
        best.map(|(i, _)| i)
    }

    /// 这个点到「这块面的边界」有多远（mm）。量不出来 ⇒ `0`（当边缘处理，宁可多抬）。
    fn edge_distance(&self, p: (f64, f64)) -> f64 {
        let Some(i) = self.cell_of(p) else {
            return 0.0;
        };
        /* 3-4 倒角的单位是 1/3 格；再减一格是画笔半径带来的膨胀 */
        (self.dist[i] as f64 / 3.0 * self.cell - self.cell).max(0.0)
    }

    /// 这个点所在那张面的短边（mm）；不在任何面上 ⇒ `0`。
    fn face_short(&self, p: (f64, f64)) -> f64 {
        let Some(i) = self.cell_of(p) else {
            return 0.0;
        };
        let id = self.comp[i];
        self.comp_short.get(id as usize).copied().unwrap_or(0.0)
    }

    /// 这个点算不算边缘带（带宽 = 百分比 × 所在面的短边，最多半张面）。
    fn is_edge(&self, p: (f64, f64), percent: f64) -> bool {
        let short = self.face_short(p);
        if short <= 0.0 {
            return false;
        }
        let band = (percent / 100.0 * short).min(short / 2.0);
        band > 0.0 && self.edge_distance(p) < band
    }
}

/// 按边缘带切段并插标记。`percent <= 0` 或 `lift <= 0` ⇒ 原样返回。
pub(crate) fn apply(iface: &[String], percent: f64, lift: f64) -> Vec<String> {
    if percent <= 0.0 || lift <= 0.0 || iface.is_empty() {
        return iface.to_vec();
    }
    let moves = collect_moves(iface);
    if moves.len() < 2 {
        return iface.to_vec();
    }
    let Some(grid) = EdgeGrid::build(&moves) else {
        return iface.to_vec();
    };

    let mut by_line: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (i, m) in moves.iter().enumerate() {
        by_line.insert(m.line, i);
    }

    let mut out: Vec<String> = Vec::with_capacity(iface.len());
    /* 当前发射端的落笔档：`false` = 正常，`true` = 抬高。只在切换处插标记 */
    let mut raised = false;

    for (idx, line) in iface.iter().enumerate() {
        let Some(&mi) = by_line.get(&idx) else {
            out.push(line.clone());
            continue;
        };
        let m = &moves[mi];
        let pieces = split_edge_pieces(m, &grid, percent);
        if pieces.is_empty() {
            out.push(line.clone());
            continue;
        }
        for p in pieces {
            if p.edge != raised {
                out.push(if p.edge {
                    MARKER_EDGE_RAISE.to_string()
                } else {
                    MARKER_EDGE_NORMAL.to_string()
                });
                raised = p.edge;
            }
            let e = m.e * (p.t1 - p.t0);
            /* 整段没被切开 ⇒ 原行原样留下（少一次格式化，也不会因为重排丢参数） */
            if p.t0 == 0.0 && p.t1 == 1.0 {
                out.push(line.clone());
            } else {
                out.push(g1_move(p.to, e));
            }
        }
    }

    out
}

/// 一段子移动（`t0/t1` 是它在原移动里的参数区间）。
struct Piece {
    t0: f64,
    t1: f64,
    to: (f64, f64),
    edge: bool,
}

/// 把一条移动按「边缘 / 里面」切成若干段（可能整段都是一种）。
fn split_edge_pieces(m: &Move, grid: &EdgeGrid, percent: f64) -> Vec<Piece> {
    let len = dist(m.from, m.to);
    if len <= 1e-9 {
        return Vec::new();
    }
    let steps = ((len / SAMPLE_MM).ceil() as usize).clamp(1, 4000);
    let mut pieces: Vec<Piece> = Vec::new();
    let mut prev_edge = grid.is_edge(m.from, percent);
    let mut start_t = 0.0;
    let mut prev_t = 0.0;
    for k in 1..=steps {
        let t = k as f64 / steps as f64;
        let e = grid.is_edge(lerp(m.from, m.to, t), percent);
        if e != prev_edge {
            /* 交界落在 [prev_t, t] 里：二分找精确一点的位置 */
            let cross = bisect(m.from, m.to, prev_t, t, prev_edge, grid, percent);
            pieces.push(Piece {
                t0: start_t,
                t1: cross,
                to: lerp(m.from, m.to, cross),
                edge: prev_edge,
            });
            start_t = cross;
            prev_edge = e;
        }
        prev_t = t;
    }
    pieces.push(Piece {
        t0: start_t,
        t1: 1.0,
        to: m.to,
        edge: prev_edge,
    });
    pieces
}

fn bisect(
    a: (f64, f64),
    b: (f64, f64),
    t0: f64,
    t1: f64,
    edge_lo: bool,
    grid: &EdgeGrid,
    percent: f64,
) -> f64 {
    let (mut lo, mut hi) = (t0, t1);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if grid.is_edge(lerp(a, b, mid), percent) == edge_lo {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt()
}

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// 取 `iface` 里所有带坐标的 `G1`（跳过跳转标记与注释行）。
fn collect_moves(iface: &[String]) -> Vec<Move> {
    let mut moves = Vec::new();
    let mut cur: Option<(f64, f64)> = None;
    for (idx, line) in iface.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains(MARKER_ZJUMP_START) || trimmed.contains(MARKER_ZJUMP_INTERNAL) {
            cur = None;
            continue;
        }
        if !trimmed.starts_with("G1 ") {
            continue;
        }
        let x = extract_coord(trimmed.as_bytes(), b'X');
        let y = extract_coord(trimmed.as_bytes(), b'Y');
        if x.is_none() && y.is_none() {
            continue;
        }
        let from = match cur {
            Some(p) => p,
            None => (x.unwrap_or(0.0), y.unwrap_or(0.0)),
        };
        let to = (x.unwrap_or(from.0), y.unwrap_or(from.1));
        moves.push(Move {
            line: idx,
            from,
            to,
            e: extract_e(trimmed).unwrap_or(0.0),
        });
        cur = Some(to);
    }
    moves
}

/// 取 `G1` 行里的 E 值（`E.01111` 这种省前导零的写法一并认）。
fn extract_e(line: &str) -> Option<f64> {
    let idx = line.find(" E")?;
    let rest = &line[idx + 2..];
    let num: String = rest
        .chars()
        .take_while(|c| c.is_ascii_digit() || matches!(c, '.' | '-' | '+'))
        .collect();
    num.parse().ok()
}

/// 生成一条涂胶移动。E 最低留一个非零值，让发射端认得出这是涂胶移动。
fn g1_move(p: (f64, f64), e: f64) -> String {
    let e = if e.abs() < 1e-6 { 1e-5 } else { e };
    format!("G1 X{} Y{} E{:.5}", format_float(p.0), format_float(p.1), e)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一张 `size × size` 的蛇形涂胶面（线距 0.4mm，与真机一个量级）。
    fn square(size: f64, x0: f64, y0: f64) -> Vec<String> {
        let rows = (size / 0.4).round() as i32;
        let mut v = Vec::new();
        for row in 0..=rows {
            let y = y0 + row as f64 * 0.4;
            let (a, b) = if row % 2 == 0 {
                (x0, x0 + size)
            } else {
                (x0 + size, x0)
            };
            v.push(format!(
                "G1 X{} Y{} E0.01",
                format_float(a),
                format_float(y)
            ));
            v.push(format!(
                "G1 X{} Y{} E0.01",
                format_float(b),
                format_float(y)
            ));
        }
        v
    }

    /// 40×40 的实心面，中间挖一个 20×20 的洞（真机上的环 / 回字形支撑面）。
    ///
    /// **蛇形**：奇偶行方向相反，行与行之间才是一条 0.4mm 的折返而不是横穿整面。
    fn filled_ring(x0: f64, y0: f64) -> Vec<String> {
        let mut v = Vec::new();
        let p = |x: f64, y: f64| format!("G1 X{} Y{} E0.01", format_float(x), format_float(y));
        for row in 0..=100 {
            let y = y0 + row as f64 * 0.4;
            let ltr = row % 2 == 0;
            if y > y0 + 10.0 && y < y0 + 30.0 {
                /* 洞把这一行切成左右两段 —— 中间跳过去（真机上就是一条跳转）。
                走的方向要接上一行的落点：偶数行从左往右，先左条；奇数行从右往左，先右条 */
                let (a, b, c, d) = if ltr {
                    (x0, x0 + 10.0, x0 + 30.0, x0 + 40.0)
                } else {
                    (x0 + 40.0, x0 + 30.0, x0 + 10.0, x0)
                };
                v.push(p(a, y));
                v.push(p(b, y));
                v.push(MARKER_ZJUMP_START.to_string());
                v.push(p(c, y));
                v.push(p(d, y));
            } else {
                let (a, b) = if ltr {
                    (x0, x0 + 40.0)
                } else {
                    (x0 + 40.0, x0)
                };
                v.push(p(a, y));
                v.push(p(b, y));
            }
        }
        v
    }

    #[test]
    fn off_returns_input_unchanged() {
        let input = square(40.0, 100.0, 100.0);
        assert_eq!(apply(&input, 0.0, 1.0), input, "百分比 0 = 关");
        assert_eq!(apply(&input, 10.0, 0.0), input, "抬起 0 = 关");
    }

    #[test]
    fn edge_pieces_are_raised_and_interior_is_not() {
        let input = square(40.0, 100.0, 100.0);
        let out = apply(&input, 10.0, 1.5);
        assert!(out.len() > input.len(), "交界处被切开，行会变多");
        assert!(out.iter().any(|l| l == MARKER_EDGE_RAISE), "边缘段要标出来");
        assert!(
            out.iter().any(|l| l == MARKER_EDGE_NORMAL),
            "里面那段要回正常"
        );

        /* 抬高标记之后的第一条移动，离 40×40 这个面的边一定很近（带宽 4mm） */
        let mut raised_next = false;
        let mut checked = 0;
        for l in &out {
            if l == MARKER_EDGE_RAISE {
                raised_next = true;
                continue;
            }
            if l == MARKER_EDGE_NORMAL {
                raised_next = false;
                continue;
            }
            if raised_next && l.starts_with("G1 X") {
                let x = extract_coord(l.as_bytes(), b'X').unwrap();
                let y = extract_coord(l.as_bytes(), b'Y').unwrap();
                let d_edge = (x - 100.0).min(140.0 - x).min(y - 100.0).min(140.0 - y);
                assert!(d_edge <= 4.0 + 0.3, "抬高段应贴着边（实测 {d_edge:.2}mm）");
                checked += 1;
                raised_next = false;
            }
        }
        assert!(checked > 0, "至少有一段被标成边缘");
    }

    /// 环形（中间一个洞）：**内圈也是边界** —— 这正是旧的外接矩形做不到的那一半。
    #[test]
    fn ring_inner_edge_counts_as_boundary() {
        let input = filled_ring(100.0, 100.0);
        let moves = collect_moves(&input);
        let grid = EdgeGrid::build(&moves).unwrap();
        /* 40×40 的面，短边 40 ⇒ 10% = 4mm 带宽 */
        assert!(grid.is_edge((100.5, 120.0), 10.0), "外沿算边界");
        assert!(
            grid.is_edge((110.5, 120.0), 10.0),
            "洞的内沿也算边界（旧实现这里一点没动）"
        );
        assert!(
            !grid.is_edge((105.0, 120.0), 10.0),
            "10mm 宽窄条的正中离两边各 5mm > 4mm，不该算边缘"
        );
        assert!(
            grid.is_edge((105.0, 120.0), 40.0),
            "带宽放到 16mm（40%）时窄条整条都算边缘"
        );
    }

    /// 两张大小差很多的面：带宽各按自己短边算（小面不会被 1mm 那样的绝对值吃光）。
    #[test]
    fn band_scales_per_face() {
        let mut input = square(10.0, 100.0, 100.0);
        /* 两张面之间是跳转（真机上也是），不然这一段会被当成一条横穿的涂胶线 */
        input.push(MARKER_ZJUMP_START.to_string());
        input.extend(square(80.0, 150.0, 100.0));
        let moves = collect_moves(&input);
        let grid = EdgeGrid::build(&moves).unwrap();
        /* 小面 10mm ⇒ 20% = 2mm；大面 80mm ⇒ 20% = 16mm。同样是「离边 5mm」： */
        assert!(
            !grid.is_edge((105.0, 105.0), 20.0),
            "小面上离边 5mm > 2mm 带宽 ⇒ 不算边缘"
        );
        assert!(
            grid.is_edge((155.0, 105.0), 20.0),
            "大面上离边 5mm < 16mm 带宽 ⇒ 算边缘"
        );
    }
}
