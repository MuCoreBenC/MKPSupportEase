//! 切片器注释标记与 MKP 注释前缀（对照 markers.go / mkp_markers.go）。

/// 结构性标记（`;WIPE_START` 家族等，值含前导 `;`，与旧侧常量逐字一致）。
pub const MARKER_WIPE_START: &str = ";WIPE_START";
pub const MARKER_WIPE_END: &str = ";WIPE_END";
pub const MARKER_DISK_WIPE_START: &str = ";DISK_WIPE_START";
pub const MARKER_DISK_WIPE_END: &str = ";DISK_WIPE_END";
pub const MARKER_ZJUMP_START: &str = ";ZJUMP_START";
pub const MARKER_ZJUMP_INTERNAL: &str = ";ZJUMP_INTERNAL";
pub const MARKER_SHIELD_NOZZLE: &str = ";Shielding Nozzle";
pub const MARKER_PREPARE_NEXT_TOWER: &str = ";Prepare for next tower";
pub const MARKER_LIFT_Z: &str = ";Lift-z";
pub const MARKER_ADJUST_COOLING: &str = ";Adjust cooling distance";
pub const MARKER_LOWER_PENTIP: &str = ";Lower pentip";
pub const MARKER_PRE_GLUE: &str = ";Pre-glue preparation";
pub const MARKER_GLUE_FINISHED: &str = ";Glueing Finished";
pub const MARKER_WAITING_GLUE: &str = ";Waiting for Glue Settling";
pub const MARKER_FILAMENT_END: &str = "; filament end gcode";
pub const MARKER_MORE_EXTRUSION: &str = ";MoreExtrusion";
pub const MARKER_RISING_NOZZLE: &str = ";Rising Nozzle a little";
pub const MARKER_MICRO_LIFT: &str = ";MICRO_LIFT";

/// MKP 注释统一前缀（含尾随空格）。操作类型常量见下方。
pub const MKP_PREFIX: &str = ";MKP ";
pub const MKP_OP_STAGE: &str = "STAGE";
pub const MKP_OP_BEGIN: &str = "BEGIN";
pub const MKP_OP_END: &str = "END";
pub const MKP_OP_SOURCE: &str = "SOURCE";
pub const MKP_OP_META: &str = "META";
pub const MKP_OP_OP: &str = "OP";
pub const MKP_OP_OP_END: &str = "OP END";
pub const MKP_OP_INSERT: &str = "insert";
pub const MKP_OP_REMOVE: &str = "remove";
pub const MKP_OP_REPLACE: &str = "replace";

/// `MatchSlicerComment`：trim 后包含 `;<marker>` 或 `; <marker>`。
///
/// 零分配形态（Task 18.2 归因后的等价改写：原 `contains(&format!(...))` 每次调用
/// 两次堆分配，per-line 调用点上它是 release 剖面的主导热点）。语义逐字等价：
/// 「某处出现 `;`+marker 或 `;`+` `+marker」⇔ 扫描每个 `;` 的后继。
pub fn match_slicer_comment(line: &str, marker: &str) -> bool {
    let trimmed = line.trim();
    let mut at = 0usize;
    while let Some(rel) = trimmed[at..].find(';') {
        at += rel;
        let rest = &trimmed[at + 1..];
        if rest.starts_with(marker)
            || rest
                .strip_prefix(' ')
                .is_some_and(|r| r.starts_with(marker))
        {
            return true;
        }
        at += 1;
    }
    false
}

/// `HasSlicerCommentPrefix`：trim 后以 `;<marker>` 或 `; <marker>` 开头。
///
/// 零分配形态（同上）：剥掉前导 `;` 后比 marker，两种间隔形态各查一次。
pub fn has_slicer_comment_prefix(line: &str, marker: &str) -> bool {
    let trimmed = line.trim();
    match trimmed.strip_prefix(';') {
        Some(rest) => {
            rest.starts_with(marker)
                || rest
                    .strip_prefix(' ')
                    .is_some_and(|r| r.starts_with(marker))
        }
        None => false,
    }
}

/// 支撑相关 feature 名（`;FEATURE: Support interface` 等）。
pub const SUPPORT_FEATURES: &[&str] = &[
    "Support interface",
    "Support transition",
    "Support body",
    "Support ironing",
];

pub const IRONING_FEATURES: &[&str] = &["Ironing", "Support ironing"];

pub const GLUE_CONTINUOUS_FEATURES: &[&str] = &["Support interface", "Ironing", "Support ironing"];

/// feature 的双语元数据（名称 / 类别 / 涂胶角色），对照 `AllFeatureTypes`。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FeatureInfo {
    pub name_en: &'static str,
    pub name_cn: &'static str,
    pub category: &'static str,
    pub glue_role: &'static str,
}

