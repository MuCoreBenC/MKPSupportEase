//! **切片器自己的擦料塔（Prime tower）** —— 只读事实，不参与任何变换。
//!
//! 为什么要在内核里统计它：报告页的「擦料塔」那一组一直只讲 MKP 的擦料塔，
//! 而切片器那份（Bambu 的 `; FEATURE: Prime tower`）在整条链上只有**排除**逻辑
//! （`postproc::disk::collision` 判碰撞时跳开它），从来没被数过。于是：
//! 用户开了官方擦料塔，界面上既看不到「开/关」，也没有它的位置与耗材。
//!
//! 这里产出的就是那几个数，**判据全部来自输入 G-code 本身**（不读预设）：
//! `; enable_prime_tower` 说开关，`; FEATURE: Prime tower` 块说形状与耗材。
//!
//! ## 包围盒为什么不能直接取「块内所有挤出点」
//!
//! 多色换料时，切片器把**换料吐料**也归在 `Prime tower` 这一段里：
//! 那些 `G1 E18 F324` 只有 E、没有 XY，会沿用「上一条移动」的坐标 ——
//! 而上一条往往是 `G1 X-48.2`（床外擦嘴位）。实测 42274 那台，
//! 直接取并集会得到 `X[-48.2, 267.0]`（横跨整板）的假框。
//!
//! 所以只认**自带 X 或 Y 的挤出**（真正在塔身上走线的那种）。
//! 顺带一个好处：`parse_xyze` 不匹配负数，床外擦嘴位的移动天然被排除。

use crate::gcode::coord::parse_xyze;
use crate::gcode::parser::is_feature_line;

/// 切片器给擦料塔块写的 feature 名（逐字，别改）。
const PRIME_TOWER_FEATURE: &str = "Prime tower";

/// 线材截面积（mm²）：π × (1.75/2)²。
const FILAMENT_AREA_MM2: f64 = 2.4053;

/// 线材密度（g/cm³）。与切片器面板的口径一致：PETG/PLA 都按 1.24 估，
/// 实测 214017 的 E 总量反算 40.5g、切片器面板 40.82g。
const FILAMENT_DENSITY_G_PER_CM3: f64 = 1.24;

/// 官方擦料塔的包围盒（俯视矩形，单位 mm）。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlicerTowerBBox {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}

impl SlicerTowerBBox {
    pub fn width(&self) -> f64 {
        self.max_x - self.min_x
    }

    pub fn depth(&self) -> f64 {
        self.max_y - self.min_y
    }
}

/// 官方擦料塔的事实集。没开 / 没塔时 `enabled=false`、`bbox=None`。
#[derive(Debug, Clone, PartialEq)]
pub struct SlicerTowerFacts {
    /// 头部 `; enable_prime_tower = 1`。
    pub enabled: bool,
    /// 有塔块的层数（一个层块算一层）。
    pub layers: i64,
    /// 塔块首末 Z 之差（mm）。
    pub height: f64,
    /// 耗材克数（E 总量 × 截面积 × 密度）。切片器面板同口径。
    pub filament_grams: f64,
    /// 俯视包围盒；没有任何塔块走线时为 `None`。
    pub bbox: Option<SlicerTowerBBox>,
}

impl SlicerTowerFacts {
    fn off() -> Self {
        Self {
            enabled: false,
            layers: 0,
            height: 0.0,
            filament_grams: 0.0,
            bbox: None,
        }
    }
}

