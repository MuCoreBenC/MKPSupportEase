//! **来源记录**（provenance）：用户自己那份（`presets-local/…`）是从哪一份云端、哪一版来的。
//!
//! # 记录写在文件里，不写在 `run/`
//!
//! 这是作者 2026-10-02 定的原则（第七层），2026-11-04 换成尾部表：
//!
//! > **凡是描述"这个 Preset 文件本身是什么"的信息，都随 TOML 文件携带；
//! > 凡是描述"这个程序现在怎么管理它"的信息，才属于 `run/`。**
//!
//! 所以来源不另立程序侧的状态文件，而是写进文件**尾部**的 `[mkp_source]` 表
//! （形状见 `preset::provenance` 模块头）。于是这份文件**拷给别人、换一台电脑**，
//! 仍然自己说得出它是从哪来的。
//!
//! # 详情页只认它（2026-11-04 作者裁定）
//!
//! 界面上「来源 / 来源版本 / 历次版本」全部读这一份记录；
//! **不再用 `run/provenance.json` 的账去顶**。那份账只当程序自己的运行索引
//! （判"要不要提示更新"），不进展示链。
//!
//! 用户手动拖进来 / 复制得到的文件：来源是**未知**，只记落地时间 ——
//! 「他哪一次拖进来的」本身就是溯源信息。
//!
//! # 实现只有一份
//!
//! 解析与渲染都在 `preset::provenance`（客户端与工作台共用；`mkpse-preset` 现在是
//! 无条件依赖，2026-11-04 起不再"默认构建不编"），这里只做转发 + 运行期需要的那层封装。
//! 头注释的 `# machine:` / `# variant:` 读写仍留在这个模块（与来源记录无关）。

use preset::model::{SourceProvenance, SourceRef};

/// 全文 sha256（小写 hex）。判「云端变了没有」。
pub fn sha256_hex(content: &str) -> String {
    preset::provenance::sha256_hex(content)
}

/// 读来源记录。没有 / 坏档 ⇒ `None`（不知道就是不知道）。
pub fn parse(content: &str) -> Option<SourceProvenance> {
    preset::provenance::parse(content).filter(|p| !p.is_empty())
}

/// 剥掉尾部来源记录（判据与「打开进编辑器」都用它）。
pub fn strip_for_compare(text: &str) -> String {
    preset::provenance::strip(text)
}

/// 把编辑后的正文写回，**来源记录原样保留**（第八层：出处没变）。
///
/// `existing` 为 `None`（这份文件本来就没有记录：手工拖进来的 / 别的程序写的）⇒
/// 正文原样返回，**不编一个出处**。
pub fn rewrite_keeping_provenance(
    edited_text: &str,
    existing: Option<&SourceProvenance>,
) -> String {
    let body = strip_for_compare(edited_text);
    match existing.filter(|p| !p.is_empty()) {
        Some(prov) => preset::provenance::write(&body, prov),
        None => body,
    }
}

/// 本地草稿落盘成用户文件时写的那一份来源记录（还不知道发布线与序号）。
///
/// 只记「我从哪个云端文件来的」+ 落地时刻；云端身份 / 版本由后续下载或接受版本补齐，
/// **不编造**。
///
/// ★ 2026-11-04：**这条不产生履历**。用户「另存」不是"接受了一版云端"，
/// 履历只在用户主动把云端新版合并进这份文件时才长（见 `ipc::param_sync::apply_at`）。
/// 所以这里只写 `origin`，`history` 留空。
pub fn commit_text(src_text: &str, file: &str) -> String {
    let origin = SourceRef {
        kind: Some("copy".to_owned()),
        file: Some(file.to_owned()),
        ..SourceRef::default()
    };
    preset::provenance::write_origin_only(src_text, origin)
}

/// 下载 / 内部自动**拷贝**一份云端预设：写 `download` 来源 + 一条履历。
///
/// 来源身份（平台 / 仓库 / 分支 / 文件）来自调用方抓到的真实来源；
/// **缺项就是不知道，不写**（不许拿相邻字段顶）。
pub fn copy_from_cloud(src_text: &str, origin: SourceRef, version: SourceRef) -> String {
    preset::provenance::copy_with_provenance(src_text, origin, version)
}

