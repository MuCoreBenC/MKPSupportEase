//! 配置文件 ⇄ [`Ir`]。
//!
//! **本项目没有预设文件**。配置文件的结构**就是 `Ir` 本身**，TOML 格式，PascalCase 键，
//! 严格反序列化（缺键即报错）。三条判定与理由（完整论证见 spec doc.md §4）：
//!
//! 1. **零映射层** ⇒ 不可能出现第二个真相源。来源仓库那 1,130 行 `TomlConfig → Ir` 映射
//!    正是「配置字段」与「算法读的字段」各自漂移的地方；这里它们**在类型上是同一个**。
//! 2. **严格反序列化保留**（不加 `#[serde(default)]`）。加了之后键名拼错会被静默吞掉，
//!    表现是「我明明写了这个参数怎么没生效」—— 最难查的一类。
//!    正因为严格，必须有 [`init_toml`] 生成完整文件：手填 145 个字段不可能。
//!    **`init` 是这个方案能成立的前提，不是附赠品。**
//! 3. **PascalCase 键不好看，接受**。改成 snake_case 要动 `types.rs` 的 `rename_all`，
//!    而那会让 3 份 golden JSON 判据全部失效（它们的键是 PascalCase）。判据比美观贵，
//!    而且 `init` 生成之后用户不用手打这些名字。
//!
//! **已知的洞（登记，不掩盖）**：serde 默认**忽略未知键**，所以
//! - 键名**拼错** ⇒ 报错（因为正确的那个键"缺了"），这条是有覆盖的；
//! - 但**多写一个**无关键（正确的键都在）⇒ 静默忽略，不报错。
//!   修法是给 145 个字段所在的 16 个结构加 `#[serde(deny_unknown_fields)]`，
//!   那要动 `types.rs` 且可能与 golden JSON 冲突，本轮**刻意不做**。

use std::collections::BTreeMap;
use std::path::Path;

use crate::diag::PostprocError;
use crate::ir::{Ir, fill_defaults};

/// 读配置文件 → [`Ir`]，并就地跑一次 [`fill_defaults`]。
///
/// **为什么 `fill_defaults` 在这里调**：来源仓库的 `ir::build()` 刻意不含它，要求
/// 「G-code 元数据提取之后同一时机」调用（对齐 Go `ConfigToParams` 的等价边界）。
/// 本项目没有 `build()` 了，这条时序落在 pipeline 的 config 步 —— 也就是这个函数。
/// 机型表填充（`fill_machine_dims_from_toml`）在**下一步**才跑，顺序与来源一致。
pub fn load(path: &Path) -> Result<Ir, PostprocError> {
    let raw = std::fs::read_to_string(path).map_err(|e| PostprocError::Io {
        op: "read_config",
        source: e,
    })?;
    load_str(&raw, &path.display().to_string(), &[])
}

/// 读配置 + 应用 `--set` 覆盖。分出这张脸是为了让判据不必碰文件系统。
///
/// `overrides` 的每一项形如 `Section.Key=value`（点号路径，见 [`apply_override`]）。
pub fn load_with_overrides(path: &Path, overrides: &[String]) -> Result<Ir, PostprocError> {
    let raw = std::fs::read_to_string(path).map_err(|e| PostprocError::Io {
        op: "read_config",
        source: e,
    })?;
    load_str(&raw, &path.display().to_string(), overrides)
}

/// 纯函数面：文本 → `Ir`。`path` 只用于错误信息。
pub fn load_str(raw: &str, path: &str, overrides: &[String]) -> Result<Ir, PostprocError> {
    let mut value: toml::Value = raw.parse().map_err(|e: toml::de::Error| {
        // toml 的错误自带行列位置，**原样透出不重新包装** ——
        // 自己拼一句「配置格式错误」会把唯一有用的信息（第几行第几列）丢掉。
        PostprocError::TomlParse {
            path: path.to_string(),
            message: e.to_string(),
        }
    })?;

    for spec in overrides {
        apply_override(&mut value, spec)?;
    }

    let mut ir: Ir = value.try_into().map_err(|e: toml::de::Error| {
        // 缺键会走到这里，消息里带字段名（serde 的 "missing field `X`"）。
        PostprocError::TomlParse {
            path: path.to_string(),
            message: e.to_string(),
        }
    })?;
    fill_defaults(&mut ir);
    Ok(ir)
}

