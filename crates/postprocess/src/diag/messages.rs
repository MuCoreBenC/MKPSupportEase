//! 文案总表 —— 用户读到的每一句错误 / 结论都从 `messages.toml`（crate 根）取。
//!
//! # 为什么要有它
//!
//! 同一件事有四个读者：**切片器**（它把钩子的 stderr 整段摆进自己的错误框，
//! macOS 实测，见 `docs/HOOK-ERRORS-AND-MESSAGES.md`）、本程序的**模态框**、
//! **执行记录**（`_meta.json`）、**日志**。以前这四处的句子是在四层代码里各拼各的 ——
//! 结果是同一件事四处不一样，改一次要 grep 五处，还出过
//! `处理失败 [E_CFG_INVALID_001]：配置无效: E_GCODE_BOUNDARY_001: …` 那种两层码糊在
//! 一句里的噪音（外层是变体名、里层才是真正的原因，而用户两边都读不懂）。
//!
//! 现在：码、人话、**给谁看**都在表里，调用点只给「键 + 参数」。
//!
//! # 编译进去，不是运行时读配置
//!
//! [`include_str!`] —— 表在编译那一刻就进了产物。于是没有「文件被删了 / 改坏了 / 读了
//! 一半」这些状态，也没有兜底分支要写；改文案 = 改这份 TOML + 重新编译。
//! 代价如实登记：用户**不能**自己改文案（那是产品决定，不是配置项）。
//!
//! # 缺什么会怎样
//!
//! - **表本身解析不了** ⇒ panic。它是编译进产物的常量，解析不过 = 这次构建是坏的；
//!   悄悄退化成空文案会让用户对着一个没有字的错误框，比崩掉更难查。判据见本文件测试。
//! - **键没登记** ⇒ 渲染出 `［未登记的文案 <键>］`，**一眼看得见**（不返回空串）：
//!   这句会出现在界面与切片器框里，谁看见谁报。
//! - **占位符没给值** ⇒ 原样留 `{名字}`。同样是"看得见的坏"，不是"悄悄少一句话"。

use std::collections::BTreeMap;
use std::sync::OnceLock;

/// 表本体（`crates/postprocess/messages.toml`）。
///
/// 相对路径是相对**本文件**：`src/diag/messages.rs` → `../../messages.toml`。
const TABLE: &str = include_str!("../../messages.toml");

/// 表里的一条。字段名与 TOML 一一对应，**多写一个字段就报错**（`deny_unknown_fields`）：
/// 表是给人手改的，`slicer` 敲成 `slice` 这种错必须当场红，而不是静静少一档口径。
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    /// 对外稳定错误码。**键本身就是码时可省**（`E_` 开头的键按码算）。
    #[serde(default)]
    code: Option<String>,
    /// 本程序里给用户看的那一句（界面、记录、日志）。
    text: String,
    /// 给切片器那一行的口径；缺省 = `text`。
    #[serde(default)]
    slicer: Option<String>,
    /// 本条用到的占位符名字（**只给判据看**，运行时零消费，见 crate 文档的字段说明）。
    ///
    /// `allow(dead_code)` 的理由：它跟 `note` 一样是**声明式**字段 —— 判据读它
    /// （`placeholders_and_declared_params_agree`），运行时不读。删掉它就没有
    /// "这条要几个参数"的目录了。
    #[serde(default)]
    #[allow(dead_code)]
    params: Vec<String>,
    /// 给改表的人看的说明。**刻意不消费**：它只为了让字段合法地存在于表里
    /// （`deny_unknown_fields` 之下，没这个字段的 `note` 会被判成敲错了字段名）。
    #[serde(default)]
    #[allow(dead_code)]
    note: Option<String>,
}

