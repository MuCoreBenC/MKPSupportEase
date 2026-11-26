//! **预设形态签名**与**兼容探测**。
//!
//! 结构签名（`runtime::structure`）回答的是「catalog JSON 的类型面变没变」，
//! 但客户端真正读的还有预设 TOML。这里把 TOML 的键集合与类型也变成机器可算的
//! 事实，交给同一张规则表登记最低客户端版本。
//!
//! ```text
//! TomlConfig 默认值（serde Serialize）
//!   → 排序后的「路径=类型」清单
//!   → presetSchemaSignature（16 hex）
//!
//! presetContractEpoch 单独记「语义代次」：
//!   字段含义 / 默认值 / 客户端消费方式变了，但 TOML 形态没变时，靠它显式 +1。
//! ```

use sha2::{Digest, Sha256};

use crate::model::TomlConfig;

/// TOML **形态**代次：字段类型、必填性这类机器能看出来的变化。
///
/// 加字段 / 删字段 / 改类型时，签名自己会变；这个 epoch 只在
/// 「signature 的算法本身变了」时 +1。
pub const PRESET_SCHEMA_EPOCH: u32 = 1;

/// TOML **语义**代次：形态没变，但值的意思或客户端消费方式变了。
///
/// 2026-11-26：涂胶速度/预热温度从预设移到客户端偏好 —— 新预设不再携带这两项，
/// 0.0.18 读新预设会静默取旧默认值。属于语义不兼容，所以从 0 记到 1。
pub const PRESET_CONTRACT_EPOCH: u32 = 1;

/// 当前构建读得懂的预设契约代次。
///
/// 与旧客户端无关：它只用于新客户端判断「这份预设是不是比我还新」。
pub fn preset_contract_epoch() -> u32 {
    PRESET_CONTRACT_EPOCH
}

/// 预设 TOML 的形态签名：键路径 + 类型（不含具体值）。
pub fn preset_schema_signature() -> String {
    schema_signature(PRESET_SCHEMA_EPOCH)
}

/// 兼容探测的结论。**只分类，不写盘、不改解析口径。**
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresetCompat {
    /// 当前解析器能完整读回
    Compatible,
    /// 多出了当前模型不认识的键（通常来自更新/更旧的版本）
    UnknownKey { key: String },
    /// 少了当前模型必需的键（容器级 default 下很少见，留作兼容面）
    MissingRequired { key: String },
    /// 键在，但类型对不上
    TypeMismatch { key: String },
    /// TOML 语法本身坏了
    Syntax,
    /// 缺 `# machine:` 头
    MissingMachine,
}

/// 读一份预设字节并给出兼容分类。
///
/// 解析口径仍是 [`crate::read::read_preset_from_bytes`]：`deny_unknown_fields` 不放宽，
/// 这里只是把失败原因摊成稳定的种类，供导入 / 本地列表 / 下载前判断。
pub fn probe_preset_compat(bytes: &[u8]) -> PresetCompat {
    let Ok(raw) = String::from_utf8(bytes.to_vec()) else {
        return PresetCompat::Syntax;
    };
    match crate::read::read_preset_from_bytes(raw) {
        Ok(_) => PresetCompat::Compatible,
        Err(err) => classify_error(&err),
    }
}

fn classify_error(err: &postprocess::diag::PostprocError) -> PresetCompat {
    if matches!(err, postprocess::diag::PostprocError::MissingMachine { .. }) {
        return PresetCompat::MissingMachine;
    }
    let message = err.to_string();
    if let Some(key) = field_name(&message, "unknown field `") {
        return PresetCompat::UnknownKey { key };
    }
    if let Some(key) = field_name(&message, "missing field `") {
        return PresetCompat::MissingRequired { key };
    }
    if message.contains("invalid type:") {
        let key = field_name(&message, "for key `").unwrap_or_default();
        return PresetCompat::TypeMismatch { key };
    }
    PresetCompat::Syntax
}

