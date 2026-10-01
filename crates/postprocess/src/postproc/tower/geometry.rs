//! 多边形几何原语（对照 tower/polygon.go + rib.go 的几何 helpers，逐行复刻）。
//!
//! 浮点纪律：运算顺序与 Go 一字不差，禁 FMA（gatecheck `no_fma_in_any_crate`）。
//! 比较用的是 Go 同款 epsilon（1e-10 / 0.001 / 0.01），**不许**顺手改成「更合理」。

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

impl Point {
    pub fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
}

fn cross2d(o: Point, a: Point, b: Point) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

pub(crate) fn convex_hull(mut points: Vec<Point>) -> Vec<Point> {
    let n = points.len();
    if n < 3 {
        return points;
    }

    points.sort_unstable_by(|i, j| {
        if (i.x - j.x).abs() > 1e-10 {
            i.x.partial_cmp(&j.x).unwrap_or(std::cmp::Ordering::Equal)
        } else {
            i.y.partial_cmp(&j.y).unwrap_or(std::cmp::Ordering::Equal)
        }
    });

    let mut unique: Vec<Point> = Vec::with_capacity(n);
    for p in &points {
        if let Some(last) = unique.last() {
            if (p.x - last.x).abs() > 1e-10 || (p.y - last.y).abs() > 1e-10 {
                unique.push(*p);
            }
        } else {
            unique.push(*p);
        }
    }
    let sorted = unique;
    let n = sorted.len();

    if n < 3 {
        return sorted;
    }

    let mut lower: Vec<Point> = Vec::with_capacity(n);
    for p in &sorted {
        while lower.len() >= 2 {
            if cross2d(lower[lower.len() - 2], lower[lower.len() - 1], *p) <= 0.0 {
                lower.pop();
            } else {
                break;
            }
        }
        lower.push(*p);
    }

    let mut upper: Vec<Point> = Vec::with_capacity(n);
    for p in sorted.iter().rev() {
        while upper.len() >= 2 {
            if cross2d(upper[upper.len() - 2], upper[upper.len() - 1], *p) <= 0.0 {
                upper.pop();
            } else {
                break;
            }
        }
        upper.push(*p);
    }

    lower.pop();
    upper.pop();
    lower.extend(upper);
    lower
}

pub(crate) fn polygon_union(polys: &[Vec<Point>]) -> Vec<Point> {
    if polys.is_empty() {
        return Vec::new();
    }
    let mut all_points: Vec<Point> = Vec::new();
    for poly in polys {
        all_points.extend_from_slice(poly);
    }
    if all_points.len() < 3 {
        return all_points;
    }
    convex_hull(all_points)
}

pub(crate) fn polygon_offset(poly: &[Point], offset: f64) -> Vec<Point> {
    let n = poly.len();
    if n < 3 {
        return poly.to_vec();
    }

    #[derive(Clone, Copy)]
    struct OffsetEdge {
        ax: f64,
        ay: f64,
        bx: f64,
        by: f64,
    }

    let mut edges = vec![
        OffsetEdge {
            ax: 0.0,
            ay: 0.0,
            bx: 0.0,
            by: 0.0
        };
        n
    ];
    for i in 0..n {
        let curr = poly[i];
        let next = poly[(i + 1) % n];
        let dx = next.x - curr.x;
        let dy = next.y - curr.y;
        let length = dx.hypot(dy);
        if length < 1e-10 {
            edges[i] = OffsetEdge {
                ax: curr.x,
                ay: curr.y,
                bx: next.x,
                by: next.y,
            };
            continue;
        }
        let nx = dy / length;
        let ny = -dx / length;
        edges[i] = OffsetEdge {
            ax: curr.x + nx * offset,
            ay: curr.y + ny * offset,
            bx: next.x + nx * offset,
            by: next.y + ny * offset,
        };
    }

    let mut new_points = vec![Point { x: 0.0, y: 0.0 }; n];
    for i in 0..n {
        let prev = (i + n - 1) % n;

        let d1x = edges[prev].bx - edges[prev].ax;
        let d1y = edges[prev].by - edges[prev].ay;
        let d2x = edges[i].bx - edges[i].ax;
        let d2y = edges[i].by - edges[i].ay;

        let cross = d1x * d2y - d1y * d2x;
        if cross.abs() < 1e-10 {
            new_points[i] = Point {
                x: (edges[prev].bx + edges[i].ax) / 2.0,
                y: (edges[prev].by + edges[i].ay) / 2.0,
            };
            continue;
        }

        let t =
            ((edges[i].ax - edges[prev].ax) * d2y - (edges[i].ay - edges[prev].ay) * d2x) / cross;

        new_points[i] = Point {
            x: edges[prev].ax + t * d1x,
            y: edges[prev].ay + t * d1y,
        };
    }

    new_points
}

