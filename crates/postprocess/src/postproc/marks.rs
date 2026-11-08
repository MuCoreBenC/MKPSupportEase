//! MKP 标记协议 v1 —— 发射端。
//!
//! 判据不在这里，在 `machine-motion/protocol/mkp-marks.v1.json`（SSOT）。
//! 本模块做三件事：
//!   ① 提供 `;MKP_BEGIN` / `;MKP_END` 的构造器（供 pass1 的涂胶块调用）；
//!   ② [`finalize`] —— 落盘前的收尾：`ev` 重编号 + 文件头 `;MKP_INFO` + 文件尾 `;MKP_STATS`；
//!   ③ [`finalize`] 同时把**层数声明**改成与文件内容一致（见 `reconciled_line`）。
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

use crate::diag::PostprocError;
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

/// 头部块结束行 —— 补写的层数声明插在它前面（切片器产物里都有这一行）。
const HEADER_BLOCK_END: &str = "HEADER_BLOCK_END";

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

/// 剔除「换笔块」（`;Pre-glue preparation` → `;Glueing Started` 之前）后再比对。
///
/// 2026-10-20 起这一段是**有意偏离**来源实现的（见
/// `docs/GLUE-TRAVEL-Z-SAFETY-2026-10-20.md`）：原来把去切笔位的横移写成
/// `G1 X.. Z{层顶+1.0}` 的 XY+Z 斜插（喷嘴从零净空起步、贴面滑行 120mm），
/// 装好笔又落回 `last+offset_z+3.0`（笔尖净空只剩 2.8mm），高层会撞倒模型。
/// 改成「先抬后走 + 与收笔同一净空」后，该段的结构与 Z 值必然与参考物不同。
///
/// 与 [`strip_mark_lines`] 同一条纪律：**判据侧只有这一份实现**。剔除整段而不是
/// 用本侧输出覆盖参考物 —— 覆盖等于把整链判据变成自比自（2026-09-12 踩过）。
/// 剔除后仍能证明「换笔段之外的所有输出逐字节一致」；换笔段本身由
/// `tests/travel_clearance.rs`（净空不等式）与 G1/G2 golden（逐字节）分别守着。
///
/// 行尾风格原样保留（`split_inclusive`，与 [`strip_mark_lines`] 同一个理由）。
pub fn strip_swap_travel_blocks(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    let mut skipping = false;
    for line in text.split_inclusive('\n') {
        let t = line.trim_end();
        if !skipping && t == ";Pre-glue preparation" {
            skipping = true;
            continue;
        }
        if skipping {
            if t == ";Glueing Started" {
                skipping = false;
                out.push_str(line);
            }
            continue;
        }
        out.push_str(line);
    }
    out.into_bytes()
}

