//! **溯源记录**（provenance）：用户预设 `[mkp_source]` 表的读 / 剥 / 追加。
//!
//! # 形状
//!
//! 记录住在文件**尾部**（不是头部）：头注释被 `# machine` / `# variant` /
//! `# generated_at` 占着，而履历会随版本数线性增长，放尾部才不会挤压头部可读性。
//!
//! ```toml
//! # ===== 来源记录（程序维护；用户编辑不会改动它） =====
//! [mkp_source]
//! schema = 1
//!
//! [mkp_source.origin]
//! kind = "download"
//! file = "mkp/presets/A1-fast.toml"
//! platform = "github"
//! repo = "owner/repo"
//! branch = "main"
//! at = "2026-10-31T00:00:00Z"
//! sha256 = "…"
//! catalog = "support-ease"
//! serial = 5
//! revision = "f09e…"
//!
//! [[mkp_source.history]]
//! at = "2026-10-31T00:00:00Z"
//! …
//! ```
//!
//! - `origin` 回答「我从哪来」；`history` 是**只追加**的履历，每接受 / 下载一版
//!   云端就多一条，旧条目的值一个字不改。
//! - 用户手动「复制」出来的新文件，`origin.kind = "duplicate"` 且带 `mother`；
//!   内部自动「拷贝」（下载 / 另存云端）写 `download`，不捏造母本。
//!
//! # 为什么用表而不是散注释
//!
//! `[mkp_source]` 是合法 TOML，读侧 `toml` 一次解析拿全；散注释要再写一套文本
//! 解析器，且字段一多就会漂。正文（`[toolhead]` / `[wiping]`）一个字节不动：
//! 本模块只做「剥尾部 → 尾部追加」，不碰中间。
//!
//! # 与读取面的关系
//!
//! `TomlConfig` 是 `deny_unknown_fields`，所以 `read_preset_from_bytes` 必须先
//! [`strip`] 再解析配置；否则 `[mkp_source]` 会被当成「这份预设比本程序新」。

use serde::Deserialize;
use sha2::{Digest, Sha256};

use crate::model::{SourceProvenance, SourceRef};