fn field_name(message: &str, marker: &str) -> Option<String> {
    let rest = message.split(marker).nth(1)?;
    let name = rest.split('`').next()?;
    (!name.is_empty()).then(|| name.to_owned())
}

fn schema_signature(epoch: u32) -> String {
    let value =
        serde_json::to_value(TomlConfig::default()).expect("默认预设必须能序列化成 JSON 值");
    let mut rows = Vec::new();
    walk("", &value, &mut rows);
    rows.sort();

    let mut input = format!("mkpse-preset-schema/epoch={epoch}\n");
    for row in rows {
        input.push_str(&row);
        input.push('\n');
    }
    let digest = Sha256::digest(input.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    hex[..16].to_owned()
}

fn walk(prefix: &str, value: &serde_json::Value, out: &mut Vec<String>) {
    match value {
        serde_json::Value::Object(table) => {
            let mut keys: Vec<&String> = table.keys().collect();
            keys.sort();
            for key in keys {
                let path = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                if let Some(child) = table.get(key) {
                    walk(&path, child, out);
                }
            }
        }
        serde_json::Value::Array(items) => {
            out.push(format!("{prefix}=array"));
            if let Some(first) = items.first() {
                walk(&format!("{prefix}[*]"), first, out);
            }
        }
        serde_json::Value::String(_) => out.push(format!("{prefix}=string")),
        serde_json::Value::Number(_) => out.push(format!("{prefix}=number")),
        serde_json::Value::Bool(_) => out.push(format!("{prefix}=bool")),
        serde_json::Value::Null => out.push(format!("{prefix}=null")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = "# machine: A1\n[toolhead]\n[wiping]\n";

    #[test]
    fn signature_is_stable_and_sixteen_hex() {
        let a = preset_schema_signature();
        let b = preset_schema_signature();
        assert_eq!(a, b);
        assert_eq!(a.len(), 16);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn schema_keys_match_the_editable_key_list() {
        let value = serde_json::to_value(TomlConfig::default()).expect("默认预设可序列化");
        let mut rows = Vec::new();
        walk("", &value, &mut rows);
        let got: std::collections::BTreeSet<String> = rows
            .iter()
            .map(|row| row.split('=').next().unwrap_or_default().to_owned())
            .filter(|path| !path.ends_with("[*]"))
            .collect();
        let expected: std::collections::BTreeSet<String> = crate::write::EDITABLE_KEYS
            .iter()
            .map(|(section, key)| format!("{section}.{key}"))
            .collect();
        assert!(
            expected.is_subset(&got),
            "可编辑面必须是签名键集合的子集：{:?}",
            expected.difference(&got).collect::<Vec<_>>()
        );
    }

    #[test]
    fn signature_changes_with_epoch() {
        assert_ne!(schema_signature(1), schema_signature(2));
    }

    #[test]
    fn probe_accepts_a_minimal_preset() {
        assert_eq!(
            probe_preset_compat(MINIMAL.as_bytes()),
            PresetCompat::Compatible
        );
    }

    #[test]
    fn probe_names_an_unknown_key() {
        let raw = "# machine: A1\n[toolhead]\nfuture_knob = 1\n";
        match probe_preset_compat(raw.as_bytes()) {
            PresetCompat::UnknownKey { key } => assert_eq!(key, "future_knob"),
            other => panic!("应报未知键，实际：{other:?}"),
        }
    }

    #[test]
    fn probe_distinguishes_missing_machine_and_syntax() {
        assert_eq!(
            probe_preset_compat(b"[toolhead]\n"),
            PresetCompat::MissingMachine
        );
        assert_eq!(
            probe_preset_compat(b"not [valid toml"),
            PresetCompat::Syntax
        );
    }

    #[test]
    fn probe_reports_a_type_mismatch() {
        let raw = "# machine: A1\n[toolhead]\nspeed_limit = \"fast\"\n";
        match probe_preset_compat(raw.as_bytes()) {
            PresetCompat::TypeMismatch { .. } => {}
            other => panic!("应报类型不匹配，实际：{other:?}"),
        }
    }
}
