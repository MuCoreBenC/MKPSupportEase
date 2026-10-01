//! 预设配方：**一台机型一份正文 + 一张变体级覆盖表** → 9 份预设。
//!
//! # 为什么正文长在机型上（这一轮翻的案）
//!
//! 上一版是「2 份模板 + 一张覆盖表」，而 `A1_MINI` **借的是 A1 的模板** ——
//! 后果不是审美问题：给 A1_MINI 加一个新变体、漏写某个键，它会**静默拿到 A1 的值**，
//! 而那是一台不同尺寸的机器（笔尖坐标错了会撞喷嘴）。
//! 那种错在产物上看不出来（它「有值」），判据也咬不到（K-G7 只保证产物与配方一致）。
//!
//! 所以这里**没有可以共享的东西**：`body` 是 `MachineRecipe` 的字段，
//! 一台机型的正文只属于它自己。这不是「加一条校验」，是让那类错**没有表达方式**。
//! 迁移是恒等变换：`body` 取自该机型 base 变体的入库产物原文
//! （渲染只重写头四行），证据是 9 份产物逐字节未变。
//!
//! # 覆盖只有一层
//!
//! 覆盖表的键**一律 `机型:变体`**。机型级那一层随共享模板一起取消了 ——
//! 一台机型共用的值就写在它自己的正文里，不必再「盖」一次。
//!
//! **规则只有一条**：正文给出完整的一份预设；覆盖按键整值替换。
//! 粒度是「一个键的完整值」或「一整段 G-code」，**绝不是行内的某个数字**。
//! 理由：`X256` 换成 `X{{park}}` 之后，配方就只有作者读得懂了。
//!
//! # 一条硬约束：生成必须幂等
//!
//! `uuid` 与 `release_time` **写在配方里**，不许在生成时现取。
//! `uuid::new_v4()` 或 `now()` 只要出现在这条路上，「重新生成 = 与现有预设逐字节相同」
//! 这条黄金判据当场失效 —— 而它是整个生成器唯一的安全网。
//! 门禁 `scripts/check_generator_purity.py` 盯着这件事。

use std::collections::{BTreeMap, BTreeSet};

use postprocess::diag::PostprocError;
use serde::Deserialize;

use crate::write::EDITABLE_KEYS;

/// 覆盖表里唯一一个**不是** TOML 键的字段：它写在文件头注释 `# uuid:` 上。
pub const UUID_FIELD: &str = "uuid";

/// 一份配方。
///
/// **开 `deny_unknown_fields`**（与 `param_registry.toml` 相反的选择）：
/// 注册表是外部下发的资产，多一个字段不该让整张表读不进来；
/// 而配方是**我们自己的**、跟着二进制走的东西，写错一个键必须当场炸 ——
/// 静默丢掉一条覆盖的后果是「某台机型的预设生成出来是错的，而判据在下一次才发现」。
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    /// 这批预设的发布时间，写进每一份的 `# release_time:`。改它 = 出新版本。
    pub release_time: String,
    /// 机型：**每台自带正文**，有哪些变体。
    pub machines: Vec<MachineRecipe>,
    /// 覆盖：键**一律 `机型:变体`**，值是「字段 → 值」。
    ///
    /// 机型级那一层已经取消（随共享模板一起）——一台机型共用的值写在它自己的正文里。
    /// 字段名要么是 [`UUID_FIELD`]，要么是 `段.键` 形式的可编辑键
    /// （与 [`EDITABLE_KEYS`] 同一份清单，解析时逐条校验）。
    #[serde(default)]
    pub overrides: BTreeMap<String, BTreeMap<String, toml::Value>>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MachineRecipe {
    /// 规范机型名（`A1` / `A1_MINI` / `P1S` / `P2S` / `X1C`），写进 `# machine:`。
    pub name: String,
    /// 有哪些变体，写进 `# variant:`。顺序就是生成顺序。
    pub variants: Vec<String>,
    /// **这台机型自己的**一份完整预设正文（键序、注释、空行就是产物的样子）。
    ///
    /// 头四行（uuid / release_time / machine / variant）由渲染重写，所以它可以直接是
    /// 某个变体的产物原文 —— 迁移就是这么做的。
    pub body: String,
}

fn invalid(message: String) -> PostprocError {
    PostprocError::InvalidConfig { message }
}

/// 可编辑键的 `段.键` 形式集合（覆盖表的字段名必须在里面，或是 `uuid`）。
fn editable_paths() -> BTreeSet<String> {
    EDITABLE_KEYS
        .iter()
        .map(|(section, key)| format!("{section}.{key}"))
        .collect()
}