// 16 个顶点按 Go 侧 append 顺序逐个压入（行级对应，不合并成 vec![] 字面量）。
#[allow(clippy::vec_init_then_push)]
pub(crate) fn rib_section(
    width: f64,
    depth: f64,
    rib_length: f64,
    rib_width: f64,
    fillet_wall: bool,
) -> Vec<Point> {
    let theta = (width / depth).atan();
    let costheta = theta.cos();
    let sintheta = theta.sin();
    let w = rib_width / 2.0;
    let diag = (width * width + depth * depth).sqrt();
    let l = (rib_length - diag) / 2.0;

    let diag_len = (width * width + depth * depth).sqrt();
    let dir1_x = width / diag_len;
    let dir1_y = depth / diag_len;
    let perp1_x = -dir1_y;
    let perp1_y = dir1_x;

    let dir2_x = -width / diag_len;
    let dir2_y = depth / diag_len;
    let perp2_x = -dir2_y;
    let perp2_y = dir2_x;

    let (p0x, p0y) = (0.0, 0.0);
    let (p1x, p1y) = (width, 0.0);
    let (p2x, p2y) = (width, depth);
    let (p3x, p3y) = (0.0, depth);

    let mut res: Vec<Point> = Vec::with_capacity(16);
    res.push(Point::new(p0x, p0y + w / sintheta));
    res.push(Point::new(
        p0x - dir1_x * l + perp1_x * w,
        p0y - dir1_y * l + perp1_y * w,
    ));
    res.push(Point::new(
        p0x - dir1_x * l - perp1_x * w,
        p0y - dir1_y * l - perp1_y * w,
    ));
    res.push(Point::new(p0x + w / costheta, p0y));

    res.push(Point::new(p1x - w / costheta, p1y));
    res.push(Point::new(
        p1x - dir2_x * l + perp2_x * w,
        p1y - dir2_y * l + perp2_y * w,
    ));
    res.push(Point::new(
        p1x - dir2_x * l - perp2_x * w,
        p1y - dir2_y * l - perp2_y * w,
    ));
    res.push(Point::new(p1x, p1y + w / sintheta));

    res.push(Point::new(p2x, p2y - w / sintheta));
    res.push(Point::new(
        p2x + dir1_x * l - perp1_x * w,
        p2y + dir1_y * l - perp1_y * w,
    ));
    res.push(Point::new(
        p2x + dir1_x * l + perp1_x * w,
        p2y + dir1_y * l + perp1_y * w,
    ));
    res.push(Point::new(p2x - w / costheta, p2y));

    res.push(Point::new(p3x + w / costheta, p3y));
    res.push(Point::new(
        p3x + dir2_x * l - perp2_x * w,
        p3y + dir2_y * l - perp2_y * w,
    ));
    res.push(Point::new(
        p3x + dir2_x * l + perp2_x * w,
        p3y + dir2_y * l + perp2_y * w,
    ));
    res.push(Point::new(p3x, p3y - w / sintheta));

    let mut res = remove_duplicate_points(&res, 0.001);

    if fillet_wall {
        res = rounding_polygon(&res, 2.0, 30.0);
    }

    res
}

pub(crate) fn polygon_walk(poly: &[Point], start_idx: usize, distance: f64) -> Vec<Point> {
    let n = poly.len();
    if n < 2 || distance <= 0.0 {
        return Vec::new();
    }

    let mut result: Vec<Point> = Vec::new();
    let mut remaining = distance;
    let mut current_idx = start_idx % n;

    while remaining > 0.0 {
        let next_idx = (current_idx + 1) % n;
        let dx = poly[next_idx].x - poly[current_idx].x;
        let dy = poly[next_idx].y - poly[current_idx].y;
        let seg_len = dx.hypot(dy);

        if seg_len < 1e-10 {
            current_idx = next_idx;
            continue;
        }

        if remaining >= seg_len {
            result.push(poly[next_idx]);
            remaining -= seg_len;
            current_idx = next_idx;
        } else {
            let t = remaining / seg_len;
            result.push(Point {
                x: poly[current_idx].x + dx * t,
                y: poly[current_idx].y + dy * t,
            });
            remaining = 0.0;
        }
    }

    result
}