/// 接受了一版新云端的履历：**只追加一条**，旧条目一个字不改。
pub fn record_version(text: &str, version: SourceRef) -> String {
    preset::provenance::append_version(text, version)
}

/// 用户手动**复制**：新文件记母本，来源退回「未知」之外的那部分照实继承。
///
/// 注意与 [`copy_from_cloud`] 的区别：只有这一条会写 `mother`（本地母本）。
pub fn duplicate_from_local(mother_text: &str, mother_path: &str) -> String {
    preset::provenance::duplicate_from_local(mother_text, mother_path)
}

/// 用户**手动拖进来**的文件：没有可追溯来源，只记「哪一次拖进来的」。
///
/// `at` 是落地时刻（下载/导入那一刻），不是配方版本时间 —— 展示上两者分得很开。
pub fn imported_at(text: &str, at: &str) -> String {
    let origin = SourceRef {
        kind: Some("import".to_owned()),
        at: Some(at.to_owned()),
        ..SourceRef::default()
    };
    preset::provenance::write(
        text,
        &SourceProvenance {
            schema: 1,
            origin: Some(origin),
            history: Vec::new(),
        },
    )
}

/// **这一版云端是什么时候发的**（文件头 `# release_time` 的原样值）。
///
/// 它是"这是哪个云端版本"的日期真值 —— **不许**拿下载时刻 / 文件 mtime /
/// 目录代时间 / 写盘记录 `generated_at` 冒充它。没有这一行就 `None`。
///
/// 实现只有 `preset::read` 那一份（2026-11-04：删掉这里的第二份）。
pub fn parse_release_time(content: &str) -> Option<String> {
    preset::read::parse_release_time_from_content(content)
}

/* ---------------- 头注释 `# machine:` / `# variant:` ---------------- */

/// 逐行找 `# <key>: <值>`，返回第一个非空值。
///
/// **匹配顺序有讲究**（调用方按长键先试）：`machine` 必须先于 `machine_x` 之类，
/// `match_header_key` 只要求键后跟可选空白 + 冒号，键后紧跟 `_` 不会误命中。
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

/// `# machine: A1`（与 `# release_time` 同一条解析路）。没有就是没有 —— 不猜。
pub fn parse_machine_from_content(content: &str) -> Option<String> {
    parse_header_value(content, "machine")
}

/// `# variant: standard`。**大小写不归它管**：B 类产物写的是版本 id 的小写
/// （`fast`），解析原样返回，归一化交给调用方（对着目录大小写无关地比对）。
pub fn parse_variant_from_content(content: &str) -> Option<String> {
    parse_header_value(content, "variant")
}

