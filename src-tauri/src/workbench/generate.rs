//! 生成 MKP TOML。
//!
//! 六步，顺序不能换（doc §9.5）：
//! 1. 读当前有效开发配方
//! 2. 解析机型基底 + 版本覆盖
//! 3. **按客户端能力定义校验**每个支持期内的版本
//! 4. 渲染 TOML（只在内存里）
//! 5. 全部渲染成功 → 才开始写盘
//! 6. 写快照（记这次用的 hash）
//!
//! 第 3 步的结果决定 `minClientVersion`：取**支持它的那些版本里最低的那个**。
//! 一个都不支持 → 直接阻断生成，提示"先升级客户端并提供新的能力定义"。
//! 这就是 doc §11 那句"必须先升级客户端，再发布依赖该能力的资源"的机械判据。
//!
//! **诚实边界**：写盘的原子性是**文件级**的（走 `fsx::atomic`：临时文件 + rename），
//! 不是批次级的。一批里第 3 个文件写到一半断电，前两个是新的、后面是旧的 ——
//! 但每个文件自己都是完整的，不会出现半个 TOML。真正被这套顺序挡住的是另一类失败：
//! 校验与渲染全在写盘之前做完，所以"配方有问题"永远不会碰到旧产物。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Serialize;
use serde_json::Value;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::fsx::paths::resolve_in;
use crate::workbench::capability::{self, ClientCapability, Violation};
use crate::workbench::catalog;
use crate::workbench::clock;
use crate::workbench::model::{Registry, ValueType};
use crate::workbench::resolve;
use crate::workbench::store::{GenFailure, Snapshot, Store};

/// 一次成功生成的结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GenOutcome {
    pub machine_id: String,
    pub version_id: String,
    pub preset_id: String,
    /// 相对发布目录
    pub output_rel: String,
    pub sha256: String,
    pub size: u64,
    pub min_client_version: String,
    /// 支持期内**不**支持这份产物的客户端版本，以及原因。
    /// 非阻断 —— 它已经体现在 minClientVersion 上了，但要让人看见
    pub unsupported: Vec<Violation>,
    pub generated_at: String,
    /// 产物字节没变。**"没变"很重要**：说明这次生成没有制造出一个
    /// 会让用户端莫名要更新的新哈希
    pub unchanged: bool,
}