#[allow(dead_code)]
pub(crate) fn generate_diagonal_band(
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    width: f64,
    extra_len: f64,
) -> Vec<Point> {
    let dx = x2 - x1;
    let dy = y2 - y1;
    let length = dx.hypot(dy);
    if length < 1e-10 {
        return Vec::new();
    }

    let dir_x = dx / length;
    let dir_y = dy / length;
    let perp_x = -dir_y;
    let perp_y = dir_x;

    let sx = x1 - dir_x * extra_len;
    let sy = y1 - dir_y * extra_len;
    let ex = x2 + dir_x * extra_len;
    let ey = y2 + dir_y * extra_len;

    let hw = width / 2.0;

    vec![
        Point::new(sx + perp_x * hw, sy + perp_y * hw),
        Point::new(ex + perp_x * hw, ey + perp_y * hw),
        Point::new(ex - perp_x * hw, ey - perp_y * hw),
        Point::new(sx - perp_x * hw, sy - perp_y * hw),
    ]
}

#[allow(dead_code)]
pub(crate) fn generate_rib_polygon(
    width: f64,
    depth: f64,
    rib_width: f64,
    extra_len: f64,
) -> Vec<Point> {
    let band1 = generate_diagonal_band(0.0, 0.0, width, depth, rib_width, extra_len);
    let band2 = generate_diagonal_band(width, 0.0, 0.0, depth, rib_width, extra_len);
    let rect = vec![
        Point::new(0.0, 0.0),
        Point::new(width, 0.0),
        Point::new(width, depth),
        Point::new(0.0, depth),
    ];
    polygon_union(&[band1, band2, rect])
}

pub(crate) fn remove_duplicate_points(points: &[Point], epsilon: f64) -> Vec<Point> {
    if points.is_empty() {
        return Vec::new();
    }
    let mut result = vec![points[0]];
    for p in &points[1..] {
        let prev = result[result.len() - 1];
        let dx = p.x - prev.x;
        let dy = p.y - prev.y;
        if (dx * dx + dy * dy).sqrt() > epsilon {
            result.push(*p);
        }
    }
    result
}

pub(crate) fn rounding_polygon(
    polygon: &[Point],
    rounding_distance: f64,
    angle_tol_deg: f64,
) -> Vec<Point> {
    let n = polygon.len();
    if n < 3 {
        return polygon.to_vec();
    }

    let angle_tol_rad = angle_tol_deg * std::f64::consts::PI / 180.0;
    let cos_angle_tol = angle_tol_rad.cos().abs();
    let arc_segments = 8usize;

    let mut result: Vec<Point> = Vec::new();

    for i in 0..n {
        let a = polygon[(i + n - 1) % n];
        let b = polygon[i];
        let c = polygon[(i + 1) % n];

        let ab_len = ((a.x - b.x) * (a.x - b.x) + (a.y - b.y) * (a.y - b.y)).sqrt();
        let bc_len = ((b.x - c.x) * (b.x - c.x) + (b.y - c.y) * (b.y - c.y)).sqrt();

        if ab_len < 0.001 || bc_len < 0.001 {
            result.push(b);
            continue;
        }

        let abx = (b.x - a.x) / ab_len;
        let aby = (b.y - a.y) / ab_len;
        let bcx = (c.x - b.x) / bc_len;
        let bcy = (c.y - b.y) / bc_len;

        let mut cosangle = abx * bcx + aby * bcy;
        cosangle = (-1.0f64).max(1.0f64.min(cosangle));

        let is_ccw = (abx * bcy - aby * bcx) > 0.0;

        if cosangle.abs() < cos_angle_tol {
            let real_rounding_dis = rounding_distance.min(ab_len / 2.1).min(bc_len / 2.1);

            let left_x = b.x - abx * real_rounding_dis;
            let left_y = b.y - aby * real_rounding_dis;
            let right_x = b.x + bcx * real_rounding_dis;
            let right_y = b.y + bcy * real_rounding_dis;

            let half_angle = cosangle.acos() / 2.0;
            if half_angle < 0.001 {
                result.push(b);
                continue;
            }

            let mut dir_x = right_x - left_x;
            let mut dir_y = right_y - left_y;
            let dir_len = (dir_x * dir_x + dir_y * dir_y).sqrt();
            if dir_len < 0.001 {
                result.push(b);
                continue;
            }
            dir_x /= dir_len;
            dir_y /= dir_len;

            let mut rot_x = -dir_y;
            let mut rot_y = dir_x;

            if !is_ccw {
                rot_x = -rot_x;
                rot_y = -rot_y;
            }

            let dis = real_rounding_dis / half_angle.sin();

            let ccx = b.x + rot_x * dis;
            let ccy = b.y + rot_y * dis;

            let radius = ((left_x - ccx) * (left_x - ccx) + (left_y - ccy) * (left_y - ccy)).sqrt();

            let mut polar_start_theta = (left_y - ccy).atan2(left_x - ccx);
            if polar_start_theta < 0.0 {
                polar_start_theta += 2.0 * std::f64::consts::PI;
            }
            let mut polar_end_theta = (right_y - ccy).atan2(right_x - ccx);
            if polar_end_theta < 0.0 {
                polar_end_theta += 2.0 * std::f64::consts::PI;
            }

            let mut angle_radians = polar_end_theta - polar_start_theta;
            if angle_radians < 0.0 && is_ccw {
                angle_radians += 2.0 * std::f64::consts::PI;
            } else if angle_radians > 0.0 && !is_ccw {
                angle_radians -= 2.0 * std::f64::consts::PI;
            }

            for j in 0..arc_segments {
                let mut cur_angle =
                    polar_start_theta + j as f64 / arc_segments as f64 * angle_radians;
                if cur_angle > 2.0 * std::f64::consts::PI {
                    cur_angle -= 2.0 * std::f64::consts::PI;
                } else if cur_angle < 0.0 {
                    cur_angle += 2.0 * std::f64::consts::PI;
                }
                let px = ccx + radius * cur_angle.cos();
                let py = ccy + radius * cur_angle.sin();
                result.push(Point::new(px, py));
            }
            result.push(Point::new(right_x, right_y));
        } else {
            result.push(b);
        }
    }

    result
}

