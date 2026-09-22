//! 客户端能力定义 —— **只读输入**。
//!
//! 这是工作台里唯一一类"不是我产出、我也不能改"的数据（doc §3.2）：只有客户端知道
//! 它自己支持哪些机型、认得哪些字段、能解析哪个版本的目录格式。工作台读它，用来
//! 回答两个问题：
//!
//! 1. 这份配方能不能被某个客户端版本吃下去？
//! 2. 这份产物的 `minClientVersion` 该填几？
//!
//! **本模块没有任何写入函数。** 不是"有但不调"，是没有 —— 想写也没有路。
//!
//! 缺文件时一律报"无法校验兼容性"，**不当成通过**（doc §12）。否则兼容性就是靠运气：
//! 能力定义没放好的那次发布，什么都不会拦你。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::workbench::model::ValueType;
use crate::workbench::store::Store;

const SUPPORT_FILE: &str = "capability/support.json";

/// 客户端认得的一个字段。
///
/// 区间与步进**由客户端声明**，可能比 `registry.json` 更严：工作台允许的范围是
/// "这个参数物理上合理"，客户端的范围是"我这版代码处理得了"。两者不是一回事
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CapField {
    pub key: String,
    pub value_type: ValueType,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCapability {
    pub client_version: String,
    /// 这一版客户端能解析哪个版本的菜单格式
    pub catalog_schema_version: u32,
    pub machines: Vec<String>,
    pub fields: Vec<CapField>,
}

impl ClientCapability {
    pub fn field(&self, key: &str) -> Option<&CapField> {
        self.fields.iter().find(|f| f.key == key)
    }
}

/// 支持期内的客户端版本清单。出货检查按这几个校验
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Support {
    pub supported: Vec<String>,
}

/// 一条不兼容
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Violation {
    pub client_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field_key: Option<String>,
    pub reason: String,
}

/// 读支持期内的全部能力定义。
///
/// 三种情况都是错，而且都**不是**"通过"：
/// - `capability/` 下一个文件都没有
/// - `support.json` 缺失或为空
/// - `support.json` 点名的某个版本没有对应的能力定义文件
pub fn load_supported(store: &Store) -> Result<Vec<ClientCapability>, AppError> {
    let support: Support = store
        .read_doc(SUPPORT_FILE, "客户端支持期清单")?
        .ok_or_else(|| {
            AppError::not_found("没有 capability/support.json，无法校验兼容性").with_detail(
                "这份清单只能由客户端侧给出：工作台不知道哪些客户端版本还在支持期",
            )
        })?;

    if support.supported.is_empty() {
        return Err(AppError::invalid_argument(
            "capability/support.json 里一个客户端版本都没列，无法校验兼容性",
        ));
    }

    let mut out = Vec::new();
    for v in &support.supported {
        let rel = format!("capability/client-{v}.json");
        let cap: ClientCapability = store.read_doc(&rel, &format!("客户端 {v} 的能力定义"))?
            .ok_or_else(|| {
                AppError::not_found(format!("支持期里列了 {v}，但找不到 {rel}")).with_detail(
                    "工作台不会替它编一份能力定义 —— 那等于自己给自己发兼容性许可",
                )
            })?;
        if cap.client_version != *v {
            return Err(AppError::corrupted(format!(
                "{rel} 里写的版本是 {}，与文件名对不上",
                cap.client_version
            )));
        }
        out.push(cap);
    }
    Ok(out)
}

/// 某个客户端版本能不能吃下这份有效配方。
///
/// 只查"客户端认不认、范围对不对"，**不查业务对不对** —— 后者是配方本身的事。
pub fn check(
    cap: &ClientCapability,
    machine_id: &str,
    values: &BTreeMap<String, Value>,
) -> Vec<Violation> {
    let mut out = Vec::new();

    if !cap.machines.iter().any(|m| m == machine_id) {
        out.push(Violation {
            client_version: cap.client_version.clone(),
            field_key: None,
            reason: format!("这一版客户端不支持机型 {machine_id}"),
        });
    }

    for (key, value) in values {
        let Some(f) = cap.field(key) else {
            out.push(Violation {
                client_version: cap.client_version.clone(),
                field_key: Some(key.clone()),
                reason: "这一版客户端不认识这个字段".into(),
            });
            continue;
        };

        if !type_matches(f.value_type, value) {
            out.push(Violation {
                client_version: cap.client_version.clone(),
                field_key: Some(key.clone()),
                reason: format!("类型不对，客户端要的是 {:?}", f.value_type),
            });
            continue;
        }

        if let Some(n) = value.as_f64() {
            if let Some(min) = f.min {
                if n < min {
                    out.push(Violation {
                        client_version: cap.client_version.clone(),
                        field_key: Some(key.clone()),
                        reason: format!("{n} 低于客户端下限 {min}"),
                    });
                }
            }
            if let Some(max) = f.max {
                if n > max {
                    out.push(Violation {
                        client_version: cap.client_version.clone(),
                        field_key: Some(key.clone()),
                        reason: format!("{n} 高于客户端上限 {max}"),
                    });
                }
            }
            if let Some(step) = f.step {
                if step > 0.0 && !on_step(n, step) {
                    out.push(Violation {
                        client_version: cap.client_version.clone(),
                        field_key: Some(key.clone()),
                        reason: format!("{n} 不在 {step} 的步进上"),
                    });
                }
            }
        }
    }

    out
}

fn type_matches(t: ValueType, v: &Value) -> bool {
    match t {
        ValueType::Float => v.is_f64() || v.is_i64() || v.is_u64(),
        ValueType::Int => v.is_i64() || v.is_u64(),
        ValueType::Bool => v.is_boolean(),
        ValueType::Text | ValueType::Gcode => v.is_string(),
    }
}

/// 步进判定带容差。
///
/// 浮点的 0.1 步进上，0.3 除以 0.1 得 2.9999999999999996 —— 不带容差的话
/// 一个完全合法的值会被判成"不在步进上"。容差取步进的百万分之一：
/// 比浮点误差大几个数量级，比任何真实的"差半步"小得多
fn on_step(value: f64, step: f64) -> bool {
    let k = (value / step).round();
    (value - k * step).abs() <= step * 1e-6
}

/// 语义化版本比较。只认 `主.次.修订`，多余的段忽略、缺的段当 0。
///
/// 刻意不引 semver crate：这里要比的是自己发布的客户端版本号，
/// 预发布标签（`-beta.1`）那套规则本稿用不上，引进来反而要解释它怎么排序
pub fn cmp_version(a: &str, b: &str) -> std::cmp::Ordering {
    let parse = |s: &str| -> Vec<u64> {
        s.split('.')
            .take(3)
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
                    .parse()
                    .unwrap_or(0)
            })
            .collect()
    };
    let (x, y) = (parse(a), parse(b));
    for i in 0..3 {
        let l = x.get(i).copied().unwrap_or(0);
        let r = y.get(i).copied().unwrap_or(0);
        match l.cmp(&r) {
            std::cmp::Ordering::Equal => {}
            other => return other,
        }
    }
    std::cmp::Ordering::Equal
}

