//! **源 TOML 的机械化重排**（2026-11-03 作者拍板）。
//!
//! # 要解决什么
//!
//! `presets/registry/param_registry.toml` 是**手写**出来的，`DocumentMut` 只就地改
//! 被编辑的那一处、其余原样保留 —— 于是历代手写的偶然差异永远攒在那儿：
//!
//! | 实测 | 数字 |
//! |---|---|
//! | 75 个 `[[params]]` 里字段序的不同排列 | **13 种** |
//! | 变体键的引号写法 | 单引号 39 / 裸键 20 / 双引号 6 |
//!
//! 作者原话：「源 toml 需要像生成的 mkp 预设的 toml 一样有序的机械检查的」。
//!
//! # 唯一一条硬检查：**结构化数据完全一致**
//!
//! 允许变的：注释（全部丢掉）、引号风格、字段顺序、空行。
//! **不许变的**：丢参数、改值、丢变体、改数组顺序、改归属、丢未知字段。
//!
//! 实现上不靠"我写得仔细"，靠 [`crate::presetdata::registry`] 的**同一个**解析器：
//! 重排后的字节再读一遍，与原始数据逐字段比。比不过就不写（[`check_roundtrip`]）。
//!
//! # 为什么交给 `serde` 而不是手写模板
//!
//! 第一版手写了 `kv("key", …)` 一串，**当场漏掉 `layout` / `showWhen` / `choices` /
//! `mergeable`** —— 而这几个正是 [`super::registry::ParamDef`] 里有、我却没想起来写的。
//! 手写模板与结构体之间没有任何机制保证"不漂"，所以改成：
//!
//! - 字段序 = 结构体的**声明序**（`#[derive(Serialize)]` 天然如此）
//! - 有什么字段就序列化什么（换成 `Option` 也不会漏）
//!
//! 这与仓里「同一判据不许两个源」是同一条纪律。

use crate::error::AppError;

/// 重排一份 `param_registry.toml` 的字节。
///
/// 输入是**原文**，输出是**规范化之后**的全文。
pub fn normalize_registry(src: &str) -> Result<String, AppError> {
    let mut file = super::registry::RegistryFile::parse_str(src)?;

    /* --- 排序：数组顺序是数据，但"按 key / order 排"是这份数据的语义。
    重排**不改变任何一门读法**（读的人按 key 查、按 order 排），只是让每次
    渲染得到同一段字节。 --- */
    file.tabs.sort_by(|a, b| {
        a.order
            .partial_cmp(&b.order)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.id.cmp(&b.id))
    });
    for tab in &mut file.tabs {
        tab.sections.sort_by(|a, b| {
            a.order
                .partial_cmp(&b.order)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.id.cmp(&b.id))
        });
    }
    file.params.sort_by(|a, b| a.key.cmp(&b.key));

    /* --- 渲染 ---
    走 `serde_json::Value` 中转，再递归写成 TOML：**字段名与顺序完全由
    `Serialize` 决定**（`ParamDef` 有什么就写什么、声明序就是槽位）。
    手写模板那条路试过 —— 当场漏了 `layout` / `showWhen` / `choices` / `mergeable`。 */
    let mut out = String::with_capacity(src.len());

    if !file.updated.is_empty() {
        out.push_str(&format!("updated = {}\n\n", quote(&file.updated)));
    }

    for tab in &file.tabs {
        let v = serde_json::to_value(tab).map_err(json_err)?;
        render_head(&mut out, "[[tabs]]", &v, &["sections"])?;
        if let Some(items) = v.get("sections").and_then(|x| x.as_array()) {
            for sec in items {
                /* 空行隔开，否则一串 `[[tabs.sections]]` 糊成一片读不动 */
                out.push('\n');
                render_head(&mut out, "[[tabs.sections]]", sec, &[])?;
            }
        }
        out.push('\n');
    }

    for p in &file.params {
        let v = serde_json::to_value(p).map_err(json_err)?;
        render_head(
            &mut out,
            "[[params]]",
            &v,
            /* 这两张都是子表，各写在下面 */
            &["machineVariants", "follows"],
        )?;
        /* 子表：`[params.machineVariants]` —— 变体键一律双引号（作者：「都用引号也可以」） */
        if let Some(mv) = v.get("machineVariants").and_then(|x| x.as_object()) {
            if !mv.is_empty() {
                out.push_str("\n[params.machineVariants]\n");
                /* `BTreeMap` 已是字典序；这里再排一次是为了不依赖上游类型 */
                let mut keys: Vec<&String> = mv.keys().collect();
                keys.sort();
                for k in keys {
                    out.push_str(&format!("{} = {}\n", quote(k), scalar(&mv[k])));
                }
            }
        }
        /* 子表：`[params.follows]` —— **跟不跟基底**（2026-11-03）。
        只有 `false`（不跟随）会在这儿，跟随 = 没有这个键。
        ⚠ 重排必须认识它：不认就会在 `render_head` 里被当成"内联表"摊成一行，
        读回来还能读，但作者看不出它在说什么，而且第二次重排会再变一遍。 */
        if let Some(fw) = v.get("follows").and_then(|x| x.as_object()) {
            if !fw.is_empty() {
                out.push_str("\n[params.follows]\n");
                let mut keys: Vec<&String> = fw.keys().collect();
                keys.sort();
                for k in keys {
                    out.push_str(&format!("{} = {}\n", quote(k), scalar(&fw[k])));
                }
            }
        }
        out.push('\n');
    }

    Ok(out)
}

