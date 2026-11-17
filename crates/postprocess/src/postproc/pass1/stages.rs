//! 阶段标记注释（对照 processor/stages.go）。
//!
//! 旧格式 `;MKP_STAGE: Name ; 中文说明` 冻结保留（Deprecated 但 pass1 仍在用）；
//! 新 GCodeWriter 格式属 pass2 面，随需再落。

/// MKPStage —— 后处理插入阶段标识。
///
/// ★ `RevitalizationStart/End` 是**润笔**（把本段前 N mm 的涂胶路径空走重画一遍，
///   让笔尖重新吃上胶）。中文一律叫「润笔」，与设置页「润笔总开关 / 每层润笔」
///   和报告页同一个词；英文标记 `;Gluepen Revitalization Start/End` 保持不动，
///   `postproc/calibration.rs` 与旧产物靠它认段。
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
            MkpStage::RevitalizationStart => "润笔开始",
            MkpStage::RevitalizationEnd => "润笔结束",
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

    /// 润笔那两句：中文叫「润笔」（与设置页 / 报告页同一个词），
    /// 不再出现「活化」；英文标记 `RevitalizationStart` 保持不动（认段靠它）。
    #[test]
    fn revitalization_is_called_run_bi() {
        let start = stage_comment_line(MkpStage::RevitalizationStart, "");
        let end = stage_comment_line(MkpStage::RevitalizationEnd, "");
        assert_eq!(start, ";MKP_STAGE: RevitalizationStart ; 润笔开始");
        assert_eq!(end, ";MKP_STAGE: RevitalizationEnd ; 润笔结束");
        assert!(!start.contains("活化") && !end.contains("活化"));
    }
}