pub(crate) fn find_closest_point_index(pts: &[Point], x: f64, y: f64) -> usize {
    if pts.is_empty() {
        return 0;
    }
    // 两遍：先求最小距离，再取**平局带里下标最小**的那个。
    //
    // 【为什么不能一步用严格 < 选】对称的塔会有两个到参考点**数学上等距**的顶点，
    // 而两个距离的浮点计算（求和顺序、有没有 FMA 融合乘加）在不同平台/不同代码
    // 生成下可能差最后一位 —— 严格 < 就会因此翻转选中的顶点，整条塔的 G-code
    // 跟着镜像。实测：同一份代码，macOS(ARM, 有 FMA) 与 Windows(x86, 无 FMA)
    // 选中的起点相差一个镜像顶点，材料塔的 G-code 就对不上。
    //
    // 平局带用相对量（最小距离的 1e-9 倍）—— 与坐标量级无关，也远大于舍入噪声。
    // 平局一律取**更小的下标**：这条规则只看下标、不看浮点，跨平台逐字节一致。
    let mut min_dist = f64::MAX;
    for p in pts.iter() {
        let dist = (p.x - x) * (p.x - x) + (p.y - y) * (p.y - y);
        if dist < min_dist {
            min_dist = dist;
        }
    }
    let tie_upper = min_dist + min_dist.abs() * 1e-9 + 1e-12;
    for (i, p) in pts.iter().enumerate() {
        let dist = (p.x - x) * (p.x - x) + (p.y - y) * (p.y - y);
        if dist <= tie_upper {
            return i;
        }
    }
    0
}

/// `getLimitDepthByHeight`：按 MinDepthPerHeight 表线性插值（两端钳位）。
pub fn get_limit_depth_by_height(tower_height: f64) -> f64 {
    let table = &MIN_DEPTH_PER_HEIGHT;
    if tower_height <= table[0].0 {
        return table[0].1;
    }
    let last = table[table.len() - 1];
    if tower_height >= last.0 {
        return last.1;
    }
    for i in 0..table.len() - 1 {
        if tower_height >= table[i].0 && tower_height <= table[i + 1].0 {
            let t = (tower_height - table[i].0) / (table[i + 1].0 - table[i].0);
            return table[i].1 + t * (table[i + 1].1 - table[i].1);
        }
    }
    last.1
}

use super::constants::MIN_DEPTH_PER_HEIGHT;
