//! 写回配方 —— **保注释、保键序、保引号形态**。
//!
//! # 为什么不是「读成结构体再序列化」
//!
//! `Recipe` 只派生 `Deserialize`，而配方文件里有 15 行注释、两处 `'''` 长段、
//! 一整套单引号字面量。`toml::to_string` 会把它们全抹平：注释没了、`'''` 变成一行带
//! `\n` 的双引号串、键序按结构体字段重排。那样写回一次，`git diff` 就没法看了 ——
//! 而 `git diff` 是这一轮**唯一剩下的安全网**（配方成了真源，见 spec §0）。
//!
//! 所以这里走 `toml_edit`，与 `write.rs`（改用户预设）同一条路、同一个理由。
//!
//! # 形态怎么跟住
//!
//! 不去碰 `toml_edit` 的 repr 内部：**先把想要的字面量拼成一行 TOML 解析一次**，
//! 再把解析出来的值接回文档，顺手把旧值的装饰（前后空白与行尾注释）搬过去。
//! 这样「这个字符串该用单引号还是 `'''`」这件事由**文本**决定，不由 API 决定 ——
//! 少一层猜测。

use std::collections::BTreeMap;

use toml_edit::{DocumentMut, Item, Table, Value};

/// 草稿里一个字段的新值。**类型跟随新值**（与 `write::apply_edits_exact` 同一立场）。
#[derive(Debug, Clone, PartialEq)]
pub enum DraftValue {
    Float(f64),
    Int(i64),
    Bool(bool),
    Str(String),
}

/// 一台新机型要填的东西。**缺一项都不许提交** —— 编出来的数值会让笔尖撞喷嘴。
#[derive(Debug, Clone, PartialEq)]
pub struct MachineDraft {
    /// 规范机型名（写进 `# machine:`）。
    pub name: String,
    /// **照哪台机型起头**：把它的正文**复制**一份给新机型。
    ///
    /// 复制而不是引用 —— 引用就又回到「借模板」那条路上了
    /// （漏写的键静默拿到别台机器的值，spec `machine-owns-its-body` §1）。
    pub body_from: String,
    /// 变体名，顺序就是生成顺序。
    pub variants: Vec<String>,
    /// 每个变体一个 `uuid` —— 人手填，生成器不许自己造。
    pub uuid_by_variant: BTreeMap<String, String>,
    /// 变体级覆盖（`offset` 的 x/y、个别 G-code 之类）：变体名 → 字段 → 值。
    ///
    /// **没有机型级那一层**：一台机型共用的值直接改它自己的正文。
    pub variant_fields: BTreeMap<String, BTreeMap<String, DraftValue>>,
    /// 复制来的正文里要改的键（两段 G-code 那种「整台机型共用」的东西）。
    pub body_edits: BTreeMap<String, DraftValue>,
}

fn parse_doc(text: &str) -> Result<DocumentMut, String> {
    text.parse::<DocumentMut>()
        .map_err(|e| format!("配方读不回来：{e}"))
}

/// 把一段 TOML 字面量解析成值。
///
/// `raw` 是**已经带好引号的字面量**（`70` / `-0.5` / `'abc'` / `'''…'''`）。
fn value_from_raw(raw: &str) -> Result<Value, String> {
    let doc = format!("x = {raw}\n")
        .parse::<DocumentMut>()
        .map_err(|e| format!("`{raw}` 不是一个合法的 TOML 值：{e}"))?;
    doc.get("x")
        .and_then(|i| i.as_value())
        .cloned()
        .ok_or_else(|| format!("`{raw}` 解析出来不是一个值"))
}