/// 全文 sha256（小写 hex）。判「云端变了没有」用它。
pub fn sha256_hex(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    h.finalize().iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// 从全文里读出溯源记录。没有 / 坏档 ⇒ `None`（当「不知道」，不阻断读预设）。
pub fn parse(content: &str) -> Option<SourceProvenance> {
    let block = extract_block(content)?;
    #[derive(Deserialize)]
    struct Wrapper {
        mkp_source: SourceProvenance,
    }
    toml::from_str::<Wrapper>(&block).ok().map(|w| w.mkp_source)
}

/// 把尾部 `[mkp_source…]` 整段剪掉，其余字节原样。
///
/// 判据：`strip(写出去的那一份) == 来源` 逐字节相同。因为记录是**紧贴正文末尾**
/// 追加的（见 [`append_block`]），剥掉记录后正文末尾会剩下一个孤立的换行 ——
/// 那正是追加时补上的那一行，所以一并收掉。
pub fn strip(content: &str) -> String {
    let Some((start, end)) = block_range(content) else {
        return content.to_owned();
    };
    let mut out = String::with_capacity(content.len() - (end - start));
    out.push_str(&content[..start]);
    out.push_str(&content[end..]);
    // 只收**我们自己加的那一个**换行，不动正文原本的口味
    if start > 0 {
        if out.ends_with("\r\n\r\n") {
            out.truncate(out.len() - 2);
        } else if out.ends_with('\n') && out[..out.len() - 1].ends_with('\n') {
            out.truncate(out.len() - 1);
        }
    }
    out
}

/// 把整段溯源记录换成 `prov`（先剥后追加）。正文一个字节不动。
pub fn write(content: &str, prov: &SourceProvenance) -> String {
    let body = strip(content);
    append_block(&body, prov)
}

/// 追加一版履历；已有记录时旧条目原样保留（值不改），没有记录就新建。
pub fn append_version(content: &str, version: SourceRef) -> String {
    let mut prov = parse(content).unwrap_or_default();
    prov.schema = prov.schema.max(1);
    prov.history.push(version);
    write(content, &prov)
}

/// 只写 `origin`、**不写履历**（2026-11-04 作者裁定）：
///
/// 「这份文件是从哪来的」与「我接受过云端哪几版」是两件事。另存 / 新建一份只在
/// 前者上有事实，履历只能由**用户主动把云端新版合并进这份文件**那一刻产生。
/// 这里若顺手塞一条履历，"下载 / 保存也算接受"就成真了。
pub fn write_origin_only(src_text: &str, origin: SourceRef) -> String {
    write(
        src_text,
        &SourceProvenance {
            schema: 1,
            origin: Some(origin),
            history: Vec::new(),
        },
    )
}

/// 内部自动**拷贝**（下载 / 另存云端）：写 `download` 来源 + 一条履历。
///
/// `origin` 与 `version` 通常同源（同一次下载），但分开传是为了让调用方
/// 能表达「来源身份」与「这一版履历」不同的情况。
pub fn copy_with_provenance(src_text: &str, mut origin: SourceRef, version: SourceRef) -> String {
    origin.kind.get_or_insert_with(|| "download".to_owned());
    let prov = SourceProvenance {
        schema: 1,
        origin: Some(origin),
        history: vec![version],
    };
    write(src_text, &prov)
}

/// 用户手动**复制**：新文件继承母本的云端身份与履历，`origin.kind = duplicate`
/// 且记下母本路径。只有这一条路写 `mother`。
pub fn duplicate_from_local(mother_text: &str, mother_path: &str) -> String {
    let mother = parse(mother_text).unwrap_or_default();
    let mut origin = mother
        .origin
        .clone()
        .or_else(|| mother.history.last().cloned())
        .unwrap_or_default();
    origin.kind = Some("duplicate".to_owned());
    origin.mother = Some(mother_path.to_owned());
    let prov = SourceProvenance {
        schema: 1,
        origin: Some(origin),
        history: mother.history,
    };
    write(mother_text, &prov)
}

/// 渲染一段记录（含结尾换行）。
pub fn render(prov: &SourceProvenance, nl: &str) -> String {
    let mut out = String::new();
    out.push_str("# ===== 来源记录（程序维护；用户编辑不会改动它） =====");
    out.push_str(nl);
    out.push_str("[mkp_source]");
    out.push_str(nl);
    out.push_str(&format!("schema = {}{nl}", prov.schema.max(1)));
    if let Some(origin) = &prov.origin {
        out.push_str(nl);
        out.push_str("[mkp_source.origin]");
        out.push_str(nl);
        render_ref(&mut out, origin, nl, true);
    }
    for v in &prov.history {
        out.push_str(nl);
        out.push_str("[[mkp_source.history]]");
        out.push_str(nl);
        render_ref(&mut out, v, nl, false);
    }
    out
}

fn render_ref(out: &mut String, r: &SourceRef, nl: &str, with_origin_fields: bool) {
    fn push(out: &mut String, nl: &str, key: &str, value: Option<&str>) {
        if let Some(v) = value.filter(|v| !v.is_empty()) {
            out.push_str(&format!("{key} = {}{nl}", toml_string(v)));
        }
    }
    if with_origin_fields {
        push(out, nl, "kind", r.kind.as_deref());
        push(out, nl, "mother", r.mother.as_deref());
    }
    push(out, nl, "file", r.file.as_deref());
    push(out, nl, "platform", r.platform.as_deref());
    push(out, nl, "repo", r.repo.as_deref());
    push(out, nl, "branch", r.branch.as_deref());
    push(out, nl, "at", r.at.as_deref());
    push(out, nl, "sha256", r.sha256.as_deref());
    push(out, nl, "catalog", r.catalog.as_deref());
    if let Some(serial) = r.serial {
        out.push_str(&format!("serial = {serial}{nl}"));
    }
    push(out, nl, "revision", r.revision.as_deref());
}

fn toml_string(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

fn append_block(body: &str, prov: &SourceProvenance) -> String {
    let nl = newline_of(body);
    /* 记录紧贴正文末尾，前面留**一个空行**做视觉分隔（这个空行由我们在追加时加上，
    `strip` 会把它原样收回来 —— 判据 `strip(写出) == 来源` 逐字节钉住这一点）。 */
    let mut out = String::with_capacity(body.len() + 512);
    out.push_str(body);
    if !out.is_empty() && !out.ends_with('\n') {
        out.push_str(nl);
    }
    if !out.is_empty() {
        out.push_str(nl);
    }
    out.push_str(&render(prov, nl));
    out
}

fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// 第一段 `[mkp_source…]` 从哪到哪（字节偏移，含该段自己的行）。
/// 记录从哪到哪。
///
/// **起点**：第一处 `[mkp_source]` 表头。后面的横幅 / 空行都是我们渲染的一部分，
/// 但它们可能在用户手改后消失，所以起点只认表头。
///
/// **终点**：`[mkp_source]` 之后**第一个顶层表头**（或文件尾）。中间不再挑挑拣拣 ——
/// 一旦按「只吃属于我们的行」挑，遇到用户手改过的片段就漏，剥不干净会在文件里
/// 攒出第二条记录（真踩过）。
fn block_range(content: &str) -> Option<(usize, usize)> {
    let lines: Vec<(usize, &str, &str)> = {
        let mut v = Vec::new();
        let mut off = 0usize;
        for (line, term) in lines_with_terminators(content) {
            v.push((off, line, term));
            off += line.len() + term.len();
        }
        v
    };
    let header = lines
        .iter()
        .position(|(_, line, _)| is_source_header(line.trim()))?;
    // 终点：表头之后第一个非本记录的表头
    let end_idx = lines
        .iter()
        .enumerate()
        .skip(header + 1)
        .find(|(_, (_, line, _))| {
            let t = line.trim();
            t.starts_with('[') && !is_source_header(t)
        })
        .map(|(i, _)| i)
        .unwrap_or(lines.len());

    /* 起点：从表头往回吃掉紧邻的空行与横幅注释 —— 那些都是我们渲染出来的，
    不连带剥掉就会每写一次多留一条横幅。碰到正文（非空、非横幅）立刻停。 */
    let mut start_idx = header;
    while start_idx > 0 {
        let prev = lines[start_idx - 1].1.trim();
        if prev.is_empty() || is_source_banner(prev) {
            start_idx -= 1;
        } else {
            break;
        }
    }
    let start = lines[start_idx].0;
    let end = if end_idx < lines.len() {
        lines[end_idx].0
    } else {
        content.len()
    };
    Some((start, end))
}

/// 记录开头那条横幅注释。
fn is_source_banner(t: &str) -> bool {
    t.starts_with("# ===== 来源记录")
}

fn extract_block(content: &str) -> Option<String> {
    let (start, end) = block_range(content)?;
    let text = &content[start..end];
    // 前面的横幅（`# ===== 来源记录…`）不是 TOML：解析时去掉它
    let body: String = text
        .lines()
        .filter(|l| !l.trim_start().starts_with("# ===== 来源记录"))
        .collect::<Vec<_>>()
        .join("\n");
    Some(body)
}

fn is_source_header(t: &str) -> bool {
    t == "[mkp_source]" || t.starts_with("[mkp_source.") || t.starts_with("[[mkp_source.")
}

/// 逐行切分，**保留每行的行尾**（`\r\n` / `\n` / 无）。
fn lines_with_terminators(text: &str) -> Vec<(&str, &str)> {
    let mut out = Vec::new();
    let mut rest = text;
    while !rest.is_empty() {
        match rest.find('\n') {
            Some(i) => {
                let (line, term) = if i > 0 && rest.as_bytes()[i - 1] == b'\r' {
                    (&rest[..i - 1], &rest[i - 1..=i])
                } else {
                    (&rest[..i], &rest[i..=i])
                };
                out.push((line, term));
                rest = &rest[i + 1..];
            }
            None => {
                out.push((rest, ""));
                break;
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "\
# machine: A1
# variant: standard

[toolhead]
offset_x = -1 # 笔尖
offset_z = 4
";

    fn origin() -> SourceRef {
        SourceRef {
            kind: Some("download".to_owned()),
            file: Some("mkp/presets/A1-fast.toml".to_owned()),
            platform: Some("github".to_owned()),
            repo: Some("owner/repo".to_owned()),
            branch: Some("main".to_owned()),
            at: Some("2026-10-31T00:00:00Z".to_owned()),
            sha256: Some("a".repeat(64)),
            catalog: Some("support-ease".to_owned()),
            serial: Some(5),
            revision: Some("f09e".to_owned()),
            ..SourceRef::default()
        }
    }

    #[test]
    fn copy_appends_a_tail_block_and_strip_restores_the_source_byte_for_byte() {
        let copy = copy_with_provenance(SRC, origin(), origin());
        assert_eq!(strip(&copy), SRC, "剥掉尾部记录必须与来源逐字节相同");
        assert!(copy.contains("[mkp_source.origin]"));
        assert!(copy.contains("[[mkp_source.history]]"));
        assert!(copy.ends_with('\n'));
    }

    #[test]
    fn history_appends_without_touching_old_entries() {
        let first = copy_with_provenance(SRC, origin(), origin());
        let second = append_version(
            &first,
            SourceRef {
                at: Some("2026-11-02T00:00:00Z".to_owned()),
                sha256: Some("b".repeat(64)),
                serial: Some(6),
                ..origin()
            },
        );
        // 第一条履历的三行原样还在（值不改）
        assert!(second.contains("at = \"2026-10-31T00:00:00Z\""));
        assert!(second.contains("sha256 = \""));
        assert!(second.contains("serial = 5"));
        assert!(second.contains("serial = 6"));
        assert_eq!(
            second.matches("[[mkp_source.history]]").count(),
            2,
            "追加一版应恰好两条履历"
        );
        assert_eq!(strip(&second), SRC);
    }

    /// 「另存」只写来源、不产生履历（2026-11-04 作者裁定）：
    /// 用户另存一份新文件 ≠ 接受了一版云端。
    #[test]
    fn origin_only_writes_no_history() {
        let only = write_origin_only(SRC, origin());
        let prov = parse(&only).expect("另存也要有来源记录");
        assert!(prov.origin.is_some(), "来源要写");
        assert!(
            prov.history.is_empty(),
            "另存不许凭空长出一条履历：{:?}",
            prov.history
        );
        assert_eq!(strip(&only), SRC, "正文仍与来源逐字节相同");
    }

    #[test]
    fn duplicate_records_the_mother_and_inherits_history() {
        let first = copy_with_provenance(SRC, origin(), origin());
        let dup = duplicate_from_local(&first, "presets-local/我的 A1.toml");
        let prov = parse(&dup).expect("复制件必须带记录");
        let origin = prov.origin.expect("必须有 origin");
        assert_eq!(origin.kind.as_deref(), Some("duplicate"));
        assert_eq!(
            origin.mother.as_deref(),
            Some("presets-local/我的 A1.toml"),
            "只有用户手动复制才写母本"
        );
        assert_eq!(prov.history.len(), 1, "履历继承母本");
        assert_eq!(strip(&dup), SRC, "正文仍与来源逐字节相同");
    }

    #[test]
    fn a_file_without_a_block_parses_as_none() {
        assert!(parse(SRC).is_none());
        assert_eq!(strip(SRC), SRC, "没有记录时 strip 是恒等");
    }

    #[test]
    fn crlf_is_preserved() {
        let crlf = SRC.replace('\n', "\r\n");
        let copy = copy_with_provenance(&crlf, origin(), origin());
        assert!(copy.contains("schema = 1\r\n"));
        assert_eq!(strip(&copy), crlf);
    }

    #[test]
    fn writing_twice_is_idempotent() {
        let once = copy_with_provenance(SRC, origin(), origin());
        let twice = write(&once, &parse(&once).unwrap());
        assert_eq!(once, twice, "同一份记录写两遍必须逐字节相同");
    }
}
