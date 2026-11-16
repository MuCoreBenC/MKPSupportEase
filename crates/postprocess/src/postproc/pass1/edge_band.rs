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
//! **带宽按条数算**：`带宽 = 条数 × 线距`，最多半张面。条数是作者的口径（「靠外几条」），
//! 线距是这一层**实测**出来的（并排两条涂胶线的垂直间距），所以换喷嘴 / 稀疏比时，同一个
//! 条数给的物理宽度也跟着变。
//!
//! 百分比退场的原因是它把「面多大」这个与 XY/Z 偏差无关的量带进了判据：同一层里一张
//! 9.5mm 的面带宽 0.95、一条 1.25mm 的窄条只剩 0.125 —— 比格子的量化台阶还小，窄条整条
//! 抬不起来。要防的东西（笔跑出面再回来刮到模型）是绝对量，跟面多大没关系。
//!
//! # 输出
//!
//! 只做两件事：在边缘/里面的交界处**插两个标记**（[`MARKER_EDGE_RAISE`] /
//! [`MARKER_EDGE_NORMAL`]），并**在交界处把移动切开**（一段移动可能一半在带内、一半在外）。
//! 路径一段都不删 —— 抬高/回落由 `glue_block::write_glue_lines` 认标记后做。
//!
//! `edge_lift <= 0` 或 `edge_lines <= 0` ⇒ **原样返回**（关闭时输出一个字节都不变）。

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
/// 名义线距（mm）：只在测不出时用 —— 0.4 喷嘴并排涂胶线的常见间距。
const NOMINAL_PITCH_MM: f64 = 0.4;
/// 实测线距的夹取范围：比这更细/更粗的多半是测错了。
const MIN_PITCH_MM: f64 = 0.15;
const MAX_PITCH_MM: f64 = 2.0;
/// 参与比较的移动至少这么长（短移动的方向不可靠）。
const MIN_LONG_MM: f64 = 1.2;
/// 两条长移动之间允许隔多远 —— 蛇形的一次折返就是一个线距。
const MAX_PITCH_GAP_MM: f64 = 3.0;
/// 样本少于这个数就回落名义线距（界面 / 转储里要看得出「回落」）。
const MIN_PITCH_SAMPLES: usize = 6;

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
    /// 这一层实测的线距（mm）—— 带宽 = 条数 × 它
    pitch: f64,
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
            pitch: estimate_pitch(moves),
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

    /// 带宽（mm）：`条数 × 线距`，最多半张面。
    ///
    /// `短边/2` 封顶保住「中间那一半永远不抬」：抬高 = 笔离面 0.5mm、那一段基本不落胶，
    /// 是有代价的动作。
    fn band_mm(&self, short: f64, lines: i64) -> f64 {
        if short <= 0.0 || lines <= 0 {
            return 0.0;
        }
        (lines as f64 * self.pitch).min(short / 2.0)
    }

    /// 这个点算不算边缘带。
    fn is_edge(&self, p: (f64, f64), lines: i64) -> bool {
        let band = self.band_mm(self.face_short(p), lines);
        band > 0.0 && self.edge_distance(p) < band
    }
}

/// 一条够长、方向可靠的移动。
struct Stroke {
    start: (f64, f64),
    end: (f64, f64),
    dir: (f64, f64),
}