/// 写一个 `[[表头]]` 块：标量字段按 `Serialize` 的顺序写；`skip` 里的键跳过
/// （它们要么是子表、要么另写）。
fn render_head(
    out: &mut String,
    head: &str,
    v: &serde_json::Value,
    skip: &[&str],
) -> Result<(), AppError> {
    let Some(obj) = v.as_object() else {
        return Err(json_err_msg("<块不是对象>"));
    };
    out.push_str(head);
    out.push('\n');
    /* `serde_json::Map` 默认保序（不开 preserve_order 时是 BTreeMap，那就不保序了）——
    所以这里按**结构体声明序**重排：`serde` 序列化成 JSON 时就是那个顺序，
    而 `serde_json::Map` 用 `preserve_order` feature 才保序。*/
    for (k, val) in obj {
        if skip.contains(&k.as_str()) {
            continue;
        }
        /* ★ TOML 没有 null：`Option<T>` 的 `None` 必须**整行不写**。
        写成 `""` 的话，`Option<ShowWhen>` 那一格读回来会变成
        `invalid type: string "", expected struct ShowWhen` —— 这正是
        "数据看起来还在、其实已经读不通"的那种坏。 */
        if val.is_null() {
            continue;
        }
        /* ★ 这两个字段在源文件里是**逗号分隔的字符串**（`"A1,A1_MINI,P1S"`），
        不是数组。`presetdata` 那边靠 `deserialize_with = "comma_separated"`
        宽容地读两种写法，但 `crates/preset` 的 `machine_filter: String`
        **只认字符串** —— 按数组写出去，客户端一读就 `invalid type: sequence`。
        这正是"两套结构体、一个源文件"的坑，所以写回时必须还原成源格式。 */
        if k == "machineFilter" || k == "deprecatedMachines" {
            let text = val
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str())
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            if text.is_empty() {
                continue; /* 空 = 不写这一行（与源文件的缺省一致） */
            }
            out.push_str(&format!("{k} = {}\n", quote(&text)));
            continue;
        }
        if val.is_object() || val.is_array() {
            /* 嵌套对象（`layout`）也按同样规则摊成内联表 / 子表 */
            if val.is_object() {
                out.push_str(&format!("{k} = {}\n", inline_table(val)?));
                continue;
            }
            out.push_str(&format!("{k} = {}\n", array(val)));
            continue;
        }
        out.push_str(&format!("{k} = {}\n", scalar(val)));
    }
    Ok(())
}

