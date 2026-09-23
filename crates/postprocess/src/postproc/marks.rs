//! MKP 标记协议 v1 —— 发射端。
//!
//! 判据不在这里，在 `machine-motion/protocol/mkp-marks.v1.json`（SSOT）。
//! 本模块只做两件事：
//!   ① 提供 `;MKP_BEGIN` / `;MKP_END` 的构造器（供 pass1 的涂胶块调用）；
//!   ② [`finalize`] —— 落盘前的收尾：`ev` 重编号 + 文件头 `;MKP_INFO` + 文件尾 `;MKP_STATS`。
//!
//! 与 `pass1::stages` 里那套 `;MKP_STAGE:` 的关系：**并存，互不干涉**。
//! 那套是历史的人读注释，不参与配对，观看端归 `LEGACY_MARK`（只报数）。
//! 所以 [`is_mark_line`] **刻意不认** `;MKP_STAGE:`。
//!
//! 为什么 `ev` 要在最后重编号，而不是发射时就编好：底面涂胶块由 pass1 存进
//! `ir.state.bottom_glue_entries`、由 pass2 在命中 `target_z` 时才落地，落点可能
//! **早于**同一轮先内联写出的主块，于是发射顺序 ≠ 最终行序，`EV_MONO` 会红。
//! 重编号只改 `ev=` 的数值，不插入 / 不移动 / 不删除任何标记行。

use std::collections::HashMap;

use crate::gcode::format_z;

/// 协议版本。加 kind / 加 key 都不动它，只有语法本身变了才 +1。
pub const VERSION: i64 = 1;

/// `tool=` 的值：出现在 `;MKP_INFO` 里，纯排查用。
const TOOL: &str = "mkp-ssr";

const BEGIN_PREFIX: &str = ";MKP_BEGIN ";
const END_PREFIX: &str = ";MKP_END ";
const INFO_PREFIX: &str = ";MKP_INFO ";
const STATS_PREFIX: &str = ";MKP_STATS ";

/// 参与协议语法校验的五个 op。`;MKP_STAGE:` 等历史标记不在其中。
const OPS: [&str; 5] = [
    ";MKP_INFO",
    ";MKP_STATS",
    ";MKP_BEGIN",
    ";MKP_END",
    ";MKP_POINT",
];

/// 配置块结束行 —— `;MKP_INFO` 插在它后面（切片器产物里都有这一行）。
const CONFIG_BLOCK_END: &str = "CONFIG_BLOCK_END";

/// 是否是协议标记行（五个 op 之一，且 `;` 顶格独占一行）。
///
/// 传入的应当是 `trim()` 过的行。pass2 的跳段穿透靠它放行标记。
pub fn is_mark_line(trimmed: &str) -> bool {
    OPS.iter().any(|op| {
        trimmed
            .strip_prefix(op)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    })
}

/// `;MKP_BEGIN swap ev=<ev> to=<to> z=<z>`（`to` 取 `pen` / `print`）。
pub fn swap_begin(ev: i64, to: &str, z: f64) -> String {
    format!("{BEGIN_PREFIX}swap ev={ev} to={to} z={}", format_z(z))
}

/// `;MKP_BEGIN paint ev=<ev> z=<z>`。
pub fn paint_begin(ev: i64, z: f64) -> String {
    format!("{BEGIN_PREFIX}paint ev={ev} z={}", format_z(z))
}

/// `;MKP_END <kind>`。
pub fn end(kind: &str) -> String {
    format!("{END_PREFIX}{kind}")
}

/// 从一份产物里剔除所有协议标记行，返回剩下的字节。
///
/// **这是判据侧的唯一落点。** 整链参考物
/// （`crates/core/tests/golden/42274.2.e2e-A1.reference.gcode`）由**来源仓库的 CLI**
/// 生成、本仓无法复现，拿本侧输出覆盖它等于把判据变成自比自（彻底空转）。所以那批
/// 「与参考物逐字节相同」的判据一律改成先过这个函数，判据由
/// 「逐字节相同」收窄为「**除新增标记行外**逐字节相同」—— 比原判据更强，
/// 因为它同时证明了标记是纯增量，没有顺带移动或改写任何一行原有输出。
///
/// 全仓有 8 处这样的比对（core / preset / src-tauri 的 5 个测试文件）外加两个 Python
/// 判据脚本。**函数只许有这一份** —— 判据分散成八份实现，就等于八个可能各自跑偏的口径
/// （同 `scripts/prune.py` 那次的教训：判断本身分成两份，「两道闸咬同一条不变量」
/// 那句话就不成立）。Python 那两个脚本没法调 Rust，只能各自照抄一行过滤，
/// 但它们的口径必须与这里一致：**只认五个 op，不碰 `;MKP_STAGE:`**。
///
/// 行尾风格原样保留（用 `split_inclusive`，不重新拼 `\n`）：参考物是 LF，
/// 重拼会把 CRLF 悄悄改掉，然后以「长度不同」的形式假红。
pub fn strip_mark_lines(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    text.split_inclusive('\n')
        .filter(|line| !is_mark_line(line.trim_end_matches(['\r', '\n']).trim()))
        .collect::<String>()
        .into_bytes()
}

