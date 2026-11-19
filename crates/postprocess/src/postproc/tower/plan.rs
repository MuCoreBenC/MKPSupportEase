//! 塔片 Z 规划：塔**自己**的连续构建计划。
//!
//! 输入只有三样：上一片打到的 Z、这一层的层顶、片高的上下限。输出是一串塔片。
//! 模型只负责"报高度"（`; Z_HEIGHT:` 那一条全局层序列，多对象时已经是并集），
//! 塔自己决定怎么长上去 —— 不跟随某一个模型，也不让某个模型的层高直接当片高。
//!
//! 两条边界来自真机反馈：
//! - 一片最多 [`Bounds::max`]（`0.7 × 喷嘴`，0.4 → 0.28）：差值超过它就拆成几片，
//!   绝不把 0.4 当一片打出来（那样会把塔片拉虚、层间留缝）。拆出来的中间片
//!   **不必对应任何模型层**，它只是塔自己往上长的台阶。
//! - 一片最少 [`Bounds::min`]（`0.2 × 喷嘴`，0.4 → 0.08）：差值不到它就并到下一层。
//!   可变层高里 0.067 那种细层如果各打一片，全是起停开销，塔也不会更结实。
//!
//! 规划是**贪心**的，不需要预读下一层：并掉的细层留下的差值可能超过上限，
//! 那时候再拆 —— 一样不留缝。

/// 塔片之间最小的 Z 增量：小于它就算同一片（浮点伪影）。
pub const SLICE_EPS: f64 = 1e-6;
/// 一层最多拆几片：再厚也不会把塔打成一摞几十片。
pub const MAX_SLICES_PER_LAYER: i64 = 32;

/// 片高的上下限（mm），由喷嘴直径决定。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min: f64,
    pub max: f64,
}

impl Bounds {
    /// 上下限夹一遍：配置坏了也不除零、不产生负片高。
    fn sanitized(self) -> Self {
        let max = if self.max > SLICE_EPS {
            self.max
        } else {
            SLICE_EPS
        };
        let min = if self.min > SLICE_EPS {
            self.min.min(max)
        } else {
            SLICE_EPS
        };
        Self { min, max }
    }
}

/// 一片塔：落在哪个 Z、这一片打多厚。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TowerSlice {
    pub z: f64,
    pub height: f64,
}

/// 从 `from_z` 长到 `to_z` 要打哪几片。
///
/// `must_reach` 为真时（最后一层）即使差值不到下限也照打 —— 塔必须跟到件顶。
pub fn plan_layer(from_z: f64, to_z: f64, bounds: Bounds, must_reach: bool) -> Vec<TowerSlice> {
    let bounds = bounds.sanitized();
    let grow = to_z - from_z;
    if grow <= SLICE_EPS {
        return Vec::new();
    }
    // 细层：并到下一层（下一片会把这一段一起打满，所以不会留缝）。
    if grow + SLICE_EPS < bounds.min && !must_reach {
        return Vec::new();
    }
    let n = ((grow / bounds.max).ceil() as i64).clamp(1, MAX_SLICES_PER_LAYER);
    let height = grow / n as f64;
    (1..=n)
        .map(|k| TowerSlice {
            // 末片落在层顶（用原值，不用累加值）：塔片的 Z 要跟切片器自己那个数对上。
            z: if k == n {
                to_z
            } else {
                from_z + height * k as f64
            },
            height,
        })
        .collect()
}