/// 剔除涂胶段（`;Glueing Started` ~ `;Glueing Finished`）里的**抬笔/落笔与走速**。
///
/// 2026-11-03 起涂胶段内的 `G1 Z`（抬笔/落笔）与配对 `G1 F` 由发射层按「笔尖净空」
/// 统一给出：每段空走先抬后走，落笔后才涂，并保留切片器拆开的多条空走。这使输出
/// 与 `mkp-sr` 参考在涂胶段内必然不同（实测 42274.2：只增不减 Z/F，**XY 与 E 一字不动**）。
///
/// 所以整链判据改问「涂胶段的几何有没有变」：两边都剔掉 Z/F 再逐字节比，
/// XY/E 仍然要求完全一致。抬升高度由 `travel_clearance` 管，块内连续性由
/// `glue_block::continuity_tests` 与离线诊断管。
///
/// 分隔符用 `;Glueing Started/Finished`（两侧都有），不能用 `;MKP_BEGIN paint`
/// —— 参考物是外部产物，里面没有 MKP 标记。
pub fn strip_glue_section_vertical_moves(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let mut out = String::with_capacity(text.len());
    let mut in_glue = false;
    let mut last_xy: Option<(f64, f64)> = None;
    for line in text.split_inclusive('\n') {
        let t = line.trim_end();
        if t == ";Glueing Finished" {
            in_glue = false;
            out.push_str(line);
            continue;
        }
        if in_glue {
            let code = t.split(';').next().unwrap_or("").trim();
            /*
             * 0mm 移动也是噪声：2026-11-03 起发射层会把「与回位同点的定位行」丢掉，
             * 参考物里还留着它们。两边都不看，判据仍然咬住真实几何。
             */
            let zero_len = {
                let mut nx = None::<f64>;
                let mut ny = None::<f64>;
                for tok in code.split_whitespace().skip(1) {
                    if let Some(v) = tok.strip_prefix('X').and_then(|v| v.parse::<f64>().ok()) {
                        nx = Some(v);
                    }
                    if let Some(v) = tok.strip_prefix('Y').and_then(|v| v.parse::<f64>().ok()) {
                        ny = Some(v);
                    }
                }
                match (nx, ny, last_xy) {
                    (Some(x), Some(y), Some((px, py))) => {
                        (x - px).abs() < 1e-6 && (y - py).abs() < 1e-6
                    }
                    _ => false,
                }
            };
            if zero_len {
                continue;
            }
            let z_only = code.starts_with("G1 Z") && !code.contains('X') && !code.contains('Y');
            let f_only = code.starts_with("G1 F")
                && !code.contains('X')
                && !code.contains('Y')
                && !code.contains('Z');
            if z_only || f_only {
                continue;
            }
        }
        if in_glue {
            let code = t.split(';').next().unwrap_or("").trim();
            let mut nx = None::<f64>;
            let mut ny = None::<f64>;
            for tok in code.split_whitespace().skip(1) {
                if let Some(v) = tok.strip_prefix('X').and_then(|v| v.parse::<f64>().ok()) {
                    nx = Some(v);
                }
                if let Some(v) = tok.strip_prefix('Y').and_then(|v| v.parse::<f64>().ok()) {
                    ny = Some(v);
                }
            }
            if let (Some(x), Some(y)) = (nx, ny) {
                last_xy = Some((x, y));
            }
        }
        out.push_str(line);
        if t == ";Glueing Started" {
            in_glue = true;
            last_xy = None;
        }
    }
    out.into_bytes()
}

/// 剔除「塔块结束后的 feature 恢复行」。
///
/// 2026-11-04 起：塔块写了自己的 `; FEATURE: Inner wall`（预览里那个黄，用户明确要留），
/// 而原文回到模型段时**不重发** `; FEATURE:`（Bambu 只在 feature 变化时才写）。
/// 于是从塔块到本层结束的所有模型走线都挂在「内墙」名下 —— 预览里模型支撑被染成内墙黄，
/// `切片结果` 面板的内墙占比也跟着虚高。修法是塔块末尾补一条
/// `; FEATURE: <切片器当前那一档>`，位置正好是 Bambu 自己会写的那一处。
///
/// 这一条是本侧**有意新增**的行：整链参考物由来源仓库的 CLI 生成、本仓无法复现，
/// 不能拿本侧输出覆盖它，所以判据先把它剔掉，再要求其余仍逐字节一致。
///
/// 认法只认 MKP 自己写的锚，三行紧邻：
/// `;Leaving Wiping Tower` → `; LAYER_HEIGHT:` → `; FEATURE: <name>`。
/// 切片器原生的 `; FEATURE:` 前面没有这个组合，不会误伤。
pub fn strip_tower_feature_restore(bytes: &[u8]) -> Vec<u8> {
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut out = String::with_capacity(text.len());
    let mut prev1 = String::new();
    let mut prev2 = String::new();
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim_end();
        /* 两种锚，都只认 MKP 自己的组合，切片器原生注释旁边没有它们：
        ① 塔层块收尾：`;Leaving Wiping Tower` → `; LAYER_HEIGHT:` → `; FEATURE:`
        ② 护套首层开头：`; FEATURE: …` 的**下一行**是 `; === Trapezoidal Sheath Start ===` */
        let next = lines.get(i + 1).map(|l| l.trim_end()).unwrap_or_default();
        let restore = t.starts_with("; FEATURE: ")
            && ((prev1.starts_with("; LAYER_HEIGHT:") && prev2.contains(";Leaving Wiping Tower"))
                || next.starts_with("; === Trapezoidal Sheath Start ==="));
        if !restore {
            out.push_str(line);
        }
        prev2 = std::mem::replace(&mut prev1, t.to_string());
    }
    out.into_bytes()
}