/// 落盘前的收尾。**必须是最后一道行变换** —— STATS 数的就是落盘的那份，
/// `STATS_MATCH` 靠这个保证。
///
/// `preset` 写进 `;MKP_INFO preset=`（一般是机型串）；为空则该键省略。
pub fn finalize(mut lines: Vec<String>, preset: &str) -> Vec<String> {
    let counts = scan_and_renumber(&mut lines);

    let info = info_line(preset);
    match lines
        .iter()
        .position(|l| l.trim_start().starts_with(';') && l.contains(CONFIG_BLOCK_END))
    {
        Some(idx) => lines.insert(idx + 1, info),
        None => lines.insert(0, info),
    }

    lines.push(counts.stats_line());
    lines
}

/// 观看端「别名表」里的塔区间：**它认现有的成对注释，所以本仓一行没为它改过**。
/// 这里只是为了把条数报进 `;MKP_STATS` 与它对账。
///
/// **必须整行精确相等**，不能 `starts_with` —— `;Tower_Layer_Gcode` 是
/// `;Tower_Layer_Gcode Finished` 的前缀，用前缀匹配会把结束行也当成开始行，
/// 计数翻倍还错位。
const TOWER_ALIASES: &[(&str, &str)] = &[
    (";Tower_Base_Layer_Gcode", ";Tower Base Layer Finished"),
    (";Tower_Layer_Gcode", ";Tower_Layer_Gcode Finished"),
];

/// 区间条数（`;MKP_STATS` 的内容）。
#[derive(Debug, Default, PartialEq, Eq)]
struct Counts {
    events: usize,
    swaps: usize,
    paints: usize,
    /// 别名认下来的塔区间数（两种写法各算一段，与观看端口径一致）。
    towers: usize,
}

impl Counts {
    fn stats_line(&self) -> String {
        format!(
            "{STATS_PREFIX}events={} swaps={} paints={} towers={}",
            self.events, self.swaps, self.paints, self.towers
        )
    }
}

/// 一次遍历干两件事：把 `ev` 按**当前行序**重编号，同时数出各类区间的条数。
///
/// `ev` 重编号：按 `;MKP_BEGIN` 首次出现映射成 1,2,3…。同一次事件的三个区间共享
/// 同一个旧 `ev`，所以映射后仍共享同一个新 `ev`，`EV_SHAPE` 不受影响。
/// `;MKP_END` 不带 `ev`，无需回写。
///
/// 塔计数：**只在看到配对的结束行时 +1** —— 与观看端 lint 的口径一致
/// （它也是在 END 处才把 span 收进结果）。孤立的开始行不计数，
/// 这样「塔块被管线丢了一半」会如实反映成少一段，而不是被我们悄悄补上。
fn scan_and_renumber(lines: &mut [String]) -> Counts {
    let mut remap: HashMap<String, i64> = HashMap::new();
    let mut next_ev = 1i64;
    let mut counts = Counts::default();
    // 别名区间不嵌套，所以「当前开着哪一种」用一个下标就够。
    let mut open_alias: Option<usize> = None;

    for line in lines.iter_mut() {
        let trimmed = line.trim();

        match open_alias {
            Some(i) if trimmed == TOWER_ALIASES[i].1 => {
                counts.towers += 1;
                open_alias = None;
            }
            None => {
                if let Some(i) = TOWER_ALIASES.iter().position(|(b, _)| trimmed == *b) {
                    open_alias = Some(i);
                }
            }
            _ => {}
        }

        let Some(kind) = begin_kind(line) else {
            continue;
        };
        match kind.as_str() {
            "swap" => counts.swaps += 1,
            "paint" => counts.paints += 1,
            _ => {}
        }
        if let Some(old_ev) = kv_value(line, "ev") {
            let new_ev = *remap.entry(old_ev).or_insert_with(|| {
                let v = next_ev;
                next_ev += 1;
                v
            });
            *line = replace_kv(line, "ev", &new_ev.to_string());
        }
    }

    counts.events = remap.len();
    counts
}

/// `;MKP_BEGIN <kind> …` → `Some(kind)`。
fn begin_kind(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix(BEGIN_PREFIX)?;
    let kind = rest.split_whitespace().next()?;
    Some(kind.to_string())
}