/// 把一条 `Section.Key=value` 覆盖应用到已解析的 TOML 树上。
///
/// **值的目标类型取自「那个位置原本是什么类型」**，不靠猜：
/// `Tower.ExtrudeRatio` 原本是 Float ⇒ `=1` 被解析成 `1.0` 而不是整数 1
/// （否则 serde 会报 "invalid type: integer, expected f64"，而用户完全看不出自己错在哪）。
///
/// **路径不存在一律报错并列出该层可用键名**，绝不静默忽略 ——
/// 静默忽略等于「我 --set 了但没生效」，与上面 `serde(default)` 那条是同一种病。
pub fn apply_override(root: &mut toml::Value, spec: &str) -> Result<(), PostprocError> {
    let Some((path, raw_value)) = spec.split_once('=') else {
        return Err(PostprocError::InvalidConfig {
            message: format!("--set 需要 `路径=值` 形式，收到 {spec:?}"),
        });
    };
    let path = path.trim();
    let raw_value = raw_value.trim();
    if path.is_empty() {
        return Err(PostprocError::InvalidConfig {
            message: format!("--set 的路径为空: {spec:?}"),
        });
    }

    let segments: Vec<&str> = path.split('.').collect();
    let (last, parents) = segments
        .split_last()
        .expect("split('.') 至少给一段，且上面已排除空路径");

    // 逐层往下走。**每轮先把 `cursor` move 出来**（`let cur = cursor;`）——
    // 直接写 `cursor = cursor.as_table_mut()…` 会被借用检查器拒（E0499：
    // 一边借着 `*cursor` 一边给 `cursor` 赋值）。这不是风格问题，是唯一能过的写法。
    let mut cursor: &mut toml::Value = root;
    for (depth, seg) in parents.iter().enumerate() {
        let cur = cursor;
        // 可用键名在**可变借用之前**就取好：否则 `get_mut` 的借用会一直活到
        // match 结束，错误分支里再读 `keys()` 就借不出来了。
        let keys = match cur.as_table() {
            Some(t) => join_keys(t),
            None => return Err(not_a_table(path, seg, depth)),
        };
        let table = cur
            .as_table_mut()
            .expect("上一行刚确认过是表，这里不可能不是");
        match table.get_mut(*seg) {
            Some(next) => cursor = next,
            None => return Err(no_such_key(path, seg, &keys)),
        }
    }

    let keys = match cursor.as_table() {
        Some(t) => join_keys(t),
        None => return Err(not_a_table(path, last, parents.len())),
    };
    let table = cursor.as_table_mut().expect("上一行刚确认过是表");
    if !table.contains_key(*last) {
        return Err(no_such_key(path, last, &keys));
    }
    let new = parse_like(&table[*last], raw_value, path)?;
    table.insert((*last).to_string(), new);
    Ok(())
}

fn join_keys(table: &toml::map::Map<String, toml::Value>) -> String {
    let mut keys: Vec<&str> = table.keys().map(String::as_str).collect();
    keys.sort_unstable();
    keys.join(" / ")
}

fn not_a_table(path: &str, seg: &str, depth: usize) -> PostprocError {
    PostprocError::InvalidConfig {
        message: format!(
            "--set 路径 {path:?} 的第 {} 段 {seg:?} 之前不是一个表（那一层是标量，不能再往下走）",
            depth + 1
        ),
    }
}

fn no_such_key(path: &str, seg: &str, keys: &str) -> PostprocError {
    PostprocError::InvalidConfig {
        message: format!("--set 路径 {path:?} 里的 {seg:?} 不存在。该层可用键名：{keys}"),
    }
}

