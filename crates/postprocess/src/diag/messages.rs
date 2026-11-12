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
use std::sync::atomic::{AtomicU8, Ordering};

/// 文案语言（2026-10-25「全站 i18n」那一刀）。
///
/// 今天两种：中文（源语言）/ 英文。加语言时：① 这里加一档；② `Entry` 加
/// `text_xx` 字段；③ `pick_text` / `pick_slicer` 各加一条分支；④ 判据会逼着把
/// 每一条都补齐（见 `every_entry_has_every_language`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    /// 简体中文 —— **源语言**，也是"什么都没设置时"的那一档（与今天的行为一致）
    #[default]
    Zh,
    /// 繁體中文（台灣用字，2026-10-30 加）。正文由 `text_zh_tw` 给，
    /// 表里缺这一格时掉回中文（`pick_text` 的兜底链）。
    ZhTw,
    /// 日本語（2026-10-30 加）。正文由 `text_ja` 给。
    Ja,
    /// English
    En,
}

impl Lang {
    /// 线上 / CLI 的字面量 → 语言。宽松认（`zh` / `en-US` 这类系统语言直传也收）。
    ///
    /// ★ `zh-TW` / `zh-Hant`（含 `zh-Hant-TW` 这种）**必须排在 `zh-` 那条前面判** ——
    /// 否则会被笼统的"zh 开头 = 简体"吃掉（2026-10-30 加繁体时的第一次顺序错误）。
    pub fn parse(raw: &str) -> Option<Self> {
        let raw = raw.trim().to_ascii_lowercase();
        if raw == "zh-tw" || raw == "zh-hk" || raw == "zh-mo" || raw.starts_with("zh-hant") {
            Some(Self::ZhTw)
        } else if raw == "ja" || raw.starts_with("ja-") || raw.starts_with("ja_") {
            Some(Self::Ja)
        } else if raw == "zh" || raw.starts_with("zh-") || raw.starts_with("zh_") {
            Some(Self::Zh)
        } else if raw == "en" || raw.starts_with("en-") || raw.starts_with("en_") {
            Some(Self::En)
        } else {
            None
        }
    }

    /// 线上字符串（IPC / 日志共用同一个字面量）。
    pub fn wire(self) -> &'static str {
        match self {
            Self::Zh => "zh-CN",
            Self::ZhTw => "zh-TW",
            Self::Ja => "ja",
            Self::En => "en",
        }
    }
}

/// 当前语言的活值（0 = 中文，1 = 繁體，2 = 日文，3 = 英文）。
///
/// 为什么是**进程级**而不是每调用点传参：这张表的读者散在管线、钩子、CLI 三层，
/// 五十来个调用点。语言是"这一趟执行的口径"，不是每句话的属性 —— 传参会把
/// 每个签名都污染一遍，还会漏。
///
/// 写入方**只有两处**：`main.rs`（CLI `--lang`）与 `hook.rs`（按 AppState 的语言格）。
static CURRENT: AtomicU8 = AtomicU8::new(0);

/// 设这一趟的语言。幂等；并发安全（单字节原子）。
pub fn set_lang(lang: Lang) {
    CURRENT.store(
        match lang {
            Lang::Zh => 0,
            Lang::ZhTw => 1,
            Lang::Ja => 2,
            Lang::En => 3,
        },
        Ordering::Relaxed,
    );
}

/// 现在用的是哪种语言。
pub fn current() -> Lang {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Lang::ZhTw,
        2 => Lang::Ja,
        3 => Lang::En,
        _ => Lang::Zh,
    }
}