/// 把**塔区间内**的 Z 数值抹成 `Z0`（其余字节原样保留）。
///
/// 塔打在**本层模型层顶**（见 `pass2` 的 `Ctx::layer_z`），参考物 `mkp-sr` 打在自己
/// 累加的记账高度上 —— 塔的入口抬升、`;Safe Z`、`;Tower Z`、`;Print Z`、离开点、螺旋抬升
/// 全跟着差一截。这跟换笔段 / 涂胶段是同一类「有意偏离」，所以判据里不参与比对。
///
/// 区间怎么划：`;Tower_Layer_Gcode` 往前 4 行（入口 `G1 F…` / `G1 X.. Y.. Z<safe>` /
/// `; FEATURE:` / `; LINE_WIDTH:`）到 `;Leaving Wiping Tower` 那一行。往前 4 行是**定值**，
/// 因为入口那几行的条数由塔模板决定，两边一致；模型段一行都没进来。
///
/// 塔本身没有被放走：净空由 `tests/travel_clearance.rs` 管，几何由
/// `tower_matrix.golden`（golden_g3）逐字节管。
pub fn zero_tower_region_z(bytes: &[u8]) -> Vec<u8> {
    /* 入口那几行是塔模板的固定前缀：`; layer num/…`、`G1 F…`、`G1 X.. Y.. Z<safe>`、
    `; FEATURE:`、`; LINE_WIDTH:`、`;Extruding Ratio:`、`; LAYER_HEIGHT:` —— 实测 7 行，
    取 8 留一行余量。两侧条数一致（同一份模板），所以按固定值切不会错位。 */
    const LEAD: usize = 8;
    let text = String::from_utf8_lossy(bytes);
    let lines: Vec<&str> = text.split_inclusive('\n').collect();
    let mut hit = vec![false; lines.len()];
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_end() != ";Tower_Layer_Gcode" {
            i += 1;
            continue;
        }
        let from = i.saturating_sub(LEAD);
        let end = lines
            .iter()
            .enumerate()
            .skip(i)
            .find(|(_, l)| l.trim_end().contains(";Leaving Wiping Tower"))
            .map(|(k, _)| k)
            .unwrap_or(i);
        for slot in hit.iter_mut().take(end + 1).skip(from) {
            *slot = true;
        }
        i = end + 1;
    }

    let mut out = String::with_capacity(text.len());
    for (i, line) in lines.iter().enumerate() {
        if hit[i] {
            out.push_str(&zero_z_token(line));
        } else {
            out.push_str(line);
        }
    }
    out.into_bytes()
}

/// 把一行里第一个 `Z<数字>` 换成 `Z0`；没有 Z 就原样返回。
fn zero_z_token(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut done = false;
    let mut rest = line;
    while let Some(pos) = rest.find('Z') {
        let (head, tail) = rest.split_at(pos);
        out.push_str(head);
        let after = &tail[1..];
        let num_len = after
            .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-'))
            .unwrap_or(after.len());
        if done || num_len == 0 || !after[..num_len].starts_with(|c: char| c.is_ascii_digit()) {
            out.push('Z');
            rest = after;
            continue;
        }
        out.push_str("Z0");
        rest = &after[num_len..];
        done = true;
    }
    out.push_str(rest);
    out
}

/// 落盘前的收尾。**必须是最后一道行变换** —— STATS 数的就是落盘的那份，
/// `STATS_MATCH` 靠这个保证。
///
/// `preset` 写进 `;MKP_INFO preset=`（一般是机型串）；为空则该键省略。
pub fn finalize(mut lines: Vec<String>, preset: &str) -> Result<Finalized, PostprocError> {
    let counts = scan_and_renumber(&mut lines);
    let declared = reconcile_layer_declarations(&mut lines)?;

    let info = info_line(preset);
    match lines
        .iter()
        .position(|l| l.trim_start().starts_with(';') && l.contains(CONFIG_BLOCK_END))
    {
        Some(idx) => lines.insert(idx + 1, info),
        None => lines.insert(0, info),
    }

    lines.push(counts.stats_line());
    Ok(Finalized {
        lines,
        layers: declared.layers as i64,
        header_added: declared.header_added,
        scarf_flattened: declared.scarf_flattened,
    })
}