/// 新值该写成什么字面量 —— **字符串的引号形态跟随旧值**。
///
/// 旧值是 `'''` 长段就还写成 `'''` 长段；是单引号字面量就还是单引号。
/// 新内容里出现单引号（TOML 的字面量字符串没有转义）时退回双引号 —— 那是形态变化，
/// 但比写出一份读不回来的文件好。
fn raw_literal(value: &DraftValue, old: Option<&Value>) -> String {
    match value {
        DraftValue::Int(i) => i.to_string(),
        DraftValue::Bool(b) => b.to_string(),
        DraftValue::Float(f) => {
            // 整数值的浮点要留住小数点，否则 TOML 里它就变成整数了（类型会漂）
            if f.fract() == 0.0 && f.is_finite() {
                format!("{f:.1}")
            } else {
                f.to_string()
            }
        }
        DraftValue::Str(s) => {
            // 旧值的字面量原样（`Value::to_string()` 连装饰一起给，`trim` 掉 `= ` 后面那个空格；
            // 行尾注释留着无所谓 —— 这里只看开头那三个引号）
            let old_raw = old
                .map(|v| v.to_string())
                .unwrap_or_default()
                .trim()
                .to_string();
            let multiline = old_raw.starts_with("'''") || old_raw.starts_with("\"\"\"");
            if multiline && !s.contains("'''") {
                // TOML 多行字面量：`'''` 之后紧跟的那个换行不算内容
                if s.ends_with('\n') {
                    format!("'''\n{s}'''")
                } else {
                    format!("'''\n{s}\n'''")
                }
            } else if !s.contains('\'') && !s.contains('\n') {
                format!("'{s}'")
            } else {
                Value::from(s.as_str()).to_string().trim().to_string()
            }
        }
    }
}

/// 覆盖表里的一张子表（`[overrides."A1:fast"]`）。
fn override_table<'d>(doc: &'d mut DocumentMut, combo_key: &str) -> Result<&'d mut Table, String> {
    let overrides = doc
        .get_mut("overrides")
        .and_then(|i| i.as_table_mut())
        .ok_or_else(|| "配方里没有覆盖表".to_string())?;
    overrides
        .get_mut(combo_key)
        .and_then(|i| i.as_table_mut())
        .ok_or_else(|| format!("覆盖表里没有 [{combo_key}] 这一组"))
}

/// 改一个字段的值。**只动那一行**：装饰（前后空白、行尾注释）从旧值搬过来。
///
/// `combo_key` **必须是 `机型:变体`**（机型级那一层已经取消），`field` 是 `uuid` 或 `段.键`。
/// 字段不存在时**新增一行**（放在这张表的末尾）—— 新机型填 `offset` 时要用。
pub fn set_override(
    text: &str,
    combo_key: &str,
    field: &str,
    value: &DraftValue,
) -> Result<String, String> {
    if !combo_key.contains(':') {
        return Err(format!(
            "`{combo_key}` 不是 `机型:变体` —— 机型级覆盖已经取消了，\
             一台机型共用的值请改它自己的正文"
        ));
    }
    let mut doc = parse_doc(text)?;
    let table = override_table(&mut doc, combo_key)?;
    let old = table.get(field).and_then(|i| i.as_value()).cloned();
    let mut new_value = value_from_raw(&raw_literal(value, old.as_ref()))?;
    if let Some(old) = &old {
        *new_value.decor_mut() = old.decor().clone();
    }
    table.insert(field, Item::Value(new_value));
    Ok(doc.to_string())
}

/// 改**这台机型正文里**的一个键 —— 影响它的每个变体。
///
/// # 为什么必须有这条路
///
/// 覆盖只剩变体级了，所以「在界面上改一个值」有两种意思：
/// **只改这个变体**（写一条变体级覆盖）或**改这台机型**（改它的正文）。
/// 后者没有这条路就只能靠人手编配方 —— 而那正是这扇窗要省掉的事。
///
/// 正文是一份完整预设的文本，所以这里走的是与「改用户预设」同一套
/// （`write::apply_edits_exact`：只换值、保注释、保键序、保 `"""` 形态）。
pub fn set_body_value(
    text: &str,
    machine: &str,
    field: &str,
    value: &DraftValue,
) -> Result<String, String> {
    let (section, key) = field
        .split_once('.')
        .ok_or_else(|| format!("`{field}` 不是 `段.键` 形式"))?;

    let mut doc = parse_doc(text)?;
    let machines = doc
        .get_mut("machines")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or_else(|| "配方里没有机型表".to_string())?;
    let entry = machines
        .iter_mut()
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(machine))
        .ok_or_else(|| format!("配方里没有机型 {machine}"))?;

    let old_item = entry
        .get("body")
        .and_then(|i| i.as_value())
        .cloned()
        .ok_or_else(|| format!("机型 {machine} 没有正文"))?;
    let body = old_item
        .as_str()
        .ok_or_else(|| format!("机型 {machine} 的正文不是字符串"))?;

    let edited = crate::write::apply_edits_exact(
        body,
        &[crate::write::Edit::new(section, key, edit_value(value))],
    )
    .map_err(|e| format!("改 {machine} 的正文失败：{e}"))?;

    let mut new_value = value_from_raw(&raw_literal(&DraftValue::Str(edited), Some(&old_item)))?;
    *new_value.decor_mut() = old_item.decor().clone();
    entry.insert("body", Item::Value(new_value));
    Ok(doc.to_string())
}

