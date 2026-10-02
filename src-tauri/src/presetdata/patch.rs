/*!
 * 按**参数 key** 改一份 MKP 预设 TOML 里的**一个值** —— 保真写回。
 *
 * # 它是干什么的（参数页底座 ②）
 *
 * 参数页编辑的是一份 TOML 草稿（`run/draft-preset.json` 里的整份正文）。用户动一个
 * 参数控件时，落到磁盘上的动作是"**把那个字段的值换掉**"，而不是"把整份参数对象
 * 重新生成一份 TOML" —— 后者会吃掉注释、打乱键序、丢掉人对文件做过的一切手工调整。
 *
 * 所以这里只做一件事：定位那个字段，换掉它的值，其余**一个字节不动**。
 *
 * # 为什么不复用 `crates/preset` 里那份写回
 *
 * `crates/preset/src/write.rs` 有几乎同样的能力（`apply_edits`），但那个 crate
 * **只在 workbench feature 下编**（`mkpse-preset` 是 optional 依赖，见 src-tauri/Cargo.toml），
 * 而参数页是**客户端默认构建**就要用的。把那个 crate 拉进默认构建会破坏隔离边界
 * （"默认构建里 `use preset` 编不过"是刻意的）。`toml_edit` 本身在这儿可用
 * （`presetdata` 读注册表用的就是它），所以这份小实现住在这里。
 *
 * # 边界（刻意不做的事）
 *
 *   · **只改已存在的键**，不新增、不删除 —— 草稿里没有这个字段就是**错**（说明它不属于
 *     这份预设），如实报错，不悄悄造一个出来；
 *   · **不碰任何 decor**（行尾注释、空行、缩进、键序）—— `toml_edit` 只在值上动刀；
 *   · **不收裸 toml 键、不收 section**：定位一律从注册表派生（`param_key` → `(section, toml_key)`），
 *     调用方不该知道 TOML 内部怎么分段的；
 *   · **不管 machineVariants / 层级** —— 那是"值从哪来"的事（`recipe` 那条链），
 *     这里只负责"把这一份文件里的这个字段改成这个值"。
 */

use toml_edit::{DocumentMut, Item, Value};

use crate::error::AppError;

use super::registry::{ParamDef, ParamRegistry, ValueType};

/// 一次字段改动：**参数 key**（注册表主键，如 `toolhead.offset.x`）+ 目标值的**字符串形式**。
///
/// 值为什么是字符串：参数页交上来的就是控件上的文本 / 选项字面量，而"该写成数字还是
/// 带引号的串"取决于注册表里的 `valueType`（由 `patch_preset_toml` 负责翻）。
/// 让调用方自己拼 TOML 字面量 = 把格式知识漏到前端。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEdit {
    pub param_key: String,
    pub value: String,
}

impl FieldEdit {
    pub fn new(param_key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            param_key: param_key.into(),
            value: value.into(),
        }
    }
}

/// 把 `param_key` 映射到这份文件里的位置：**数据域分区 + 字段名**。
///
/// `param_key` 是注册表主键，形状必然是 `<section>.<名字…>`（如 `toolhead.offset.x`）。
/// section 取注册表给的 `section`、字段名取注册表给的 `toml_key` —— 这两个才是权威，
/// 不靠拆 `param_key` 去猜（`offset_x` 与 `offset.x` 对不上，那正是 PR #20 拆内联表
/// 要解决的问题；按 key 后半段猜会又把它猜回去）。
fn locate<'a>(reg: &'a ParamRegistry, param_key: &str) -> Result<&'a ParamDef, AppError> {
    reg.param(param_key).ok_or_else(|| {
        AppError::invalid_argument(format!("不认识这个参数：{param_key}"))
            .with_detail("它不在参数注册表里 —— 参数页不该发来一个注册表里没有的 key")
    })
}

/// 按 `value_type` 把一个字符串翻成 `toml_edit` 的值。
///
/// **形态由 `value_type` 决定，不由"有没有 choices"决定** —— 比如 `prime_enabled` 的选项写
/// `off` / `on`，但 TOML 里是**布尔**；照选项直接写会得到 `"on"`，那会让引擎读不到这个字段。
fn to_toml_value(raw: &str, ty: ValueType) -> Result<Value, AppError> {
    let t = raw.trim();
    match ty {
        ValueType::Float => {
            let n: f64 = t.parse().map_err(|_| {
                AppError::invalid_argument(format!("`{raw}` 不是个数 —— 这一项要浮点数"))
            })?;
            Ok(Value::from(n))
        }
        ValueType::Int => {
            let n: i64 = t.parse().map_err(|_| {
                AppError::invalid_argument(format!("`{raw}` 不是个数 —— 这一项要整数"))
            })?;
            Ok(Value::from(n))
        }
        ValueType::Bool => match t.to_ascii_lowercase().as_str() {
            "true" | "1" | "on" | "yes" => Ok(Value::from(true)),
            "false" | "0" | "off" | "no" => Ok(Value::from(false)),
            _ => Err(AppError::invalid_argument(format!(
                "`{raw}` 不是个开关值 —— 这一项要 true / false"
            ))),
        },
        /* 文本与 G-code 都走字符串；**多行由 toml_edit 自己按需渲染**：
        值里带换行时它会写成 `"""…"""`，单行就是普通串（与产出侧同一套规则） */
        ValueType::Text => Ok(Value::from(t)),
    }
}

