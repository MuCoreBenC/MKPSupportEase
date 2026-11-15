//! 涂胶面内缩（`wiping.glue_inset`）：把离涂胶区域外沿不足 N mm 的移动裁掉。
//!
//! 涂胶路径只是一串 `G1 X Y`，没有多边形轮廓可做几何内缩。这里取本层涂胶点的
//! 外接矩形，四边各向内收 N mm，逐条移动裁到收缩后的矩形里 —— 效果就是
//! 「最外圈（或最外几圈）不涂」，形状不改、路径不重排。收空时整层退回原路径并记
//! warn，不静默变成「这一层没涂胶」。

use crate::gcode::{MARKER_ZJUMP_INTERNAL, MARKER_ZJUMP_START, extract_coord, format_float};

/// 一条带坐标的 `G1` 移动，`line` 是它在输入里的行号。
struct Move {
    line: usize,
    from: (f64, f64),
    to: (f64, f64),
    /// 原行的挤出量。裁断后按长度比例分给各子段 —— 发射端靠「段内有没有带 E 的移动」
    /// 判断这一段是不是空走（`emit_glue_block` 里 `if !seg.iter().any(|l| l.contains(" E"))`）。
    e: f64,
}

/// 收缩后的矩形。
struct Rect {
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
}

/// 把 `iface` 里落在四边各收 `inset` mm 之外的移动裁掉。`inset <= 0` 原样返回。
pub(crate) fn apply(iface: &[String], inset: f64) -> Vec<String> {
    if inset <= 0.0 || iface.is_empty() {
        return iface.to_vec();
    }
    let moves = collect_moves(iface);
    if moves.len() < 2 {
        return iface.to_vec();
    }
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for m in &moves {
        for p in [m.from, m.to] {
            min_x = min_x.min(p.0);
            min_y = min_y.min(p.1);
            max_x = max_x.max(p.0);
            max_y = max_y.max(p.1);
        }
    }
    let rect = Rect {
        x0: min_x + inset,
        x1: max_x - inset,
        y0: min_y + inset,
        y1: max_y - inset,
    };
    if rect.x1 <= rect.x0 || rect.y1 <= rect.y0 {
        tracing::warn!(inset, "涂胶内缩把整层收空了，本层按原路径涂胶");
        return iface.to_vec();
    }

    let mut out: Vec<String> = Vec::with_capacity(iface.len());
    let mut kept_any = false;
    let mut cursor = 0usize;
    let mut need_jump = false;

    for (idx, line) in iface.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.contains(MARKER_ZJUMP_START) || trimmed.contains(MARKER_ZJUMP_INTERNAL) {
            out.push(line.clone());
            need_jump = false;
            continue;
        }
        if cursor >= moves.len() || moves[cursor].line != idx {
            out.push(line.clone());
            continue;
        }
        let mv = &moves[cursor];
        cursor += 1;

        let pieces = clip_segment(mv.from, mv.to, |p| rect.contains(p));
        if pieces.is_empty() {
            need_jump = true;
            continue;
        }
        let whole = pieces.len() == 1
            && (pieces[0].0.0 - mv.from.0).abs() < 1e-9
            && (pieces[0].0.1 - mv.from.1).abs() < 1e-9
            && (pieces[0].1.0 - mv.to.0).abs() < 1e-9
            && (pieces[0].1.1 - mv.to.1).abs() < 1e-9;
        if whole {
            if need_jump && !out.is_empty() {
                out.push(MARKER_ZJUMP_START.to_string());
            }
            need_jump = false;
            out.push(line.clone());
            kept_any = true;
            continue;
        }
        for (a, b, t0, t1) in pieces {
            if need_jump && !out.is_empty() {
                out.push(MARKER_ZJUMP_START.to_string());
            }
            need_jump = false;
            out.push(g1_move(a, mv.e));
            out.push(g1_move(b, mv.e * (t1 - t0)));
            kept_any = true;
        }
    }

    if !kept_any {
        tracing::warn!(inset, "涂胶内缩把整层收空了，本层按原路径涂胶");
        return iface.to_vec();
    }
    out
}

impl Rect {
    fn contains(&self, p: (f64, f64)) -> bool {
        p.0 >= self.x0 && p.0 <= self.x1 && p.1 >= self.y0 && p.1 <= self.y1
    }
}

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