/// 单个版本。
///
/// `dist_root` 显式传进来而不是在函数里查：单测要往临时目录写，而"改进程环境变量
/// 再查一次"在并行测试下会互相踩（同一个进程里所有测试共享 env）。
/// 参数多一个，换来的是这个函数在任何目录上都能跑。
///
/// `caps` 由调用方一次读好传进来 —— 生成一批时不该把能力定义读 N 遍
pub fn generate_one(
    store: &Store,
    dist_root: &Path,
    machine_id: &str,
    version_id: &str,
    caps: &[ClientCapability],
) -> Result<GenOutcome, AppError> {
    let reg = store.registry()?;
    let fb = store.fallback()?;
    let machine = store.machine(machine_id)?;
    let version = store.version(machine_id, version_id)?;
    let eff = resolve::resolve(&reg, &fb, &machine, &version);

    if eff.is_unconfigured() {
        return Err(AppError::invalid_argument(format!(
            "{machine_id}/{version_id} 还没写配方，没有东西可生成"
        ))
        .with_detail("这不是生成失败，是机型基底与版本覆盖都是空的"));
    }
    if !eff.unknown_keys.is_empty() {
        return Err(AppError::corrupted(format!(
            "{machine_id}/{version_id} 的覆盖里有字段定义里没有的 key：{}",
            eff.unknown_keys.join("、")
        )));
    }

    let values = eff.plain();

    /* 逐个支持期版本校验。分成两堆：支持的与不支持的 */
    let mut supporting = Vec::new();
    let mut unsupported = Vec::new();
    for cap in caps {
        let v = capability::check(cap, machine_id, &values);
        if v.is_empty() {
            supporting.push(cap.client_version.clone());
        } else {
            unsupported.extend(v);
        }
    }

    let min_client_version = capability::lowest(&supporting).ok_or_else(|| {
        AppError::invalid_argument(format!(
            "{machine_id}/{version_id} 在支持期内的每个客户端版本上都用不了"
        ))
        .with_detail(format!(
            "需要先升级客户端并提供新的能力定义。原因：{}",
            unsupported
                .iter()
                .map(|v| format!(
                    "[{}] {}{}",
                    v.client_version,
                    v.field_key.clone().unwrap_or_default(),
                    if v.field_key.is_some() {
                        format!(" {}", v.reason)
                    } else {
                        v.reason.clone()
                    }
                ))
                .collect::<Vec<_>>()
                .join("；")
        ))
    })?;

    let preset_id = preset_id_of(store, machine_id, version_id)?;
    let output_rel = format!("presets/{preset_id}.toml");

    // 第 4 步：只在内存里渲染。渲染不出来就不该动盘上任何东西
    let text = render_toml(
        &reg,
        &preset_id,
        machine_id,
        version_id,
        &values,
        &min_client_version,
    );
    let bytes = text.into_bytes();
    let digest = sha256_bytes(&bytes);

    let target = resolve_in(dist_root, &output_rel)?;
    let unchanged = std::fs::read(&target)
        .map(|old| sha256_bytes(&old) == digest)
        .unwrap_or(false);

    // 第 5 步：写盘。unchanged 时刻意**不写** —— 写一遍字节一样但 mtime 变了，
    // 而 mtime 变化会让某些同步工具以为有更新
    if !unchanged {
        atomic_write(&target, &bytes)?;
    }

    let generated_at = clock::now_iso8601();

    // 第 6 步：快照。记的是"这次用的有效配方 hash"，状态判定就看它
    store.save_snapshot(
        machine_id,
        version_id,
        &Snapshot {
            recipe_hash: eff.hash.clone(),
            generated_at: generated_at.clone(),
            overrides: version.overrides.clone(),
            base: machine.base.clone(),
            output_rel: output_rel.clone(),
            output_sha256: digest.clone(),
            min_client_version: Some(min_client_version.clone()),
            last_failure: None,
        },
    )?;

    Ok(GenOutcome {
        machine_id: machine_id.to_owned(),
        version_id: version_id.to_owned(),
        preset_id,
        output_rel,
        size: bytes.len() as u64,
        sha256: digest,
        min_client_version,
        unsupported,
        generated_at,
        unchanged,
    })
}

/// 生成失败时**只记一笔，不碰产物**。
///
/// 快照里其余字段保持上一次成功的样子 —— 所以"上次成功生成时间"仍然是真的，
/// 而"当前产物对不对应当前配方"仍然由 recipe_hash 说话
pub fn record_failure(
    store: &Store,
    machine_id: &str,
    version_id: &str,
    reason: &str,
) -> Result<(), AppError> {
    let failure = GenFailure {
        at: clock::now_iso8601(),
        reason: reason.to_owned(),
    };
    match store.snapshot(machine_id, version_id)? {
        Some(mut s) => {
            s.last_failure = Some(failure);
            store.save_snapshot(machine_id, version_id, &s)
        }
        // 从没成功过：只留失败记录，别的字段留空
        None => store.save_snapshot(
            machine_id,
            version_id,
            &Snapshot {
                recipe_hash: String::new(),
                generated_at: String::new(),
                overrides: BTreeMap::new(),
                base: BTreeMap::new(),
                output_rel: String::new(),
                output_sha256: String::new(),
                min_client_version: None,
                last_failure: Some(failure),
            },
        ),
    }
}

/// 预设身份：菜单里有就用菜单里那个，没有就按 `机型-版本` 造一个。
///
/// **不用文件名**（doc §5）：换文件路径不该让客户端把它当成新预设
pub fn preset_id_of(store: &Store, machine_id: &str, version_id: &str) -> Result<String, AppError> {
    let cat = catalog::load_catalog(store)?;
    if let Some(p) = cat
        .presets
        .iter()
        .find(|p| p.machine == machine_id && p.version == version_id)
    {
        return Ok(p.preset_id.clone());
    }
    Ok(format!(
        "{}-{}",
        machine_id.to_lowercase(),
        version_id.to_lowercase()
    ))
}

fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/* ---------- TOML 渲染 ---------- */

/// 渲染成 MKP TOML。
///
/// 手写而不是用 toml crate 的理由有两个，都是这份产物的硬要求：
/// 1. **注释**要跟着字段走（`tomlComment`），序列化库不给这个
/// 2. **顺序必须稳定**，且由 `registry.json` 的 order 决定 —— 顺序一抖动，
///    产物 sha256 就变，用户端就会莫名要更新
fn render_toml(
    reg: &Registry,
    preset_id: &str,
    machine_id: &str,
    version_id: &str,
    values: &BTreeMap<String, Value>,
    min_client_version: &str,
) -> String {
    let mut out = String::new();
    out.push_str("# 由 SupportEase 后厨工作台生成。**不要手改这个文件** —— 改开发配方再重新生成。\n");
    out.push_str("# 开发配方在仓库的 workbench/ 下，这里只是烤出来的成品。\n\n");

    // 顶层身份信息。客户端靠 source 区分官方与用户副本，靠 preset_id 认身份
    out.push_str(&format!("source = {}\n", toml_str("official")));
    out.push_str(&format!("preset_id = {}\n", toml_str(preset_id)));
    out.push_str(&format!("machine = {}\n", toml_str(machine_id)));
    out.push_str(&format!("version = {}\n", toml_str(version_id)));
    out.push_str(&format!(
        "min_client_version = {}\n",
        toml_str(min_client_version)
    ));

    /* 分节。节的先后由该节里 order 最小的字段决定 —— 这样节序也是 registry 说了算，
    而不是 BTreeMap 的字母序（字母序会让 motion 排在 toolhead 前面，读起来别扭） */
    let mut fields: Vec<_> = reg
        .fields
        .iter()
        .filter(|f| values.contains_key(&f.key))
        .collect();
    fields.sort_by(|a, b| a.order.total_cmp(&b.order));

    let mut sections: Vec<&str> = Vec::new();
    for f in &fields {
        if !sections.contains(&f.section.as_str()) {
            sections.push(&f.section);
        }
    }

    for section in sections {
        out.push_str(&format!("\n[{section}]\n"));
        for f in fields.iter().filter(|f| f.section == section) {
            let Some(v) = values.get(&f.key) else { continue };
            if !f.desc.is_empty() {
                out.push_str(&format!("# {}\n", f.desc.replace('\n', " ")));
            }
            out.push_str(&format!("{} = {}\n", f.toml_key, toml_value(f.value_type, v)));
        }
    }

    out
}

fn toml_value(t: ValueType, v: &Value) -> String {
    match t {
        ValueType::Bool => v.as_bool().unwrap_or(false).to_string(),
        ValueType::Int => v
            .as_i64()
            .map(|n| n.to_string())
            .unwrap_or_else(|| "0".into()),
        ValueType::Float => match v.as_f64() {
            // 整数值的浮点也写成小数：TOML 里 1 是整数、1.0 是浮点，
            // 客户端按浮点解析时类型对不上会报错
            Some(n) if n.fract() == 0.0 => format!("{n:.1}"),
            Some(n) => n.to_string(),
            None => "0.0".into(),
        },
        ValueType::Text | ValueType::Gcode => toml_str(v.as_str().unwrap_or_default()),
    }
}

