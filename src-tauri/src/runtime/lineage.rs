//! **血统三行**：用户自己那份（`presets-mine/…`）是从哪一份官方、哪一版拷出来改的。
//!
//! # 血统写在文件里，不写在 `run/`
//!
//! 这是作者 2026-10-02 定的原则（第七层）：
//!
//! > **凡是描述"这个 Preset 文件本身是什么"的信息，都随 TOML 文件携带；
//! > 凡是描述"这个程序现在怎么管理它"的信息，才属于 `run/`。**
//!
//! 所以血缘**不**另立一个 `run/preset-user-state.json`，而是照 [`make_copy`] 的既定形状
//! 写进文件**头注释块**：
//!
//! ```text
//! # release_time: 2026-08-19 01:38:13
//! # machine: A1
//! # variant: standard
//! # based_on: mkp/presets/A1-standard.toml      ← 从哪一份官方拷的（角色目录 + 文件名）
//! # based_on_release_time: 2026-08-19 01:38:13  ← 那时候官方那一版的版本号（有才写）
//! # based_on_sha256: 3f2a…                      ← 那时候官方那一版**全文**的摘要
//! [toolhead]
//! …
//! ```
//!
//! 头里**没有 uuid**：现役产物不写它（2026-10-05 删的死字段，见工作台 `app::build`
//! 模块文档）。旧年份的官方文件与基于它们的老副本头里可能还带着那一行 ——
//! 解析只认 `based_on*` 三行，多出来的头注释一律当背景噪音，不影响任何行为。
//!
//! 于是这份文件**拷给别人、换一台电脑**，仍然自己说得出它是从哪来的；
//! `presets-mine/` 里也不会多出程序写的第二份文件。
//!
//! # 副本 = 来源 + 三行（正文一个字节不动）
//!
//! 与工作台建副本同一条硬规矩（那边判据 K-O2 咬着）：正文（`[toolhead]` 之后的一切）
//! 逐字节不动，注释、键序、`offset` 的 inline table、`"""` 多行字面量原样保留。
//! **不许"读成结构体再写出来"** —— 那会把用户预设里约 70 条注释全抹掉。
//!
//! # 两条写路（只有"那三行从哪来"不一样）
//!
//! ```text
//! make_copy                建副本：三行从**来源**算（来源全文摘要 + 来源的 release_time）
//! rewrite_keeping_lineage  写回自己：三行**照抄文件里原来那三行**（出处没变）
//! ```
//!
//! 所以「改我那份 → 保存」**不会**产生 `（已修改）2.toml`，也不会把血统改成"基于我自己"——
//! 出处还是当初那一版官方（第八层）。两条写路共用同一个插入规则（[`splice_lineage`]），
//! 于是"保存一份没改过的副本"= 文件逐字节不变。
//!
//! # 为什么这一段有两份实现（客户端 / 工作台）
//!
//! 工作台那份在 `crates/preset/src/lineage.rs`（那份还带 `DiffState`/`diff` 等要
//! `write::EditValue` 的东西）。**默认构建不编 `mkpse-preset`**（隔离纪律：客户端不许
//! `use preset`，那个 crate 带 56 KB 注册表与 9 份内置预设），所以客户端这份是重写的。
//!
//! 两份之间没有编译器，靠一条**两端一致性判据**钉住：`src-tauri/src/workbench/lineage_parity.rs`
//! 在 workbench feature 下对 9 份入库产物逐字节比 `make_copy` 与 `parse_lineage_from_content`。
//! 谁改了形状、那边就红。**将来更彻底的一条路**是把这一段文本逻辑下移到共享小 crate、
//! 两边转调 —— 那是动 crate 结构的事，与第八层（把用户改动迁到新版）一起裁。

use sha2::{Digest, Sha256};

/// 三行血统注释的键名。**单点具名**：写（[`make_copy`]）与读（[`parse_lineage_from_content`]）
/// 咬同一份（工作台那份的 `LINEAGE_KEYS` 是同一个清单）。
const LINEAGE_KEYS: [&str; 3] = ["based_on", "based_on_release_time", "based_on_sha256"];