/// 按 `old` 的类型解析 `raw`。类型不匹配时报错点名**期望什么类型**。
fn parse_like(old: &toml::Value, raw: &str, path: &str) -> Result<toml::Value, PostprocError> {
    let want = match old {
        toml::Value::String(_) => return Ok(toml::Value::String(raw.to_string())),
        toml::Value::Integer(_) => {
            return raw
                .parse::<i64>()
                .map(toml::Value::Integer)
                .map_err(|_| type_err(path, raw, "整数"));
        }
        toml::Value::Float(_) => {
            return raw
                .parse::<f64>()
                .map(toml::Value::Float)
                .map_err(|_| type_err(path, raw, "浮点数"));
        }
        toml::Value::Boolean(_) => {
            return raw
                .parse::<bool>()
                .map(toml::Value::Boolean)
                .map_err(|_| type_err(path, raw, "布尔值（true/false）"));
        }
        // 数组 / 表 / 时间：不支持用 --set 改。
        // 理由：它们的字面量里必然含 `=` 或 `.`，与 --set 的分隔符打架，
        // 支持它们要引入一套转义规则 —— 那种复杂度换来的是几乎没人会用的能力。
        // 要改这些字段就编辑配置文件本身。
        other => other,
    };
    Err(PostprocError::InvalidConfig {
        message: format!(
            "--set 不支持修改 {path:?}（它当前是 {} 类型）；请直接编辑配置文件",
            want.type_str()
        ),
    })
}

fn type_err(path: &str, raw: &str, want: &str) -> PostprocError {
    PostprocError::InvalidConfig {
        message: format!("--set {path}={raw:?} 解析失败：这个字段需要{want}"),
    }
}

// ---------------------------------------------------------------------------
// init：吐一份完整的默认配置
// ---------------------------------------------------------------------------

/// 每个段落的说明。**这两条注释是这份文件唯一的"文档"**，
/// 比另写一份 README 更不容易腐烂（它跟着产物走，用户一定会看到）。
fn section_comment(section: &str) -> Option<&'static str> {
    match section {
        "Machine" => Some(
            "除 MachineType 外，多数字段会被内置机型表覆盖（运动范围 / 涂胶区 / 禁区），\n\
             在这里改了不生效。层高 / 喷嘴 / 速度这几个是零值兜底：填 0 就用默认值。",
        ),
        "State" => Some(
            "运行态，不是配置。pass1 写、pass2 读 —— 手改这里没有意义，可能让结果不可解释。\n\
             它出现在这里是因为配置文件的结构就是 IR 本身（见 src/config.rs 模块文档 §3）。",
        ),
        "Meta" => Some("展示用元信息，对处理结果无影响；全空即可。"),
        _ => None,
    }
}

/// 生成一份**完整**的默认配置（145 个字段全在），带段落注释。
///
/// 用 `Ir::default()` 而不是 `default + fill_defaults`：让用户看到的是**原始零值**，
/// 于是「哪些字段是零值兜底」这件事在文件里是可见的（配合 Machine 段的注释）。
/// 想看兜底之后的真实取值用 `dump-ir`。
pub fn init_toml() -> String {
    to_toml(&Ir::default())
}

/// 把任意 `Ir` 写成本模块能读回来的 TOML（`init` 与判据 fixture 共用这一条路）。
///
/// 为什么公开：`tests/fixtures/config/*.toml` 是从既有的 IR JSON 判据资产**派生**
/// 出来的，派生器就是这个函数 —— 手抄一份 TOML 会立刻变成第二个真相源，
/// 而「fixture 与 JSON 不一致」这种漂移没有任何判据能抓。
pub fn to_toml(ir: &Ir) -> String {
    let value = toml::Value::try_from(ir).expect("Ir 序列化成 toml::Value 不应失败");
    let table = value
        .as_table()
        .expect("Ir 的顶层必然是表")
        .clone()
        .into_iter()
        .collect::<BTreeMap<String, toml::Value>>();

    let mut out = String::new();
    out.push_str("# mkp-pp 配置文件（由 `mkp-pp init` 生成）\n");
    out.push_str("# 结构与内核的 IR 完全一致：改这里就是改 IR，没有中间映射层。\n");
    out.push_str("# 缺任何一个键都会报错（刻意的：拼错键名不会被静默忽略）。\n");
    out.push_str(
        "# 想看这份配置经过默认值兜底与机型表填充之后的真实取值：`mkp-pp dump-ir -c 本文件`\n",
    );
    emit_table(&mut out, "", &table);
    out
}