/// `finalize` 的产物：要落盘的行 + 实测层数的对账结果。
#[derive(Debug)]
pub struct Finalized {
    pub lines: Vec<String>,
    /// 成品真实的层数（`; CHANGE_LAYER` 条数）。报告里的 `totalLayerNumber` 用它。
    pub layers: i64,
    /// 输入原本没有 `; total layer number:` 这一行，是按实测值补写的。
    pub header_added: bool,
    /// 把 `; has_scarf_joint_seam` 从非 0 改成了 0（观看端层表兼容）。
    pub scarf_flattened: bool,
}

/// 层数**声明**的两种形态。`; CHANGE_LAYER` / `; Z_HEIGHT:` 是内容锚，不在这里改。
const TOTAL_LAYER_PREFIX: &str = "; total layer number:";
const LAYER_NUM_PREFIX: &str = "; layer num/total_layer_count:";
/// 内容锚之一：一层开始的标记。
const CHANGE_LAYER_MARKER: &str = "; CHANGE_LAYER";
/// 内容锚之二：该层的 Z 高度。
const Z_HEIGHT_PREFIX: &str = "; Z_HEIGHT:";
/// 观看端（Bambu Studio）选哪种口径定层的开关，见 [`flatten_scarf_seam`]。
const HAS_SCARF_JOINT_SEAM_PREFIX: &str = "; has_scarf_joint_seam =";

/// 把 `; has_scarf_joint_seam` 压成 0，让观看端按 **Z 平台段** 定层。
///
/// Bambu Studio 的层表由 `m_detect_layer_based_on_tag` 二选一（`GCodeProcessor.cpp:2052-2056`）：
///
/// - `; spiral_vase = 1` 或 **`; has_scarf_joint_seam = 1`** ⇒ 每个 `; CHANGE_LAYER` 一层、
///   层 Z 取层内挤出 Z 的最大值（`:3524-3536` / `:4409-4421`）；
/// - 两者都 0 ⇒ 按挤出移动的 Z 平台段数层（`AdvancedRenderer.cpp:947-960`，容差 `1e-4`）。
///
/// 开了斜接缝的文件里，同一层的挤出 Z 是一条斜坡。tag 口径下每条 `; CHANGE_LAYER` 只留
/// 一个层 Z，而那串 Z 与层内的实际几何对不上 —— 实测预览会把模型与擦料塔的上半截整个画不出来
/// （层滑块已拉满，图上却只到三分之一高）。压成 0 之后它按平台段数层：层数会变多
/// （斜坡被逐段数开，实测 627 层 → 约 1218 段），但每一段都对得上实际几何，画得全。
///
/// 代价写在明处：这一格本来是切片器的设置记录，成品里它与你切片的真实设置不再一致。
/// 调用方要把这件事报进 `ir.warnings`，不能悄悄改。
fn flatten_scarf_seam(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let value = trimmed.strip_prefix(HAS_SCARF_JOINT_SEAM_PREFIX)?;
    let parsed = value.trim().parse::<f64>().ok()?;
    if parsed == 0.0 {
        return None;
    }
    Some(format!(
        "{}{HAS_SCARF_JOINT_SEAM_PREFIX} 0",
        indent_of(line)
    ))
}

fn indent_of(line: &str) -> &str {
    &line[..line.len() - line.trim_start().len()]
}

/// 若这一行是层数声明，返回它**应当**写成的样子；不是则 `None`。
///
/// 两种声明各有各的错法：头部 `; total layer number:` 是切片器按别的算法算出来的，
/// 逐层 `; layer num/total_layer_count: k/M` 的分母来自 `layer_change_gcode` 模板里的
/// `[total_layer_count]`。后处理会往层间插抬笔落笔，两者都可能与成品内容对不上 ——
/// 采集端（Bambu Studio 的预览）遇到互相矛盾的声明会自己挑一个信，同一个文件于是
/// 能显示成两个层数。这里让它们只可能等于内容：`total` 由 [`reconcile_layer_declarations`]
/// 从 `; CHANGE_LAYER` 数出来，`layer_no` 是按出现顺序重编的序号。
fn reconciled_line(line: &str, layer_no: usize, total: usize) -> Option<String> {
    let trimmed = line.trim_start();
    let indent = indent_of(line);
    if trimmed.starts_with(TOTAL_LAYER_PREFIX) {
        return Some(format!("{indent}{TOTAL_LAYER_PREFIX} {total}"));
    }
    if trimmed.starts_with(LAYER_NUM_PREFIX) {
        return Some(format!("{indent}{LAYER_NUM_PREFIX} {layer_no}/{total}"));
    }
    None
}