/// 一份副本的血统。三项各自 `Option`：手工拷的副本可能只有一半，
/// **缺谁就是谁不知道，不许拿默认值假装知道**。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Lineage {
    /// 来源，形如 `mkp/presets/A1-standard.toml`（角色目录 + 文件名，**不写绝对路径**——
    /// 数据根可能被搬走，绝对路径会失效）。
    pub based_on: Option<String>,
    /// 建副本那一刻，来源文件头的 `# release_time`（给人看的版本号）。
    pub based_on_release_time: Option<String>,
    /// 建副本那一刻，来源文件**全文的 sha256**（给程序判「官方变了没有」）。
    pub based_on_sha256: Option<String>,
}

impl Lineage {
    /// 三项全空 = 没有血统（不是「血统未知」）
    pub fn is_empty(&self) -> bool {
        self.based_on.is_none()
            && self.based_on_release_time.is_none()
            && self.based_on_sha256.is_none()
    }
}

/// 全文 sha256（小写 hex）。
///
/// 算的是**全文**（含头注释）：`release_time` 改了、注释改了也算变 —— 这正是我们要的
/// （宁可多问一句「官方变了」，也别把一个真变化漏过去）。
pub fn sha256_hex(content: &str) -> String {
    super::catalog::hex(&Sha256::digest(content.as_bytes()))
}

/// 建一份副本的**文本**：拷字节 + 在头注释块末尾追加三行。
///
/// 摘要与 `based_on_release_time` 都从 `src_text` **自己算/自己读**，不从外面传 ——
/// 外面传就有两处事实，迟早不一致。
///
/// 来源本身已经带血统时（副本再拷副本），旧的三行会被**换掉**而不是叠加：
/// 叠加之后只有第一条读得出来，血统就成了骗人的。
pub fn make_copy(src_text: &str, based_on: &str) -> String {
    let body = strip_lineage_lines(src_text);
    let lineage = Lineage {
        based_on: Some(based_on.to_owned()),
        based_on_release_time: parse_release_time_from_content(src_text),
        based_on_sha256: Some(sha256_hex(src_text)),
    };
    let block = lineage_block(&lineage, newline_of(&body));
    splice_lineage(&body, &block)
}

/// **保存回同一份**（第八层）：把编辑后的正文写回它自己，**出处那三行原样保留**。
///
/// 与 [`make_copy`] 的区别只有一件事：**那三行的来源**。
/// 建副本是"我从哪来"（从**来源**算：来源的全文摘要 + 来源的 `release_time`）；
/// 写回自己时出处没变，所以**照抄文件里原来那三行** —— 重新算的话，
/// 摘要会变成"我自己那份改过的字节"，血统就说不出它是从哪一版官方派生的了。
///
/// `existing` 是 `None`（这份文件本来就没有血统：手工拷的 / 别的程序写出来的）⇒
/// 正文原样写回，**不编一个出处**。
///
/// 另一条用途：**保存一份没改过的副本 = 文件逐字节不变**（下面那条判据咬着），
/// 因为插入位置与内容都与原来那次插入一致。
pub fn rewrite_keeping_lineage(edited_text: &str, existing: Option<&Lineage>) -> String {
    let body = strip_lineage_lines(edited_text);
    let Some(lineage) = existing else {
        return body;
    };
    let block = lineage_block(lineage, newline_of(&body));
    splice_lineage(&body, &block)
}

/// 三行血统的文本（每行带换行）。**缺谁就不写谁** —— 不知道的东西不许拿默认值假装知道。
fn lineage_block(lineage: &Lineage, nl: &str) -> String {
    let mut out = String::new();
    let mut push = |key: &str, value: Option<&str>| {
        if let Some(v) = value.filter(|v| !v.is_empty()) {
            out.push_str(&format!("# {key}: {v}{nl}"));
        }
    };
    push("based_on", lineage.based_on.as_deref());
    push(
        "based_on_release_time",
        lineage.based_on_release_time.as_deref(),
    );
    push("based_on_sha256", lineage.based_on_sha256.as_deref());
    out
}

/// 把血统三行插到正文的**头注释块末尾**（正文其余字节一个都不动）
fn splice_lineage(body: &str, block: &str) -> String {
    if block.is_empty() {
        return body.to_owned();
    }
    let at = header_block_end(body);
    let mut out = String::with_capacity(body.len() + block.len());
    out.push_str(&body[..at]);
    out.push_str(block);
    out.push_str(&body[at..]);
    out
}