/// 解析一次、全局共用。
///
/// 解析失败就 panic：理由见模块头（表是编译进产物的常量，解析不了等于这次构建是坏的）。
/// 这条 panic 在**判据里**一定会先响（`the_table_parses`），进不到用户的机器上。
fn table() -> &'static BTreeMap<String, Entry> {
    static TABLE_ONCE: OnceLock<BTreeMap<String, Entry>> = OnceLock::new();
    TABLE_ONCE.get_or_init(|| {
        toml::from_str(TABLE).expect("messages.toml 解析不了 —— 这份表是编译进产物的，构建期就该红")
    })
}

/// 键没登记时长什么样。**刻意的丑**：它会出现在用户眼前的框里。
fn missing_key(key: &str) -> String {
    format!("［未登记的文案 {key}］")
}

/// `text` 字段：本程序里给用户看的那一句（界面模态框、执行记录、日志）。
pub fn text(key: &str, params: &[(&str, &str)]) -> String {
    match table().get(key) {
        Some(entry) => render(&entry.text, params),
        None => missing_key(key),
    }
}

/// `slicer` 字段：给**切片器**那一行的口径（缺省与 [`text`] 同一句）。
///
/// 「让切片器少看 / 别看 / 换个说法」= 在表里给这条加 / 改 `slicer`。
pub fn slicer(key: &str, params: &[(&str, &str)]) -> String {
    match table().get(key) {
        Some(entry) => render(entry.slicer.as_ref().unwrap_or(&entry.text), params),
        None => missing_key(key),
    }
}

/// 这一条在表里吗。判据用（生产路径不需要"先问再取"）。
pub fn has(key: &str) -> bool {
    table().contains_key(key)
}

/// 表里的全部键（判据用；顺序稳定，便于 diff）。
pub fn keys() -> Vec<&'static str> {
    table().keys().map(String::as_str).collect()
}

/// 这条键对应的**对外码**。
///
/// 规则两条，够用且不歧义：
/// 1. 条目自己声明了 `code` ⇒ 用它（`hook.*` 那几条走这条 —— 键是说法，码是契约）；
/// 2. 否则键本身就是码（`E_*` 开头）⇒ 用键；其余（`boundary.*` 那些零件）没有码。
pub fn code_of(key: &str) -> Option<String> {
    match table().get(key) {
        Some(entry) => entry
            .code
            .clone()
            .or_else(|| key.starts_with("E_").then(|| key.to_owned())),
        None => None,
    }
}