pub const ALL_FEATURE_TYPES: &[FeatureInfo] = &[
    FeatureInfo {
        name_en: "Support interface",
        name_cn: "支撑面",
        category: "支撑",
        glue_role: "连续",
    },
    FeatureInfo {
        name_en: "Support transition",
        name_cn: "支撑过渡",
        category: "支撑",
        glue_role: "转换",
    },
    FeatureInfo {
        name_en: "Support body",
        name_cn: "支撑体",
        category: "支撑",
        glue_role: "忽略",
    },
    FeatureInfo {
        name_en: "Support ironing",
        name_cn: "支撑面熨烫",
        category: "熨烫",
        glue_role: "连续",
    },
    FeatureInfo {
        name_en: "Ironing",
        name_cn: "熨烫",
        category: "熨烫",
        glue_role: "连续",
    },
    FeatureInfo {
        name_en: "Inner wall",
        name_cn: "内墙",
        category: "非支撑",
        glue_role: "转换",
    },
    FeatureInfo {
        name_en: "Outer wall",
        name_cn: "外墙",
        category: "非支撑",
        glue_role: "转换",
    },
    FeatureInfo {
        name_en: "Internal infill",
        name_cn: "内部填充",
        category: "非支撑",
        glue_role: "转换",
    },
    FeatureInfo {
        name_en: "Top surface",
        name_cn: "顶面",
        category: "非支撑",
        glue_role: "转换",
    },
    FeatureInfo {
        name_en: "Bridge",
        name_cn: "桥接",
        category: "非支撑",
        glue_role: "转换",
    },
    FeatureInfo {
        name_en: "Brim",
        name_cn: "裙边",
        category: "非支撑",
        glue_role: "转换",
    },
    FeatureInfo {
        name_en: "Support",
        name_cn: "支撑(泛指)",
        category: "支撑",
        glue_role: "转换",
    },
];

/// `;FEATURE: <name>` 前缀命中的零分配形态：语义等价于
/// `has_slicer_comment_prefix(line, &format!("FEATURE: {name}"))`（前缀允许
/// name 之后有任意内容——旧侧 Go 的 HasPrefix 语义如此）。
fn has_feature(line: &str, name: &str) -> bool {
    let trimmed = line.trim();
    let Some(rest) = trimmed.strip_prefix(';') else {
        return false;
    };
    let rest = rest.strip_prefix(' ').unwrap_or(rest);
    rest.strip_prefix("FEATURE: ")
        .is_some_and(|after| after.starts_with(name))
}

/// `IsSupportRelatedFeature`：`;FEATURE: <支撑族名>` 前缀命中。
pub fn is_support_related_feature(line: &str) -> bool {
    SUPPORT_FEATURES.iter().any(|f| has_feature(line, f))
}

/// `IsIroningFeature`：`;FEATURE: <熨烫族名>` 前缀命中。
pub fn is_ironing_feature(line: &str) -> bool {
    IRONING_FEATURES.iter().any(|f| has_feature(line, f))
}

/// `IsGlueContinuousFeature`：涂胶「连续」角色族命中。
pub fn is_glue_continuous_feature(line: &str) -> bool {
    GLUE_CONTINUOUS_FEATURES
        .iter()
        .any(|f| has_feature(line, f))
}

/// `MatchAnyFeature`：按 `AllFeatureTypes` 顺序返回首个命中的 feature 名。
/// 顺序敏感：`Support ironing` 排在 `Ironing` 前（更具体的在前）。
pub fn match_any_feature(line: &str) -> Option<&'static str> {
    ALL_FEATURE_TYPES
        .iter()
        .find(|ft| has_feature(line, ft.name_en))
        .map(|ft| ft.name_en)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slicer_comment_prefix_forms() {
        assert!(has_slicer_comment_prefix(";WIPE_START", "WIPE_START"));
        assert!(has_slicer_comment_prefix("  ; WIPE_START  ", "WIPE_START"));
        // Go 的 HasPrefix 是纯前缀判定：";WIPE_STARTER" 也以 ";WIPE_START" 开头 → 命中。
        // （旧侧语义如此——已知宽松，靠调用方传完整标记名兜住。）
        assert!(has_slicer_comment_prefix(";WIPE_STARTER", "WIPE_START"));
        assert!(!has_slicer_comment_prefix("X;WIPE_START", "WIPE_START"));
        assert!(match_slicer_comment(
            "G1 X1 ; WIPE_START tail",
            "WIPE_START"
        ));
        assert!(!match_slicer_comment("G1 X1", "WIPE_START"));
    }

    /// 前缀判定的边界：`;FEATURE: Support` 应命中 Support 而不被
    /// "Support interface" 抢占（前缀要求整段匹配到族名之后可以是任意内容——
    /// 旧侧语义即如此，";FEATURE: Support" 是 Support 泛指）。
    #[test]
    fn feature_matching_order() {
        assert_eq!(
            match_any_feature("; FEATURE: Support interface"),
            Some("Support interface")
        );
        assert_eq!(
            match_any_feature("; FEATURE: Support ironing"),
            Some("Support ironing")
        );
        // "Support interface" 前缀也在 "Support" 前，但 "Support interface" 更长优先
        assert_eq!(match_any_feature("; FEATURE: Support"), Some("Support"));
        assert_eq!(match_any_feature("; FEATURE: Ironing"), Some("Ironing"));
        assert_eq!(match_any_feature("; FEATURE: Unknown"), None);
        assert_eq!(match_any_feature("G1 X1"), None);
    }

    #[test]
    fn feature_family_predicates() {
        assert!(is_support_related_feature("; FEATURE: Support interface"));
        assert!(is_support_related_feature(";FEATURE: Support body"));
        assert!(!is_support_related_feature("; FEATURE: Ironing"));
        assert!(!is_support_related_feature("; FEATURE: Support_not_exist"));

        assert!(is_ironing_feature("; FEATURE: Ironing"));
        assert!(is_ironing_feature("; FEATURE: Support ironing"));
        assert!(!is_ironing_feature("; FEATURE: Inner wall"));

        assert!(is_glue_continuous_feature("; FEATURE: Ironing"));
        assert!(!is_glue_continuous_feature("; FEATURE: Inner wall"));
    }
}