/// 嵌套对象 → 内联表 `{ sectionId = "offset", order = 1.0 }`
fn inline_table(v: &serde_json::Value) -> Result<String, AppError> {
    let Some(obj) = v.as_object() else {
        return Err(json_err_msg("<不是对象>"));
    };
    let parts: Vec<String> = obj
        .iter()
        .filter(|(_, val)| !val.is_null())
        /* 键也要加引号：`P1S:LITE` 这种带冒号的在裸键语法里不合法，
        不加就是 `machineMinVariants = { P1S:LITE = -50.0 }` —— 读回来直接报错 */
        /* 递归：子表的值也可能是数组（`subfieldsOrder`）或再一层对象 */
        .map(|(k, val)| format!("{} = {}", quote(k), value_literal(val)))
        .collect();
    Ok(format!("{{ {} }}", parts.join(", ")))
}

/// 数组 → `[a, b]`；元素是对象时写成内联表（`choices` 就是这种）。
///
/// 不能一律走 `scalar()` —— 那会把 `{{\"label\":\"关闭\"}}` 整个序列化成**字符串**，
/// 读回来是 `invalid type: string, expected struct Choice`（真文件当场报的）。
fn array(v: &serde_json::Value) -> String {
    let Some(items) = v.as_array() else {
        return "[]".to_owned();
    };
    let parts: Vec<String> = items.iter().map(value_literal).collect();
    format!("[{}]", parts.join(", "))
}

/// 标量 → TOML 字面量（字符串一律双引号，与变体键同一套写法）
fn scalar(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => quote(s),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => "\"\"".to_owned(),
        serde_json::Value::Number(n) => n.to_string(),
        other => quote(&other.to_string()),
    }
}

/// TOML 字符串：一律双引号 + 转义。
///
/// 一律加引号的好处是不用去查"这个键在 TOML 裸键语法里合不合法"——
/// 那种判断本身就是漂移来源（作者：「没必要特殊，都用引号也可以」）。
fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// **任何** JSON 值 → TOML 字面量（对象→内联表，数组→`[...]`，其余→标量）。
///
/// 只有一个入口，因为手写分支迟早漏一种组合 —— 真文件里
/// `serialization.subfieldsOrder` 就是"对象里的数组"，第一版漏了它。
fn value_literal(v: &serde_json::Value) -> String {
    if v.is_object() {
        inline_table(v).unwrap_or_else(|_| scalar(v))
    } else if v.is_array() {
        array(v)
    } else {
        scalar(v)
    }
}

fn json_err(e: serde_json::Error) -> AppError {
    json_err_msg(&e.to_string())
}

fn json_err_msg(detail: &str) -> AppError {
    AppError::corrupted(postprocess::diag::messages::text(
        "ui.presetdata.registry.292",
        &[("v1", "<内存>")],
    ))
    .with_detail(detail.to_owned())
}

/// **无损检查**：重排后的字节再读一遍，与原始数据逐项比。
///
/// 返回 `ok = false` 时**不许写盘** —— 这是作者那句「通过无损检查后，才允许
/// 写回源文件」的实现。比的是**解析后的结构**，不是字节：注释、引号、空行、
/// 字段顺序本来就允许变。
pub fn check_roundtrip(src: &str, normalized: &str) -> Result<RoundtripReport, AppError> {
    let before = super::registry::RegistryFile::parse_str(src)?;
    let after = super::registry::RegistryFile::parse_str(normalized)?;

    let mut problems = Vec::new();

    if before.updated != after.updated {
        problems.push("`updated` 变了".to_owned());
    }

    /* 参数按 key 对齐（顺序允许变 —— 那正是重排的目的） */
    let mut by_key: std::collections::BTreeMap<&str, &super::registry::ParamDef> =
        std::collections::BTreeMap::new();
    for p in &before.params {
        by_key.insert(p.key.as_str(), p);
    }
    if before.params.len() != after.params.len() {
        problems.push(format!(
            "参数个数变了：{} → {}",
            before.params.len(),
            after.params.len()
        ));
    }
    for p in &after.params {
        match by_key.get(p.key.as_str()) {
            None => problems.push(format!("多出参数 `{}`", p.key)),
            Some(old) => {
                let a = serde_json::to_value(old).unwrap_or_default();
                let b = serde_json::to_value(p).unwrap_or_default();
                if a != b {
                    for field in diff_fields(&a, &b) {
                        problems.push(format!("`{}` 的 `{field}` 变了", p.key));
                    }
                }
            }
        }
    }
    for p in &before.params {
        if !after.params.iter().any(|q| q.key == p.key) {
            problems.push(format!("丢了参数 `{}`", p.key));
        }
    }

    /* 分组：按 id 对齐，sections 也按 id 对齐 */
    if before.tabs.len() != after.tabs.len() {
        problems.push(format!(
            "分组个数变了：{} → {}",
            before.tabs.len(),
            after.tabs.len()
        ));
    }
    for t in &before.tabs {
        match after.tabs.iter().find(|u| u.id == t.id) {
            None => problems.push(format!("丢了分组 `{}`", t.id)),
            Some(u) => {
                let a = serde_json::to_value(t).unwrap_or_default();
                let b = serde_json::to_value(u).unwrap_or_default();
                if a != b {
                    for field in diff_fields(&a, &b) {
                        problems.push(format!("分组 `{}` 的 `{field}` 变了", t.id));
                    }
                }
            }
        }
    }
    for u in &after.tabs {
        if !before.tabs.iter().any(|t| t.id == u.id) {
            problems.push(format!("多出分组 `{}`", u.id));
        }
    }

    Ok(RoundtripReport {
        ok: problems.is_empty(),
        problems,
        params: after.params.len(),
        tabs: after.tabs.len(),
    })
}

