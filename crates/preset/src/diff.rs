//! 参数差异：一个键在「本地副本 / 当初的基线 / 云端现在」三者之间的状态。
//!
//! 这块从原 `lineage.rs` 分出来（2026-11-04）：溯源记录（provenance）讲「我从哪来」，
//! 差异（diff）讲「这三份里哪些值不一样」，两件事不再挤在一个模块里。

/// 一个键在「本地副本 / 当初的基线 / 云端现在」三者之间的状态。
///
/// **为什么必须有基线这一档**：只比「本地」与「云端的」分不出
/// 「云端改了而我没动过这项」（可以放心采纳）与「两边都改了」（真冲突，只能人来选）。
/// 分不出的后果只有两种：要么把用户的值悄悄盖掉，要么每一项都得问一遍。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffState {
    /// 只有云端变了（我这一项还是当初拷来的值）⇒ 采纳是安全的，允许批量
    CloudOnly,
    /// 只有我改了（云端没动）⇒ 保持，什么都不用做
    LocalOnly,
    /// 两边都变了 ⇒ **真冲突**，只能逐项选，界面上不许给批量按钮
    BothChanged,
    /// 云端新增了这个键，本地副本里没有
    CloudAdded,
    /// 云端移除了这个键，本地副本里还留着
    CloudRemoved,
}

/// 一项差异。三个值都可能缺（键不在那一份里，或者基线不可用）。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDiff {
    /// `段.键`，与表单的 `path` 同形（`toolhead.offset.x`）
    pub path: String,
    pub local: Option<crate::write::EditValue>,
    /// 当初拷来时的值。基线不可用 ⇒ `None`，此时状态一律保守按 `BothChanged` 算
    pub base: Option<crate::write::EditValue>,
    pub cloud: Option<crate::write::EditValue>,
    pub state: DiffState,
}

/// 比差异。**纯计算**：三份文本进、一串差异出，不碰文件系统。
///
/// - `base_text`：建副本那一刻来源的全文。拿不到就传 `None`（老副本、手工拖进来的）——
///   那时任何不同都算 `BothChanged`：**多问一句，而不是丢用户的值**。
/// - 值相同的键不出现在结果里；「两边都改成了同一个值」也不算差异（没什么要决定的）。
/// - 键的增删按**键在不在文件里**判，不按值判：`0` 与「没有这一项」不是一回事
///   （注册表的零值豁免已经证明过这个坑）。
pub fn diff(
    local_text: &str,
    base_text: Option<&str>,
    cloud_text: &str,
) -> Result<Vec<FieldDiff>, postprocess::diag::PostprocError> {
    use std::collections::BTreeMap;

    type Row = (bool, Option<crate::write::EditValue>);
    fn index(text: &str) -> Result<BTreeMap<String, Row>, postprocess::diag::PostprocError> {
        Ok(crate::write::snapshot(text)?
            .into_iter()
            .map(|s| (format!("{}.{}", s.section, s.key), (s.present, s.value)))
            .collect())
    }

    let local = index(local_text)?;
    let cloud = index(cloud_text)?;
    let base = match base_text {
        Some(t) => Some(index(t)?),
        None => None,
    };

    let mut out = Vec::new();
    for (path, (local_present, local_value)) in &local {
        // 两侧的键集合都来自同一份 `EDITABLE_KEYS`，查不到只可能是那份清单变了
        let Some((cloud_present, cloud_value)) = cloud.get(path) else {
            continue;
        };
        let base_row = base.as_ref().and_then(|b| b.get(path));
        let state = match (local_present, cloud_present) {
            (false, true) => DiffState::CloudAdded,
            (true, false) => DiffState::CloudRemoved,
            (false, false) => continue, // 两边都没有这一项
            (true, true) => {
                if local_value == cloud_value {
                    continue;
                }
                match base_row {
                    Some((true, base_value)) if base_value == local_value => DiffState::CloudOnly,
                    Some((true, base_value)) if base_value == cloud_value => DiffState::LocalOnly,
                    _ => DiffState::BothChanged,
                }
            }
        };
        out.push(FieldDiff {
            path: path.clone(),
            local: local_value.clone(),
            base: base_row.and_then(|(present, v)| if *present { v.clone() } else { None }),
            cloud: cloud_value.clone(),
            state,
        });
    }
    Ok(out)
}