/// 这一层的线距（mm）：并排两条涂胶线的垂直间距。
///
/// 取**相邻的两条长移动**，方向大致平行（`|cos| > 0.7`）、垂直间距落在 (0.1, 3)mm、
/// 两者之间不跨远，就记一次样本，最后取中位数。蛇形填充的相邻两行、环形轮廓的相邻两圈
/// 都落进这条判据；同一行被切成好几条首尾相接的移动时垂直间距约为 0，被下限挡掉。
///
/// 只量**带 E 的移动**：iface 里的空走（无 E）不是涂胶线，量进来会把线距带偏。
fn estimate_pitch(moves: &[Move]) -> f64 {
    let mut samples: Vec<f64> = Vec::new();
    let mut prev: Option<Stroke> = None;
    for m in moves {
        if m.e.abs() < 1e-9 {
            continue;
        }
        let (dx, dy) = (m.to.0 - m.from.0, m.to.1 - m.from.1);
        let len = (dx * dx + dy * dy).sqrt();
        if len < MIN_LONG_MM {
            continue;
        }
        let dir = (dx / len, dy / len);
        if let Some(p) = &prev {
            let gap = dist(p.end, m.from);
            let cos = p.dir.0 * dir.0 + p.dir.1 * dir.1;
            if gap <= MAX_PITCH_GAP_MM && cos.abs() > 0.7 {
                let perp =
                    ((m.from.0 - p.start.0) * p.dir.1 - (m.from.1 - p.start.1) * p.dir.0).abs();
                if perp > 0.1 && perp < MAX_PITCH_GAP_MM {
                    samples.push(perp);
                }
            }
        }
        prev = Some(Stroke {
            start: m.from,
            end: m.to,
            dir,
        });
    }
    if samples.len() < MIN_PITCH_SAMPLES {
        return NOMINAL_PITCH_MM;
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mid = samples.len() / 2;
    let median = if samples.len().is_multiple_of(2) {
        (samples[mid - 1] + samples[mid]) / 2.0
    } else {
        samples[mid]
    };
    median.clamp(MIN_PITCH_MM, MAX_PITCH_MM)
}

/// 按边缘带切段并插标记。`lines <= 0` 或 `lift <= 0` ⇒ 原样返回。
pub(crate) fn apply(iface: &[String], lines: i64, lift: f64) -> Vec<String> {
    if lines <= 0 || lift <= 0.0 || iface.is_empty() {
        return iface.to_vec();
    }
    let moves = collect_moves(iface);
    if moves.len() < 2 {
        return iface.to_vec();
    }
    let Some(grid) = EdgeGrid::build(&moves) else {
        return iface.to_vec();
    };

    let debug_dir = debug_target();

    let mut by_line: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for (i, m) in moves.iter().enumerate() {
        by_line.insert(m.line, i);
    }

    let mut out: Vec<String> = Vec::with_capacity(iface.len());
    /* 当前发射端的落笔档：`false` = 正常，`true` = 抬高。只在切换处插标记 */
    let mut raised = false;

    for (idx, line) in iface.iter().enumerate() {
        /*
         * 段边界要**重新声明**一次档位。
         *
         * 发射端是按 `;ZJUMP_START` 切段、一段一次调用 `write_glue_lines` 的，每次调用
         * 自己的状态从"正常档"起步；而这里是整层连续扫的。上一段结束时正好是抬高档的话，
         * 不在这里复位，下一段开头就不会插 `;EDGE_RAISE` —— 发射端永远不知道它该抬，
         * 那一段就落到正常高度（2026-11-16 实测漏判的那段方形就是这个）。
         */
        if line.contains(crate::gcode::MARKER_ZJUMP_START) {
            raised = false;
        }
        let Some(&mi) = by_line.get(&idx) else {
            out.push(line.clone());
            continue;
        };
        let m = &moves[mi];
        let pieces = split_edge_pieces(m, &grid, lines);
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

    if let Some(dir) = debug_dir {
        write_debug_dump(&dir, iface, &moves, &grid, lines, &out);
    }
    out
}

/// **边缘带的转储开关**（`MKP_EDGE_DEBUG`，2026-11-16）。
///
/// 现实里最难查的一类问题是「按规则该抬的那一段没抬」：只看产物 G-code 分不出是
/// *没判成边缘* 还是 *标记被发射端吞了*。打开它就把每一次 `apply` 的输入路径、
/// 每张面的短边/带宽、每一条移动算出来的距离与判定原样落盘。
///
/// - `MKP_EDGE_DEBUG=<目录>` 写进那个目录（不存在就现建）；
/// - `MKP_EDGE_DEBUG=1` 写进系统临时目录。
///
/// 关着的时候一次 `env` 查询都没有额外开销可言（每次 `apply` 一次），也不写任何文件。
fn debug_target() -> Option<std::path::PathBuf> {
    if let Some(raw) = std::env::var_os("MKP_EDGE_DEBUG") {
        let s = raw.to_string_lossy();
        let s = s.trim();
        if s.is_empty() || s == "0" {
            return None;
        }
        if s != "1" {
            return Some(std::path::PathBuf::from(s));
        }
        /* `=1`：落在本次产物旁边（管线 / 钩子每次运行前设好），没设过才退回临时目录 */
        return dump_root().or_else(|| Some(std::env::temp_dir()));
    }
    /*
     * 没有环境变量也照样能开（2026-11-16 补）：钩子这个进程是**切片器拉起来的**，
     * 终端里设的 env 根本传不进去 —— 所以外壳那侧另有开关（见 `hook.rs` 的哨兵文件），
     * 它设好落点之后，这里直接认。
     */
    dump_root()
}

/// `MKP_EDGE_DEBUG=1` 时转储写到哪儿 —— 由管线在每次运行开始时设成产物所在目录。
static DUMP_ROOT: std::sync::Mutex<Option<std::path::PathBuf>> = std::sync::Mutex::new(None);

pub(crate) fn set_dump_root(dir: Option<std::path::PathBuf>) {
    if let Ok(mut slot) = DUMP_ROOT.lock() {
        *slot = dir;
    }
}

fn dump_root() -> Option<std::path::PathBuf> {
    DUMP_ROOT.lock().ok().and_then(|slot| slot.clone())
}

fn write_debug_dump(
    dir: &std::path::Path,
    iface: &[String],
    moves: &[Move],
    grid: &EdgeGrid,
    lines: i64,
    out: &[String],
) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static SEQ: AtomicUsize = AtomicUsize::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);

    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let mut text = String::new();
    let raise = out.iter().filter(|l| *l == MARKER_EDGE_RAISE).count();
    let normal = out.iter().filter(|l| *l == MARKER_EDGE_NORMAL).count();
    text.push_str(&format!(
        "# 第 {seq} 次 apply：iface {} 行 / 移动 {} 条 → 输出 {} 行（RAISE {raise} · NORMAL {normal}）\n",
        iface.len(),
        moves.len(),
        out.len()
    ));
    text.push_str(&format!(
        "# 格子 {}x{} 每格 {}mm；条数 {lines} · 线距 {:.3}mm（{}）\n",
        grid.nx,
        grid.ny,
        grid.cell,
        grid.pitch,
        if (grid.pitch - NOMINAL_PITCH_MM).abs() < 1e-9 {
            "回落名义值"
        } else {
            "实测"
        }
    ));
    text.push_str("# 面：id 短边 带宽 带宽/格\n");
    for id in 1..grid.comp_short.len() {
        let short = grid.comp_short[id];
        let band = grid.band_mm(short, lines);
        text.push_str(&format!(
            "face {id}: short={short:.3} band={band:.3} band/cell={:.1}\n",
            band / grid.cell
        ));
    }
    text.push_str("# 移动：行号 from → to | 距离 带宽 判定\n");
    for m in moves {
        let d = grid.edge_distance(m.from);
        let short = grid.face_short(m.from);
        let band = grid.band_mm(short, lines);
        text.push_str(&format!(
            "m line={} ({:.3},{:.3})->({:.3},{:.3}) d={:.3} short={:.3} band={:.3} edge={}\n",
            m.line,
            m.from.0,
            m.from.1,
            m.to.0,
            m.to.1,
            d,
            short,
            band,
            grid.is_edge(m.from, lines) as u8
        ));
    }
    text.push_str("# 输出逐行（↑ 标记 / G1 / 其他）\n");
    for (i, l) in out.iter().enumerate() {
        text.push_str(&format!("{i}\t{l}\n"));
    }
    /*
     * 写盘纪律（`clippy.toml`）：生产代码不许用会截断已有文件的入口。这里是**调试转储**，
     * 目标路径带 pid + 序号、每次都是新文件 —— 用 `create_new` 表达"绝不覆盖"，
     * 万一崩在半路也只会留下半份转储，不会动到任何既有文件。
     */
    let path = dir.join(format!("edge-dump-{}-{seq:03}.txt", std::process::id()));
    match std::fs::File::create_new(&path) {
        Ok(mut f) => {
            use std::io::Write;
            if let Err(e) = f.write_all(text.as_bytes()) {
                tracing::warn!(error = %e, path = %path.display(), "边缘带转储写不出去");
            } else {
                tracing::info!(path = %path.display(), "边缘带转储已写");
            }
        }
        Err(e) => tracing::warn!(error = %e, path = %path.display(), "边缘带转储打不开"),
    }
}