/// 把文件头的 `# machine:` / `# variant:` 两行**换成新值**（用户线「改归属」用）。
///
/// 规则与来源记录那两条写路同族：**只动这两行注释，正文其余字节一个不碰** ——
/// 已有的行在原位替换；缺哪行就在头注释块里补上（两个都缺时一起插在头注释块末尾）。
/// 行尾跟着文件自己（CRLF 的文件改完还是 CRLF）。
pub fn rewrite_machine_variant(text: &str, machine: &str, variant: &str) -> String {
    let nl = newline_of(text);
    let has_machine = parse_machine_from_content(text).is_some();
    let has_variant = parse_variant_from_content(text).is_some();
    /* 两行都在文件里 ⇒ 不需要插入位置；否则插在头注释块末尾 */
    let insert_at = if has_machine && has_variant {
        None
    } else {
        Some(header_block_end(text))
    };

    let mut out = String::with_capacity(text.len() + machine.len() + variant.len() + 32);
    let mut machine_written = false;
    let mut variant_written = false;
    let mut offset = 0usize;
    for (line, term) in lines_with_terminators(text) {
        /* 该插的那一格：插在头注释块末尾（一次插两行，写过的键不再重复写） */
        if Some(offset) == insert_at && !machine_written {
            out.push_str(&format!("# machine: {machine}{nl}"));
            machine_written = true;
        }
        if Some(offset) == insert_at && !variant_written {
            out.push_str(&format!("# variant: {variant}{nl}"));
            variant_written = true;
        }
        let t = line.trim();
        if match_header_key(t, "machine").is_some() {
            out.push_str(&format!("# machine: {machine}{nl}"));
            machine_written = true;
            /* 文件里有 machine 行而没有 variant 行 ⇒ variant 紧跟着补上（同族挨着） */
            if !has_variant && !variant_written {
                out.push_str(&format!("# variant: {variant}{nl}"));
                variant_written = true;
            }
            offset += line.len() + term.len();
            continue;
        }
        if match_header_key(t, "variant").is_some() {
            if !machine_written && !has_machine {
                /* 文件里有 variant 行而没有 machine 行 ⇒ machine 补在它前面 */
                out.push_str(&format!("# machine: {machine}{nl}"));
                machine_written = true;
            }
            out.push_str(&format!("# variant: {variant}{nl}"));
            variant_written = true;
            offset += line.len() + term.len();
            continue;
        }
        out.push_str(line);
        out.push_str(term);
        offset += line.len() + term.len();
    }
    /* 两行都缺、且文件一条头注释都没有（insert_at == 0）时循环里已插过；
    到这里还没写（理论到不了：insert_at 之内必有一格命中）就在末尾兜底补上 */
    if !machine_written {
        out.push_str(&format!("# machine: {machine}{nl}"));
    }
    if !variant_written {
        out.push_str(&format!("# variant: {variant}{nl}"));
    }
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

/// 文件头注释块的末尾（字节偏移）：**最后一条头注释行之后**。
///
/// 头注释块 = 从第 0 行起、由注释行与空行组成的最长前缀。插在「最后一条注释行之后」
/// 而不是「第一条非注释行之前」，是为了让新行紧贴着 `# machine:` 那一族，
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

#[cfg(test)]
mod tests {
    use super::*;

    /// 一份带文件头注释的云端预设（与入库产物同形）
    const SRC: &str = "\
# release_time: 2026-08-19 01:38:13
# machine: A1
# variant: standard

[toolhead]
offset_x = -1
offset_y = 18.6
offset_z = 4 # 笔尖偏移
";

    fn origin() -> SourceRef {
        SourceRef {
            kind: Some("download".to_owned()),
            file: Some("mkp/presets/A1-standard.toml".to_owned()),
            platform: Some("github".to_owned()),
            repo: Some("owner/repo".to_owned()),
            branch: Some("main".to_owned()),
            at: Some("2026-10-31T00:00:00Z".to_owned()),
            sha256: Some(sha256_hex(SRC)),
            catalog: Some("support-ease".to_owned()),
            serial: Some(5),
            revision: Some("f09e".to_owned()),
            ..SourceRef::default()
        }
    }

    /// **核心不变式**：副本 = 来源 + 尾部记录。剥掉记录必须与来源逐字节相同。
    #[test]
    fn the_copy_is_the_source_plus_a_tail_block() {
        let copy = copy_from_cloud(SRC, origin(), origin());
        assert_eq!(
            strip_for_compare(&copy),
            SRC,
            "剥掉尾部记录之后必须与来源逐字节相同 —— 正文被动过了"
        );
        // 记录在尾部，头注释没有被塞进任何新行
        assert!(copy.starts_with("# release_time: 2026-08-19 01:38:13\n"));
        assert!(copy.contains("[mkp_source]"));
        assert!(copy.contains("[mkp_source.origin]"));
        let prov = parse(&copy).expect("必须读得回来");
        assert_eq!(prov.history.len(), 1);
    }

    /// 记一笔新版本：只追加，旧履历一个字不改。
    #[test]
    fn a_new_version_only_appends() {
        let first = copy_from_cloud(SRC, origin(), origin());
        let second = record_version(
            &first,
            SourceRef {
                at: Some("2026-11-02T00:00:00Z".to_owned()),
                serial: Some(6),
                sha256: Some("b".repeat(64)),
                ..origin()
            },
        );
        let prov = parse(&second).expect("两版都在");
        assert_eq!(prov.history.len(), 2, "追加一版 = 两条履历");
        assert_eq!(
            prov.history[0].serial,
            Some(5),
            "旧条目一个字不改（序号仍是 5）"
        );
        assert_eq!(prov.history[0].at.as_deref(), Some("2026-10-31T00:00:00Z"));
        assert_eq!(prov.history[1].serial, Some(6));
        assert_eq!(strip_for_compare(&second), SRC, "正文仍逐字节相同");
    }

    /// **来源版本**取履历最后一条（不是 origin：复制出来的文件两者可以不同）。
    #[test]
    fn the_source_version_is_the_latest_history_entry() {
        let first = copy_from_cloud(SRC, origin(), origin());
        let second = record_version(
            &first,
            SourceRef {
                at: Some("2026-11-02T00:00:00Z".to_owned()),
                serial: Some(6),
                ..origin()
            },
        );
        let prov = parse(&second).unwrap();
        assert_eq!(
            prov.latest_version().and_then(|v| v.at.as_deref()),
            Some("2026-11-02T00:00:00Z"),
            "来源版本 = 最近接受的那一版"
        );
    }

    /// 写盘记录不许变成「哪一版」的时间（2026-11-04）
    #[test]
    fn generated_at_never_becomes_the_source_version() {
        let only_generated = SRC.replacen(
            "# release_time: 2026-08-19 01:38:13\n",
            "# generated_at: 2026-11-01T12:30:00+08:00\n",
            1,
        );
        assert_eq!(
            parse_release_time(&only_generated),
            None,
            "只有写盘记录时，配方版本时间是未知"
        );
        let copy = copy_from_cloud(&only_generated, origin(), origin());
        let prov = parse(&copy).unwrap();
        assert_eq!(
            prov.origin.and_then(|o| o.at).as_deref(),
            Some("2026-10-31T00:00:00Z"),
            "来源时间来自目录发布时刻（调用方给的），不是文件里的 generated_at"
        );
    }

    /// 用户手动复制：记母本；内部拷贝不记。
    #[test]
    fn only_a_user_duplicate_records_the_mother() {
        let copy = copy_from_cloud(SRC, origin(), origin());
        assert!(!copy.contains("mother"), "内部自动拷贝不许捏造母本");
        let dup = duplicate_from_local(&copy, "presets-local/我的 A1.toml");
        let prov = parse(&dup).unwrap();
        let o = prov.origin.unwrap();
        assert_eq!(o.kind.as_deref(), Some("duplicate"));
        assert_eq!(o.mother.as_deref(), Some("presets-local/我的 A1.toml"));
        assert_eq!(strip_for_compare(&dup), SRC);
    }

    /// 拖进来的文件：来源未知，只记落地时间。
    #[test]
    fn an_imported_file_only_records_when_it_landed() {
        let land = imported_at(SRC, "2026-11-04T09:00:00+08:00");
        let prov = parse(&land).expect("拖进来的也要有记录");
        let o = prov.origin.expect("有 origin");
        assert_eq!(o.kind.as_deref(), Some("import"));
        assert_eq!(o.at.as_deref(), Some("2026-11-04T09:00:00+08:00"));
        assert_eq!(
            (o.file, o.platform, o.branch, o.serial),
            (None, None, None, None),
            "没有可追溯来源就得留空 —— 不许编"
        );
        assert_eq!(strip_for_compare(&land), SRC);
    }

    /// 写回自己：出处照旧（不重算成"基于我自己"）。
    #[test]
    fn rewriting_keeps_the_existing_provenance() {
        let copy = copy_from_cloud(SRC, origin(), origin());
        let prov = parse(&copy).unwrap();
        let edited = copy.replace("offset_z = 4", "offset_z = 5");
        let out = rewrite_keeping_provenance(&edited, Some(&prov));
        assert_eq!(
            parse(&out).unwrap().origin.unwrap().sha256,
            prov.origin.clone().unwrap().sha256,
            "出处没变，摘要不该被重算成编辑后的字节"
        );
        assert!(out.contains("offset_z = 5"));
    }

    /// 本来没有记录的：写回**不许编一个出处**。
    #[test]
    fn rewriting_without_provenance_invents_nothing() {
        assert_eq!(rewrite_keeping_provenance(SRC, None), SRC);
        assert!(!rewrite_keeping_provenance(SRC, None).contains("mkp_source"));
    }

    /// CRLF 跟着来源（不许混进裸 `\n`）
    #[test]
    fn crlf_source_keeps_crlf() {
        let crlf = SRC.replace('\n', "\r\n");
        let copy = copy_from_cloud(&crlf, origin(), origin());
        assert!(copy.contains("[mkp_source]\r\n"));
        assert_eq!(strip_for_compare(&copy), crlf);
    }

    /* ---------- 改归属（# machine / # variant 两行） ---------- */

    /// 两行都在：**原位换值**，正文与来源记录一个字节不动
    #[test]
    fn rewriting_machine_variant_replaces_in_place() {
        let copy = copy_from_cloud(SRC, origin(), origin());
        let out = rewrite_machine_variant(&copy, "A1_MINI", "FAST");

        let mut machine = out.lines().filter(|l| l.starts_with("# machine:"));
        let mut variant = out.lines().filter(|l| l.starts_with("# variant:"));
        assert_eq!(machine.next(), Some("# machine: A1_MINI"));
        assert_eq!(machine.next(), None, "machine 行只有一条");
        assert_eq!(variant.next(), Some("# variant: FAST"));
        assert_eq!(variant.next(), None, "variant 行只有一条");

        /* 去掉 machine / variant 两行之后 = 来源 + 尾部记录（正文没被动过） */
        let stripped: String = out
            .lines()
            .filter(|l| !l.starts_with("# machine:") && !l.starts_with("# variant:"))
            .map(|l| format!("{l}\n"))
            .collect();
        let copy_stripped: String = copy
            .lines()
            .filter(|l| !l.starts_with("# machine:") && !l.starts_with("# variant:"))
            .map(|l| format!("{l}\n"))
            .collect();
        assert_eq!(stripped, copy_stripped, "除那两行外逐字节相同");
        assert_eq!(
            parse(&out),
            parse(&copy),
            "来源记录原样（改归属不影响出处）"
        );
    }

    /// 缺哪行补哪行；两行全缺时一起插进头注释块
    #[test]
    fn rewriting_adds_the_missing_lines() {
        /* 只有 variant：machine 补在它前面 */
        let only_variant = "# release_time: t\n# variant: standard\n[toolhead]\nx = 1\n";
        let out = rewrite_machine_variant(only_variant, "A1", "FAST");
        let head: Vec<&str> = out.lines().take(3).collect();
        assert_eq!(head[0], "# release_time: t");
        assert_eq!(head[1], "# machine: A1");
        assert_eq!(head[2], "# variant: FAST");

        /* 两行全缺：插在头注释块末尾（紧贴 [toolhead]，不进正文） */
        let out = rewrite_machine_variant(SRC_WITHOUT_MV, "A1", "FAST");
        assert!(out.contains("# machine: A1"));
        assert!(out.contains("# variant: FAST"));
        assert!(out.find("# machine:").unwrap() < out.find("[toolhead]").unwrap());

        /* CRLF 跟着文件自己 */
        let crlf = rewrite_machine_variant(&SRC.replace('\n', "\r\n"), "A1", "FAST");
        assert!(crlf.contains("# machine: A1\r\n"));
        assert!(crlf.contains("# variant: FAST\r\n"));
    }

    /// 一份**没有** machine / variant 两行的云端形文件（测试夹具）
    const SRC_WITHOUT_MV: &str = "\
# release_time: 2026-08-19 01:38:13

[toolhead]
offset_x = -1
";
}