/// 草稿值 → 写回器要的值（**类型跟随新值**，与预设编辑面同一立场）。
fn edit_value(value: &DraftValue) -> crate::write::EditValue {
    use crate::write::EditValue;
    match value {
        DraftValue::Float(f) => EditValue::Float(*f),
        DraftValue::Int(i) => EditValue::Int(*i),
        DraftValue::Bool(b) => EditValue::Bool(*b),
        DraftValue::Str(s) => EditValue::Str(s.clone()),
    }
}

/// 加一台新机型：`[[machines]]` 一条（**正文是复制来的**）+ 每个变体一张覆盖表。
///
/// **校验在前**（与 `Recipe::parse` 的六条同一套说法）：重名、变体重复、缺 `uuid`、
/// 起头的那台机型不存在，都在这里就拒绝，不写出半份配方。
pub fn add_machine(text: &str, draft: &MachineDraft) -> Result<String, String> {
    if draft.name.trim().is_empty() {
        return Err("机型名是空的".to_string());
    }
    if draft.variants.is_empty() {
        return Err(format!("机型 {} 没有任何变体", draft.name));
    }
    let mut seen = std::collections::BTreeSet::new();
    for v in &draft.variants {
        if !seen.insert(v.as_str()) {
            return Err(format!("机型 {} 的变体 {v} 出现了两次", draft.name));
        }
        if !draft.uuid_by_variant.contains_key(v) {
            return Err(format!(
                "{}:{v} 没有 uuid —— 每个机型变体都要有自己的身份，而且必须写在配方里",
                draft.name
            ));
        }
    }

    let mut doc = parse_doc(text)?;

    let machines = doc
        .get_mut("machines")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or_else(|| "配方里没有机型表".to_string())?;
    if machines
        .iter()
        .any(|t| t.get("name").and_then(|v| v.as_str()) == Some(draft.name.as_str()))
    {
        return Err(format!("机型 {} 已经在配方里了", draft.name));
    }
    // **复制**那台机型的正文。引用就又回到「借模板」那条路上了。
    let body = machines
        .iter()
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(draft.body_from.as_str()))
        .and_then(|t| t.get("body").and_then(|v| v.as_str()))
        .map(|s| s.to_string())
        .ok_or_else(|| {
            format!(
                "配方里没有机型 {} —— 「照哪台机型起头」只能选已经在配方里的",
                draft.body_from
            )
        })?;
    // 复制来的正文里要改的那几个键（两段 G-code 那种整台机型共用的东西）
    let mut edits = Vec::new();
    for (path, value) in &draft.body_edits {
        let (section, key) = path
            .split_once('.')
            .ok_or_else(|| format!("`{path}` 不是 `段.键` 形式"))?;
        edits.push(crate::write::Edit::new(section, key, edit_value(value)));
    }
    let body = if edits.is_empty() {
        body
    } else {
        crate::write::apply_edits_exact(&body, &edits)
            .map_err(|e| format!("改新机型的正文失败：{e}"))?
    };

    let mut entry = Table::new();
    entry.decor_mut().set_prefix("\n");
    entry.insert(
        "name",
        Item::Value(value_from_raw(&format!("'{}'", draft.name))?),
    );
    let variants = draft
        .variants
        .iter()
        .map(|v| format!("'{v}'"))
        .collect::<Vec<_>>()
        .join(", ");
    entry.insert(
        "variants",
        Item::Value(value_from_raw(&format!("[{variants}]"))?),
    );
    entry.insert(
        "body",
        Item::Value(value_from_raw(&raw_literal(&DraftValue::Str(body), None))?),
    );
    machines.push(entry);

    let overrides = doc
        .get_mut("overrides")
        .and_then(|i| i.as_table_mut())
        .ok_or_else(|| "配方里没有覆盖表".to_string())?;

    for variant in &draft.variants {
        let mut t = Table::new();
        t.decor_mut().set_prefix("\n");
        let uuid = &draft.uuid_by_variant[variant];
        t.insert("uuid", Item::Value(value_from_raw(&format!("'{uuid}'"))?));
        if let Some(fields) = draft.variant_fields.get(variant) {
            for (field, value) in fields {
                t.insert(
                    field,
                    Item::Value(value_from_raw(&raw_literal(value, None))?),
                );
            }
        }
        overrides.insert(&format!("{}:{variant}", draft.name), Item::Table(t));
    }

    Ok(doc.to_string())
}