/// 这份文本用哪个行尾（跟着它自己，别混进另一种）
fn newline_of(text: &str) -> &'static str {
    if text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// 把已有的 `# based_on*` 行整行删掉（含行尾），其余字节原样。
///
/// **两条写路的公共前半段**（剪掉 → 重新插入），也用于"打开我那份进编辑器"
/// （第八层：编辑器里给的是**正文**，那三行是程序的元数据，不是用户该改的内容）。
/// 判据用它比较"剪完是否与来源逐字节相同"。
pub fn strip_lineage_lines(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for (line, term) in lines_with_terminators(text) {
        if is_lineage_line(line) {
            continue;
        }
        out.push_str(line);
        out.push_str(term);
    }
    out
}

fn is_lineage_line(line: &str) -> bool {
    let t = line.trim();
    let Some(rest) = t.strip_prefix('#') else {
        return false;
    };
    let rest = rest.trim_start();
    LINEAGE_KEYS.iter().any(|k| {
        rest.strip_prefix(*k)
            .is_some_and(|after| after.trim_start().starts_with(':'))
    })
}

/// 文件头注释块的末尾（字节偏移）：**最后一条头注释行之后**。
///
/// 头注释块 = 从第 0 行起、由注释行与空行组成的最长前缀。插在「最后一条注释行之后」
/// 而不是「第一条非注释行之前」，是为了让三行紧贴着 `# machine:` 那一族，
/// 而不是被空行隔到 `[toolhead]` 头上。
/// 一条注释都没有 ⇒ 插在文件最前面（工作台那份同一条规则）。
fn header_block_end(text: &str) -> usize {
    let mut offset = 0usize;
    let mut insert_at = 0usize;
    for (line, term) in lines_with_terminators(text) {
        let t = line.trim();
        let is_comment = t.starts_with('#');
        if !is_comment && !t.is_empty() {
            break;
        }
        offset += line.len() + term.len();
        if is_comment {
            insert_at = offset;
        }
    }
    insert_at
}

/// 逐行切分，**保留每行的行尾**（`\r\n` / `\n` / 无）。
/// `str::lines()` 会把行尾吃掉，那样拼不回原字节。
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

/// 把副本文本里的三行血统注释剪掉 —— **判据用**：剪完必须与来源逐字节相同。
///
/// 放在产品代码里（而不是判据里）的理由与工作台那份相同：它与 [`make_copy`] 的插入规则是
/// **同一份**知识（哪三行、怎么认），两处各写一遍必然漂移。
pub fn strip_lineage_for_compare(text: &str) -> String {
    strip_lineage_lines(text)
}

/// `# based_on` / `# based_on_release_time` / `# based_on_sha256` 三行。
///
/// 三项**全都缺** ⇒ `None`（"这份文件没有血统"：手工拷的、或官方原件本身）；
/// 只缺其中几项 ⇒ `Some`，缺的那几项是 `None`（"这几项不知道"）。
/// 这两种情况在界面上说的话不一样，所以类型上必须分得开。
pub fn parse_lineage_from_content(content: &str) -> Option<Lineage> {
    let lineage = Lineage {
        based_on: parse_header_value(content, "based_on"),
        based_on_release_time: parse_header_value(content, "based_on_release_time"),
        based_on_sha256: parse_header_value(content, "based_on_sha256"),
    };
    if lineage.is_empty() {
        return None;
    }
    Some(lineage)
}

/// `^#\s*release_time\s*:\s*(.+)$`（与工作台那份同一条路）
fn parse_release_time_from_content(content: &str) -> Option<String> {
    parse_header_value(content, "release_time")
}