/// 裁断后的一段：两端坐标 + 它在原移动里的参数区间（用来按比例分 E）。
type Piece = ((f64, f64), (f64, f64), f64, f64);

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// 一条移动里落在保留区的子段（可能 0 段，也可能被内部空洞切成多段）。
fn clip_segment(a: (f64, f64), b: (f64, f64), keep: impl Fn((f64, f64)) -> bool) -> Vec<Piece> {
    let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
    let steps = ((len / 0.1).ceil() as usize).clamp(1, 4000);
    let mut pieces = Vec::new();
    let mut start: Option<(f64, f64, f64)> = None;
    let mut prev_keep = false;
    let mut prev_t = 0.0;
    for k in 0..=steps {
        let t = k as f64 / steps as f64;
        let kp = keep(lerp(a, b, t));
        if kp && !prev_keep {
            start = Some(if k == 0 {
                (a.0, a.1, 0.0)
            } else {
                let p = bisect_between(a, b, prev_t, t, &keep);
                (p.0, p.1, (prev_t + t) / 2.0)
            });
        } else if !kp && prev_keep {
            let p = bisect_between(a, b, prev_t, t, &keep);
            if let Some(s) = start.take() {
                pieces.push(((s.0, s.1), p, s.2, (prev_t + t) / 2.0));
            }
        }
        prev_keep = kp;
        prev_t = t;
    }
    if prev_keep {
        if let Some(s) = start.take() {
            pieces.push(((s.0, s.1), b, s.2, 1.0));
        }
    }
    pieces
}

/// 在 `[t0, t1]` 这一小段里二分找 `keep` 翻转处。
fn bisect_between(
    a: (f64, f64),
    b: (f64, f64),
    t0: f64,
    t1: f64,
    keep: &impl Fn((f64, f64)) -> bool,
) -> (f64, f64) {
    let keep_lo = keep(lerp(a, b, t0));
    let (mut lo, mut hi) = (t0, t1);
    for _ in 0..20 {
        let mid = (lo + hi) / 2.0;
        if keep(lerp(a, b, mid)) == keep_lo {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    lerp(a, b, (lo + hi) / 2.0)
}

/// 生成一条涂胶移动。E 按裁剪比例给，最低留一个非零值让发射端认得出这是涂胶移动。
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

    /// 20×20 的蛇形涂胶面。
    fn square() -> Vec<String> {
        let mut v = Vec::new();
        for row in 0..21 {
            let y = 100.0 + row as f64;
            let x0 = 100.0;
            let x1 = 120.0;
            let (a, b) = if row % 2 == 0 { (x0, x1) } else { (x1, x0) };
            v.push(format!("G1 X{} Y{}", format_float(a), format_float(y)));
            v.push(format!("G1 X{} Y{}", format_float(b), format_float(y)));
        }
        v
    }

    #[test]
    fn inset_zero_keeps_everything() {
        let input = square();
        assert_eq!(apply(&input, 0.0), input);
    }

    #[test]
    fn inset_trims_all_four_sides() {
        let input = square();
        let out = apply(&input, 5.0);
        assert!(out.len() < input.len(), "内缩后路径应更短");
        assert!(
            !out.iter().any(|l| l.contains("Y100.0")),
            "最外一行应被裁掉"
        );
        assert!(
            out.iter()
                .any(|l| l.contains("Y110.0") || l.contains("Y111.0")),
            "中心区域应保留"
        );
        // 左右两侧也各收 5mm：裁完不该还有 X100 / X120
        assert!(!out.iter().any(|l| l.contains("X100.0")));
        assert!(!out.iter().any(|l| l.contains("X120.0")));
    }

    #[test]
    fn inset_keeps_shape_for_thin_part() {
        let input = lines(&["G1 X0 Y0", "G1 X1 Y0", "G1 X1 Y1"]);
        // 三角形太小，收 10mm 会空；兜底应原样返回
        assert_eq!(apply(&input, 10.0), input);
    }

    #[test]
    fn inset_marks_break_with_zjump() {
        let input = square();
        let out = apply(&input, 5.0);
        assert!(
            out.iter().any(|l| l.contains(MARKER_ZJUMP_START)),
            "裁剪产生的断口要打 ZJUMP 标记"
        );
    }

    #[test]
    fn rect_contains_is_inclusive() {
        let r = Rect {
            x0: 1.0,
            x1: 2.0,
            y0: 3.0,
            y1: 4.0,
        };
        assert!(r.contains((1.0, 3.0)));
        assert!(r.contains((2.0, 4.0)));
        assert!(!r.contains((0.9, 3.5)));
        assert!(!r.contains((1.5, 4.1)));
    }
}
