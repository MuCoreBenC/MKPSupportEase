//! 几何原语（对照 internal/geometry 包：SAT 相交 / MTV / BBox / 移动方向）。

/// Go `CeilTo01`：向上取到 0.1 精度。
pub fn ceil_to_01(v: f64) -> f64 {
    (v * 10.0).ceil() / 10.0
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct BBox {
    pub x_min: f64,
    pub x_max: f64,
    pub y_min: f64,
    pub y_max: f64,
}

impl BBox {
    pub fn width(&self) -> f64 {
        self.x_max - self.x_min
    }
    pub fn height(&self) -> f64 {
        self.y_max - self.y_min
    }
    pub fn intersects(&self, other: &BBox) -> bool {
        self.x_min < other.x_max
            && self.x_max > other.x_min
            && self.y_min < other.y_max
            && self.y_max > other.y_min
    }
    pub fn contains(&self, inner: &BBox) -> bool {
        inner.x_min >= self.x_min
            && inner.x_max <= self.x_max
            && inner.y_min >= self.y_min
            && inner.y_max <= self.y_max
    }
    pub fn translated(&self, dx: f64, dy: f64) -> BBox {
        BBox {
            x_min: self.x_min + dx,
            x_max: self.x_max + dx,
            y_min: self.y_min + dy,
            y_max: self.y_max + dy,
        }
    }
}

pub type Polygon = Vec<Vec<f64>>;

pub fn polygons_intersect(poly1: &Polygon, poly2: &Polygon) -> bool {
    sat_test(poly1, poly2) && sat_test(poly2, poly1)
}

fn sat_test(poly1: &Polygon, poly2: &Polygon) -> bool {
    let n = poly1.len();
    for i in 0..n {
        let j = (i + 1) % n;
        let edge_x = poly1[j][0] - poly1[i][0];
        let edge_y = poly1[j][1] - poly1[i][1];
        let axis_x = -edge_y;
        let axis_y = edge_x;

        let (min1, max1) = project_polygon(poly1, axis_x, axis_y);
        let (min2, max2) = project_polygon(poly2, axis_x, axis_y);

        if max1 < min2 || max2 < min1 {
            return false;
        }
    }
    true
}

fn project_polygon(poly: &Polygon, axis_x: f64, axis_y: f64) -> (f64, f64) {
    let mut min = poly[0][0] * axis_x + poly[0][1] * axis_y;
    let mut max = min;
    for p in &poly[1..] {
        let proj = p[0] * axis_x + p[1] * axis_y;
        if proj < min {
            min = proj;
        }
        if proj > max {
            max = proj;
        }
    }
    (min, max)
}

/// `MinTranslationVector`：把 poly1 移出 poly2 的最小平移（SAT，取穿透最浅轴）。
pub fn min_translation_vector(poly1: &Polygon, poly2: &Polygon) -> (f64, f64) {
    let mut min_overlap = f64::MAX;
    let mut best_axis_x = 0.0;
    let mut best_axis_y = 0.0;

    fn check_axes(
        p: &Polygon,
        poly1: &Polygon,
        poly2: &Polygon,
        min_overlap: &mut f64,
        best_axis_x: &mut f64,
        best_axis_y: &mut f64,
    ) {
        let n = p.len();
        for i in 0..n {
            let j = (i + 1) % n;
            let edge_x = p[j][0] - p[i][0];
            let edge_y = p[j][1] - p[i][1];
            let mut axis_x = -edge_y;
            let mut axis_y = edge_x;
            let axis_len = (axis_x * axis_x + axis_y * axis_y).sqrt();
            if axis_len == 0.0 {
                continue;
            }
            axis_x /= axis_len;
            axis_y /= axis_len;

            let (min1, max1) = project_polygon(poly1, axis_x, axis_y);
            let (min2, max2) = project_polygon(poly2, axis_x, axis_y);

            if max1 < min2 || max2 < min1 {
                return; // 不相交，此轴无穿透
            }
            let overlap = max1.min(max2) - min1.max(min2);
            if overlap < *min_overlap {
                *min_overlap = overlap;
                *best_axis_x = axis_x;
                *best_axis_y = axis_y;
                let (c1x, c1y) = polygon_centroid(poly1);
                let (c2x, c2y) = polygon_centroid(poly2);
                if (c1x - c2x) * axis_x + (c1y - c2y) * axis_y < 0.0 {
                    *best_axis_x = -*best_axis_x;
                    *best_axis_y = -*best_axis_y;
                }
            }
        }
    }

    check_axes(
        poly1,
        poly1,
        poly2,
        &mut min_overlap,
        &mut best_axis_x,
        &mut best_axis_y,
    );
    check_axes(
        poly2,
        poly1,
        poly2,
        &mut min_overlap,
        &mut best_axis_x,
        &mut best_axis_y,
    );

    if min_overlap == f64::MAX {
        return (0.0, 0.0);
    }
    (best_axis_x * min_overlap, best_axis_y * min_overlap)
}

fn polygon_centroid(poly: &Polygon) -> (f64, f64) {
    let mut cx = 0.0;
    let mut cy = 0.0;
    for p in poly {
        cx += p[0];
        cy += p[1];
    }
    let n = poly.len() as f64;
    (cx / n, cy / n)
}

pub fn polygon_to_bbox(poly: &Polygon) -> BBox {
    if poly.is_empty() {
        return BBox::default();
    }
    let mut min_x = poly[0][0];
    let mut max_x = poly[0][0];
    let mut min_y = poly[0][1];
    let mut max_y = poly[0][1];
    for p in &poly[1..] {
        if p[0] < min_x {
            min_x = p[0];
        }
        if p[0] > max_x {
            max_x = p[0];
        }
        if p[1] < min_y {
            min_y = p[1];
        }
        if p[1] > max_y {
            max_y = p[1];
        }
    }
    BBox {
        x_min: min_x,
        x_max: max_x,
        y_min: min_y,
        y_max: max_y,
    }
}

/// 移动方向（对照 direction.go；中文名是 UI 消息的一部分，保真保留）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MoveDirection {
    #[default]
    None,
    Left,
    Right,
    Front,
    Back,
    LeftFront,
    RightFront,
    LeftBack,
    RightBack,
}