/// 逐行找 `# <key>: <值>`，返回第一个非空值。
///
/// **匹配顺序有讲究**（调用方按长键先试）：`based_on_release_time` 必须比 `based_on` 先试 ——
/// `match_header_key` 只要求键后跟可选空白 + 冒号，`based_on_release_time` 这一行用
/// `based_on` 去匹配时键后紧跟的是 `_`（不是冒号）⇒ 不会误命中。
fn parse_header_value(content: &str, key: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(rest) = match_header_key(trimmed, key) {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// 行头 `#` + 可选空白 + 字面键 + 可选空白 + `:` + 可选空白，返回冒号后的原样剩余
/// （值至少一个字符）。键后必须是冒号 —— 防 `machine_x:` 误命中 `machine`。
fn match_header_key<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let after_hash = line.strip_prefix('#')?;
    let after_ws1 = after_hash.trim_start();
    let after_key = after_ws1.strip_prefix(key)?;
    let after_ws2 = after_key.trim_start();
    let after_colon = after_ws2.strip_prefix(':')?;
    let rest = after_colon.trim_start();
    if rest.is_empty() {
        return None; // (.+) 要求至少一个字符
    }
    Some(rest)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一份带文件头注释的官方预设（与入库产物同形）
    const SRC: &str = "\
# uuid: 1111
# release_time: 2026-08-19 01:38:13
# machine: A1
# variant: standard

[toolhead]
offset_x = -1
offset_y = 18.6
offset_z = 4 # 笔尖偏移
";

    /// **核心不变式**：副本 = 来源 + 三行。剪掉三行必须与来源逐字节相同
    #[test]
    fn the_copy_is_the_source_plus_three_lines() {
        let copy = make_copy(SRC, "mkp/presets/A1-standard.toml");
        assert_eq!(
            strip_lineage_for_compare(&copy),
            SRC,
            "剪掉三行之后必须与来源逐字节相同 —— 正文被动过了"
        );
        // 三行紧贴头注释块，不被空行隔到 [toolhead] 头上
        let head: Vec<&str> = copy.lines().take(7).collect();
        assert_eq!(head[3], "# variant: standard");
        assert_eq!(head[4], "# based_on: mkp/presets/A1-standard.toml");
        assert_eq!(head[5], "# based_on_release_time: 2026-08-19 01:38:13");
        assert!(head[6].starts_with("# based_on_sha256: "));
    }

    /// 副本再拷副本：旧血统被**换掉**而不是叠加（叠加之后只读得出第一条）
    #[test]
    fn copying_a_copy_replaces_the_old_lineage() {
        let first = make_copy(SRC, "mkp/presets/A1-standard.toml");
        let second = make_copy(&first, "mkp/presets/A1-fast.toml");
        assert_eq!(second.matches("# based_on:").count(), 1);
        assert!(second.contains("# based_on: mkp/presets/A1-fast.toml"));
        assert!(
            second.contains(&format!("# based_on_sha256: {}", sha256_hex(&first))),
            "第二次的摘要算的是**来源那份全文**（第一份副本），不是更早的来源"
        );
    }

    /// CRLF 跟着来源（不许混进裸 `\n`）
    #[test]
    fn crlf_source_keeps_crlf() {
        let crlf = SRC.replace('\n', "\r\n");
        let copy = make_copy(&crlf, "mkp/presets/A1-standard.toml");
        assert!(copy.contains("# based_on: mkp/presets/A1-standard.toml\r\n"));
        assert_eq!(strip_lineage_for_compare(&copy), crlf);
    }

    /// 来源没有 `# release_time` 就只写两行 —— **不知道就不要写这一行**
    #[test]
    fn a_source_without_release_time_gets_two_lines() {
        let no_rt = SRC.replacen("# release_time: 2026-08-19 01:38:13\n", "", 1);
        let copy = make_copy(&no_rt, "mkp/presets/A1-standard.toml");
        assert!(!copy.contains("based_on_release_time"));
        assert!(copy.contains("# based_on: mkp/presets/A1-standard.toml"));
        assert_eq!(strip_lineage_for_compare(&copy), no_rt);
    }

    /// 读得回来；**没有血统 = `None`**（不是三项全空的空壳）
    #[test]
    fn lineage_roundtrips_and_absence_is_none() {
        assert_eq!(parse_lineage_from_content(SRC), None, "官方原件没有血统");

        let copy = make_copy(SRC, "mkp/presets/A1-standard.toml");
        let got = parse_lineage_from_content(&copy).expect("三行都在，读得出来");
        assert_eq!(
            got.based_on.as_deref(),
            Some("mkp/presets/A1-standard.toml")
        );
        assert_eq!(
            got.based_on_release_time.as_deref(),
            Some("2026-08-19 01:38:13")
        );
        assert_eq!(
            got.based_on_sha256.as_deref(),
            Some(sha256_hex(SRC).as_str())
        );
    }

    /// 只缺一半也算有血统（缺的那项是"不知道"，不是"空"）
    #[test]
    fn a_half_lineage_is_still_a_lineage() {
        let only_source = format!("# based_on: mkp/presets/A1-standard.toml\n{SRC}");
        let got = parse_lineage_from_content(&only_source).expect("有一项就算有");
        assert_eq!(
            got.based_on.as_deref(),
            Some("mkp/presets/A1-standard.toml")
        );
        assert_eq!(got.based_on_sha256, None, "不知道就是不知道");
        assert!(!got.is_empty());
    }

    /// 键后必须是冒号：`# based_on_x: v` 不许被当成 `based_on`
    #[test]
    fn a_lookalike_key_is_not_matched() {
        let text = format!("# based_on_extra: nope\n# based_on: mkp/presets/A1.toml\n{SRC}");
        let got = parse_lineage_from_content(&text).expect("有一项就算有");
        assert_eq!(got.based_on.as_deref(), Some("mkp/presets/A1.toml"));
    }

    /* ---------- 写回自己（第八层） ---------- */

    /// **出处不变**：写回时那三行照抄原来的 —— 摘要不许变成"我自己改过的字节"
    #[test]
    fn saving_back_keeps_the_original_lineage() {
        let copy = make_copy(SRC, "mkp/presets/A1-standard.toml");
        let lineage = parse_lineage_from_content(&copy).expect("副本该有血统");
        let edited = strip_lineage_for_compare(&copy).replace("-1", "-2");

        let saved = rewrite_keeping_lineage(&edited, Some(&lineage));

        assert_eq!(
            strip_lineage_for_compare(&saved),
            edited,
            "正文就是改过的那份"
        );
        let after = parse_lineage_from_content(&saved).expect("血统还在");
        assert_eq!(after, lineage, "出处那三行一个字都没变");
        assert_ne!(
            after.based_on_sha256.as_deref(),
            Some(sha256_hex(&edited).as_str()),
            "摘要要是**来源**那份全文的，不是我自己改过的字节 —— 否则血统就说不出从哪一版来"
        );
    }

    /// **保存一份没改过的副本 = 文件逐字节不变**（两条写路共用同一个插入规则）
    #[test]
    fn saving_an_untouched_copy_changes_nothing() {
        let copy = make_copy(SRC, "mkp/presets/A1-standard.toml");
        let lineage = parse_lineage_from_content(&copy).expect("副本该有血统");

        assert_eq!(
            rewrite_keeping_lineage(strip_lineage_for_compare(&copy).as_str(), Some(&lineage)),
            copy,
            "打开又保存、什么都没改，文件必须逐字节不变"
        );
    }

    /// 没有血统的那份（手工拷的 / 别的程序写的）写回时**不编一个出处**
    #[test]
    fn saving_back_without_lineage_writes_no_lineage() {
        let saved = rewrite_keeping_lineage(SRC, None);
        assert_eq!(saved, SRC);
        assert_eq!(parse_lineage_from_content(&saved), None);
    }

    /// 行尾跟着**正文自己**：CRLF 的那份改了再存还是 CRLF，不许混进裸 `\n`
    #[test]
    fn saving_back_keeps_the_line_endings() {
        let copy = make_copy(&SRC.replace('\n', "\r\n"), "mkp/presets/A1-standard.toml");
        let lineage = parse_lineage_from_content(&copy).expect("副本该有血统");
        let body = strip_lineage_for_compare(&copy).replace("-1", "-2");

        let saved = rewrite_keeping_lineage(&body, Some(&lineage));
        let block: String = saved
            .lines()
            .filter(|l| l.trim_start().starts_with("# based_on"))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(block.contains("# based_on:"), "三行还在");
        assert!(
            saved.contains("# based_on: mkp/presets/A1-standard.toml\r\n"),
            "插回去的三行也要是 CRLF"
        );
    }
}