impl Recipe {
    /// 读配方并**当场校验**。校验不过就返回错误，**不返回半份配方**。
    ///
    /// 校验的六条，每条对应一种「写错了但看起来没事」的情形：
    /// 1. 机型的正文是空的 —— 那台机型渲染不出任何东西；
    /// 2. 机型重名 —— 两份配方合并时最容易出的错；
    /// 3. 变体空或重复 —— 会生成两份同名文件；
    /// 4. 覆盖键不是 `机型:变体` 形式 —— 机型级那一层已经取消，写了不会生效；
    /// 5. 覆盖键的机型/变体不存在，或字段名不是可编辑键 —— 两种都**静默无效**；
    /// 6. 某个 `(机型,变体)` 拿不到 `uuid` —— 生成出来的预设没有身份。
    pub fn parse(text: &str) -> Result<Self, PostprocError> {
        let recipe: Recipe = toml::from_str(text).map_err(|e| {
            invalid(format!(
                "配方读不回来（`preset_recipes.toml` 是我们自己的资产，写错了必须当场炸）：{e}"
            ))
        })?;
        recipe.validate()?;
        Ok(recipe)
    }

    fn validate(&self) -> Result<(), PostprocError> {
        let mut seen_machines: BTreeSet<&str> = BTreeSet::new();
        for m in &self.machines {
            if m.body.trim().is_empty() {
                return Err(invalid(format!(
                    "机型 {} 的正文是空的 —— 每台机型自带一份完整预设正文（没有共享模板了）",
                    m.name
                )));
            }
            if !seen_machines.insert(m.name.as_str()) {
                return Err(invalid(format!("机型 {} 出现了两次", m.name)));
            }
            if m.variants.is_empty() {
                return Err(invalid(format!("机型 {} 没有任何变体", m.name)));
            }
            let mut seen_variants: BTreeSet<&str> = BTreeSet::new();
            for v in &m.variants {
                if !seen_variants.insert(v.as_str()) {
                    return Err(invalid(format!("机型 {} 的变体 {v} 出现了两次", m.name)));
                }
            }
        }

        let paths = editable_paths();
        for (key, fields) in &self.overrides {
            let Some((machine, variant)) = key.split_once(':') else {
                return Err(invalid(format!(
                    "覆盖 [{key}] 不是 `机型:变体` 形式 —— 机型级覆盖已经取消了，\
                     一台机型共用的值请写进它自己的正文（`body`）"
                )));
            };
            let m = self
                .machines
                .iter()
                .find(|m| m.name == machine)
                .ok_or_else(|| {
                    invalid(format!(
                        "覆盖 [{key}] 指向不存在的机型 `{machine}` —— 这条覆盖不会生效，\
                         而那种错在产物上看不出来"
                    ))
                })?;
            if !m.variants.iter().any(|v| v == variant) {
                return Err(invalid(format!(
                    "覆盖 [{key}] 指向机型 {machine} 不存在的变体 `{variant}`（它有的是：{}）",
                    m.variants.join(" / ")
                )));
            }
            for field in fields.keys() {
                if field != UUID_FIELD && !paths.contains(field) {
                    return Err(invalid(format!(
                        "覆盖 [{key}] 里的 `{field}` 不是可编辑键 —— 要么是 `{UUID_FIELD}`，\
                         要么是 `段.键`（清单来自 model.rs 的 serde 键，共 {} 条）",
                        paths.len()
                    )));
                }
            }
        }

        for (machine, variant) in self.combos() {
            if !self.effective(&machine, &variant).contains_key(UUID_FIELD) {
                return Err(invalid(format!(
                    "{machine}:{variant} 没有 `{UUID_FIELD}` —— 每个机型变体都要有自己的身份，\
                     而且必须写在配方里（生成时现取会让「重新生成逐字节相同」失效）"
                )));
            }
        }
        Ok(())
    }

    /// 全部 `(机型, 变体)` 组合，顺序 = 配方里的书写顺序。
    pub fn combos(&self) -> Vec<(String, String)> {
        self.machines
            .iter()
            .flat_map(|m| m.variants.iter().map(|v| (m.name.clone(), v.clone())))
            .collect()
    }

    /// 某个机型变体的**有效覆盖**。只有一层（变体级）—— 机型级那一层已经取消。
    pub fn effective(&self, machine: &str, variant: &str) -> BTreeMap<String, toml::Value> {
        self.overrides
            .get(&format!("{machine}:{variant}"))
            .cloned()
            .unwrap_or_default()
    }

    /// 这台机型自己的正文。
    pub fn body_of(&self, machine: &str) -> Result<&str, PostprocError> {
        self.machines
            .iter()
            .find(|m| m.name == machine)
            .map(|m| m.body.as_str())
            .ok_or_else(|| invalid(format!("配方里没有机型 {machine}")))
    }
}

