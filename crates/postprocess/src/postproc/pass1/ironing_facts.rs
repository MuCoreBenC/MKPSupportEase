//! 熨烫判据的**事后事实**（2026-11-15）：只记录，不参与判定。
//!
//! # 为什么要有它
//!
//! 作者实测：一张看着远超「熨烫挤出上限」（出厂 7.9mm）的熨烫面没被删掉，把上限改成
//! 0.10 才消失。追查的结论是 —— 判据按**切片器打的一小块**（一对
//! `M1031 S1 ;IRONING_EXTRUSIONS_START` … `IRONING_EXTRUSIONS_END`）各自的挤出合计判，
//! 不是按整张面。作者原话：「打印的时候它得跳起来，那个算拆分，但是判断的时候又不需要」
//! —— 所以判据改成**把相邻的块连成一整段（一张面）再合计**（见 `sum_ironing_run_e_sum`）。
//!
//! 这一组事实记的就是那个"整段"：一张面记一条，不是几十块各记一条。
//!
//! # 它不改任何行为
//!
//! 三个 `record_*` 只是在**原本已经算出来的那个数**上累加：
//! `should_remove_ironing_segment` 的比较方向与 `>` 语义一个字不动，
//! `build_per_model_iface` 输出的行序列逐字节不变。

/// 每张面 / 每层 / 每次配对的总账。
///
/// ★ **"块"的粒度是一整段连续熨烫**（作者口径：一张面），不是切片器打的 `M1031` 块：
///   同一张面会被切片器拆成几十块（中间只隔着抬笔空走），判定时连起来合计。
///
/// ★ **不存逐条明细**：真机上一份 22MB 的件能有上千段熨烫，逐条落进 `_meta.json`
/// 会把一份记录撑到几十 KB，而作者要看的是"这一盘判掉了没有、最大一张面多大"。
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IroningFacts {
    /// 见到几段连续熨烫（= 几张面，任何模式都记）。
    pub blocks: u32,
    /// 被判掉、没进产物的段数（`off` 全删；`auto` 超上限才删；`on` 恒 0）。
    pub removed: u32,
    /// 所有段里最大的挤出合计（mm，耗材长度）。一段都没有 ⇒ `null`。
    pub max_e_sum: Option<f64>,
    /// 这一层最终用熨烫路径的层数（含"熨烫 + 支撑面"混合）。
    pub layers_ironing: u32,
    /// 这一层整层用支撑面路径的层数。
    pub layers_moisture: u32,
    /// 这一层回退到支撑体（`iface_fallback`）的层数。
    pub layers_fallback: u32,
    /// 参与过配对的「支撑面组 ↔ 熨烫组」里覆盖率最小的那个（%）。
    /// 一次都没配对（阈值=0 / 没有相交的组）⇒ `null`。
    pub min_coverage: Option<f64>,
}

/// 这一层的涂胶路径最终从哪儿来 —— **一层只记一次**（在收口处记，避免分支多计）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) enum IroningLayerPath {
    /// 这一层没有可用的熨烫 / 支撑面数据，沿用上一层的 `iface`。
    #[default]
    None,
    /// 熨烫路径（含与支撑面按区域混合的结果）。
    Ironing,
    /// 支撑面路径。
    Moisture,
    /// 支撑体（`iface_fallback`）那条兜底路径。
    Fallback,
}

/// [`super::model_iface::build_per_model_iface`] 这一趟顺手算出来的配对统计。
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct MergeStats {
    /// 这些配对里最小的覆盖率（%）；`None` = 一次都没配对（阈值=0 或没有相交的组）。
    pub(crate) min_coverage: Option<f64>,
}

impl IroningFacts {
    /// 记一块：`e_sum` 是它的挤出合计，`removed` 是它有没有被判掉。
    pub(crate) fn record_block(&mut self, e_sum: f64, removed: bool) {
        self.blocks += 1;
        if removed {
            self.removed += 1;
        }
        self.max_e_sum = Some(match self.max_e_sum {
            Some(m) => m.max(e_sum),
            None => e_sum,
        });
    }

    /// 记一层最终走了哪条路（`None` 什么也不加）。
    pub(crate) fn record_layer(&mut self, path: IroningLayerPath) {
        match path {
            IroningLayerPath::None => {}
            IroningLayerPath::Ironing => self.layers_ironing += 1,
            IroningLayerPath::Moisture => self.layers_moisture += 1,
            IroningLayerPath::Fallback => self.layers_fallback += 1,
        }
    }

    /// 收下 [`MergeStats`]（每次混合调用收一次）。
    pub(crate) fn record_merge(&mut self, m: MergeStats) {
        if let Some(c) = m.min_coverage {
            self.min_coverage = Some(match self.min_coverage {
                Some(prev) => prev.min(c),
                None => c,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blocks_track_max_and_removed() {
        let mut f = IroningFacts::default();
        f.record_block(0.05, false);
        f.record_block(0.34, true);
        f.record_block(0.12, false);
        assert_eq!(f.blocks, 3);
        assert_eq!(f.removed, 1);
        assert_eq!(f.max_e_sum, Some(0.34));
    }

    #[test]
    fn no_block_leaves_max_empty() {
        let f = IroningFacts::default();
        assert_eq!(f.blocks, 0);
        assert_eq!(f.max_e_sum, None, "一块都没有 ⇒ 未知，不是 0");
        assert_eq!(f.min_coverage, None);
    }

    #[test]
    fn layers_count_by_path() {
        let mut f = IroningFacts::default();
        f.record_layer(IroningLayerPath::Ironing);
        f.record_layer(IroningLayerPath::Moisture);
        f.record_layer(IroningLayerPath::Moisture);
        f.record_layer(IroningLayerPath::Fallback);
        f.record_layer(IroningLayerPath::None);
        assert_eq!(f.layers_ironing, 1);
        assert_eq!(f.layers_moisture, 2);
        assert_eq!(f.layers_fallback, 1);
    }

    #[test]
    fn merge_keeps_the_smallest_coverage() {
        let mut f = IroningFacts::default();
        f.record_merge(MergeStats {
            min_coverage: Some(42.0),
        });
        f.record_merge(MergeStats {
            min_coverage: Some(8.5),
        });
        assert_eq!(f.min_coverage, Some(8.5));
        /* 没有配对的那些层不把最小值抹成 0 */
        f.record_merge(MergeStats::default());
        assert_eq!(f.min_coverage, Some(8.5));
    }
}