/// 层数声明与内容的对账结果。
struct Declared {
    layers: usize,
    header_added: bool,
    scarf_flattened: bool,
}

/// 把层数声明改成与文件内容一致。
///
/// 口径只有一个：`; CHANGE_LAYER` 的条数。`; Z_HEIGHT:` 是**另一条**独立的锚，
/// 两者本该一层一行 —— 都出现了却不等，说明管线里有一层被丢或被复制了，这时直接失败
/// （「缺字段就报错，不许兜底」）：凑出来的层数会把真正的问题盖住。
///
/// 没有 `; CHANGE_LAYER` 的文件（校准片段、判据里手写的几行、非 Bambu 系的切片器）
/// 没有层结构可对账，原样返回 0。
fn reconcile_layer_declarations(lines: &mut Vec<String>) -> Result<Declared, PostprocError> {
    let mut scarf_flattened = false;
    for line in lines.iter_mut() {
        if let Some(rebuilt) = flatten_scarf_seam(line) {
            *line = rebuilt;
            scarf_flattened = true;
        }
    }

    let mut layers = 0usize;
    let mut z_heights = 0usize;
    for line in lines.iter() {
        let trimmed = line.trim_start();
        if trimmed.starts_with(CHANGE_LAYER_MARKER) {
            layers += 1;
        } else if trimmed.starts_with(Z_HEIGHT_PREFIX) {
            z_heights += 1;
        }
    }

    if layers == 0 {
        return Ok(Declared {
            layers: 0,
            header_added: false,
            scarf_flattened,
        });
    }

    if z_heights > 0 && layers != z_heights {
        return Err(PostprocError::Processing {
            message: format!(
                "层数声明与内容对不上：`{CHANGE_LAYER_MARKER}` {layers} 条、\
                 `{Z_HEIGHT_PREFIX}` {z_heights} 条。两者应当一层一条，\
                 不等说明有一层在管线里被丢了或被复制了 —— 拒绝写出层数无法自洽的 G-code。"
            ),
        });
    }

    let mut seen = 0usize;
    for line in lines.iter_mut() {
        if line.trim_start().starts_with(CHANGE_LAYER_MARKER) {
            seen += 1;
            continue;
        }
        if let Some(rebuilt) = reconciled_line(line, seen, layers) {
            *line = rebuilt;
        }
    }

    check_tower_prints_at_layer_z(lines)?;

    let header_added = !lines
        .iter()
        .any(|l| l.trim_start().starts_with(TOTAL_LAYER_PREFIX));
    if header_added {
        let header = format!("{TOTAL_LAYER_PREFIX} {layers}");
        match lines
            .iter()
            .position(|l| l.trim_start().starts_with(';') && l.contains(HEADER_BLOCK_END))
        {
            Some(idx) => lines.insert(idx, header),
            None => lines.insert(0, header),
        }
    }

    Ok(Declared {
        layers,
        header_added,
        scarf_flattened,
    })
}