/// 在一组版本里取最小的那个
pub fn lowest(versions: &[String]) -> Option<String> {
    versions
        .iter()
        .min_by(|a, b| cmp_version(a, b))
        .map(|s| s.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::store::Store;

    fn cap() -> ClientCapability {
        ClientCapability {
            client_version: "0.2.0".into(),
            catalog_schema_version: 1,
            machines: vec!["A1".into()],
            fields: vec![
                CapField {
                    key: "toolhead.z_offset".into(),
                    value_type: ValueType::Float,
                    min: Some(-1.0),
                    max: Some(1.0),
                    step: Some(0.01),
                },
                CapField {
                    key: "support.enable_brim".into(),
                    value_type: ValueType::Bool,
                    min: None,
                    max: None,
                    step: None,
                },
            ],
        }
    }

    fn values(pairs: &[(&str, Value)]) -> BTreeMap<String, Value> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn accepts_values_within_declared_limits() {
        let v = check(
            &cap(),
            "A1",
            &values(&[
                ("toolhead.z_offset", serde_json::json!(0.15)),
                ("support.enable_brim", serde_json::json!(true)),
            ]),
        );
        assert!(v.is_empty(), "{v:?}");
    }

    #[test]
    fn reports_unknown_field() {
        let v = check(&cap(), "A1", &values(&[("toolhead.mustard", serde_json::json!(1))]));
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].field_key.as_deref(), Some("toolhead.mustard"));
    }

    #[test]
    fn reports_unsupported_machine() {
        let v = check(&cap(), "P1", &values(&[]));
        assert_eq!(v.len(), 1);
        assert!(v[0].field_key.is_none());
    }

    #[test]
    fn reports_out_of_range() {
        let v = check(&cap(), "A1", &values(&[("toolhead.z_offset", serde_json::json!(9.0))]));
        assert_eq!(v.len(), 1);
        assert!(v[0].reason.contains("上限"));
    }

    #[test]
    fn reports_wrong_type() {
        let v = check(
            &cap(),
            "A1",
            &values(&[("support.enable_brim", serde_json::json!("yes"))]),
        );
        assert_eq!(v.len(), 1);
        assert!(v[0].reason.contains("类型"));
    }

    /// 0.3 / 0.1 在浮点上不是整数。不带容差这条会误报，
    /// 而误报的后果是一个合法配方被判成不兼容、发布被拦住
    #[test]
    fn step_check_tolerates_float_error() {
        assert!(on_step(0.3, 0.1));
        assert!(on_step(-0.15, 0.01));
        assert!(!on_step(0.15, 0.1), "真正的半步要判出来");
    }

    /// 缺文件 = 无法校验，**必须是错误** —— 不能当通过
    #[test]
    fn missing_capability_is_an_error_not_a_pass() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        let e = load_supported(&s).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /// 支持期里列了某版本、却没给它的能力定义：同样是错
    #[test]
    fn support_pointing_at_missing_file_is_an_error() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        s.write_doc(
            SUPPORT_FILE,
            &Support {
                supported: vec!["9.9.9".into()],
            },
        )
        .unwrap();
        let e = load_supported(&s).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /// 文件名与文件里写的版本对不上 → CORRUPTED，不静默用文件里那个
    #[test]
    fn version_mismatch_between_name_and_content_is_corrupted() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        s.write_doc(
            SUPPORT_FILE,
            &Support {
                supported: vec!["0.1.0".into()],
            },
        )
        .unwrap();
        s.write_doc("capability/client-0.1.0.json", &cap()).unwrap(); // 里面写的是 0.2.0
        let e = load_supported(&s).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    #[test]
    fn loads_supported_capabilities() {
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();
        s.write_doc(
            SUPPORT_FILE,
            &Support {
                supported: vec!["0.2.0".into()],
            },
        )
        .unwrap();
        s.write_doc("capability/client-0.2.0.json", &cap()).unwrap();
        let caps = load_supported(&s).unwrap();
        assert_eq!(caps.len(), 1);
        assert_eq!(caps[0].client_version, "0.2.0");
    }

    #[test]
    fn version_compare_and_lowest() {
        use std::cmp::Ordering::*;
        assert_eq!(cmp_version("0.2.0", "0.10.0"), Less);
        assert_eq!(cmp_version("1.0", "1.0.0"), Equal);
        assert_eq!(cmp_version("2.0.0", "1.9.9"), Greater);
        assert_eq!(
            lowest(&["0.2.0".into(), "0.1.9".into(), "1.0.0".into()]),
            Some("0.1.9".into())
        );
        assert_eq!(lowest(&[]), None);
    }

    /// **本模块不提供写入路径。** 这条断言看的是源码：
    /// 哪天有人加了个 `save_capability`，它会立刻红
    #[test]
    fn module_has_no_write_functions() {
        let src = include_str!("capability.rs");
        for bad in ["write_doc(SUPPORT", "fn save_", "atomic_write"] {
            // 测试自己用到 write_doc 是为了造夹具，所以只查非测试区段
            let code = src.split("#[cfg(test)]").next().unwrap_or(src);
            assert!(
                !code.contains(bad),
                "能力定义是只读输入，非测试代码里不该出现 {bad}"
            );
        }
    }
}