/// 两个 JSON 对象差在哪几个字段（报问题用）
fn diff_fields(a: &serde_json::Value, b: &serde_json::Value) -> Vec<String> {
    let (Some(a), Some(b)) = (a.as_object(), b.as_object()) else {
        return vec!["<整个对象>".to_owned()];
    };
    let mut out = Vec::new();
    for (k, v) in a {
        if b.get(k) != Some(v) {
            out.push(k.clone());
        }
    }
    for k in b.keys() {
        if !a.contains_key(k) {
            out.push(k.clone());
        }
    }
    out
}

/// 无损检查的结论
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RoundtripReport {
    /// 结构完全一致（可以写盘）
    pub ok: bool,
    /// 不一致的地方（`ok` 为真时为空）
    pub problems: Vec<String>,
    pub params: usize,
    pub tabs: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"updated = "2026-11-01 10:00:00"

[[tabs]]
id = 'toolhead'
label = '工具头'
labelEn = 'Toolhead'
labelJa = 'ツールヘッド'
labelTw = '工具頭'
icon = 'wrench'
order = 1

[[tabs.sections]]
id = 'offset'
label = '空间偏移'
labelEn = 'Offset'
labelJa = 'オフセット'
labelTw = '空間偏移'
description = ''
descriptionEn = ''
descriptionJa = ''
descriptionTw = ''
order = 1

[[params]]
key = 'toolhead.offset.z'
configKey = 'OFFSET_Z'
tomlKey = 'offset_z'
jsonKey = 'offset_z'
label = 'Z 轴偏移'
labelEn = 'Z offset'
labelJa = 'Z オフセット'
labelTw = 'Z 軸偏移'
desc = '使喷嘴与热床的距离'
descEn = ''
descJa = ''
descTw = ''
tomlComment = '笔尖偏移'
valueType = 'float'
uiComponent = 'number'
defaultValue = 4.0
scope = 'machine_specific'
section = 'offset'
layout = { sectionId = 'offset', order = 1.0 }
deprecatedMachines = []
min = -50.0
max = 50.0
step = 0.05
unit = 'mm'

[params.machineVariants]
"A1:FAST" = 3.9
A1 = 3.85
"P1S" = 1.1
"#;

    /// **无损**：重排不改变解析结果（注释/引号/顺序都可以变，数据不行）
    #[test]
    fn normalizing_keeps_the_data() {
        let out = normalize_registry(SAMPLE).expect("应当能读");
        let report = check_roundtrip(SAMPLE, &out).expect("两次都要能读");
        assert!(report.ok, "结构必须完全一致，实际有：{:?}", report.problems);
        assert_eq!(report.params, 1);
        assert_eq!(report.tabs, 1);
    }

    /// **幂等**：再规范化一次，字节一个都不许变。
    ///
    /// 作者那句「连续规范化两次，第二次必须没有任何 diff」——
    /// 这条不成立的话，每次保存都会改一批字节，diff 永远是红的。
    #[test]
    fn normalizing_twice_is_the_same_bytes() {
        let once = normalize_registry(SAMPLE).unwrap();
        let twice = normalize_registry(&once).unwrap();
        assert_eq!(once, twice, "第二次规范化必须一个字节都不改");
    }

    /// 变体键**统一双引号**（作者：「都用引号也可以」）
    #[test]
    fn variant_keys_are_all_double_quoted() {
        let out = normalize_registry(SAMPLE).unwrap();
        assert!(
            out.contains("\"A1:FAST\" = 3.9"),
            "带冒号的键要加引号：\n{out}"
        );
        assert!(
            out.contains("\"A1\" = 3.85"),
            "不带的也要加（统一）：\n{out}"
        );
    }

    /// 字段序 = `ParamDef` 的声明序（不是原文件里的偶然顺序）
    #[test]
    fn fields_follow_the_struct_declaration_order() {
        let out = normalize_registry(SAMPLE).unwrap();
        let key = out.find("key = \"toolhead.offset.z\"").unwrap();
        let cfg = out.find("configKey = \"OFFSET_Z\"").unwrap();
        let label = out.find("label = \"Z 轴偏移\"").unwrap();
        let vt = out.find("valueType = \"float\"").unwrap();
        let dv = out.find("defaultValue = 4.0").unwrap();
        let unit = out.find("unit = \"mm\"").unwrap();
        assert!(
            key < cfg && cfg < label && label < vt && vt < dv && dv < unit,
            "字段序不对：\n{out}"
        );
    }

    /// **`layout` / `machineVariants` 这类子表不许丢**（第一版手写模板就漏了）
    #[test]
    fn nested_tables_are_kept() {
        let out = normalize_registry(SAMPLE).unwrap();
        assert!(out.contains("sectionId"), "`layout` 不许丢：\n{out}");
        assert!(out.contains("\"A1:FAST\""), "变体表不许丢：\n{out}");
    }

    /// **改值要被抓出来**（`check_roundtrip` 是写盘的闸）
    #[test]
    fn roundtrip_catches_a_changed_value() {
        let out = normalize_registry(SAMPLE).unwrap();
        let changed = out.replace("\"A1:FAST\" = 3.9", "\"A1:FAST\" = 9.9");
        let report = check_roundtrip(SAMPLE, &changed).unwrap();
        assert!(!report.ok, "改了一个值必须被抓出来");
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.contains("machineVariants")),
            "要指出是变体表变了：{:?}",
            report.problems
        );
    }

    /// **丢参数要被抓出来**
    #[test]
    fn roundtrip_catches_a_dropped_param() {
        let out = normalize_registry(SAMPLE).unwrap();
        let broken = out.replace("toolhead.offset.z", "toolhead.other");
        let report = check_roundtrip(SAMPLE, &broken).unwrap();
        assert!(!report.ok, "换了 key 必须被抓出来");
        assert!(
            report
                .problems
                .iter()
                .any(|p| p.contains("丢") || p.contains("多出")),
            "要说清是丢还是多：{:?}",
            report.problems
        );
    }

    /// **真文件**：把仓库里那份 75 个参数的注册表跑一遍。
    ///
    /// 合成夹具只能证明"我想到的情况都对"；真文件里那些
    /// `showWhen` / `choices` / `parentKey` / 多行 G-code 才是检验。
    /// 这条同时验三件事：无损、幂等、不丢子表。
    #[test]
    fn the_real_registry_survives_normalization() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../presets/registry/param_registry.toml");
        let Ok(src) = std::fs::read_to_string(&path) else {
            return; // 换个 checkout 没这份数据：跳过，不当失败
        };

        let once = normalize_registry(&src).expect("真文件必须能规范化");
        let report = check_roundtrip(&src, &once).expect("规范化后的字节必须还能读");
        assert!(
            report.ok,
            "真文件的结构必须完全一致，实际有 {} 处不一致：{:#?}",
            report.problems.len(),
            report.problems
        );
        assert_eq!(report.params, 75, "参数个数");
        assert_eq!(report.tabs, 7, "分组个数");

        let twice = normalize_registry(&once).expect("第二遍");
        assert_eq!(once, twice, "第二遍必须一个字节都不改");
    }
}