/// 递归输出一个表：**先所有非表项，再所有子表**。
///
/// 这个顺序是必须的，不是风格问题：TOML 里 `[Section]` 头之后的裸键值属于那个 section，
/// 所以标量若排在子表后面会被归错段（`toml` crate 的序列化器对此直接报
/// `values must be emitted before tables`）。而 `Ir` 的字段按字母序恰好会踩到 ——
/// 例如 `Glue` 段里 `ZCompBed`（子表）排在 `ZCompEnabled`（标量）前面。
/// 所以这里不能直接 `toml::to_string(&ir)`，必须自己控顺序。
fn emit_table(out: &mut String, prefix: &str, table: &BTreeMap<String, toml::Value>) {
    for (key, value) in table.iter().filter(|(_, v)| !v.is_table()) {
        out.push_str(&format!("{key} = {value}\n"));
    }
    for (key, value) in table.iter().filter(|(_, v)| v.is_table()) {
        let full = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{prefix}.{key}")
        };
        out.push('\n');
        if let Some(comment) = section_comment(&full) {
            for line in comment.lines() {
                out.push_str(&format!("# {}\n", line.trim_start()));
            }
        }
        out.push_str(&format!("[{full}]\n"));
        let sub = value
            .as_table()
            .expect("上面 filter 过了")
            .clone()
            .into_iter()
            .collect::<BTreeMap<String, toml::Value>>();
        emit_table(out, &full, &sub);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 判据 ①：`init` 的产物能被 `load_str` 读回，且等于 `default + fill_defaults`。
    ///
    /// 这条同时钉住三件事：init 输出的是**完整**的 145 个字段（缺一个就报错）、
    /// 段落顺序合法（标量不会被归错段）、`load` 确实跑了 `fill_defaults`。
    #[test]
    fn init_output_round_trips_to_filled_default() {
        let text = init_toml();
        let got = load_str(&text, "<init>", &[]).expect("init 的产物必须能被 load 读回");
        let mut want = Ir::default();
        fill_defaults(&mut want);
        assert_eq!(got, want);
    }

    /// 判据 ②：删掉任意一个键 ⇒ 报错，且**消息里点名那个键**。
    #[test]
    fn missing_key_is_reported_by_name() {
        let text = init_toml();
        let broken = text.replace("SafeZOffset = ", "__Removed__ = ");
        assert_ne!(broken, text, "扰动没生效 ⇒ 判据空转（键名或格式变了？）");
        let err = load_str(&broken, "<broken>", &[]).expect_err("缺键必须报错");
        let msg = err.to_string();
        assert!(
            msg.contains("SafeZOffset"),
            "报错必须点名缺失的键，实际: {msg}"
        );
    }

    /// 判据 ③：键名**拼错**（多一个字母）同样被抓到 —— 因为正确的那个键"缺了"。
    ///
    /// 这条与 ② 是不同的形态：② 是键消失，③ 是键还在但名字错了。
    /// serde 默认忽略未知键，所以 ③ 能红**只是因为正确键缺失**这个副作用；
    /// 「多写一个无关键」那种形态是抓不到的（模块文档已登记）。
    #[test]
    fn misspelled_key_is_caught_via_the_missing_correct_one() {
        let text = init_toml();
        let broken = text.replace("SafeZOffset = ", "SafeZOffsett = ");
        assert_ne!(broken, text, "扰动没生效 ⇒ 判据空转");
        let err = load_str(&broken, "<typo>", &[]).expect_err("拼错键名必须报错");
        assert!(err.to_string().contains("SafeZOffset"));
    }

    /// `--set` 四种标量类型各一条。
    #[test]
    fn set_parses_scalar_types_by_target_type() {
        let text = init_toml();
        let ir = load_str(
            &text,
            "<set>",
            &[
                "Tower.ExtrudeRatio=1.05".to_string(),
                "Glue.PassCount=2".to_string(),
                "Tower.UseTowers=true".to_string(),
                "Machine.MachineType=A1_MINI".to_string(),
            ],
        )
        .expect("四条覆盖都应成功");
        assert_eq!(ir.tower.extrude_ratio, 1.05);
        assert_eq!(ir.glue.pass_count, 2);
        assert!(ir.tower.use_towers);
        assert_eq!(ir.machine.machine_type, "A1_MINI");
    }

    /// `--set` 给 Float 字段一个整数字面量 ⇒ 必须成功（按目标类型解析成 1.0）。
    ///
    /// 单独一条，因为这正是「不按目标类型解析」时会炸的形态：
    /// 猜成 Integer 的话 serde 会报 "invalid type: integer, expected f64"，
    /// 而用户看不出自己错在哪 —— 他写的 `=1` 在他眼里没有类型。
    #[test]
    fn set_accepts_integer_literal_for_float_field() {
        let ir = load_str(&init_toml(), "<set>", &["Tower.ExtrudeRatio=1".to_string()])
            .expect("整数字面量赋给浮点字段应当成功");
        assert_eq!(ir.tower.extrude_ratio, 1.0);
    }

    /// `--set` 类型不合 ⇒ 报错点名期望的类型。
    #[test]
    fn set_reports_expected_type_on_parse_failure() {
        let err = load_str(
            &init_toml(),
            "<set>",
            &["Tower.ExtrudeRatio=abc".to_string()],
        )
        .expect_err("非法值必须报错");
        assert!(err.to_string().contains("浮点数"), "实际: {err}");
    }

    /// `--set` 路径不存在 ⇒ 报错并**列出该层可用键名**，不静默忽略。
    #[test]
    fn set_unknown_path_lists_available_keys() {
        let err = load_str(&init_toml(), "<set>", &["Tower.NoSuchKey=1".to_string()])
            .expect_err("不存在的路径必须报错");
        let msg = err.to_string();
        assert!(msg.contains("NoSuchKey"), "实际: {msg}");
        assert!(
            msg.contains("ExtrudeRatio"),
            "必须列出该层可用键名帮用户改对，实际: {msg}"
        );
    }

    /// `--set` 少了 `=` ⇒ 报错，不当成「路径存在但值为空」。
    #[test]
    fn set_without_equals_sign_is_rejected() {
        let err = load_str(&init_toml(), "<set>", &["Tower.ExtrudeRatio".to_string()])
            .expect_err("缺 = 必须报错");
        assert!(err.to_string().contains("路径=值"));
    }

    /// TOML 语法错 ⇒ 报错里**带行列位置**（原样透出解析器消息，不自己包一句）。
    #[test]
    fn toml_syntax_error_keeps_position_info() {
        let err = load_str("Tower = [[[", "<bad>", &[]).expect_err("语法错必须报错");
        let msg = err.to_string();
        assert!(msg.contains("TOML 解析失败"), "实际: {msg}");
        assert!(
            msg.contains("line") || msg.contains('1'),
            "必须保留位置信息，实际: {msg}"
        );
    }

    /// 反空转哨兵：`init` 的产物必须真的很大且含关键段落头 ——
    /// 万一 `emit_table` 退化成空实现，上面那些判据里有几条会以奇怪的方式通过。
    #[test]
    fn init_output_is_not_degenerate() {
        let text = init_toml();
        assert!(
            text.lines().count() > 120,
            "init 产物只有 {} 行，远少于 145 个字段 ⇒ 生成侧已退化",
            text.lines().count()
        );
        for header in ["[Machine]", "[Glue]", "[Glue.ZCompBed]", "[State]"] {
            assert!(text.contains(header), "init 产物缺少段落 {header}");
        }
    }
}