/// 取 `key=value` 的值（值里不含空格，协议规定）。
fn kv_value(line: &str, key: &str) -> Option<String> {
    let needle = format!(" {key}=");
    let start = line.find(&needle)? + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    Some(rest[..end].to_string())
}

/// 就地替换 `key=value` 的值；找不到该键时原样返回。
fn replace_kv(line: &str, key: &str, value: &str) -> String {
    let needle = format!(" {key}=");
    let Some(pos) = line.find(&needle) else {
        return line.to_string();
    };
    let start = pos + needle.len();
    let rest = &line[start..];
    let end = rest.find(' ').unwrap_or(rest.len());
    format!("{}{}{}", &line[..start], value, &rest[end..])
}

fn info_line(preset: &str) -> String {
    if preset.is_empty() {
        format!("{INFO_PREFIX}version={VERSION} tool={TOOL}")
    } else {
        format!("{INFO_PREFIX}version={VERSION} tool={TOOL} preset={preset}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(lines: &[&str]) -> Vec<String> {
        lines.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn begin_end_format() {
        assert_eq!(
            swap_begin(3, "pen", 2.6),
            ";MKP_BEGIN swap ev=3 to=pen z=2.6"
        );
        assert_eq!(paint_begin(3, 2.6), ";MKP_BEGIN paint ev=3 z=2.6");
        assert_eq!(end("swap"), ";MKP_END swap");
    }

    #[test]
    fn is_mark_line_rejects_legacy_stage() {
        assert!(is_mark_line(";MKP_BEGIN swap ev=1"));
        assert!(is_mark_line(";MKP_END swap"));
        assert!(is_mark_line(";MKP_POINT trigger ev=1"));
        assert!(is_mark_line(";MKP_INFO version=1"));
        assert!(is_mark_line(";MKP_STATS events=1"));
        // 历史标记不参与配对，不该被穿透逻辑放行
        assert!(!is_mark_line(";MKP_STAGE: GlueStart ; 涂胶开始"));
        assert!(!is_mark_line(";Pre-glue preparation"));
        assert!(!is_mark_line("G1 X10 Y10"));
    }

    /// 发射顺序 ≠ 最终行序时（底面涂胶块被 pass2 提前落地），重编号必须把
    /// 最终行序上的第一块编成 ev=1。
    #[test]
    fn renumber_follows_final_line_order() {
        let mut lines = v(&[
            ";MKP_BEGIN swap ev=2 to=pen z=1.0",
            ";MKP_END swap",
            ";MKP_BEGIN paint ev=2 z=1.0",
            ";MKP_END paint",
            ";MKP_BEGIN swap ev=2 to=print z=1.0",
            ";MKP_END swap",
            ";MKP_BEGIN swap ev=1 to=pen z=1.2",
            ";MKP_END swap",
            ";MKP_BEGIN paint ev=1 z=1.2",
            ";MKP_END paint",
            ";MKP_BEGIN swap ev=1 to=print z=1.2",
            ";MKP_END swap",
        ]);
        let counts = scan_and_renumber(&mut lines);

        assert_eq!(lines[0], ";MKP_BEGIN swap ev=1 to=pen z=1.0");
        assert_eq!(lines[2], ";MKP_BEGIN paint ev=1 z=1.0");
        assert_eq!(lines[4], ";MKP_BEGIN swap ev=1 to=print z=1.0");
        assert_eq!(lines[6], ";MKP_BEGIN swap ev=2 to=pen z=1.2");
        assert_eq!(lines[8], ";MKP_BEGIN paint ev=2 z=1.2");
        assert_eq!(lines[10], ";MKP_BEGIN swap ev=2 to=print z=1.2");
        assert_eq!(
            counts,
            Counts {
                events: 2,
                swaps: 4,
                paints: 2,
                towers: 0
            }
        );
    }

    #[test]
    fn finalize_puts_info_after_config_block_end() {
        let out = finalize(
            v(&[
                "; HEADER_BLOCK_END",
                "; CONFIG_BLOCK_END",
                ";MKP_BEGIN swap ev=1 to=pen z=1.0",
                ";MKP_END swap",
                ";MKP_BEGIN paint ev=1 z=1.0",
                ";MKP_END paint",
                ";MKP_BEGIN swap ev=1 to=print z=1.0",
                ";MKP_END swap",
            ]),
            "A1MF",
        );
        assert_eq!(out[2], ";MKP_INFO version=1 tool=mkp-ssr preset=A1MF");
        assert_eq!(
            out[out.len() - 1],
            ";MKP_STATS events=1 swaps=2 paints=1 towers=0"
        );
    }

    #[test]
    fn finalize_falls_back_to_file_head() {
        let out = finalize(v(&["G1 X0 Y0", "G1 X1 Y1"]), "");
        assert_eq!(out[0], ";MKP_INFO version=1 tool=mkp-ssr");
        assert_eq!(
            out[out.len() - 1],
            ";MKP_STATS events=0 swaps=0 paints=0 towers=0"
        );
    }

    /// 塔按**别名的成对注释**数，两种写法各算一段 —— 与观看端 lint 同一个口径。
    #[test]
    fn tower_alias_spans_are_counted_per_pair() {
        let mut lines = v(&[
            ";Tower_Base_Layer_Gcode",
            "G1 X1 Y1 E1",
            ";Tower Base Layer Finished",
            ";Tower_Layer_Gcode",
            "G1 X2 Y2 E1",
            ";Tower_Layer_Gcode Finished",
            ";Tower_Layer_Gcode",
            "G1 X3 Y3 E1",
            ";Tower_Layer_Gcode Finished",
        ]);
        assert_eq!(scan_and_renumber(&mut lines).towers, 3);
    }

    /// 只在**闭合**时 +1：塔块被管线丢了一半，就该如实少一段，不许我们悄悄补上。
    #[test]
    fn an_unclosed_tower_span_is_not_counted() {
        let mut lines = v(&[
            ";Tower_Layer_Gcode",
            "G1 X1 Y1 E1",
            ";Tower_Layer_Gcode Finished",
            ";Tower_Layer_Gcode", // 结束行被丢了
            "G1 X2 Y2 E1",
        ]);
        assert_eq!(scan_and_renumber(&mut lines).towers, 1);
    }

    /// `;Tower_Layer_Gcode` 是结束行的**前缀** —— 用 `starts_with` 会把结束行当成开始行，
    /// 计数翻倍还错位。这条判据钉住「整行精确相等」。
    #[test]
    fn the_finished_line_is_not_mistaken_for_a_start() {
        // 只有结束行、没有开始行 ⇒ 一段都不算
        let mut only_end = v(&[";Tower_Layer_Gcode Finished"]);
        assert_eq!(scan_and_renumber(&mut only_end).towers, 0);

        // 正常一对 ⇒ 恰好一段（若前缀误判，这里会数成 0 或 2）
        let mut one_pair = v(&[";Tower_Layer_Gcode", ";Tower_Layer_Gcode Finished"]);
        assert_eq!(scan_and_renumber(&mut one_pair).towers, 1);
    }

    /// 判据侧那唯一一份剔除函数：只拿掉五个 op，历史标记与正文一行不动。
    #[test]
    fn strip_mark_lines_removes_only_protocol_marks() {
        let src = concat!(
            "; CONFIG_BLOCK_END\n",
            ";MKP_INFO version=1 tool=mkp-ssr\n",
            "G1 X0 Y0\n",
            ";Pre-glue preparation\n",
            ";MKP_BEGIN swap ev=1 to=pen z=0.6\n",
            ";MKP_STAGE: RiseNozzle ; 抬升喷嘴\n",
            ";MKP_END swap\n",
            ";Tower_Layer_Gcode\n",
            ";MKP_STATS events=1 swaps=2 paints=1 towers=1\n",
        );
        let want = concat!(
            "; CONFIG_BLOCK_END\n",
            "G1 X0 Y0\n",
            ";Pre-glue preparation\n",
            ";MKP_STAGE: RiseNozzle ; 抬升喷嘴\n",
            ";Tower_Layer_Gcode\n",
        );
        assert_eq!(
            String::from_utf8(strip_mark_lines(src.as_bytes())).expect("utf8"),
            want
        );
    }

    /// 行尾风格必须原样留着：重拼 `\n` 会把 CRLF 悄悄改掉，然后以「长度不同」的形式假红。
    #[test]
    fn strip_mark_lines_keeps_line_endings() {
        let src = "G1 X0\r\n;MKP_END swap\r\nG1 X1\r\n";
        assert_eq!(
            String::from_utf8(strip_mark_lines(src.as_bytes())).expect("utf8"),
            "G1 X0\r\nG1 X1\r\n"
        );
        // 最后一行没有结尾换行时，不许被补上
        let no_eol = "G1 X0\n;MKP_END swap\nG1 X1";
        assert_eq!(
            String::from_utf8(strip_mark_lines(no_eol.as_bytes())).expect("utf8"),
            "G1 X0\nG1 X1"
        );
    }

    #[test]
    fn finalize_leaves_legacy_stage_untouched() {
        let out = finalize(v(&[";MKP_STAGE: GlueStart ; 涂胶开始"]), "");
        assert!(out.iter().any(|l| l == ";MKP_STAGE: GlueStart ; 涂胶开始"));
    }
}