/// 「跟随系统」在**没有前端**的进程里怎么落地。
///
/// 顺序：
///   ① **macOS**：读用户偏好 `AppleLanguages` 的第一语言（GUI 拉起的子进程里
///      `LC_*`/`LANG` 常常是空的，环境变量那一套在 mac 上基本失灵 —— 这正是
///      2026-10-25 登记的局限，今天补上）。`defaults` 读不到 / 输出怪异就跳过；
///   ② 其余平台（macOS 读不到时也一样）：`LC_ALL` / `LC_MESSAGES` / `LANG`；
///   ③ 全都空 ⇒ **中文**（与"今天全是中文"一致，宁可选保守 —— 不会把中文用户翻成英文）。
pub fn resolve_system_lang() -> Lang {
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("defaults")
            .args(["read", "-g", "AppleLanguages"])
            .output()
        {
            /* 输出形如 `(\n    "zh-Hans-CN",\n    "en-US",\n)` —— 取第一对引号里的语言段 */
            let s = String::from_utf8_lossy(&out.stdout);
            if let Some(lang) = s.split('"').nth(1).and_then(Lang::parse) {
                return lang;
            }
        }
    }
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let Some(raw) = std::env::var_os(name) else {
            continue;
        };
        let raw = raw.to_string_lossy();
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        return Lang::parse(raw).unwrap_or(Lang::En);
    }
    Lang::Zh
}

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
    /// 英文正文（2026-10-25）。**每一条都要有** —— 用 `Option` 而不是必填，
    /// 是为了让"漏写"表现为**判据失败并指名哪一条**（`every_entry_has_every_language`），
    /// 而不是整张表解析 panic、所有测试连坐。
    /// 运行期缺了会掉回中文（宁可夹一句中文，也不要空着）。
    #[serde(default)]
    text_en: Option<String>,
    /// 繁體中文正文（2026-10-30）。同上：缺了掉回中文，判据会指名哪条没补。
    #[serde(default)]
    text_zh_tw: Option<String>,
    /// 日本語正文（2026-10-30）。同上：缺了掉回中文。
    #[serde(default)]
    text_ja: Option<String>,
    /// 给切片器那一行的口径；缺省 = `text`。
    #[serde(default)]
    slicer: Option<String>,
    /// 英文的切片器口径；缺省 = `text_en`。
    #[serde(default)]
    slicer_en: Option<String>,
    /// 繁體的切片器口径；缺省 = `text_zh_tw`。
    #[serde(default)]
    slicer_zh_tw: Option<String>,
    /// 日文的切片器口径；缺省 = `text_ja`。
    #[serde(default)]
    slicer_ja: Option<String>,
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
    /// 这条说法**改口吻之前的原句**（2026-11-09）。
    ///
    /// 老执行记录里存的是当年渲好的句子，读的时候靠它认回本条（`key_of_exact_text`）
    /// —— 认回了就按**现在的语言与现在的措辞**重说。不参与渲染，也不许拿它当兜底文案。
    ///
    /// 只收**没有占位符**的旧句：带 `{参数}` 的那种填过值就再也认不回来，
    /// 别为了它去写模糊匹配（AGENTS §8.1：认不出就是认不出）。
    #[serde(default)]
    legacy: Vec<String>,
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
    match current() {
        Lang::Zh => format!("［未登记的文案 {key}］"),
        Lang::ZhTw => format!("［未登記的文案 {key}］"),
        Lang::Ja => format!("［未登録の文面 {key}］"),
        Lang::En => format!("[missing message {key}]"),
    }
}

/// 这一条在**这一种语言**下的正文（缺了掉回中文 —— 宁可夹一句中文，也不要空着）。
fn pick_text(lang: Lang, entry: &Entry) -> &str {
    match lang {
        Lang::Zh => &entry.text,
        Lang::ZhTw => entry.text_zh_tw.as_deref().unwrap_or(&entry.text),
        Lang::Ja => entry.text_ja.as_deref().unwrap_or(&entry.text),
        Lang::En => entry.text_en.as_deref().unwrap_or(&entry.text),
    }
}

/// 这一条在**这一种语言**下的切片器口径。
///
/// 缺省链：`slicer_xx` → `text_xx` → 中文 `text`（最后那一步只为"永远有话说"）。
fn pick_slicer(lang: Lang, entry: &Entry) -> &str {
    match lang {
        Lang::Zh => entry.slicer.as_deref().unwrap_or(&entry.text),
        Lang::ZhTw => entry
            .slicer_zh_tw
            .as_deref()
            .or(entry.text_zh_tw.as_deref())
            .unwrap_or(&entry.text),
        Lang::Ja => entry
            .slicer_ja
            .as_deref()
            .or(entry.text_ja.as_deref())
            .unwrap_or(&entry.text),
        Lang::En => entry
            .slicer_en
            .as_deref()
            .or(entry.text_en.as_deref())
            .unwrap_or(&entry.text),
    }
}

