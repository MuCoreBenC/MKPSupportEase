//! 「仅有支撑面、无支撑体」对象的回退检测（对照 support_fallback.go）。
//!
//! 消费方：`gcodebiz/wiping_decision.go`（DecideWiping 的输入之一）。
//!
//! 与旧侧的等价性说明：Go 用 `map[string]*objectSupportInfo`，遍历序不确定，
//! 返回的 objIDs 顺序因而不稳定；Rust 侧用 BTreeMap 使顺序确定。
//! 布尔结果（消费方真正依赖的面）两侧一致。

use std::collections::BTreeMap;

#[derive(Default, Clone, Copy)]
struct ObjectSupportInfo {
    has_interface: bool,
    has_body: bool,
}

fn has_support_interface(content: &[String]) -> bool {
    content
        .iter()
        .any(|l| l.contains("FEATURE: Support interface"))
}

fn has_support_body(content: &[String]) -> bool {
    content.iter().any(|l| {
        let trimmed = l.trim();
        trimmed.contains("FEATURE: Support")
            && !trimmed.contains("FEATURE: Support interface")
            && !trimmed.contains("FEATURE: Support ironing")
    })
}

/// `DetectSupportFallback`：返回 (是否命中, 命中的对象 ID 列表)。
///
/// - 有对象标记（`OBJECT_ID:` / `unique label id:` / `stop printing object`）时：
///   逐对象统计「支撑面出现而支撑体从未出现」；
/// - 全程无对象标记时：退化为整文件级判定，ID 列表为空。
pub fn detect_support_fallback(content: &[String]) -> (bool, Vec<String>) {
    let mut objects: BTreeMap<String, ObjectSupportInfo> = BTreeMap::new();
    let mut current_object = String::new();
    let mut found_object_markers = false;

    for line in content {
        let trimmed = line.trim();

        if let Some(idx) = trimmed.find("OBJECT_ID:") {
            let id = trimmed[idx + "OBJECT_ID:".len()..].trim();
            if !id.is_empty() {
                current_object = id.to_string();
                found_object_markers = true;
            }
            continue;
        }

        if trimmed.contains("start printing object, unique label id:") {
            if let Some(idx) = trimmed.find("unique label id:") {
                let id = trimmed[idx + "unique label id:".len()..].trim();
                if !id.is_empty() {
                    current_object = id.to_string();
                    found_object_markers = true;
                }
            }
            continue;
        }

        if trimmed.contains("stop printing object") {
            current_object = String::new();
            continue;
        }

        if current_object.is_empty() || !trimmed.contains("FEATURE: Support") {
            continue;
        }

        let info = objects.entry(current_object.clone()).or_default();
        if trimmed.contains("FEATURE: Support interface") {
            info.has_interface = true;
        } else if !trimmed.contains("FEATURE: Support ironing") {
            info.has_body = true;
        }
    }

    if !found_object_markers {
        return (
            has_support_interface(content) && !has_support_body(content),
            Vec::new(),
        );
    }

    let fallback_objects: Vec<String> = objects
        .iter()
        .filter(|(_, info)| info.has_interface && !info.has_body)
        .map(|(id, _)| id.clone())
        .collect();

    (!fallback_objects.is_empty(), fallback_objects)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn per_object_fallback_detected() {
        let content = lines(&[
            "; OBJECT_ID: 100",
            "; FEATURE: Support interface",
            "; OBJECT_ID: 200",
            "; FEATURE: Support interface",
            "; FEATURE: Support body",
        ]);
        let (hit, ids) = detect_support_fallback(&content);
        assert!(hit);
        assert_eq!(ids, vec!["100"]);
    }

    #[test]
    fn stop_printing_resets_current_object() {
        let content = lines(&[
            "; OBJECT_ID: 100",
            "; stop printing object",
            "; FEATURE: Support interface", // current 为空 → 不归属任何对象
        ]);
        let (hit, ids) = detect_support_fallback(&content);
        assert!(!hit, "标记后未再归属对象");
        assert!(ids.is_empty());
    }

    #[test]
    fn no_markers_falls_back_to_whole_file() {
        let only_interface = lines(&["; FEATURE: Support interface"]);
        assert_eq!(detect_support_fallback(&only_interface), (true, vec![]));

        let with_body = lines(&["; FEATURE: Support interface", "; FEATURE: Support body"]);
        assert_eq!(detect_support_fallback(&with_body), (false, vec![]));
    }

    #[test]
    fn support_ironing_does_not_count_as_body() {
        // Support ironing 属熨烫族，不算支撑体——这正是回退误报防护的边界
        let content = lines(&[
            "; OBJECT_ID: 300",
            "; FEATURE: Support interface",
            "; FEATURE: Support ironing",
        ]);
        let (hit, ids) = detect_support_fallback(&content);
        assert!(hit);
        assert_eq!(ids, vec!["300"]);
    }
}