/// 塔块必须打在**本层自己的 Z** 上。
///
/// 为什么这条非拦不可：Bambu Studio 的层表只认「挤出移动的 Z」——
/// 无斜接缝按 Z 平台段数层（`AdvancedRenderer.cpp:947-960`，容差 `1e-4`），
/// 有斜接缝取每层挤出 Z 的最大值（`GCodeProcessor.cpp:4409-4421`）。
/// 塔原来按自己的记账高度推进（比模型层低 0.04~0.47mm），于是在**没有**斜接缝时
/// 每层多出一个假层（627 层被显示成 1215 层），有斜接缝时又把层表搅乱。
///
/// 认的是塔自己写的两条注释：`G1 Z<x>;Tower Z` 与 `G1 Z<x> ;Print Z`。
/// 容差见 [`TOWER_LAYER_Z_TOLERANCE`] —— 只容忍注释精度，不容忍"各打各的"。
fn check_tower_prints_at_layer_z(lines: &[String]) -> Result<(), PostprocError> {
    let mut layer_z: Option<f64> = None;
    let mut layer_no: usize = 0;

    for line in lines {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(Z_HEIGHT_PREFIX) {
            layer_z = rest.trim().parse::<f64>().ok();
            layer_no += 1;
            continue;
        }
        let Some(layer_z) = layer_z else {
            continue;
        };
        let is_tower_z = trimmed.contains(";Tower Z") || trimmed.contains(";Print Z");
        if !is_tower_z {
            continue;
        }
        // 取 `;` 之前的那段：注释是紧贴着数字写的（`Z0.48;Tower Z`），连注释一起切会解析不出来。
        let code = trimmed.split(';').next().unwrap_or("");
        let Some(z) = code
            .split_whitespace()
            .find_map(|tok| tok.strip_prefix('Z'))
            .and_then(|v| v.parse::<f64>().ok())
        else {
            continue;
        };
        if (z - layer_z).abs() > TOWER_LAYER_Z_TOLERANCE {
            return Err(PostprocError::Processing {
                message: format!(
                    "第 {layer_no} 层的塔打在 Z={z}，而本层是 {layer_z}                     （允许差 {TOWER_LAYER_Z_TOLERANCE}）—— 塔必须在模型层顶落笔，\
                     否则预览的层表会与内容错位。"
                ),
            });
        }
    }
    Ok(())
}