/// 一段子移动（`t0/t1` 是它在原移动里的参数区间）。
struct Piece {
    t0: f64,
    t1: f64,
    to: (f64, f64),
    edge: bool,
}

/// 把一条移动按「边缘 / 里面」切成若干段（可能整段都是一种）。
fn split_edge_pieces(m: &Move, grid: &EdgeGrid, lines: i64) -> Vec<Piece> {
    let len = dist(m.from, m.to);
    if len <= 1e-9 {
        return Vec::new();
    }
    let steps = ((len / SAMPLE_MM).ceil() as usize).clamp(1, 4000);
    let mut pieces: Vec<Piece> = Vec::new();
    let mut prev_edge = grid.is_edge(m.from, lines);
    let mut start_t = 0.0;
    let mut prev_t = 0.0;
    for k in 1..=steps {
        let t = k as f64 / steps as f64;
        let e = grid.is_edge(lerp(m.from, m.to, t), lines);
        if e != prev_edge {
            /* 交界落在 [prev_t, t] 里：二分找精确一点的位置 */
            let cross = bisect(m.from, m.to, prev_t, t, prev_edge, grid, lines);
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
    lines: i64,
) -> f64 {
    let (mut lo, mut hi) = (t0, t1);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if grid.is_edge(lerp(a, b, mid), lines) == edge_lo {
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

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

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

    /// 一块 `w × h` 的蛇形涂胶面：每行一条长移动，行间是一次长度为线距的折返。
    /// 与真产物的排法一致 —— 线距实测就是量这些折返。
    fn rect(w: f64, h: f64, x0: f64, y0: f64) -> Vec<String> {
        let pitch = 0.4;
        let rows = (h / pitch).round().max(1.0) as i32;
        let mut v = Vec::new();
        for row in 0..=rows {
            let y = y0 + row as f64 * h / rows as f64;
            let (a, b) = if row % 2 == 0 {
                (x0, x0 + w)
            } else {
                (x0 + w, x0)
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

    /// **回放真实产物**（诊断用，不入 CI）：读 `MKP_EDGE_SAMPLE` 指向的产物，把每个涂胶块
    /// 还原成 iface，再跑一次 `apply`，报告判了多少段边缘、探针那条移动被判成什么。
    /// 没设环境变量就直接跳过。
    #[test]
    fn replay_real_output_when_sample_is_given() {
        let Some(path) = std::env::var_os("MKP_EDGE_SAMPLE") else {
            return;
        };
        let text = std::fs::read_to_string(&path).expect("示例文件读不出来");
        let lines: i64 = std::env::var("MKP_EDGE_LINES")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let lift: f64 = std::env::var("MKP_EDGE_LIFT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0.5);
        let probe = std::env::var("MKP_EDGE_PROBE").ok();

        let blocks = paint_blocks(&text);
        println!(
            "涂胶块 {} 个（外 {lines} 条 / 抬起 {lift}mm）",
            blocks.len()
        );
        let mut total_moves = 0usize;
        let mut total_edge_moves = 0usize;
        let mut total_raise = 0usize;
        let mut total_normal = 0usize;
        for (bi, block) in blocks.iter().enumerate() {
            /* 落笔高度 = 低于块内最高 Z 的那些；抬高档（+lift）也算落笔 */
            let max_z = block
                .iter()
                .filter_map(|l| l.trim().strip_prefix("G1 Z"))
                .filter_map(|z| z.parse::<f64>().ok())
                .fold(f64::MIN, f64::max);
            let mut iface: Vec<String> = Vec::new();
            let mut at_pen_down = false;
            let mut run_start = false;
            for l in block {
                let t = l.trim();
                if let Some(z) = t.strip_prefix("G1 Z") {
                    let z: f64 = z.parse().unwrap_or(max_z);
                    at_pen_down = z < max_z - 1e-9;
                    run_start = at_pen_down;
                    continue;
                }
                if at_pen_down && t.starts_with("G1 X") {
                    if run_start {
                        iface.push(MARKER_ZJUMP_START.to_string());
                        run_start = false;
                    }
                    iface.push(t.to_string());
                }
            }
            if iface.len() < 2 {
                continue;
            }
            let moves = collect_moves(&iface);
            let grid = EdgeGrid::build(&moves);
            let edge_moves = grid
                .as_ref()
                .map(|g| moves.iter().filter(|m| g.is_edge(m.from, lines)).count())
                .unwrap_or(0);
            let out = apply(&iface, lines, lift);
            let raise = out.iter().filter(|l| *l == MARKER_EDGE_RAISE).count();
            let normal = out.iter().filter(|l| *l == MARKER_EDGE_NORMAL).count();
            total_moves += moves.len();
            total_edge_moves += edge_moves;
            total_raise += raise;
            total_normal += normal;
            if let Some(p) = &probe {
                let mut raised = false;
                for l in &out {
                    if l == MARKER_EDGE_RAISE {
                        raised = true;
                    } else if l == MARKER_EDGE_NORMAL {
                        raised = false;
                    } else if l.replace(' ', "") == p.replace(' ', "") {
                        println!("块 {bi}: 探针 {p} → 抬高档={raised}");
                    }
                }
            }
            /* 逐块摊开：改口径时要比的就是「哪一块多抬了几条」 */
            let (pitch, faces) = match &grid {
                Some(g) => (
                    g.pitch,
                    g.comp_short
                        .iter()
                        .skip(1)
                        .map(|s| format!("{s:.2}"))
                        .collect::<Vec<_>>()
                        .join("/"),
                ),
                None => (NOMINAL_PITCH_MM, String::from("—")),
            };
            println!(
                "块 {bi}: 移动 {} 条、判边缘 {} 条、RAISE {raise} / NORMAL {normal}、线距 {pitch:.3}mm、面 {faces}",
                moves.len(),
                edge_moves
            );
        }
        println!(
            "合计：移动 {total_moves} 条、其中判边缘 {total_edge_moves} 条；RAISE {total_raise} · NORMAL {total_normal}"
        );
    }

    /// 把一份产物切成「涂胶块」（`;MKP_BEGIN paint` 之间）。
    fn paint_blocks(text: &str) -> Vec<Vec<String>> {
        let mut blocks = Vec::new();
        let mut cur: Option<Vec<String>> = None;
        for line in text.lines() {
            let t = line.trim();
            if t.starts_with(";MKP_BEGIN paint") {
                cur = Some(Vec::new());
                continue;
            }
            if t.starts_with(";MKP_END paint") {
                if let Some(b) = cur.take() {
                    blocks.push(b);
                }
                continue;
            }
            if let Some(b) = cur.as_mut() {
                b.push(t.to_string());
            }
        }
        blocks
    }

    /// 没有环境变量时，外壳设好的落点照样生效（钩子那条路就靠这个）。
    #[test]
    fn dump_root_is_honored_without_env() {
        if std::env::var_os("MKP_EDGE_DEBUG").is_some() {
            return; // 环境变量优先，这条判据在那种环境下没有意义
        }
        let dir = std::path::PathBuf::from("/tmp/mkp-edge-dump-probe");
        set_dump_root(Some(dir.clone()));
        assert_eq!(debug_target(), Some(dir));
        set_dump_root(None);
        assert_eq!(debug_target(), None, "关掉之后必须回到「不写」");
    }

    #[test]
    fn off_returns_input_unchanged() {
        let input = square(40.0, 100.0, 100.0);
        assert_eq!(apply(&input, 0, 1.0), input, "0 条 = 关");
        assert_eq!(apply(&input, 2, 0.0), input, "抬起 0 = 关");
    }

    #[test]
    fn edge_pieces_are_raised_and_interior_is_not() {
        let input = rect(40.0, 40.0, 100.0, 100.0);
        let out = apply(&input, 2, 1.5);
        assert!(out.len() > input.len(), "交界处被切开，行会变多");
        assert!(out.iter().any(|l| l == MARKER_EDGE_RAISE), "边缘段要标出来");
        assert!(
            out.iter().any(|l| l == MARKER_EDGE_NORMAL),
            "里面那段要回正常"
        );

        /* 抬高标记之后的第一条移动，离 40×40 这个面的边一定很近（带宽 0.8mm） */
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
                assert!(d_edge <= 0.8 + 0.3, "抬高段应贴着边（实测 {d_edge:.2}mm）");
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
        /* 40×40 的面，线距 0.4 ⇒ 两条 = 0.8mm 带宽 */
        assert!(grid.is_edge((100.5, 120.0), 2), "外沿算边界");
        assert!(
            grid.is_edge((110.5, 120.0), 2),
            "洞的内沿也算边界（旧实现这里一点没动）"
        );
        assert!(
            !grid.is_edge((105.0, 120.0), 2),
            "10mm 宽窄条的正中离两边各 5mm > 0.8mm，不该算边缘"
        );
        /* 带宽本身：线距 0.4 × 两条，与面多大无关 */
        assert!((grid.band_mm(grid.face_short((100.5, 120.0)), 2) - 0.8).abs() < 1e-9);
    }

    /// 实测漏判的形状：**只由外边界构成的一圈**（8.7×5.8 的方形闭合环）。
    ///
    /// 环上任何一点离「没涂胶」都在半格以内 ⇒ 整圈都该是边缘。这条钉住
    /// 「一圈纯边界也算边缘」，防止再退化成"必须有填充才判得出"。
    #[test]
    fn closed_boundary_ring_is_all_edge() {
        let input = lines(&[
            /* 段首那条空走（切片器的习惯）：零位移，原样保留，不该因此丢掉标记 */
            "G1 X114.216 Y98.337",
            "G1 X122.924 Y98.337 E0.04",
            "G1 X122.924 Y104.169 E0.03",
            "G1 X114.216 Y104.169 E0.04",
            "G1 X114.216 Y98.337 E0.03",
        ]);
        let out = apply(&input, 2, 0.5);
        assert_eq!(out[0], "G1 X114.216 Y98.337", "零位移那条原样留着");
        assert_eq!(out[1], MARKER_EDGE_RAISE, "整圈都要抬");
        assert!(
            !out.iter().any(|l| l == MARKER_EDGE_NORMAL),
            "这一层没有里面那段，不该回落"
        );
        assert_eq!(
            out.iter().filter(|l| l.starts_with("G1 X")).count(),
            input.len(),
            "路径一段不少"
        );
    }

    /// 真实失配形状的缩小版：外圈方形 + 内部几条平行料 ⇒ 外圈边缘、里面不抬。
    #[test]
    fn outer_ring_is_edge_but_fill_inside_is_not() {
        let mut input = lines(&[
            "G1 X114.216 Y98.337 E0.04",
            "G1 X122.924 Y98.337 E0.03",
            "G1 X122.924 Y104.169 E0.04",
            "G1 X114.216 Y104.169 E0.03",
            "G1 X114.216 Y98.337 E0.03",
        ]);
        /* 内部填充：与真机同量级的密排（0.4mm 行距），这样"里面"才真的离边界远 */
        for row in 0..14 {
            let y = 98.7 + row as f64 * 0.4;
            let (a, b) = if row % 2 == 0 {
                (114.6, 122.5)
            } else {
                (122.5, 114.6)
            };
            input.push(format!(
                "G1 X{} Y{} E0.03",
                format_float(a),
                format_float(y)
            ));
            input.push(format!(
                "G1 X{} Y{} E0.03",
                format_float(b),
                format_float(y)
            ));
        }
        let out = apply(&input, 2, 0.5);
        assert!(out.iter().any(|l| l == MARKER_EDGE_RAISE), "外圈在边缘带上");
        assert!(
            out.iter().any(|l| l == MARKER_EDGE_NORMAL),
            "里面有一段正常档"
        );

        /* 切段只切不丢：所有子段长度之和必须等于原来的总长 */
        let sum_len = |v: &[String]| -> f64 {
            let mut total = 0.0;
            let mut prev: Option<(f64, f64)> = None;
            for l in v {
                if !l.starts_with("G1 X") {
                    continue;
                }
                let x = extract_coord(l.as_bytes(), b'X').unwrap_or(0.0);
                let y = extract_coord(l.as_bytes(), b'Y').unwrap_or(0.0);
                if let Some(p) = prev {
                    total += ((x - p.0).powi(2) + (y - p.1).powi(2)).sqrt();
                }
                prev = Some((x, y));
            }
            total
        };
        let before = sum_len(&input);
        let after = sum_len(&out);
        assert!(
            (before - after).abs() < 1e-6,
            "路径总长变了：{before} → {after}"
        );
    }

    /// 窄条（6.75×1.25，实测漏判的那一条）：两条就够把它整条抬起来。
    ///
    /// 百分比带宽在这里只剩 0.125mm（比格子还细），中间几行必然回落；条数口径给的
    /// 0.8mm 被 `短边/2` 封顶到 0.63，中线两侧都在带里。
    #[test]
    fn narrow_strip_is_raised_all_the_way_through() {
        let input = rect(6.75, 1.25, 121.97, 98.08);
        let moves = collect_moves(&input);
        let grid = EdgeGrid::build(&moves).unwrap();
        assert!(grid.is_edge((125.0, 98.7), 2), "中线也在带里");
        assert!(grid.is_edge((123.0, 98.2), 2), "左沿算边界");
        assert!(!grid.is_edge((125.0, 98.7), 0), "0 条 = 一条都不抬");
    }

    /// 面大面小，给的物理带宽是同一个（0.8mm）—— 与百分比口径最根本的差别。
    #[test]
    fn band_is_the_same_mm_on_big_and_small_faces() {
        let mut input = rect(10.0, 10.0, 100.0, 100.0);
        /* 两张面之间是跳转（真机上也是），不然这一段会被当成一条横穿的涂胶线 */
        input.push(MARKER_ZJUMP_START.to_string());
        input.extend(rect(80.0, 80.0, 150.0, 100.0));
        let moves = collect_moves(&input);
        let grid = EdgeGrid::build(&moves).unwrap();
        let small = grid.face_short((105.0, 105.0));
        let big = grid.face_short((190.0, 140.0));
        assert!(small < big, "两张面确实一大一小（{small:.2} / {big:.2}）");
        assert!((grid.band_mm(small, 2) - 0.8).abs() < 1e-9);
        assert!((grid.band_mm(big, 2) - 0.8).abs() < 1e-9);
    }
}
