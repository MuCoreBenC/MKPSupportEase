//! 血统：建**用户副本**、算内容摘要。
//!
//! # 一条硬规矩：副本的正文与来源逐字节相同
//!
//! 建副本只做两件事 —— 拷字节、在文件头注释块末尾**追加**三行 `# based_on*`。
//! 正文（`[toolhead]` 之后的一切）一个字节都不动，注释、键序、`offset` 的 inline table、
//! `"""` 多行字面量都原样保留。判据 K-O2 把三行剪掉再 `cmp`，必须与来源逐字节相同。
//!
//! **为什么不用「读成结构体再写出来」**：那会把用户预设里约 70 条注释 / 100 行全抹掉
//! （这条教训已经写在 `write.rs` 的头上了）。
//!
//! # 为什么是纯函数
//!
//! `make_copy` 不碰文件系统 —— 判据能逐字节咬它，也不用临时目录。落盘在 `src-tauri` 那侧
//! （`preset_copy.rs`：唯一可写目录 + 不覆盖已有副本 + 同目录临时文件 + fsync + rename）。

use sha2::{Digest, Sha256};

/// 内容摘要（小写 hex）。**判「官方变了没有」用它** ——
/// 云端那套系统不会通知我们，只能自己记账。
///
/// 算的是**全文**（含头注释）：`release_time` 改了、注释改了也算变，
/// 这正是我们想要的（宁可多问一句）。
pub fn sha256_hex(content: &str) -> String {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    h.finalize().iter().fold(String::new(), |mut s, b| {
        use std::fmt::Write as _;
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// 三行血统注释的键名。**单点具名**：读（`read.rs`）与写（这里）咬同一份。
const LINEAGE_KEYS: [&str; 3] = ["based_on", "based_on_release_time", "based_on_sha256"];

/// 建一份副本的**文本**。
///
/// - `src_text`：来源文件的原文（原样拷走）；
/// - `based_on`：来源的标签，形如 `mkp/A1MF_260628.toml`（角色目录 + 文件名，
///   **不写绝对路径**：数据根可能被搬走）。
///
/// 摘要与 `based_on_release_time` 都从 `src_text` **自己算/自己读**，不从外面传 ——
/// 外面传就有两处事实，迟早不一致。
///
/// 来源本身已经带血统时（副本再拷副本），旧的三行会被**换掉**而不是叠加：
/// 叠加之后 `parse_header_value` 只会看见第一条，血统就成了骗人的。
pub fn make_copy(src_text: &str, based_on: &str) -> String {
    let sha = sha256_hex(src_text);
    let release_time = crate::read::parse_release_time_from_content(src_text);
    let nl = if src_text.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };

    let stripped = strip_lineage_lines(src_text);
    let at = header_block_end(&stripped);

    let mut block = String::new();
    block.push_str(&format!("# based_on: {based_on}{nl}"));
    if let Some(rt) = release_time {
        block.push_str(&format!("# based_on_release_time: {rt}{nl}"));
    }
    block.push_str(&format!("# based_on_sha256: {sha}{nl}"));

    let mut out = String::with_capacity(stripped.len() + block.len());
    out.push_str(&stripped[..at]);
    out.push_str(&block);
    out.push_str(&stripped[at..]);
    out
}

/// 把已有的 `# based_on*` 行整行删掉（含行尾），其余字节原样。
fn strip_lineage_lines(text: &str) -> String {
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
/// 一条注释都没有（理论上读不出 `# machine:`，读取面会先报错）⇒ 插在文件最前面。
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
/// 放在产品代码里（而不是判据里）的理由：它与 `make_copy` 的插入规则是**同一份**知识
/// （哪三行、怎么认），两处各写一遍必然漂移。
pub fn strip_lineage_for_compare(text: &str) -> String {
    strip_lineage_lines(text)
}

/// 一个键在「我的副本 / 当初的基线 / 官方现在」三者之间的状态。
///
/// **为什么必须有基线这一档**：只比「我的」与「官方的」分不出
/// 「官方改了而我没动过这项」（可以放心采纳）与「两边都改了」（真冲突，只能人来选）。
/// 分不出的后果只有两种：要么把用户的值悄悄盖掉，要么每一项都得问一遍。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum DiffState {
    /// 只有官方变了（我这一项还是当初拷来的值）⇒ 采纳是安全的，允许批量
    OfficialOnly,
    /// 只有我改了（官方没动）⇒ 保持，什么都不用做
    MineOnly,
    /// 两边都变了 ⇒ **真冲突**，只能逐项选，界面上不许给批量按钮
    BothChanged,
    /// 官方新增了这个键，我的副本里没有
    OfficialAdded,
    /// 官方移除了这个键，我的副本里还留着
    OfficialRemoved,
}

/// 一项差异。三个值都可能缺（键不在那一份里，或者基线不可用）。
#[derive(Debug, Clone, PartialEq)]
pub struct FieldDiff {
    /// `段.键`，与表单的 `path` 同形（`toolhead.offset.x`）
    pub path: String,
    pub mine: Option<crate::write::EditValue>,
    /// 当初拷来时的值。基线不可用 ⇒ `None`，此时状态一律保守按 `BothChanged` 算
    pub base: Option<crate::write::EditValue>,
    pub official: Option<crate::write::EditValue>,
    pub state: DiffState,
}

/// 比差异。**纯计算**：三份文本进、一串差异出，不碰文件系统。
///
/// - `base_text`：建副本那一刻来源的全文。拿不到就传 `None`（老副本、手工拷的副本）——
///   那时任何不同都算 `BothChanged`：**多问一句，而不是丢用户的值**。
/// - 值相同的键不出现在结果里；「两边都改成了同一个值」也不算差异（没什么要决定的）。
/// - 键的增删按**键在不在文件里**判，不按值判：`0` 与「没有这一项」不是一回事
///   （注册表的零值豁免已经证明过这个坑）。
pub fn diff(
    mine_text: &str,
    base_text: Option<&str>,
    official_text: &str,
) -> Result<Vec<FieldDiff>, postprocess::diag::PostprocError> {
    use std::collections::BTreeMap;

    type Row = (bool, Option<crate::write::EditValue>);
    fn index(text: &str) -> Result<BTreeMap<String, Row>, postprocess::diag::PostprocError> {
        Ok(crate::write::snapshot(text)?
            .into_iter()
            .map(|s| (format!("{}.{}", s.section, s.key), (s.present, s.value)))
            .collect())
    }

    let mine = index(mine_text)?;
    let official = index(official_text)?;
    let base = match base_text {
        Some(t) => Some(index(t)?),
        None => None,
    };

    let mut out = Vec::new();
    for (path, (mine_present, mine_value)) in &mine {
        // 两侧的键集合都来自同一份 `EDITABLE_KEYS`，查不到只可能是那份清单变了
        let Some((official_present, official_value)) = official.get(path) else {
            continue;
        };
        let base_row = base.as_ref().and_then(|b| b.get(path));
        let state = match (mine_present, official_present) {
            (false, true) => DiffState::OfficialAdded,
            (true, false) => DiffState::OfficialRemoved,
            (false, false) => continue, // 两边都没有这一项
            (true, true) => {
                if mine_value == official_value {
                    continue;
                }
                match base_row {
                    Some((true, base_value)) if base_value == mine_value => DiffState::OfficialOnly,
                    Some((true, base_value)) if base_value == official_value => DiffState::MineOnly,
                    _ => DiffState::BothChanged,
                }
            }
        };
        out.push(FieldDiff {
            path: path.clone(),
            mine: mine_value.clone(),
            base: base_row.and_then(|(present, v)| if *present { v.clone() } else { None }),
            official: official_value.clone(),
            state,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SRC: &str = "\
# uuid: 1111
# release_time: 2026-08-19 01:38:13
# machine: A1
# variant: standard

[toolhead]
offset = { x = -1, y = 18.6, z = 4 } # 笔尖偏移
";

    #[test]
    fn copy_keeps_everything_but_the_three_lines() {
        let copy = make_copy(SRC, "mkp/A1.toml");
        assert_eq!(
            strip_lineage_for_compare(&copy),
            SRC,
            "剪掉三行之后必须与来源逐字节相同"
        );
        // 三行紧贴头注释块，不是被空行隔开塞到 [toolhead] 上
        let head: Vec<&str> = copy.lines().take(7).collect();
        assert_eq!(head[3], "# variant: standard");
        assert_eq!(head[4], "# based_on: mkp/A1.toml");
        assert_eq!(head[5], "# based_on_release_time: 2026-08-19 01:38:13");
        assert!(head[6].starts_with("# based_on_sha256: "));
    }

    #[test]
    fn copying_a_copy_replaces_the_old_lineage() {
        let first = make_copy(SRC, "mkp/A1.toml");
        let second = make_copy(&first, "mine/A1.toml");
        assert_eq!(
            second.matches("# based_on:").count(),
            1,
            "旧血统必须被换掉而不是叠加 —— 叠加之后解析只看见第一条，血统就成了骗人的"
        );
        assert!(second.contains("# based_on: mine/A1.toml"));
        // 第二次的摘要算的是**第一份副本**的全文，不是原始来源的
        assert!(second.contains(&format!("# based_on_sha256: {}", sha256_hex(&first))));
    }

    #[test]
    fn crlf_source_keeps_crlf() {
        let crlf = SRC.replace('\n', "\r\n");
        let copy = make_copy(&crlf, "mkp/A1.toml");
        assert!(
            copy.contains("# based_on: mkp/A1.toml\r\n"),
            "行尾要跟着来源"
        );
        assert!(
            !copy.contains("mkp/A1.toml\n#") || copy.contains("\r\n"),
            "不许混进裸 \\n"
        );
        assert_eq!(strip_lineage_for_compare(&copy), crlf);
    }

    #[test]
    fn sha_is_stable_and_content_sensitive() {
        assert_eq!(sha256_hex(SRC), sha256_hex(SRC));
        assert_ne!(sha256_hex(SRC), sha256_hex(&SRC.replace("-1", "-2")));
        assert_eq!(sha256_hex("").len(), 64, "hex 摘要恒 64 字符");
    }

    #[test]
    fn a_source_without_release_time_gets_two_lines() {
        let no_rt = SRC.replacen("# release_time: 2026-08-19 01:38:13\n", "", 1);
        let copy = make_copy(&no_rt, "mkp/A1.toml");
        assert!(
            !copy.contains("based_on_release_time"),
            "不知道就不要写这一行"
        );
        assert!(copy.contains("# based_on: mkp/A1.toml"));
        assert_eq!(strip_lineage_for_compare(&copy), no_rt);
    }
}