/// 把 `{名字}` 换成参数值。**只扫模板，不扫替换进去的值** —— 值里带花括号不会二次展开。
///
/// 找不到值的占位符**原样留着**（模块头第 3 条）。逐字符推进是因为模板里有中文，
/// 按字节切会切进一个字的中间。
fn render(template: &str, params: &[(&str, &str)]) -> String {
    let mut out = String::with_capacity(template.len() + 16);
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let name = &after[..close];
                match params.iter().find(|(k, _)| *k == name) {
                    Some((_, value)) => out.push_str(value),
                    /* 没给值：连同花括号一起留着，看得见 */
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            /* 光有 `{` 没有 `}`：当作普通字符 */
            None => {
                out.push_str(&rest[open..]);
                return out;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 表解析得动、每条都有正文。**这条红了说明这次构建是坏的**（模块头第 1 条）。
    #[test]
    fn the_table_parses() {
        let all = table();
        assert!(!all.is_empty(), "表是空的");
        for (key, entry) in all {
            assert!(!entry.text.trim().is_empty(), "{key} 的 text 是空的");
            if let Some(slicer) = &entry.slicer {
                assert!(
                    !slicer.trim().is_empty(),
                    "{key} 的 slicer 是空的（不写就整条省掉）"
                );
            }
        }
    }

    /// 每个条目的占位符与 `params` **两边对齐**：用了的必须登记，登记的必须真用。
    ///
    /// 为什么值得一条判据：`params` 是给改表的人看的目录，漂了之后"这条要几个参数"就只能
    /// 靠读模板数 —— 而漏一个参数的表现是用户那边多出一截 `{detail}`，很难在自己机器上撞见。
    #[test]
    fn placeholders_and_declared_params_agree() {
        for (key, entry) in table() {
            let mut used: Vec<String> = Vec::new();
            for template in std::iter::once(&entry.text).chain(entry.slicer.iter()) {
                for name in placeholders(template) {
                    if !used.contains(&name) {
                        used.push(name);
                    }
                }
            }
            for name in &used {
                assert!(
                    entry.params.contains(name),
                    "{key} 的模板里用了 {{{name}}}，但 params 没登记它"
                );
            }
            for name in &entry.params {
                assert!(
                    used.contains(name),
                    "{key} 的 params 登记了 {name}，但模板里没用到它"
                );
            }
        }
    }

    /// 表里的每一条都能**不带残留**地渲染出来（判据自检用的假值）。
    #[test]
    fn every_entry_renders_clean() {
        for (key, entry) in table() {
            let owned: Vec<(&str, &str)> = entry.params.iter().map(|p| (p.as_str(), "×")).collect();
            let t = text(key, &owned);
            let s = slicer(key, &owned);
            assert!(!t.contains('{'), "{key} 的 text 渲染后还有 {t}");
            assert!(!t.contains('}'), "{key} 的 text 渲染后还有 {t}");
            assert!(!s.contains('{'), "{key} 的 slicer 渲染后还有 {s}");
            assert!(!t.contains("未登记的文案"), "{key} 取不到");
        }
    }

    /// 没登记的键要**响亮**，不能返回空串（空串在界面上等于"这里本来没话"）。
    #[test]
    fn unknown_key_is_loud() {
        let got = text("E_THIS_KEY_DOES_NOT_EXIST_999", &[]);
        assert!(got.contains("未登记的文案"), "{got}");
        assert!(got.contains("E_THIS_KEY_DOES_NOT_EXIST_999"), "{got}");
        assert!(!has("E_THIS_KEY_DOES_NOT_EXIST_999"));
    }

    /// `slicer` 缺省 = `text`（这是"只改给切片器那一面"的支点）。
    #[test]
    fn slicer_defaults_to_text() {
        // `E_CFG_INVALID_001` 刻意没有 slicer 字段
        assert_eq!(
            slicer("E_CFG_INVALID_001", &[("detail", "x")]),
            text("E_CFG_INVALID_001", &[("detail", "x")])
        );
        // 有 slicer 的那条：两面**必须不一样**，否则这个字段就是白写的
        assert_ne!(
            slicer("E_TOML_PARSE_001", &[("detail", "x")]),
            text("E_TOML_PARSE_001", &[("detail", "x")])
        );
    }

    /// 码的取法：键本身就是码 / 条目自己声明码 / 零件条目没有码。
    #[test]
    fn code_of_answers_the_three_shapes() {
        assert_eq!(
            code_of("E_GCODE_BOUNDARY_001").as_deref(),
            Some("E_GCODE_BOUNDARY_001")
        );
        assert_eq!(
            code_of("hook.preset_missing").as_deref(),
            Some("E_FS_NOT_FOUND_001")
        );
        assert_eq!(code_of("boundary.clause"), None);
        assert_eq!(code_of("hook.cancelled"), None, "用户自己按的停止不给码");
        assert_eq!(code_of("没有这条"), None);
    }

    /// 值里的花括号不被二次展开（`render` 的实现承诺）。
    #[test]
    fn values_are_not_re_expanded() {
        let got = render("A{one}B", &[("one", "{two}"), ("two", "never")]);
        assert_eq!(got, "A{two}B");
    }

    /// 模板里的占位符名字（`{name}`；光有 `{` 不算）。
    fn placeholders(template: &str) -> Vec<String> {
        let mut found = Vec::new();
        let mut rest = template;
        while let Some(open) = rest.find('{') {
            let after = &rest[open + 1..];
            match after.find('}') {
                Some(close) => {
                    found.push(after[..close].to_owned());
                    rest = &after[close + 1..];
                }
                None => break,
            }
        }
        found
    }
}