/// 扫一遍输入 G-code，得出官方擦料塔的事实。
///
/// 只读、不分配（除了 `layer` 计数的整数），可以随 `header_facts` 一起搭车。
pub fn scan(content: &[String]) -> SlicerTowerFacts {
    let mut facts = SlicerTowerFacts::off();
    let mut in_tower = false;

    let mut x = f64::NAN;
    let mut y = f64::NAN;
    let mut z = f64::NAN;

    let mut layer = 0i64;
    let mut last_layer_with_tower = -1i64;
    let mut e_total = 0.0f64;
    let mut z_min = f64::INFINITY;
    let mut z_max = f64::NEG_INFINITY;
    let mut bbox: Option<SlicerTowerBBox> = None;

    for line in content {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with(';') {
            if let Some(rest) = trimmed.strip_prefix("; enable_prime_tower") {
                if let Some(v) = rest.split('=').nth(1) {
                    facts.enabled = v.trim().parse::<i64>().unwrap_or(0) != 0;
                }
            } else if trimmed.starts_with("; CHANGE_LAYER") {
                layer += 1;
            } else if let Some(feat) = is_feature_line(trimmed) {
                in_tower = feat == PRIME_TOWER_FEATURE;
            }
            continue;
        }
        if !in_tower {
            continue;
        }
        if !(trimmed.starts_with("G1") || trimmed.starts_with("G0")) {
            continue;
        }
        let v = parse_xyze(trimmed.as_bytes());
        if v.has_x {
            x = v.x;
        }
        if v.has_y {
            y = v.y;
        }
        if v.has_z {
            z = v.z;
        }
        // 行程（没有 E）与吐料（没有 XY）都不算塔身。
        if !v.has_e || v.e <= 0.0 || !(v.has_x || v.has_y) {
            continue;
        }
        if x.is_nan() || y.is_nan() {
            continue;
        }
        e_total += v.e;
        if !z.is_nan() {
            z_min = z_min.min(z);
            z_max = z_max.max(z);
        }
        if last_layer_with_tower != layer {
            last_layer_with_tower = layer;
            facts.layers += 1;
        }
        bbox = Some(match bbox {
            None => SlicerTowerBBox {
                min_x: x,
                max_x: x,
                min_y: y,
                max_y: y,
            },
            Some(b) => SlicerTowerBBox {
                min_x: b.min_x.min(x),
                max_x: b.max_x.max(x),
                min_y: b.min_y.min(y),
                max_y: b.max_y.max(y),
            },
        });
    }

    facts.bbox = bbox;
    if z_max.is_finite() && z_min.is_finite() {
        facts.height = z_max - z_min;
    }
    facts.filament_grams = e_total * FILAMENT_AREA_MM2 * FILAMENT_DENSITY_G_PER_CM3 / 1000.0;
    facts
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn no_tower_means_disabled_and_empty() {
        let f = scan(&lines(&[
            "; enable_prime_tower = 0",
            "; CHANGE_LAYER",
            "G1 X1 Y1",
        ]));
        assert!(!f.enabled);
        assert_eq!(f.layers, 0);
        assert!(f.bbox.is_none());
    }

    #[test]
    fn body_is_measured_and_purge_lines_are_ignored() {
        let f = scan(&lines(&[
            "; enable_prime_tower = 1",
            "; CHANGE_LAYER",
            "; FEATURE: Prime tower",
            "G1 Z10.000",
            "G1 X30.000 Y190.000 E1.000",
            "G1 X60.000 E1.000",
            "G1 Y210.000 E1.000",
            "G1 X30.000 E1.000",
            // 换料吐料：只有 E，没有 XY —— 上一条坐标是床外擦嘴位，不能算进塔身
            "G1 X-48.200",
            "G1 E18.000 F324.284",
            "G1 X-38.200",
            "G1 E5.000",
            "; FEATURE: Support",
            "G1 X200.000 Y100.000 E99.000",
        ]));
        assert!(f.enabled);
        assert_eq!(f.layers, 1);
        let b = f.bbox.expect("有塔块就该有包围盒");
        assert_eq!(
            (b.min_x, b.max_x, b.min_y, b.max_y),
            (30.0, 60.0, 190.0, 210.0)
        );
        // 只有四条塔身线进账，吐料那两条不计
        let expect_e = 4.0;
        assert!((f.filament_grams - expect_e * FILAMENT_AREA_MM2 * 1.24 / 1000.0).abs() < 1e-9);
        assert_eq!(f.height, 0.0, "单层塔没有高度");
    }

    #[test]
    fn layers_count_distinct_layer_blocks() {
        let f = scan(&lines(&[
            "; CHANGE_LAYER",
            "; FEATURE: Prime tower",
            "G1 X10.000 Y10.000 E1.000",
            "; CHANGE_LAYER",
            "; FEATURE: Prime tower",
            "G1 X10.000 Y10.000 E1.000",
        ]));
        assert_eq!(f.layers, 2);
    }
}