/// `text` 字段：本程序里给用户看的那一句（界面模态框、执行记录、日志）。
///
/// 读**当前语言**（[`set_lang`]）。调用点不用改、也不该自己传语言。
pub fn text(key: &str, params: &[(&str, &str)]) -> String {
    text_in(current(), key, params)
}

/// 显式指定语言的 `text`。给 CLI / 判据 / 跨语言对比用。
pub fn text_in(lang: Lang, key: &str, params: &[(&str, &str)]) -> String {
    match table().get(key) {
        Some(entry) => render(pick_text(lang, entry), params),
        None => missing_key(key),
    }
}

/// 一条**指代**：表里的键 + 参数。句子本身不在这里 —— 谁读谁按自己的语言渲。
///
/// # 为什么不是 `String`
///
/// 执行记录（`_meta.json`）要活过语言切换。存渲好的句子 = 把写入那一刻的语言冻进去：
/// 钩子是切片器拉起来的，默认简体，于是日文 / 英文界面上翻旧记录还是中文。
/// `error` 那一格 2026-10-31 就是这么修的（见 `src-tauri/src/ipc/report.rs` 的
/// `localized_error`），警告是同一类事，只是当时没跟着改。
///
/// # 参数可以是另一条句子
///
/// `postproc.range_violation` 的 `{source}` 本身就是表里的条目 —— 存成字面值
/// 会把渲染那一刻的语言一起冻住。所以参数是 [`Part`]：要么字面值，要么另一条
/// [`MessageRef`]；渲染时**同一种语言**一路解到底。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MessageRef {
    /// 表里的键（`postproc.scarf_joint_seam` 这种）。
    pub key: String,
    /// 占位符 → 值。空的不写进 JSON（多数警告没有参数）。
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub args: BTreeMap<String, Part>,
}

/// [`MessageRef`] 的一个参数值。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum Part {
    /// 字面值：数、方向记号、文件名……
    Text(String),
    /// 另一条表里的句子：语言跟着外层走。
    Msg(MessageRef),
}

impl MessageRef {
    /// 挑一条键 + 几条**字面**参数（最常用的一档）。
    pub fn new(key: &str, args: &[(&str, &str)]) -> Self {
        Self {
            key: key.to_owned(),
            args: args
                .iter()
                .map(|(k, v)| ((*k).to_owned(), Part::Text((*v).to_owned())))
                .collect(),
        }
    }

    /// 给一个参数，值本身是表里的句子（`{source}` / `{limit}` 那种）。
    pub fn with_msg(mut self, name: &str, msg: MessageRef) -> Self {
        self.args.insert(name.to_owned(), Part::Msg(msg));
        self
    }

    /// 按**当前语言**渲这一句。
    pub fn text(&self) -> String {
        self.text_in(current())
    }

    /// 按指定语言渲这一句（归档读口用：记录里存的是键，说的时候按现在的语言）。
    pub fn text_in(&self, lang: Lang) -> String {
        let pairs: Vec<(String, String)> = self
            .args
            .iter()
            .map(|(k, v)| {
                let value = match v {
                    Part::Text(t) => t.clone(),
                    Part::Msg(m) => m.text_in(lang),
                };
                (k.clone(), value)
            })
            .collect();
        let refs: Vec<(&str, &str)> = pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        text_in(lang, &self.key, &refs)
    }
}

/// `slicer` 字段：给**切片器**那一行的口径（缺省与 [`text`] 同一句）。
///
/// 「让切片器少看 / 别看 / 换个说法」= 在表里给这条加 / 改 `slicer`。
pub fn slicer(key: &str, params: &[(&str, &str)]) -> String {
    slicer_in(current(), key, params)
}

