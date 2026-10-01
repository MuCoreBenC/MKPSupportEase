//! 阶段标记注释（对照 processor/stages.go）。
//!
//! 旧格式 `;MKP_STAGE: Name ; 中文说明` 冻结保留（Deprecated 但 pass1 仍在用）；
//! 新 GCodeWriter 格式属 pass2 面，随需再落。

/// MKPStage —— 后处理插入阶段标识。
/// LeaveTower/PressureRestore/ReturnModel/LayerProgress/M991Notify 由 pass2
/// 消费（Task 12 接线），枚举面与 Go stages.go 对齐保持完整。
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MkpStage {
    LeaveTower,
    RiseNozzle,
    MountToolhead,
    PressureRestore,
    ReturnModel,
    GlueStart,
    GlueEnd,
    RevitalizationStart,
    RevitalizationEnd,
    GluePass,
    LayerProgress,
    M991Notify,
}

impl MkpStage {
    fn name(self) -> &'static str {
        match self {
            MkpStage::LeaveTower => "LeaveTower",
            MkpStage::RiseNozzle => "RiseNozzle",
            MkpStage::MountToolhead => "MountToolhead",
            MkpStage::PressureRestore => "PressureRestore",
            MkpStage::ReturnModel => "ReturnModel",
            MkpStage::GlueStart => "GlueStart",
            MkpStage::GlueEnd => "GlueEnd",
            MkpStage::RevitalizationStart => "RevitalizationStart",
            MkpStage::RevitalizationEnd => "RevitalizationEnd",
            MkpStage::GluePass => "GluePass",
            MkpStage::LayerProgress => "LayerProgress",
            MkpStage::M991Notify => "M991Notify",
        }
    }

    /// 阶段标记对应的中文说明（stages.go stageChineseDesc）。
    fn chinese_desc(self) -> &'static str {
        match self {
            MkpStage::LeaveTower => "离开擦料塔",
            MkpStage::RiseNozzle => "抬升喷嘴",
            MkpStage::MountToolhead => "挂载工具头",
            MkpStage::PressureRestore => "恢复压力",
            MkpStage::ReturnModel => "返回模型",
            MkpStage::GlueStart => "涂胶开始",
            MkpStage::GlueEnd => "涂胶结束",
            MkpStage::RevitalizationStart => "涂胶笔活化开始",
            MkpStage::RevitalizationEnd => "涂胶笔活化结束",
            MkpStage::GluePass => "涂胶 Pass",
            MkpStage::LayerProgress => "层进度",
            MkpStage::M991Notify => "M991 通知",
        }
    }
}

/// `WriteStageComment`：写 `;MKP_STAGE: <Name> ; <中文说明>[ <extra>]\n`。
pub(crate) fn stage_comment_line(stage: MkpStage, extra: &str) -> String {
    if !extra.is_empty() {
        format!(
            ";MKP_STAGE: {} ; {} {}",
            stage.name(),
            stage.chinese_desc(),
            extra
        )
    } else {
        format!(";MKP_STAGE: {} ; {}", stage.name(), stage.chinese_desc())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_comment_format() {
        assert_eq!(
            stage_comment_line(MkpStage::RiseNozzle, ""),
            ";MKP_STAGE: RiseNozzle ; 抬升喷嘴"
        );
        assert_eq!(
            stage_comment_line(MkpStage::GluePass, "2"),
            ";MKP_STAGE: GluePass ; 涂胶 Pass 2"
        );
    }
}