/// 塔 Z 与本层 Z 之间允许的差：**0.01mm**。
///
/// 两边夹出来的数：`; Z_HEIGHT:` 只印 4~5 位小数、塔那几行印 3 位，
/// 同一层本来就可能差到 `5e-4`（实测 `1.07273` vs `1.073`）；而真正"各打各的"至少偏
/// 一整个安全高度（实测 0.04~0.47mm）。0.01 落在这两段之间，离两边各有一个数量级。
const TOWER_LAYER_Z_TOLERANCE: f64 = 1e-2;

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

    fn fin(lines: &[&str], preset: &str) -> Vec<String> {
        finalize(v(lines), preset).expect("finalize").lines
    }

    #[test]
    fn finalize_puts_info_after_config_block_end() {
        let out = fin(
            &[
                "; HEADER_BLOCK_END",
                "; CONFIG_BLOCK_END",
                ";MKP_BEGIN swap ev=1 to=pen z=1.0",
                ";MKP_END swap",
                ";MKP_BEGIN paint ev=1 z=1.0",
                ";MKP_END paint",
                ";MKP_BEGIN swap ev=1 to=print z=1.0",
                ";MKP_END swap",
            ],
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
        let out = fin(&["G1 X0 Y0", "G1 X1 Y1"], "");
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
        let out = fin(&[";MKP_STAGE: GlueStart ; 涂胶开始"], "");
        assert!(out.iter().any(|l| l == ";MKP_STAGE: GlueStart ; 涂胶开始"));
    }

    /// 三层文件：头部声明错、逐层分母错，两层都能被改成内容那一个数。
    const THREE_LAYERS: &[&str] = &[
        "; HEADER_BLOCK_START",
        "; BambuStudio 02.08.02.60",
        "; total layer number: 1323",
        "; HEADER_BLOCK_END",
        "; CONFIG_BLOCK_END",
        "; CHANGE_LAYER",
        "; Z_HEIGHT: 0.2",
        "; layer num/total_layer_count: 1/1323",
        "G1 X1 Y1 E1",
        "; CHANGE_LAYER",
        "; Z_HEIGHT: 0.4",
        "; layer num/total_layer_count: 2/1323",
        "G1 X2 Y2 E1",
        "; CHANGE_LAYER",
        "; Z_HEIGHT: 0.6",
        "; layer num/total_layer_count: 3/1323",
        "G1 X3 Y3 E1",
    ];

    /// 声明只有一个来源：`; CHANGE_LAYER` 的条数。头部与逐层分母都必须落到它上面。
    #[test]
    fn declarations_are_rewritten_to_the_real_layer_count() {
        let out = fin(THREE_LAYERS, "");
        assert_eq!(out[2], "; total layer number: 3");
        let per_layer: Vec<&String> = out
            .iter()
            .filter(|l| l.starts_with("; layer num/total_layer_count:"))
            .collect();
        assert_eq!(
            per_layer,
            vec![
                "; layer num/total_layer_count: 1/3",
                "; layer num/total_layer_count: 2/3",
                "; layer num/total_layer_count: 3/3",
            ]
        );
    }

    #[test]
    fn finalize_reports_the_layer_count_it_wrote() {
        let out = finalize(v(THREE_LAYERS), "").expect("finalize");
        assert_eq!(out.layers, 3);
        assert!(!out.header_added);
    }

    /// 只改声明行：任何一条移动指令都不许被碰。
    #[test]
    fn only_declaration_lines_change() {
        let out = fin(THREE_LAYERS, "");
        let moved: Vec<&String> = out.iter().filter(|l| l.starts_with("G1 ")).collect();
        assert_eq!(moved, vec!["G1 X1 Y1 E1", "G1 X2 Y2 E1", "G1 X3 Y3 E1"]);
        for keep in ["; Z_HEIGHT: 0.2", "; Z_HEIGHT: 0.4", "; Z_HEIGHT: 0.6"] {
            assert!(out.iter().any(|l| l == keep), "{keep} 被改了");
        }
    }

    /// 已经自洽的文件再跑一遍，声明逐字节不变（幂等）。
    ///
    /// `;MKP_INFO` / `;MKP_STATS` 每跑一次都会多一条，所以只比声明与内容。
    #[test]
    fn rewriting_layer_declarations_is_idempotent() {
        let once = fin(THREE_LAYERS, "");
        let twice = fin(&once.iter().map(String::as_str).collect::<Vec<_>>(), "");
        let body = |v: &[String]| {
            v.iter()
                .filter(|l| !is_mark_line(l.trim()))
                .cloned()
                .collect::<Vec<_>>()
        };
        assert_eq!(body(&once), body(&twice));
    }

    /// 一层有一条 `; CHANGE_LAYER`、也有一条 `; Z_HEIGHT:`；不等就是管线丢了层，必须报错。
    #[test]
    fn a_layer_count_mismatch_is_an_error() {
        let err = finalize(
            v(&[
                "; total layer number: 5",
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.2",
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.4",
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.6",
                "; CHANGE_LAYER",
                "; CHANGE_LAYER",
            ]),
            "",
        )
        .expect_err("层数不等必须失败");
        assert_eq!(err.code(), "E_GCODE_FAILED_001");
        let text = err.body();
        assert!(
            text.contains("5") && text.contains("3"),
            "错误里要带两个数：{text}"
        );
    }

    /// 头部声明缺失时补写，并且报出来 —— 不许悄悄少一行。
    #[test]
    fn a_missing_header_is_written_and_reported() {
        let out = finalize(
            v(&[
                "; HEADER_BLOCK_START",
                "; BambuStudio 02.08.02.60",
                "; HEADER_BLOCK_END",
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.2",
            ]),
            "",
        )
        .expect("finalize");
        assert!(out.header_added);
        let end = out
            .lines
            .iter()
            .position(|l| l == "; HEADER_BLOCK_END")
            .expect("头部块没被破坏");
        assert_eq!(out.lines[end - 1], "; total layer number: 1");
    }

    /// 塔按自己的记账高度打（比本层低一截）⇒ 界面上每层多一个假层，必须失败。
    /// 这条正是「627 层被显示成 1215 层」那个回归的护栏。
    #[test]
    fn a_tower_that_prints_off_the_layer_z_is_an_error() {
        let err = finalize(
            v(&[
                "; total layer number: 2",
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.6",
                "G1 Z0.6",
                "G1 X1 Y1 E1",
                ";Tower_Layer_Gcode",
                "G1 Z0.48;Tower Z", // 塔的记账高度，不是本层 0.6
                "G1 X2 Y2 E1",
            ]),
            "",
        )
        .expect_err("塔不在本层 Z 上必须失败");
        assert_eq!(err.code(), "E_GCODE_FAILED_001");
        let text = err.body();
        assert!(
            text.contains("0.48") && text.contains("0.6"),
            "要带两个 Z：{text}"
        );
        assert!(text.contains("第 1 层"), "要带层号：{text}");
    }

    /// 注释精度造成的同层小差（`; Z_HEIGHT: 1.07273` vs 塔那行 `Z1.073`）不算错。
    #[test]
    fn a_comment_precision_difference_is_not_an_error() {
        finalize(
            v(&[
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 1.07273",
                "G1 X1 Y1 E1",
                "G1 Z1.073;Tower Z",
                "G1 X2 Y2 E1",
            ]),
            "",
        )
        .expect("0.0003 的差是注释精度，不该报错");
    }

    /// 塔之外的 Z 一律不管：涂胶笔的喷嘴高度本来就在层顶之上（笔长 + 净空），
    /// 抬笔 / 回抽 / 补料也都在别的 Z 上，它们都不是「塔压错层」。
    #[test]
    fn only_tower_z_lines_are_checked() {
        finalize(
            v(&[
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.4",
                "G1 Z0.4",
                "G1 X1 Y1 E1",
                "G1 Z4.35",  // 涂胶笔抬起后的喷嘴高度
                "G1 X2 Y2",  // 空走
                "G1 Z-0.05", // 落笔（笔尖压进上一层）
                "G1 X3 Y3",  // 涂胶
                "G1 E-5",    // 回抽
                "G1 E5.5",   // 补料
                "G1 Z0.4",
                "G1 X4 Y4 E1",
            ]),
            "",
        )
        .expect("塔以外的 Z 不该被这条拦");
    }

    /// 开了斜接缝 ⇒ 把观看端选口径的那一格压成 0（否则它按 `; CHANGE_LAYER` 定层、
    /// 层 Z 与层内几何对不上，预览会把上半截画不出来）。
    #[test]
    fn a_scarf_seam_config_is_flattened_to_zero() {
        let out = finalize(
            v(&[
                "; has_scarf_joint_seam = 1",
                "; CONFIG_BLOCK_END",
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.2",
                "G1 Z0.2",
                "G1 X1 Y1 E1",
            ]),
            "",
        )
        .expect("finalize");
        assert!(out.scarf_flattened);
        assert!(out.lines.iter().any(|l| l == "; has_scarf_joint_seam = 0"));
        assert!(!out.lines.iter().any(|l| l == "; has_scarf_joint_seam = 1"));
    }

    /// 本来就 0（没开斜接缝）⇒ 一个字节都不动，也不报「改过」。
    #[test]
    fn a_zero_scarf_seam_config_is_left_alone() {
        let out = finalize(
            v(&[
                "; has_scarf_joint_seam = 0",
                "; CHANGE_LAYER",
                "; Z_HEIGHT: 0.2",
                "G1 X1 Y1 E1",
            ]),
            "",
        )
        .expect("finalize");
        assert!(!out.scarf_flattened);
        assert!(out.lines.iter().any(|l| l == "; has_scarf_joint_seam = 0"));
    }

    /// 没有这一格（非 Bambu 系产物）⇒ 不凭空造一行。
    #[test]
    fn a_missing_scarf_seam_config_is_not_invented() {
        let out = finalize(v(&["; CHANGE_LAYER", "; Z_HEIGHT: 0.2", "G1 X1 Y1 E1"]), "")
            .expect("finalize");
        assert!(!out.scarf_flattened);
        assert!(!out.lines.iter().any(|l| l.contains("has_scarf_joint_seam")));
    }

    /// 没有 `; CHANGE_LAYER` 的文件（校准片段）不参与对账，也不许被写成 0 层。
    #[test]
    fn a_file_without_layer_markers_is_left_alone() {
        let out = finalize(v(&["G1 X0 Y0", "; Z_HEIGHT: 0.2"]), "").expect("finalize");
        assert_eq!(out.layers, 0);
        assert!(!out.header_added);
        assert_eq!(out.lines[0], ";MKP_INFO version=1 tool=mkp-ssr");
    }
}