/// 删掉一台机型：`[[machines]]` 那条 + 它的**全部**变体级覆盖。
///
/// 入库产物那几份文件不在这里删（这是纯文本函数，不碰文件系统）——
/// 删文件由命令那一层做，因为它要能在二次确认里说清「删几份文件」。
pub fn remove_machine(text: &str, name: &str) -> Result<String, String> {
    let mut doc = parse_doc(text)?;

    let machines = doc
        .get_mut("machines")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or_else(|| "配方里没有机型表".to_string())?;
    let at = machines
        .iter()
        .position(|t| t.get("name").and_then(|v| v.as_str()) == Some(name))
        .ok_or_else(|| format!("配方里没有机型 {name}"))?;
    if machines.len() == 1 {
        return Err(format!(
            "{name} 是配方里最后一台机型 —— 删掉它就没有预设可生成了"
        ));
    }
    machines.remove(at);

    let overrides = doc
        .get_mut("overrides")
        .and_then(|i| i.as_table_mut())
        .ok_or_else(|| "配方里没有覆盖表".to_string())?;
    let prefix = format!("{name}:");
    let doomed: Vec<String> = overrides
        .iter()
        .map(|(k, _)| k.to_string())
        .filter(|k| k.starts_with(&prefix))
        .collect();
    for k in &doomed {
        overrides.remove(k);
    }

    Ok(doc.to_string())
}