/// 整条层序列折出来的塔片（判据用）：把 [`plan_layer`] 一层层接下去。
pub fn plan_sequence(layer_zs: &[f64], start_z: f64, bounds: Bounds) -> Vec<TowerSlice> {
    let mut out = Vec::new();
    let mut z = start_z;
    for (i, &layer_z) in layer_zs.iter().enumerate() {
        let must_reach = i + 1 == layer_zs.len();
        let slices = plan_layer(z, layer_z, bounds, must_reach);
        if let Some(last) = slices.last() {
            z = last.z;
        }
        out.extend(slices);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 0.4 喷嘴：上限 0.28、下限 0.08。
    const B: Bounds = Bounds {
        min: 0.08,
        max: 0.28,
    };

    fn heights(slices: &[TowerSlice]) -> Vec<f64> {
        slices
            .iter()
            .map(|s| (s.height * 1000.0).round() / 1000.0)
            .collect()
    }

    /// 判据的公共部分：片高都在 [min, max]、Z 严格递增、末片落顶、薄片不出现。
    fn assert_invariants(slices: &[TowerSlice], start_z: f64, top_z: f64) {
        assert!(!slices.is_empty(), "塔从起点到件顶必须至少有一片");
        let mut prev = start_z;
        for (i, s) in slices.iter().enumerate() {
            assert!(
                s.height <= B.max + 1e-9,
                "第 {i} 片高 {} 超过上限 {}",
                s.height,
                B.max
            );
            // 末片的余量可以低于下限（塔必须跟到件顶），其余各片不许。
            if i + 1 < slices.len() {
                assert!(
                    s.height >= B.min - 1e-9,
                    "第 {i} 片高 {} 低于下限 {}",
                    s.height,
                    B.min
                );
            }
            assert!(
                s.z > prev + 1e-9,
                "第 {i} 片 Z {} 没有高于上一片 {prev}",
                s.z
            );
            // 每一片都必须与上一片相接：Z 差就是它自己声明的高度。
            assert!(
                (s.z - prev - s.height).abs() <= 1e-6,
                "第 {i} 片声明 {} 但 Z 差是 {}",
                s.height,
                s.z - prev
            );
            prev = s.z;
        }
        assert!(
            (prev - top_z).abs() <= 1e-6,
            "塔顶 {prev} 没有到件顶 {top_z}"
        );
    }

    /// ① 固定 0.2 层：一层一片，片高 0.2，塔跟到顶。
    #[test]
    fn fixed_layers_give_one_slice_each() {
        let zs: Vec<f64> = (1..=10).map(|i| 0.2 * i as f64).collect();
        let slices = plan_sequence(&zs, 0.0, B);
        assert_eq!(slices.len(), zs.len());
        assert!(heights(&slices).iter().all(|h| (h - 0.2).abs() < 1e-9));
        assert_invariants(&slices, 0.0, 2.0);
    }

    /// ② 有一层 0.4（比上限厚）：拆成两片 0.2，中间那片不在层顶。
    #[test]
    fn a_layer_thicker_than_the_cap_is_split() {
        let slices = plan_layer(0.2, 0.6, B, false);
        assert_eq!(slices.len(), 2);
        assert!((slices[0].z - 0.4).abs() < 1e-9, "中间补片落在 0.4");
        assert!((slices[1].z - 0.6).abs() < 1e-9, "末片必须落在层顶");
        assert!(heights(&slices).iter().all(|h| (h - 0.2).abs() < 1e-9));
    }

    /// ③ 0.45 的台阶：拆两片 0.225（两片一样厚，不留 0.28+0.17 那种尾巴）。
    #[test]
    fn an_odd_step_splits_evenly() {
        let slices = plan_layer(0.0, 0.45, B, false);
        assert_eq!(slices.len(), 2);
        assert!(heights(&slices).iter().all(|h| (h - 0.225).abs() < 1e-9));
        assert!((slices[1].z - 0.45).abs() < 1e-9);
    }

    /// ④ 细层区（0.05 一层）：并到下一层，不出薄片，也不留缝。
    #[test]
    fn thin_layers_are_merged_into_the_next_one() {
        // 0.05 太薄（< 0.08）⇒ 并掉；0.10 够一片就出；0.15 再薄掉；
        // 最后 0.30 从 0.10 一次补 0.20 —— 四层出两片。
        let zs = vec![0.05, 0.10, 0.15, 0.30];
        let slices = plan_sequence(&zs, 0.0, B);
        assert_eq!(slices.len(), 2, "四层细层应当并成两片：{slices:?}");
        assert!((slices[0].height - 0.10).abs() < 1e-9);
        assert!((slices[1].height - 0.20).abs() < 1e-9);
        assert!((slices[1].z - 0.30).abs() < 1e-9, "末片落在件顶");
    }

    /// ⑤ 细层并完超上限：照样拆开，不留缝。
    #[test]
    fn merging_thin_layers_never_exceeds_the_cap() {
        // 0.04 × 7 = 0.28，再来一层 0.04 ⇒ 攒到 0.32 必须拆
        let zs: Vec<f64> = (1..=8).map(|i| 0.04 * i as f64).collect();
        let slices = plan_sequence(&zs, 0.0, B);
        assert_invariants(&slices, 0.0, 0.32);
    }

    /// ⑥ 多个模型各用不同可变层高：内核读到的是并集，序列不规则（0.16/0.08/0.20 混着）。
    #[test]
    fn a_merged_multi_model_sequence_stays_continuous() {
        let zs = vec![
            0.16, 0.24, 0.40, 0.56, 0.72, 0.80, 0.96, 1.12, 1.20, 1.36, 1.52, 1.60, 1.76,
        ];
        let slices = plan_sequence(&zs, 0.0, B);
        assert_invariants(&slices, 0.0, 1.76);
        assert!(
            slices.iter().all(|s| s.height >= B.min - 1e-9),
            "并集里 0.08 的细层不该单独成片：{:?}",
            heights(&slices)
        );
    }

    /// ⑦ 最后一层差一点点：薄也要打，塔必须跟到件顶。
    #[test]
    fn the_last_layer_is_always_reached() {
        let slices = plan_layer(0.20, 0.24, B, true);
        assert_eq!(slices.len(), 1);
        assert!((slices[0].z - 0.24).abs() < 1e-9);
        // 不是最后一层时同样的差值会被并掉。
        assert!(plan_layer(0.20, 0.24, B, false).is_empty());
    }

    /// ⑧ 回退/同高的层（浮点伪影、并集里的重复 Z）不出片，也不产生负片高。
    #[test]
    fn repeated_or_backwards_z_is_ignored() {
        assert!(plan_layer(0.4, 0.4, B, true).is_empty());
        assert!(plan_layer(0.4, 0.3, B, true).is_empty());
    }

    /// ⑨ 上下限写反或为 0：不除零、不 panic，仍按能打的范围切。
    #[test]
    fn broken_bounds_do_not_panic() {
        let odd = Bounds { min: 0.5, max: 0.1 };
        let slices = plan_layer(0.0, 0.3, odd, false);
        assert!(!slices.is_empty());
        assert!(
            slices
                .iter()
                .all(|s| s.height.is_finite() && s.height > 0.0)
        );
    }
}