/// 改一个字段，返回**新的整份正文**。
///
/// 失败一律不动原文（返回 `Err`，调用方手里的旧正文还是好的）。
pub fn patch_preset_toml(
    raw: &str,
    reg: &ParamRegistry,
    edit: &FieldEdit,
) -> Result<String, AppError> {
    let def = locate(reg, &edit.param_key)?;
    let section = def.section.clone();
    let toml_key = def.toml_key.clone();

    let mut doc: DocumentMut = raw.parse().map_err(|e| {
        AppError::invalid_argument("这份草稿不是合法 TOML，改不动").with_detail(format!("{e}"))
    })?;

    let table = doc
        .get_mut(&section)
        .and_then(Item::as_table_like_mut)
        .ok_or_else(|| {
            AppError::invalid_argument(format!("草稿里没有 `[{section}]` 这一段"))
                .with_detail(format!("参数 {} 按定义住在这一段里", edit.param_key))
        })?;

    let item = table.get_mut(&toml_key).ok_or_else(|| {
        AppError::invalid_argument(format!("草稿的 `[{section}]` 里没有 `{toml_key}`"))
            .with_detail("只改已存在的字段：没有它说明这份预设不含这个参数，别凭空造一个")
    })?;

    let new_value = to_toml_value(&edit.value, def.value_type)?;

    /*
     * **只替换值，保留 decor**：整块替换 `*item` 会把行尾注释一起冲掉
     * （`Item::Value` 的 decor 挂在值上）。所以走 `Item::Value` → 换 `Value`，
     * 并把旧值的 `decor` 原样搬到新值上 —— 注释、缩进、前后空白全跟着留。
     *
     * 整数/浮点/串的**字面量形状**（`-1` 还是 `-1.0`、普通串还是 `"""`）由 `Value::from`
     * 的默认渲染 + 这里的 decor 搬迁共同决定；不做"跟随旧形态"的额外逻辑 ——
     * 参数页改值就是要换成新值，旧形态（比如 `4` 与 `4.0`）跟不跟不重要，
     * 而**注释必须保住**。
     */
    match item {
        Item::Value(old) => {
            let decor = old.decor().clone();
            let mut v = new_value;
            *v.decor_mut() = decor;
            *item = Item::Value(v);
        }
        _ => {
            /* 表 / 数组之类：参数页不该改这种结构（一个参数一个标量字段） */
            return Err(AppError::invalid_argument(format!(
                "`[{section}].{toml_key}` 不是个标量值，改不了"
            ))
            .with_detail("一个参数对应一个标量字段；表 / 数组不归参数页动"));
        }
    }

    Ok(doc.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **真注册表**（`<repo>/presets/registry/param_registry.toml`）——
    /// 判据要拿真数据跑：`param_key` → `(section, toml_key)` 的映射正是这里最要紧的东西，
    /// 用一份手写夹具去测等于自己给自己出题。
    fn reg() -> ParamRegistry {
        let root = super::super::repo_presets_root().expect("仓库预设根（测试专用）");
        ParamRegistry::load_from(&root).expect("读字段定义")
    }

    const DOC: &str = "\
# MKP 预设
[toolhead]
speed_limit = 70 # 速度上限(mm/s)
offset_x = -1 # 笔尖偏移
offset_y = 18.6 # 笔尖偏移
first_pen_revitalization_flag = false
custom_mount_gcode = \"\"

[wiping]
mode = \"tower\"
";

    /// **保真**：换了值，注释 / 键序 / 其它行一个字节不动。
    #[test]
    fn patches_one_value_and_keeps_everything_else() {
        let out = patch_preset_toml(DOC, &reg(), &FieldEdit::new("toolhead.offset.x", "-1.5"))
            .expect("改 offset_x");

        let line = out
            .lines()
            .find(|l| l.starts_with("offset_x"))
            .expect("offset_x 那一行还在");
        assert_eq!(line, "offset_x = -1.5 # 笔尖偏移", "值换了、注释留着");

        /* 其余每一行逐字节不变 */
        for keep in [
            "# MKP 预设",
            "[toolhead]",
            "speed_limit = 70 # 速度上限(mm/s)",
            "offset_y = 18.6 # 笔尖偏移",
            "first_pen_revitalization_flag = false",
            "custom_mount_gcode = \"\"",
            "[wiping]",
            "mode = \"tower\"",
        ] {
            assert!(out.contains(keep), "不该动的东西被动了：{keep}\n---\n{out}");
        }
        /* 而且只有那一行变了 */
        let changed: Vec<(&str, &str)> = DOC
            .lines()
            .zip(out.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert_eq!(changed.len(), 1, "只该有一行变化，实际：{changed:?}");
    }

    /// **形态由 value_type 决定**：bool 写成裸 `true`，不是带引号的 `"on"`。
    #[test]
    fn bool_is_written_as_a_bool_not_a_quoted_string() {
        let out = patch_preset_toml(
            DOC,
            &reg(),
            &FieldEdit::new("toolhead.first_pen_revitalization_flag", "on"),
        )
        .expect("开关值 on → true");
        assert!(
            out.contains("first_pen_revitalization_flag = true"),
            "`on` 该落成裸 true：\n{out}"
        );
        assert!(!out.contains("\"true\""), "不许写成字符串");
    }

    /// 整数按 int 写（`70`），浮点按 float 写（`-1.5`，不丢小数）。
    #[test]
    fn numbers_keep_their_type() {
        let out = patch_preset_toml(DOC, &reg(), &FieldEdit::new("toolhead.speed_limit", "80"))
            .expect("改整数");
        assert!(out.contains("speed_limit = 80"), "\n{out}");

        let out = patch_preset_toml(DOC, &reg(), &FieldEdit::new("toolhead.offset.x", "-1.5"))
            .expect("改浮点");
        assert!(out.contains("offset_x = -1.5"), "\n{out}");
    }

    /// 值里有换行 → 渲染成多行字面量 `"""…"""`（与产出侧同一套规则）。
    #[test]
    fn multiline_text_becomes_a_literal_string() {
        let out = patch_preset_toml(
            DOC,
            &reg(),
            &FieldEdit::new("toolhead.custom_mount_gcode", "G1 X0\nG1 Y0\n"),
        )
        .expect("改 G-code");
        assert!(out.contains("custom_mount_gcode = \"\"\""), "\n{out}");
        assert!(out.contains("G1 X0"), "\n{out}");
    }

    /// 类型不对 → 如实拒（不是静默写下坏值）。
    #[test]
    fn wrong_value_type_is_refused() {
        let e = patch_preset_toml(DOC, &reg(), &FieldEdit::new("toolhead.speed_limit", "快"))
            .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
    }

    /// **注册表里没有这个 key** → 如实拒（参数页不该发来一个注册表里没有的 key）。
    #[test]
    fn unknown_key_is_refused() {
        let e = patch_preset_toml(DOC, &reg(), &FieldEdit::new("toolhead.nope", "1")).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(e.message.contains("不认识这个参数"), "{}", e.message);
    }

    /// **草稿里缺这一段 / 缺这个字段** → 各自如实拒。
    ///
    /// 判据的核心是**只改已存在的字段**：草稿里没有它，说明这份预设不含这个参数，
    /// 不该悄悄造一个出来（那会让"改了一份文件"变成"改了另一份文件"）。
    #[test]
    fn missing_section_and_field_are_refused_not_created() {
        /* 整段不在（`toolhead.x_offset` 确实在注册表里，但这份草稿没有 [toolhead]） */
        let no_section = patch_preset_toml(
            "[wiping]\nmode = \"tower\"\n",
            &reg(),
            &FieldEdit::new("toolhead.offset.x", "-1"),
        )
        .unwrap_err();
        assert_eq!(no_section.code, crate::error::ErrorCode::InvalidArgument);
        assert!(
            no_section.message.contains("没有 `[toolhead]`"),
            "{}",
            no_section.message
        );

        /* 段在、但这个字段不在（草稿的 [toolhead] 只有 speed_limit） */
        let no_field = patch_preset_toml(
            "[toolhead]\nspeed_limit = 70\n",
            &reg(),
            &FieldEdit::new("toolhead.offset.x", "-1"),
        )
        .unwrap_err();
        assert_eq!(no_field.code, crate::error::ErrorCode::InvalidArgument);
        assert!(
            no_field.message.contains("没有 `offset_x`"),
            "{}",
            no_field.message
        );
    }
}