/// 渲染一份预设：这台机型的正文 → 改四个头字段 → 应用覆盖。
///
/// **产物必须与现有预设逐字节相同**（判据 K-G1），所以两件事都不能自己发明：
/// - 头字段只换**冒号后面那一段**，行首的 `# ` 与缩进原样保留；
/// - 键值走 `write::apply_edits`（`toml_edit` 只换值、保注释、保键序、保 inline table 与
///   `"""` 形态）—— **不新写一套文本拼接**。上一轮已经证明过那条路是对的
///   （K-P1：零编辑往返逐字节相同）。
pub fn render(recipe: &Recipe, machine: &str, variant: &str) -> Result<String, PostprocError> {
    let body = recipe.body_of(machine)?;
    let fields = recipe.effective(machine, variant);

    let uuid = fields
        .get(UUID_FIELD)
        .and_then(|v| v.as_str())
        .ok_or_else(|| invalid(format!("{machine}:{variant} 没有 `{UUID_FIELD}`")))?;

    let with_header = rewrite_header(
        body,
        &[
            (UUID_FIELD, uuid),
            ("release_time", &recipe.release_time),
            ("machine", machine),
            ("variant", variant),
        ],
    )?;

    let mut edits = Vec::new();
    for (path, value) in &fields {
        if path == UUID_FIELD {
            continue;
        }
        let (section, key) = path.split_once('.').ok_or_else(|| {
            invalid(format!(
                "覆盖字段 `{path}` 不是 `段.键` 形式（校验应当拦住它）"
            ))
        })?;
        edits.push(crate::write::Edit::new(
            section,
            key,
            to_edit_value(path, value)?,
        ));
    }
    crate::write::apply_edits_exact(&with_header, &edits)
}

/// 把 `# <key>: <旧值>` 换成 `# <key>: <新值>`，**只动冒号后面**。
///
/// 四个头字段必须都在模板里找到：找不到说明模板不是一份真预设
/// （或者头字段的写法变了），那种情况下继续渲染只会产出一份少了身份的预设。
fn rewrite_header(body: &str, pairs: &[(&str, &str)]) -> Result<String, PostprocError> {
    let mut out = String::with_capacity(body.len());
    let mut hit = vec![false; pairs.len()];

    for line in body.split_inclusive('\n') {
        let mut replaced = false;
        for (i, (key, value)) in pairs.iter().enumerate() {
            if hit[i] {
                continue; // 只换第一处（后面同名的注释不动）
            }
            if let Some(colon) = header_colon(line, key) {
                out.push_str(&line[..=colon]);
                out.push(' ');
                out.push_str(value);
                // 行尾原样跟上（CRLF 也保住）
                out.push_str(line_ending(line));
                hit[i] = true;
                replaced = true;
                break;
            }
        }
        if !replaced {
            out.push_str(line);
        }
    }

    for (i, (key, _)) in pairs.iter().enumerate() {
        if !hit[i] {
            return Err(invalid(format!(
                "模板里没有 `# {key}:` 这一行 —— 模板得是一份真预设的原文"
            )));
        }
    }
    Ok(out)
}

/// 这一行是 `# <key>:` 吗；是就返回冒号的字节下标。
fn header_colon(line: &str, key: &str) -> Option<usize> {
    let trimmed_start = line.len() - line.trim_start().len();
    let rest = line.trim_start();
    let rest = rest.strip_prefix('#')?;
    let after_hash = trimmed_start + 1;
    let ws = rest.len() - rest.trim_start().len();
    let rest = rest.trim_start();
    let rest = rest.strip_prefix(key)?;
    let key_end = after_hash + ws + key.len();
    let ws2 = rest.len() - rest.trim_start().len();
    if !rest.trim_start().starts_with(':') {
        return None;
    }
    Some(key_end + ws2)
}

fn line_ending(line: &str) -> &str {
    if line.ends_with("\r\n") {
        "\r\n"
    } else if line.ends_with('\n') {
        "\n"
    } else {
        ""
    }
}