impl MoveDirection {
    pub fn as_str_cn(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Left => "向左",
            Self::Right => "向右",
            Self::Front => "向上",
            Self::Back => "向下",
            Self::LeftFront => "向左上",
            Self::RightFront => "向右上",
            Self::LeftBack => "向左下",
            Self::RightBack => "向右下",
        }
    }

    /// `DeltaFromDirection`：单位分量（-1/0/1）。
    pub fn delta(self) -> (f64, f64) {
        match self {
            Self::Left => (-1.0, 0.0),
            Self::Right => (1.0, 0.0),
            Self::Front => (0.0, 1.0),
            Self::Back => (0.0, -1.0),
            Self::LeftFront => (-1.0, 1.0),
            Self::RightFront => (1.0, 1.0),
            Self::LeftBack => (-1.0, -1.0),
            Self::RightBack => (1.0, -1.0),
            Self::None => (0.0, 0.0),
        }
    }

    /// `DirectionFromDelta`（dx/dy ∈ {-1,0,1}）。
    pub fn from_delta(dx: i32, dy: i32) -> Self {
        match (dx, dy) {
            (-1, -1) => Self::LeftBack,
            (-1, 0) => Self::Left,
            (-1, 1) => Self::LeftFront,
            (0, -1) => Self::Back,
            (0, 0) => Self::None,
            (0, 1) => Self::Front,
            (1, -1) => Self::RightBack,
            (1, 0) => Self::Right,
            (1, 1) => Self::RightFront,
            _ => Self::None,
        }
    }
}