/// TOML 字符串。多行走 `"""`，单行走 `"`。
///
/// 反斜杠必须转义：TOML 的多行基本字符串里，行尾一个反斜杠是**续行符**，
/// 而 G-code 里出现反斜杠并不稀奇
fn toml_str(s: &str) -> String {
    let escaped = s.replace('\\', "\\\\").replace('"', "\\\"");
    if s.contains('\n') {
        // 开头那个换行按 TOML 规范会被丢掉，正好让内容从下一行开始，读起来整齐
        format!("\"\"\"\n{escaped}\"\"\"")
    } else {
        format!("\"{escaped}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::capability::{CapField, Support};
    use crate::workbench::model::{builtin_registry, BbsBinding, Machine, Version};
    use crate::workbench::store::empty_params;

    fn caps() -> Vec<ClientCapability> {
        vec![ClientCapability {
            client_version: "0.1.0".into(),
            catalog_schema_version: 1,
            machines: vec!["A1".into()],
            fields: builtin_registry()
                .fields
                .iter()
                .map(|f| CapField {
                    key: f.key.clone(),
                    value_type: f.value_type,
                    min: f.min,
                    max: f.max,
                    step: f.step,
                })
                .collect(),
        }]
    }

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, Store) {
        /* 发布目录用临时目录，而且是**参数传进去**的。
        刻意不改 `MKPSE_REPO_DIR` 环境变量：env 是整个进程共享的，
        cargo test 默认并行跑，一个测试改了它就会把别的测试指到错地方 ——
        那种失败看起来像随机偶发，查起来极贵 */
        let dist = tempfile::tempdir().unwrap();
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();

        let mut base = empty_params();
        base.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
        base.insert("motion.travel_speed".into(), serde_json::json!(300.0));
        base.insert(
            "toolhead.custom_mount_gcode".into(),
            serde_json::json!("G92 E0\nG1 E-5 F1800"),
        );
        base.insert("support.enable_brim".into(), serde_json::json!(true));
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base,
            default_bbs: vec![],
        })
        .unwrap();

        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A1".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        s.write_doc(
            "capability/support.json",
            &Support {
                supported: vec!["0.1.0".into()],
            },
        )
        .unwrap();

        (dist, d, s)
    }

    #[test]
    fn generates_and_records_snapshot() {
        let (_dist, _d, s) = setup();
        let out = generate_one(&s, _dist.path(), "A1", "std", &caps()).unwrap();

        assert_eq!(out.preset_id, "a1-std");
        assert_eq!(out.output_rel, "presets/a1-std.toml");
        assert_eq!(out.min_client_version, "0.1.0");
        assert!(!out.unchanged, "第一次生成不该被判成'没变'");

        let snap = s.snapshot("A1", "std").unwrap().unwrap();
        assert_eq!(snap.output_sha256, out.sha256);
        assert!(snap.last_failure.is_none());

        let text = std::fs::read_to_string(resolve_in(_dist.path(), &out.output_rel).unwrap()).unwrap();
        assert!(text.contains("source = \"official\""));
        assert!(text.contains("preset_id = \"a1-std\""));
        assert!(text.contains("min_client_version = \"0.1.0\""));
        assert!(text.contains("[toolhead]"));
        // 浮点写成带小数点的形式
        assert!(text.contains("z_offset = 0.1"));
        // 多行 G-code 走 """
        assert!(text.contains("custom_mount_gcode = \"\"\""));
    }

    /// 同一份配方生成两次：第二次必须判为"没变"，而且**不重写文件**。
    /// 这条直接对应 doc §16.4 —— 没改的产物字节不变，用户端不会莫名要更新
    #[test]
    fn regenerating_unchanged_recipe_is_byte_identical() {
        let (_dist, _d, s) = setup();
        let first = generate_one(&s, _dist.path(), "A1", "std", &caps()).unwrap();
        let path = resolve_in(_dist.path(), &first.output_rel).unwrap();
        let mtime1 = std::fs::metadata(&path).unwrap().modified().unwrap();

        let second = generate_one(&s, _dist.path(), "A1", "std", &caps()).unwrap();
        assert!(second.unchanged);
        assert_eq!(first.sha256, second.sha256);
        assert_eq!(
            std::fs::metadata(&path).unwrap().modified().unwrap(),
            mtime1,
            "字节没变却重写了文件，mtime 变化会让同步工具以为有更新"
        );
    }

    /// 未配置的版本：报的是"还没写配方"，不是"生成失败"
    #[test]
    fn unconfigured_version_is_refused_with_the_right_reason() {
        let (_dist, _d, s) = setup();
        s.save_machine(&Machine {
            id: "A2L".into(),
            display_name: "A2L".into(),
            base: empty_params(),
            default_bbs: vec![],
        })
        .unwrap();
        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A2L".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        let e = generate_one(&s, _dist.path(), "A2L", "std", &caps()).unwrap_err();
        assert!(e.message.contains("还没写配方"), "{}", e.message);
    }

    /// 支持期内没有任何客户端能吃下它 → 阻断，并说清要先升级客户端
    #[test]
    fn unsupported_everywhere_blocks_generation() {
        let (_dist, _d, s) = setup();
        // 能力定义里不含 A1 → 一个都不支持
        let narrow = vec![ClientCapability {
            client_version: "0.1.0".into(),
            catalog_schema_version: 1,
            machines: vec!["P1".into()],
            fields: vec![],
        }];
        let e = generate_one(&s, _dist.path(), "A1", "std", &narrow).unwrap_err();
        assert!(e.message.contains("用不了"), "{}", e.message);
        assert!(
            e.detail.unwrap_or_default().contains("升级客户端"),
            "要说清该怎么办"
        );
    }

    /// 部分支持：minClientVersion 取**支持它的那些版本里最低的**
    #[test]
    fn min_client_version_is_the_lowest_supporting_one() {
        let (_dist, _d, s) = setup();
        let mut old = caps()[0].clone();
        old.client_version = "0.0.9".into();
        old.machines = vec!["P1".into()]; // 老版本不支持 A1
        let mut newer = caps()[0].clone();
        newer.client_version = "0.2.0".into();

        let out = generate_one(&s, _dist.path(), "A1", "std", &[old, caps()[0].clone(), newer]).unwrap();
        assert_eq!(out.min_client_version, "0.1.0");
        assert!(!out.unsupported.is_empty(), "不支持的那个要被列出来");
    }

    /// 生成失败只记一笔：**旧产物与"上次成功时间"都不动**
    #[test]
    fn failure_does_not_touch_the_previous_output() {
        let (_dist, _d, s) = setup();
        let ok = generate_one(&s, _dist.path(), "A1", "std", &caps()).unwrap();
        let path = resolve_in(_dist.path(), &ok.output_rel).unwrap();
        let before = std::fs::read(&path).unwrap();

        record_failure(&s, "A1", "std", "磁盘满了").unwrap();

        let snap = s.snapshot("A1", "std").unwrap().unwrap();
        assert_eq!(snap.recipe_hash, {
            let reg = s.registry().unwrap();
            let fb = s.fallback().unwrap();
            resolve::resolve(
                &reg,
                &fb,
                &s.machine("A1").unwrap(),
                &s.version("A1", "std").unwrap(),
            )
            .hash
        });
        assert_eq!(snap.generated_at, ok.generated_at, "上次成功时间被改了");
        assert_eq!(snap.last_failure.unwrap().reason, "磁盘满了");
        assert_eq!(std::fs::read(&path).unwrap(), before, "旧产物被动了");
    }

    /// 从没成功过就失败：也要留下记录，且不伪造 recipe_hash
    #[test]
    fn failure_without_prior_success_leaves_empty_hash() {
        let (_dist, _d, s) = setup();
        record_failure(&s, "A1", "std", "写不进去").unwrap();
        let snap = s.snapshot("A1", "std").unwrap().unwrap();
        assert!(snap.recipe_hash.is_empty());
        assert!(snap.last_failure.is_some());
    }

    /// 菜单里已经有 presetId 时用它，不按机型-版本另造一个
    #[test]
    fn preset_id_comes_from_catalog_when_present() {
        let (_dist, _d, s) = setup();
        catalog::save_catalog(
            &s,
            &catalog::Catalog {
                presets: vec![catalog::PresetEntry {
                    preset_id: "a1-fast-default".into(),
                    machine: "A1".into(),
                    version: "std".into(),
                    display_name: "随便".into(),
                    resource: "presets/whatever.toml".into(),
                    sha256: None,
                    size: None,
                    min_client_version: None,
                    standalone: true,
                }],
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(preset_id_of(&s, "A1", "std").unwrap(), "a1-fast-default");
    }

    /// 反斜杠与引号必须转义，否则产物不是合法 TOML
    #[test]
    fn strings_escape_backslash_and_quote() {
        assert_eq!(toml_str(r#"a\b"c"#), r#""a\\b\"c""#);
        let multi = toml_str("L1\\\nL2");
        assert!(multi.starts_with("\"\"\"\n"));
        assert!(multi.contains("\\\\"), "行尾反斜杠没转义会变成续行符");
    }
}