fn to_edit_value(
    path: &str,
    value: &toml::Value,
) -> Result<crate::write::EditValue, PostprocError> {
    use crate::write::EditValue;
    match value {
        toml::Value::Integer(i) => Ok(EditValue::Int(*i)),
        toml::Value::Float(f) => Ok(EditValue::Float(*f)),
        toml::Value::Boolean(b) => Ok(EditValue::Bool(*b)),
        toml::Value::String(s) => Ok(EditValue::Str(s.clone())),
        other => Err(invalid(format!(
            "覆盖字段 `{path}` 的值类型是 {}，预设里只会出现数/布尔/字符串",
            other.type_str()
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一份最小但**合法**的配方（一台机型自带正文、两个变体各一条覆盖）。
    ///
    /// 用 `r##"…"##` 而不是 `r#"…"#`：正文里有 `"#`（`body = "# uuid…`），
    /// 单个 `#` 的原始字符串会在那里提前结束（实测编译器把后面的内容当 Rust 代码解析）。
    fn minimal() -> String {
        r##"
release_time = "2026-08-19 01:38:13"

[[machines]]
name = "A1"
variants = ["standard", "fast"]
body = "# uuid: x\n# machine: A1\n[toolhead]\nspeed_limit = 70\n"

[overrides."A1:standard"]
uuid = "11111111-1111-1111-1111-111111111111"

[overrides."A1:fast"]
uuid = "22222222-2222-2222-2222-222222222222"
"toolhead.speed_limit" = 80
"##
        .to_string()
    }

    #[test]
    fn a_valid_recipe_parses_and_takes_values_from_its_own_body() {
        let r = Recipe::parse(&minimal()).expect("合法配方");
        assert_eq!(
            r.combos(),
            vec![
                ("A1".to_string(), "standard".to_string()),
                ("A1".to_string(), "fast".to_string()),
            ]
        );
        // 变体级覆盖有它自己的值
        assert_eq!(
            r.effective("A1", "fast").get("toolhead.speed_limit"),
            Some(&toml::Value::Integer(80))
        );
        // 没写覆盖的那一份**什么都不覆盖** —— 值来自这台机型自己的正文（70）
        assert_eq!(
            r.effective("A1", "standard").get("toolhead.speed_limit"),
            None
        );
        assert!(r.body_of("A1").expect("正文").contains("speed_limit = 70"));
    }

    /// K-G5：六种写错法，每种都要**点名**报出来。
    #[test]
    fn kg5_every_way_of_writing_it_wrong_gets_named() {
        let cases: [(&str, &str, &str); 6] = [
            (
                "机型级覆盖（这一层已经取消）",
                &minimal().replace(
                    "[overrides.\"A1:standard\"]",
                    "[overrides.\"A1\"]\n\"toolhead.speed_limit\" = 70\n\n[overrides.\"A1:standard\"]",
                ),
                "不是 `机型:变体` 形式",
            ),
            (
                "覆盖指向不存在的机型",
                &minimal().replace("[overrides.\"A1:fast\"]", "[overrides.\"A2L:fast\"]"),
                "不存在的机型",
            ),
            (
                "覆盖指向不存在的变体",
                &minimal().replace("[overrides.\"A1:fast\"]", "[overrides.\"A1:nosuch\"]"),
                "不存在的变体",
            ),
            (
                "覆盖里的字段名拼错",
                &minimal().replace(
                    "\"toolhead.speed_limit\" = 80",
                    "\"toolhead.speedlimit\" = 80",
                ),
                "不是可编辑键",
            ),
            (
                "某个机型变体缺 uuid",
                &minimal().replace("uuid = \"22222222-2222-2222-2222-222222222222\"\n", ""),
                "没有 `uuid`",
            ),
            (
                // 追加在末尾会落进最后那个表里（`[overrides."A1:fast"]`），所以要插在**最前面**
                "顶层多一个键",
                &format!("what_is_this = 1\n{}", minimal()),
                "配方读不回来",
            ),
        ];
        for (name, text, expect) in cases {
            let err = Recipe::parse(text)
                .expect_err(&format!("K-G5 红：「{name}」应当报错，实测却读成功了"));
            let msg = err.to_string();
            assert!(
                msg.contains(expect),
                "K-G5 红：「{name}」的报错要包含 `{expect}`，实测：{msg}"
            );
        }
        println!("K-G5 绿：6 种写错法各报一次，且都点了名");
    }

    /// 正文空了要拒绝（没有共享模板可以兜底了）。
    #[test]
    fn an_empty_body_is_refused() {
        let text = minimal().replace(
            "body = \"# uuid: x\\n# machine: A1\\n[toolhead]\\nspeed_limit = 70\\n\"",
            "body = \"\"",
        );
        let err = Recipe::parse(&text).expect_err("空正文必须拒绝");
        assert!(err.to_string().contains("正文是空的"), "实测：{err}");
    }

    /// 变体重复会生成两份同名文件 —— 这条单独咬一次。
    #[test]
    fn duplicate_variants_are_refused() {
        let text = minimal().replace(
            "variants = [\"standard\", \"fast\"]",
            "variants = [\"standard\", \"standard\"]",
        );
        let err = Recipe::parse(&text).expect_err("重复变体必须拒绝");
        assert!(err.to_string().contains("出现了两次"), "实测：{err}");
    }
}