/// 删掉一个变体：那条覆盖 + 机型 `variants` 里那一项。
///
/// **只剩一个变体时拒绝**：机型没有变体等于渲染不出任何东西，
/// 那种配方 `Recipe::parse` 也不收 —— 与其写出一份非法配方再报错，不如现在就说清怎么办。
pub fn remove_variant(text: &str, machine: &str, variant: &str) -> Result<String, String> {
    let mut doc = parse_doc(text)?;

    let machines = doc
        .get_mut("machines")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or_else(|| "配方里没有机型表".to_string())?;
    let entry = machines
        .iter_mut()
        .find(|t| t.get("name").and_then(|v| v.as_str()) == Some(machine))
        .ok_or_else(|| format!("配方里没有机型 {machine}"))?;
    let variants = entry
        .get_mut("variants")
        .and_then(|i| i.as_array_mut())
        .ok_or_else(|| format!("机型 {machine} 没有变体表"))?;
    let at = variants
        .iter()
        .position(|v| v.as_str() == Some(variant))
        .ok_or_else(|| format!("机型 {machine} 没有变体 {variant}"))?;
    if variants.len() == 1 {
        return Err(format!(
            "{variant} 是 {machine} 最后一个变体 —— 机型至少要有一个变体，\
             想整台删就用「删掉机型」"
        ));
    }
    variants.remove(at);

    let overrides = doc
        .get_mut("overrides")
        .and_then(|i| i.as_table_mut())
        .ok_or_else(|| "配方里没有覆盖表".to_string())?;
    overrides.remove(&format!("{machine}:{variant}"));

    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generate::recipe_path;
    use crate::recipe::{Recipe, render};

    fn recipe_text() -> String {
        std::fs::read_to_string(recipe_path()).expect("仓库里的配方")
    }

    /// 改了几行（按行逐一比，返回变化的行号，1 起）。
    fn changed_lines(before: &str, after: &str) -> Vec<usize> {
        let a: Vec<&str> = before.lines().collect();
        let b: Vec<&str> = after.lines().collect();
        assert_eq!(a.len(), b.len(), "行数变了：{} → {}", a.len(), b.len());
        (0..a.len())
            .filter(|i| a[*i] != b[*i])
            .map(|i| i + 1)
            .collect()
    }

    /// K-W3：改一个标量 ⇒ 配方文件**只有那一行变**。
    #[test]
    fn kw3_editing_one_scalar_changes_exactly_one_line() {
        let before = recipe_text();
        let after = set_override(
            &before,
            "A1:fast",
            "toolhead.offset.y",
            &DraftValue::Float(26.4),
        )
        .expect("写回");
        let changed = changed_lines(&before, &after);
        assert_eq!(
            changed.len(),
            1,
            "K-W3 红：应当只有一行变，实测变了 {changed:?}"
        );
        assert!(
            after.contains("\"toolhead.offset.y\" = 26.4"),
            "K-W3 红：新值不在文件里"
        );
        println!("K-W3 绿：只有第 {} 行变了", changed[0]);
    }

    /// K-W3b：改一段 `'''` G-code ⇒ 段内容变、`'''` 形态不变。
    #[test]
    fn kw3b_editing_a_gcode_block_keeps_the_triple_quotes() {
        let before = recipe_text();
        let field = "toolhead.custom_unmount_gcode";
        let recipe = Recipe::parse(&before).expect("配方合法");
        let old = recipe
            .effective("A1", "fastv3.3")
            .get(field)
            .and_then(|v| v.as_str())
            .expect("A1:fastv3.3 有这一段")
            .to_string();
        assert!(old.contains("G1 X273 F10000"), "前提：这一段里有 X273");

        let after = set_override(
            &before,
            "A1:fastv3.3",
            field,
            &DraftValue::Str(old.replace("G1 X273 F10000", "G1 X274 F10000")),
        )
        .expect("写回");

        let changed = changed_lines(&before, &after);
        assert_eq!(
            changed.len(),
            1,
            "K-W3b 红：一段里改一行，应当只有那一行变，实测 {changed:?}"
        );
        assert!(
            after.contains("\"toolhead.custom_unmount_gcode\" = '''"),
            "K-W3b 红：`'''` 形态没保住"
        );
        assert!(after.contains("G1 X274 F10000"), "K-W3b 红：新值不在文件里");
        println!("K-W3b 绿：`'''` 形态保住，只有第 {} 行变了", changed[0]);
    }

    /// K-W2：写回之后重读渲染 —— **只有被改的那一份变**，另外 8 份逐字节相同。
    #[test]
    fn kw2_write_back_only_moves_the_one_preset() {
        let before = recipe_text();
        let r0 = Recipe::parse(&before).expect("配方合法");
        let rendered_before: Vec<(String, String, String)> = r0
            .combos()
            .into_iter()
            .map(|(m, v)| {
                let t = render(&r0, &m, &v).expect("渲染");
                (m, v, t)
            })
            .collect();

        let after = set_override(
            &before,
            "A1:fast",
            "toolhead.offset.y",
            &DraftValue::Float(26.4),
        )
        .expect("写回");
        let r1 = Recipe::parse(&after).expect("写回之后仍然合法");

        let mut moved = Vec::new();
        for (m, v, text_before) in &rendered_before {
            let text_after = render(&r1, m, v).expect("渲染");
            if &text_after != text_before {
                moved.push(format!("{m}:{v}"));
            }
        }
        assert_eq!(
            moved,
            vec!["A1:fast".to_string()],
            "K-W2 红：动的应当只有 A1:fast，实测 {moved:?}"
        );
        println!("K-W2 绿：9 份里只有 A1:fast 变了");
    }

    /// 加一台新机型：配方仍然合法、组合数 +1、新那份能渲染出来。
    ///
    /// **K-M3 也在这里**：正文是**复制**的 —— 被复制那台机型渲染出来的 9 份一个字节都不变。
    #[test]
    fn adding_a_machine_copies_the_body_and_leaves_the_source_alone() {
        let before = recipe_text();
        let r0 = Recipe::parse(&before).expect("配方合法");

        let mut uuid_by_variant = BTreeMap::new();
        uuid_by_variant.insert(
            "standard".to_string(),
            "00000000-0000-4000-8000-000000000001".to_string(),
        );
        let mut variant_fields = BTreeMap::new();
        let mut f = BTreeMap::new();
        f.insert("toolhead.offset.x".to_string(), DraftValue::Float(-1.0));
        f.insert("toolhead.offset.y".to_string(), DraftValue::Float(18.6));
        variant_fields.insert("standard".to_string(), f);

        let draft = MachineDraft {
            name: "A1_TEST".to_string(),
            body_from: "A1_MINI".to_string(),
            variants: vec!["standard".to_string()],
            uuid_by_variant,
            variant_fields,
            body_edits: BTreeMap::new(),
        };
        let after = add_machine(&before, &draft).expect("加机型");
        let r1 = Recipe::parse(&after).expect("加完仍然合法");
        assert_eq!(r1.combos().len(), r0.combos().len() + 1);
        let text = render(&r1, "A1_TEST", "standard").expect("新那份能渲染");
        assert!(text.contains("# machine: A1_TEST"));
        assert!(text.contains("00000000-0000-4000-8000-000000000001"));
        // 正文真的是从 A1_MINI 抄来的（那台的装笔段坐标是 X168）
        assert!(text.contains("G1 X168"), "正文应当是 A1_MINI 的那一份");
        // 原来那 9 份一个字节都没动
        for (m, v) in r0.combos() {
            assert_eq!(
                render(&r1, &m, &v).expect("渲染"),
                render(&r0, &m, &v).expect("渲染"),
                "{m}:{v} 被加机型这件事影响到了"
            );
        }

        // K-M3：改新机型的正文之后，被复制那台仍然一个字节不变（复制不是引用）。
        //
        // **扰动只许落在新机型那一份上**（这里踩过一次）：两份正文是逐字节相同的，
        // 全局 `replace` 会同时改掉 A1_MINI 的那一份 —— 那样测出来的「被影响了」
        // 是扰动自己造成的，什么也证明不了。A1_TEST 是**追加**在机型表末尾的，
        // 所以取那一行的**最后一次**出现。
        let needle = "G1 X168 F30000";
        let at = after.rfind(needle).expect("前提：正文里有那一行");
        let mut poked = after.clone();
        poked.replace_range(at..at + needle.len(), "G1 X169 F30000");
        let r2 = Recipe::parse(&poked).expect("改完仍然合法");
        assert!(
            render(&r2, "A1_TEST", "standard")
                .expect("渲染")
                .contains("G1 X169 F30000"),
            "前提：扰动应当落在 A1_TEST 的正文里"
        );
        assert_eq!(
            render(&r2, "A1_MINI", "standard").expect("渲染"),
            render(&r0, "A1_MINI", "standard").expect("渲染"),
            "K-M3 红：改新机型的正文动到了 A1_MINI —— 那说明正文是共享的"
        );
    }

    /// 缺 uuid / 重名 / 变体重复 / 起头的机型不存在：提交前就拒绝，且点名。
    #[test]
    fn a_bad_machine_draft_is_refused_by_name() {
        let before = recipe_text();
        let base = MachineDraft {
            name: "A1_TEST".to_string(),
            body_from: "A1".to_string(),
            variants: vec!["standard".to_string()],
            uuid_by_variant: BTreeMap::new(),
            variant_fields: BTreeMap::new(),
            body_edits: BTreeMap::new(),
        };
        let err = add_machine(&before, &base).expect_err("缺 uuid 必须拒绝");
        assert!(err.contains("没有 uuid"), "实测：{err}");

        let mut dup = base.clone();
        dup.name = "A1".to_string();
        dup.uuid_by_variant.insert(
            "standard".to_string(),
            "00000000-0000-4000-8000-000000000002".to_string(),
        );
        let err = add_machine(&before, &dup).expect_err("重名必须拒绝");
        assert!(err.contains("已经在配方里了"), "实测：{err}");

        let mut twice = base.clone();
        twice.variants = vec!["standard".to_string(), "standard".to_string()];
        twice.uuid_by_variant.insert(
            "standard".to_string(),
            "00000000-0000-4000-8000-000000000003".to_string(),
        );
        let err = add_machine(&before, &twice).expect_err("变体重复必须拒绝");
        assert!(err.contains("出现了两次"), "实测：{err}");

        let mut nosuch = base.clone();
        nosuch.body_from = "A9".to_string();
        nosuch.uuid_by_variant.insert(
            "standard".to_string(),
            "00000000-0000-4000-8000-000000000004".to_string(),
        );
        let err = add_machine(&before, &nosuch).expect_err("起头的机型不存在必须拒绝");
        assert!(err.contains("照哪台机型起头"), "实测：{err}");
    }

    /// 删机型：配方里没有它了、它的覆盖也一起走，**别的机型渲染出来一个字节不变**。
    #[test]
    fn removing_a_machine_takes_its_overrides_and_leaves_the_others_alone() {
        let before = recipe_text();
        let r0 = Recipe::parse(&before).expect("配方合法");
        let target = "A1_MINI";
        let kept: Vec<(String, String)> = r0
            .combos()
            .into_iter()
            .filter(|(m, _)| m != target)
            .collect();

        let after = remove_machine(&before, target).expect("删机型");
        let r1 = Recipe::parse(&after).expect("删完仍然合法");

        assert!(
            r1.machines.iter().all(|m| m.name != target),
            "机型表里还留着 {target}"
        );
        assert!(
            r1.overrides.keys().all(|k| !k.starts_with("A1_MINI:")),
            "覆盖表里还留着 {target} 的条目：{:?}",
            r1.overrides.keys().collect::<Vec<_>>()
        );
        assert_eq!(r1.combos().len(), kept.len());
        for (m, v) in &kept {
            assert_eq!(
                render(&r1, m, v).expect("渲染"),
                render(&r0, m, v).expect("渲染"),
                "{m}:{v} 被删 {target} 这件事影响到了"
            );
        }
    }

    /// 删变体：那条覆盖与 `variants` 里那一项都走，同机型别的变体不变。
    #[test]
    fn removing_a_variant_only_takes_that_one() {
        let before = recipe_text();
        let r0 = Recipe::parse(&before).expect("配方合法");
        let after = remove_variant(&before, "A1", "fast").expect("删变体");
        let r1 = Recipe::parse(&after).expect("删完仍然合法");

        assert!(
            !r1.combos()
                .contains(&("A1".to_string(), "fast".to_string()))
        );
        assert!(!r1.overrides.contains_key("A1:fast"));
        for v in ["standard", "fastv3.3"] {
            assert_eq!(
                render(&r1, "A1", v).expect("渲染"),
                render(&r0, "A1", v).expect("渲染"),
                "A1:{v} 被删 A1:fast 影响到了"
            );
        }
    }

    /// K-U5：删到只剩一个变体时拒绝并点名（那种配方 `parse` 也不收）。
    #[test]
    fn ku5_removing_the_last_variant_is_refused_by_name() {
        let before = recipe_text();
        // P1S 只有一个变体 `lite`
        let err = remove_variant(&before, "P1S", "lite").expect_err("最后一个变体必须拒绝");
        assert!(err.contains("最后一个变体"), "实测：{err}");
        assert!(err.contains("删掉机型"), "要告诉人怎么办，实测：{err}");
        println!("K-U5 绿：{err}");
    }

    /// 不存在的机型 / 变体：报错点名，不静默成功。
    #[test]
    fn removing_something_that_is_not_there_is_named() {
        let before = recipe_text();
        let err = remove_machine(&before, "A9").expect_err("不存在的机型要报错");
        assert!(err.contains("没有机型 A9"), "实测：{err}");
        let err = remove_variant(&before, "A1", "nosuch").expect_err("不存在的变体要报错");
        assert!(err.contains("没有变体 nosuch"), "实测：{err}");
    }

    /// 机型级覆盖已经取消：`set_override` 收到不带冒号的键要拒绝并说清。
    #[test]
    fn a_machine_level_override_is_refused() {
        let err = set_override(
            &recipe_text(),
            "A1",
            "toolhead.speed_limit",
            &DraftValue::Int(71),
        )
        .expect_err("机型级必须拒绝");
        assert!(err.contains("机型级覆盖已经取消"), "实测：{err}");
    }

    /// 改**正文**里的一个键：影响这台机型的每个变体，别的机型一个字节不动。
    #[test]
    fn editing_a_body_value_moves_every_variant_of_that_machine_only() {
        let before = recipe_text();
        let r0 = Recipe::parse(&before).expect("配方合法");
        let after = set_body_value(
            &before,
            "A1_MINI",
            "toolhead.speed_limit",
            &DraftValue::Int(71),
        )
        .expect("改正文");
        let r1 = Recipe::parse(&after).expect("改完仍然合法");

        // A1_MINI 的三个变体都跟着变了
        for v in ["standard", "fast", "fastv3.3"] {
            let text = render(&r1, "A1_MINI", v).expect("渲染");
            assert!(
                text.contains("speed_limit = 71"),
                "A1_MINI:{v} 应当跟着正文变，实测没变"
            );
        }
        // 别的机型一个字节都没动
        for (m, v) in r0.combos() {
            if m == "A1_MINI" {
                continue;
            }
            assert_eq!(
                render(&r1, &m, &v).expect("渲染"),
                render(&r0, &m, &v).expect("渲染"),
                "{m}:{v} 被改 A1_MINI 的正文影响到了"
            );
        }
        // 正文里只有那一行变（形态也没变：`'''` 还在）
        let changed = changed_lines(&before, &after);
        assert_eq!(changed.len(), 1, "实测变了 {changed:?}");
        assert!(after.contains("body = '''"), "`'''` 形态没保住");
    }
}