/// 显式指定语言的 `slicer`。
pub fn slicer_in(lang: Lang, key: &str, params: &[(&str, &str)]) -> String {
    match table().get(key) {
        Some(entry) => render(pick_slicer(lang, entry), params),
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

/// 按**原文**反查键（2026-10-27）。
///
/// 老归档只存了当年那句话（没有键）—— 报告页要按当前语言重说一遍，就得先认出
/// 它是表里哪一条。比的是四格正文 + `legacy` 那张旧句清单的**整句相等**
/// （四本字典都算：老记录可能是在日文 / 英文界面下写的）；带占位符的句子（值已经
/// 填进去了）对不上，那就对不上，不猜、不模糊匹配。
pub fn key_of_exact_text(s: &str) -> Option<&'static str> {
    let want = s.trim();
    if want.is_empty() {
        return None;
    }
    table()
        .iter()
        .find(|(_, e)| {
            e.text.trim() == want
                || e.text_en.as_deref().is_some_and(|t| t.trim() == want)
                || e.text_zh_tw.as_deref().is_some_and(|t| t.trim() == want)
                || e.text_ja.as_deref().is_some_and(|t| t.trim() == want)
                || e.legacy.iter().any(|t| t.trim() == want)
        })
        .map(|(k, _)| k.as_str())
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
            /* 空串与"没写"一样坏：都会让英文界面掉回中文，判据要抓得住 */
            if let Some(en) = &entry.text_en {
                assert!(
                    !en.trim().is_empty(),
                    "{key} 的 text_en 是空的（漏翻就整条省掉，别留空串）"
                );
            }
        }
    }

    /// **每一条都必须有英文、繁體和日文**（2026-10-25 英文 / 2026-10-30 繁體與日文）。
    ///
    /// 这条红了 = 有一条会在那种语言的界面 / 切片器框里掉回中文。
    /// 加新条目时忘了补，就是这里先红，而不是用户先看见。
    #[test]
    fn every_entry_has_every_language() {
        let mut missing_en = Vec::new();
        let mut missing_tw = Vec::new();
        let mut missing_ja = Vec::new();
        for (key, entry) in table() {
            if !entry
                .text_en
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty())
            {
                missing_en.push(key.as_str());
            }
            if !entry
                .text_zh_tw
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty())
            {
                missing_tw.push(key.as_str());
            }
            if !entry
                .text_ja
                .as_deref()
                .is_some_and(|s| !s.trim().is_empty())
            {
                missing_ja.push(key.as_str());
            }
        }
        assert!(
            missing_en.is_empty(),
            "这些条目没有英文（text_en）：{missing_en:?}"
        );
        assert!(
            missing_tw.is_empty(),
            "这些条目没有繁體（text_zh_tw）：{missing_tw:?}"
        );
        assert!(
            missing_ja.is_empty(),
            "这些条目没有日文（text_ja）：{missing_ja:?}"
        );
    }

    /// 按原文反查键：四本字典的任何一句都能认回来；带填值的句子认不出（返回 `None`）。
    #[test]
    fn exact_text_finds_its_key() {
        let key = "hook.tower_relocate_declined";
        for lang in [Lang::Zh, Lang::ZhTw, Lang::En, Lang::Ja] {
            let said = text_in(lang, key, &[]);
            assert_eq!(key_of_exact_text(&said), Some(key), "认不回 {lang:?}");
        }
        /* 带填值的句子：表里是 `{id}`，盘上是填好的 —— 整句不等，认不出 */
        let filled = text_in(
            Lang::Zh,
            "E_FS_NOT_FOUND_001",
            &[("target", "x"), ("detail", "d")],
        );
        assert_eq!(key_of_exact_text(&filled), None);
        assert_eq!(key_of_exact_text("  根本不存在的句子  "), None);
        assert_eq!(key_of_exact_text(""), None);
    }

    /// **改过口吻**的那条：老记录里那句原话也要认回本条 —— 报告页据此按现在的语言
    /// 重说一遍，于是"后处理那一屏"与"执行记录"说的是同一句。
    ///
    /// 作者 2026-11-09：「他们两个警告最好用一样的吧，就是说没必要单独分开写了」。
    #[test]
    fn a_legacy_wording_finds_its_key() {
        /* 2026-11-08 那几次实验构建写进执行记录的原话（连 markdown / G-code 记号一起） */
        let olds = [
            "成品把 `; has_scarf_joint_seam` 置成了 0：Bambu Studio 只在这一格为 1 时按 `; CHANGE_LAYER` 定层，那种层表会让预览把模型与擦料塔的上半截画不出来。置 0 后它按 Z 平台段数层 —— 层数会比模型层数多（斜接缝的 Z 斜坡被逐段数开），但每一段都对得上实际几何。",
            "这个件开了斜拼接缝。Bambu Studio 会在「切片完成」的那一屏沿用切片时的层表（它跑完后处理只更新走线、不更新层表），于是预览只画得出半截 —— 这是 Bambu 的缺陷，文件本身没问题。成品已把 `; has_scarf_joint_seam` 置 0，**重新打开（或拖入）这份 G-code** 就能看到完整图形；那一屏的层数会比模型层数多，因为斜接缝的 Z 斜坡被逐段数开了。",
        ];
        for old in olds {
            assert_eq!(
                key_of_exact_text(old),
                Some("postproc.scarf_joint_seam"),
                "旧句认不回来 ⇒ 报告页只能原样摆那句技术话"
            );
        }

        /* 重说出来的是**现在**那句：技术词与 G-code 记号都不许再出现 */
        let now = text_in(Lang::Zh, "postproc.scarf_joint_seam", &[]);
        for bad in ["层表", "has_scarf_joint_seam", "**", "`"] {
            assert!(!now.contains(bad), "新说法里还有「{bad}」：{now}");
        }
    }

    /// 一条指代：键 + 字面参数，按语言渲出来的句子与 `text_in` 一字不差。
    #[test]
    fn a_ref_renders_like_text() {
        let r = MessageRef::new("postproc.header_layer_declared", &[("layers", "627")]);
        for lang in [Lang::Zh, Lang::ZhTw, Lang::En, Lang::Ja] {
            assert_eq!(
                r.text_in(lang),
                text_in(lang, "postproc.header_layer_declared", &[("layers", "627")]),
            );
        }
    }

    /// 参数也可以是另一条句子：两层都按**同一种语言**解，外层不会夹内层的中文。
    #[test]
    fn a_ref_can_carry_another_ref() {
        let r = MessageRef::new(
            "postproc.range_violation",
            &[("dir", "X+"), ("actual", "256"), ("bound", "255")],
        )
        .with_msg(
            "source",
            MessageRef::new("postproc.range_check.first_pen", &[]),
        );

        /* 内层也得是日文：整句里要能找到内层那条**日文**原文（不是中文原文） */
        let ja = r.text_in(Lang::Ja);
        assert!(
            ja.contains(&text_in(Lang::Ja, "postproc.range_check.first_pen", &[])),
            "内层没跟着外层语言走：{ja}"
        );
        for lang in [Lang::Zh, Lang::ZhTw, Lang::En, Lang::Ja] {
            let said = r.text_in(lang);
            assert!(!said.contains('{'), "占位符没解干净（{lang:?}）：{said}");
        }
    }

    /// 警告文案不许露内部机制（层表 / 后处理 / G-code 注释名）：那是给工程师看的。
    ///
    /// 作者 2026-11-09：「这个警告的文案都太不贴切那种正常软件了」—— 这一条把
    /// **进 `warnings` 的那几句**钉在"用户看得见的东西"上说。
    #[test]
    fn warning_lines_do_not_leak_internals() {
        const BANNED: [(&str, &str); 6] = [
            ("postproc.scarf_joint_seam", "层表"),
            ("postproc.scarf_joint_seam", "后处理"),
            ("postproc.header_layer_declared", "total layer number"),
            ("postproc.header_layer_declared", "G-code"),
            ("postproc.range_check.first_pen", "工具头"),
            ("postproc.range_check.first_pen", "活化"),
        ];
        for (key, word) in BANNED {
            let zh = text_in(Lang::Zh, key, &[]);
            assert!(!zh.contains(word), "{key} 里还有内部说法「{word}」：{zh}");
        }
    }

    /// 指代能进 JSON 再出来（归档就是这条路径）：键与参数都还在。
    #[test]
    fn a_ref_survives_json() {
        let r = MessageRef::new("postproc.header_layer_declared", &[("layers", "627")]);
        let json = serde_json::to_string(&r).unwrap();
        let back: MessageRef = serde_json::from_str(&json).unwrap();
        assert_eq!(back, r);
        assert_eq!(back.text_in(Lang::En), r.text_in(Lang::En));

        /* 没有参数 ⇒ 不写 `args`（记录里那一条短得能一眼读完） */
        let plain = MessageRef::new("postproc.scarf_joint_seam", &[]);
        assert_eq!(
            serde_json::to_string(&plain).unwrap(),
            r#"{"key":"postproc.scarf_joint_seam"}"#
        );
    }

    /// 英文那几栏里**不许有中文**（分隔符也换过：`，` → `, `）。
    ///
    /// 漏翻的典型长相就是"英文表里夹着中文"，肉眼容易漏，判据不会。
    #[test]
    fn english_lines_have_no_chinese() {
        fn has_cjk(s: &str) -> bool {
            s.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
        }
        let mut bad = Vec::new();
        for (key, entry) in table() {
            for (what, value) in [
                ("text_en", entry.text_en.as_deref()),
                ("slicer_en", entry.slicer_en.as_deref()),
            ] {
                let Some(v) = value else { continue };
                if has_cjk(v) {
                    bad.push(format!("{key}.{what} = {v}"));
                }
            }
        }
        assert!(bad.is_empty(), "英文栏里还有中文：{bad:#?}");
    }

    /// 每个条目的占位符与 `params` **两边对齐**：用了的必须登记，登记的必须真用。
    ///
    /// 为什么值得一条判据：`params` 是给改表的人看的目录，漂了之后"这条要几个参数"就只能
    /// 靠读模板数 —— 而漏一个参数的表现是用户那边多出一截 `{detail}`，很难在自己机器上撞见。
    #[test]
    fn placeholders_and_declared_params_agree() {
        for (key, entry) in table() {
            let mut used: Vec<String> = Vec::new();
            /* 四种语言栏一起扫：英文模板漏一个占位符，用户在英文下就看不见那个数了 */
            for template in std::iter::once(&entry.text)
                .chain(entry.text_en.iter())
                .chain(entry.slicer.iter())
                .chain(entry.slicer_en.iter())
            {
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
        for lang in [Lang::Zh, Lang::En] {
            for (key, entry) in table() {
                let owned: Vec<(&str, &str)> =
                    entry.params.iter().map(|p| (p.as_str(), "×")).collect();
                let t = text_in(lang, key, &owned);
                let s = slicer_in(lang, key, &owned);
                assert!(!t.contains('{'), "{key} 的 {:?} text 渲染后还有 {t}", lang);
                assert!(!t.contains('}'), "{key} 的 {:?} text 渲染后还有 {t}", lang);
                assert!(
                    !s.contains('{'),
                    "{key} 的 {:?} slicer 渲染后还有 {s}",
                    lang
                );
                assert!(!t.contains("未登记的文案"), "{key} 取不到");
                assert!(!t.contains("missing message"), "{key} 取不到");
            }
        }
    }

    /// 语言是**进程级活值**：设了 English，后面所有 `text()` 都跟着换，
    /// 而且切回中文能复原（钩子一趟一个进程，但测试同一个进程里要能来回开）。
    #[test]
    fn the_language_switch_is_process_wide_and_reversible() {
        let before = current();
        set_lang(Lang::En);
        assert_eq!(current(), Lang::En);
        assert!(
            text("E_FS_NOT_FOUND_001", &[("target", "f"), ("detail", "d")])
                .to_ascii_lowercase()
                .contains("not found")
        );
        set_lang(Lang::Zh);
        assert_eq!(current(), Lang::Zh);
        assert!(text("E_FS_NOT_FOUND_001", &[("target", "f"), ("detail", "d")]).contains("找不到"));
        set_lang(before);
    }

    /// 没登记的键要**响亮**，不能返回空串（空串在界面上等于"这里本来没话"）。
    ///
    /// ★ 先把语言钉回中文：语言是**进程级活值**（`set_lang`），
    /// `the_language_switch_is_process_wide_and_reversible` 会在别的线程上把它切成英文 ——
    /// 并行跑时这条断言中文兜底句的用例就会偶发失败（本项目实测撞到过两次）。
    #[test]
    fn unknown_key_is_loud() {
        set_lang(Lang::Zh);
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
        /* 2026-10-17 改口径（作者：「就算信息不全，也能通过错误码找到对应的」）：
        用户自己按的停止**现在也给码**，只是换成 `W_` 打头 —— 一眼分得出
        "出故障"还是"你自己让它停的"。原来是 `None`，于是日志里那一行是 `code=""`。 */
        assert_eq!(
            code_of("hook.cancelled").as_deref(),
            Some("W_SYS_CANCELLED_001"),
            "主动停止也给码，但必须是 W_ 打头（不是故障）"
        );
        assert_eq!(code_of("没有这条"), None);
    }

    /// ★★★ **每一条 `hook.*` 都必须有码**，且**不许是 `E_` 打头以外的哑码**。
    ///
    /// 这条判据是给"AI 只看一行日志"用的（作者 2026-10-17）：日志里出现 `code=""`
    /// 就等于这句话在码表里不存在，搜不到、也定位不了。以前 `hook.cancelled` /
    /// `hook.mismatch_stopped` / `hook.calib_stopped` 就是这么漏的 ——
    /// 偏偏这几条是**最常出现**的那几条（用户自己按的停止）。
    ///
    /// 三档前缀：`E_` 故障 / `W_` 停下（含降级） / `N_` 成功 / `Q_` 提问。
    #[test]
    fn every_hook_line_carries_a_code_with_its_class() {
        const CLASSES: &[&str] = &["E_", "W_", "N_", "Q_"];
        let mut missing = Vec::new();
        let mut bad_class = Vec::new();
        for key in keys() {
            if !key.starts_with("hook.") {
                continue;
            }
            match code_of(key) {
                None => missing.push(key),
                Some(code) => {
                    if !CLASSES.iter().any(|p| code.starts_with(p)) {
                        bad_class.push((key, code));
                    }
                }
            }
        }
        assert!(
            missing.is_empty(),
            "这些 hook.* 没有码，日志里会是 code=\"\"：{missing:?}"
        );
        assert!(
            bad_class.is_empty(),
            "这些码不是 E_/W_/N_/Q_ 打头，AI 分不出类别：{bad_class:?}"
        );
    }

    /// 码不许重号 —— 两个不同的键共用一个码，靠码反查就查到两句话，
    /// "信息不全也能定位"这件事当场破产。
    ///
    /// 唯一允许的重复是**同一件事在两层各有一条**（内核那句 + 钩子那句），
    /// 前提是两条的 `text` 不一样（说法不同是常态）。这里把这条写成判据：
    /// 同码可以，同码又同句不行。
    #[test]
    fn codes_do_not_collide_silently() {
        use std::collections::HashMap;
        let mut by_code: HashMap<String, Vec<&str>> = HashMap::new();
        for key in keys() {
            if let Some(code) = code_of(key) {
                by_code.entry(code).or_default().push(key);
            }
        }
        let mut problems = Vec::new();
        for (code, ks) in by_code {
            if ks.len() < 2 {
                continue;
            }
            /* 同码的多条之间，文案必须真的不一样，否则就是纯重号 */
            for (i, a) in ks.iter().enumerate() {
                for b in &ks[i + 1..] {
                    if text(a, &[]) == text(b, &[]) {
                        problems.push((code.clone(), *a, *b));
                    }
                }
            }
        }
        problems.sort();
        assert!(
            problems.is_empty(),
            "同一个码下有两句一模一样的话（纯重号）：{problems:?}"
        );
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
