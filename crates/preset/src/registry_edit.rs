//! 改注册表里的**区间与步进** —— 保注释、保键序、只动那三行。
//!
//! # 为什么这件事能做、又只能在开发态做
//!
//! `param_registry.toml` 是 `include_str!` 编进二进制的（见 `lib.rs`）：
//! 分发出去的 `.app` 里**没有这个文件**，改不了。而从仓库工作副本跑起来的 debug 程序
//! 找得到它（`CARGO_MANIFEST_DIR` 是编译期常量，与 `generate::recipe_path` 同一套路），
//! 所以「在界面上调 min/max/step」是**配方工作台**那扇窗的能力，不是产品功能。
//!
//! 还有一条必须让用户知道：**改完要重新编译才生效**。当前进程用的是编译时那一份文本。
//!
//! # 为什么不是「读成结构体再序列化」
//!
//! 与 `recipe_edit` 同一个理由，只是更严重：这份文件 2461 行、74 条参数、
//! 大量单引号字面量与行内注释。`toml::to_string` 会把它们全抹平，
//! 于是「我只改了一个数字」变成一个 2400 行的 `git diff` —— 而 `git diff`
//! 是这类"改仓库资产"的操作**唯一的安全网**。
//!
//! 所以走 `toml_edit` 定点改值，并且**只认那三个键**（`min` / `max` / `step`）。

use std::path::{Path, PathBuf};

use serde::Serialize;
use toml_edit::{DocumentMut, Value};

/// 仓库里那份参数注册表的路径（与 `generate::recipe_path` 同一套路：编译期常量）。
///
/// **M4c 起指向唯一真源** `presets/registry/param_registry.toml`，
/// 与 `lib.rs` 那个 `include_str!` 咬的是同一个文件 —— 否则"改完重编译才生效"
/// 这句话就不成立了（改的是一份、编进去的是另一份）。
///
/// 代价照实说：这个模块从此**有能力写真数据**。它只在开发态有意义（见模块头），
/// 而兜着它的是「`presets/` 12 个文件 sha256 在整个迁移里不变」那条验收。
pub fn registry_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets/registry/param_registry.toml")
}

/// 一次区间编辑的结果。`changed` 是**真的动了的那几个键名**（给界面报数用）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RangeEdit {
    pub param_key: String,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub step: Option<f64>,
    pub changed: Vec<String>,
}

/// 把一个 `f64` 写成 TOML 字面量。
///
/// **整数值也要带小数点**（`-50` 写成 `-50.0`）：注册表里这三个键是浮点，
/// 写成整数字面量之后 `serde` 那边就成了 `i64`，`ParamEntry` 会解析失败。
/// 形态由**文本**决定而不是由 API 决定（与 `recipe_edit` 同一手法）。
fn num_literal(x: f64) -> String {
    if x.fract() == 0.0 {
        format!("{x:.1}")
    } else {
        format!("{x}")
    }
}

fn value_from_raw(raw: &str) -> Result<Value, String> {
    let doc = format!("x = {raw}\n")
        .parse::<DocumentMut>()
        .map_err(|e| format!("`{raw}` 不是一个合法的 TOML 值：{e}"))?;
    doc.get("x")
        .and_then(|i| i.as_value())
        .cloned()
        .ok_or_else(|| format!("`{raw}` 解析出来不是一个值"))
}

/// 表里那个键现在的数值（整数也当数用）。
fn num_of(t: &toml_edit::Table, key: &str) -> Option<f64> {
    let v = t.get(key)?.as_value()?;
    v.as_float().or_else(|| v.as_integer().map(|i| i as f64))
}

/// 改一条参数的全局 `min` / `max` / `step`，返回新文本。
///
/// **三个入参是「改完之后的状态」，不是增量**：`None` = 这一档留空 ⇒
/// 把那个键从注册表里**删掉**（注册表里没有 `min` 就是没有下界）。
/// 界面上清空那个输入框就是这个意思。
///
/// 三条校验，任一条不过就**整个拒掉、一个字节都不写**：
/// - `min > max`：那样任何值都过不了闸，等于把这个参数锁死；
/// - `step <= 0`：`<input step>` 会当成非法值，界面上的 ▲▼ 直接失灵；
/// - 现有 `defaultValue` 落在新区间外：**这一条是最要紧的** ——
///   写进去之后每次读预设都会在校验那一关炸，而那时人早忘了改过约束。
pub fn set_range_in_text(
    text: &str,
    param_key: &str,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
) -> Result<(String, RangeEdit), String> {
    if let (Some(lo), Some(hi)) = (min, max)
        && lo > hi
    {
        return Err(format!(
            "最小值 {lo} 比最大值 {hi} 还大 —— 那样这个参数就锁死了"
        ));
    }
    if let Some(s) = step.filter(|s| *s <= 0.0) {
        return Err(format!("步进要是正数，收到 {s}"));
    }

    let mut doc = text
        .parse::<DocumentMut>()
        .map_err(|e| format!("注册表读不回来：{e}"))?;

    let params = doc
        .get_mut("params")
        .and_then(|i| i.as_array_of_tables_mut())
        .ok_or_else(|| "注册表里找不到 [[params]]".to_string())?;

    let entry = params
        .iter_mut()
        .find(|t| t.get("key").and_then(|i| i.as_str()) == Some(param_key))
        .ok_or_else(|| format!("注册表里没有这个参数：{param_key}"))?;

    // 默认值必须落在新区间里（它是"这个参数最起码得能用"的那个值）
    if let Some(d) = num_of(entry, "defaultValue") {
        if let Some(lo) = min.filter(|lo| d < *lo) {
            return Err(format!(
                "这个参数的默认值是 {d}，比新的最小值 {lo} 还小 —— \
                 写进去之后每次读预设都会在校验那一关报错"
            ));
        }
        if let Some(hi) = max.filter(|hi| d > *hi) {
            return Err(format!(
                "这个参数的默认值是 {d}，比新的最大值 {hi} 还大 —— \
                 写进去之后每次读预设都会在校验那一关报错"
            ));
        }
    }

    let mut changed = Vec::new();
    for (key, want) in [("min", min), ("max", max), ("step", step)] {
        let had = num_of(entry, key);
        match want {
            Some(x) => {
                if had == Some(x) {
                    continue;
                }
                let v = value_from_raw(&num_literal(x))?;
                // 老值的装饰（前后空白、行尾注释）搬过来，免得写回时把注释挤掉
                let decor = entry
                    .get(key)
                    .and_then(|i| i.as_value())
                    .map(|old| old.decor().clone());
                let mut v = v;
                if let Some(d) = decor {
                    *v.decor_mut() = d;
                }
                entry.insert(key, toml_edit::Item::Value(v));
                changed.push(key.to_string());
            }
            None => {
                if had.is_some() {
                    entry.remove(key);
                    changed.push(key.to_string());
                }
            }
        }
    }

    Ok((
        doc.to_string(),
        RangeEdit {
            param_key: param_key.to_string(),
            min,
            max,
            step,
            changed,
        },
    ))
}

/// 同上，但直接改**仓库工作副本**里那份文件。
///
/// 没有改动时**不写盘**：无意义的 mtime 跳动会让 `git status` 与构建缓存都变吵。
pub fn set_range(
    param_key: &str,
    min: Option<f64>,
    max: Option<f64>,
    step: Option<f64>,
) -> Result<RangeEdit, String> {
    let path = registry_path();
    let text = std::fs::read_to_string(&path)
        .map_err(|e| format!("读不到注册表（{}）：{e}", path.display()))?;
    let (out, edit) = set_range_in_text(&text, param_key, min, max, step)?;
    if edit.changed.is_empty() {
        return Ok(edit);
    }
    std::fs::write(&path, out).map_err(|e| format!("写不进注册表（{}）：{e}", path.display()))?;
    Ok(edit)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 一份缩小版注册表：够这几条测试用，形态（单引号、行尾注释、子表）都留着。
    const SAMPLE: &str = "\
# 头部注释不许被吃掉
[[params]]
key = 'toolhead.MKP_retract'
label = '回抽长度'
valueType = 'float'
defaultValue = 0.0
min = -50.0
max = 50.0
step = 0.1 # 行尾注释

[params.layout]
sectionId = 'motion'
order = 200.0

[[params]]
key = 'wiping.wiper_x'
valueType = 'float'
defaultValue = 20.0
min = 0.0
max = 226.0
";

    #[test]
    fn only_those_lines_change() {
        let (out, edit) = set_range_in_text(
            SAMPLE,
            "toolhead.MKP_retract",
            Some(-60.0),
            Some(60.0),
            Some(0.5),
        )
        .expect("这一改应当被接受");
        assert_eq!(edit.changed, ["min", "max", "step"]);

        // 逐行比：只许那三行不同
        let diff: Vec<_> = SAMPLE
            .lines()
            .zip(out.lines())
            .filter(|(a, b)| a != b)
            .map(|(a, b)| (a.to_string(), b.to_string()))
            .collect();
        assert_eq!(diff.len(), 3, "动的行数不对：{diff:?}");
        assert!(out.contains("min = -60.0"), "min 没写对：\n{out}");
        assert!(out.contains("max = 60.0"));
        // **行尾注释要还在**（全量重写就会把它吃掉，那正是不用 to_string 序列化的理由）
        assert!(
            out.contains("step = 0.5 # 行尾注释"),
            "行尾注释被吃了：\n{out}"
        );
        assert!(out.starts_with("# 头部注释不许被吃掉"));
        assert_eq!(SAMPLE.lines().count(), out.lines().count());
    }

    #[test]
    fn integers_keep_the_decimal_point() {
        // 写成 `50` 的话 serde 那边会解析成 i64，ParamEntry 直接读不回来
        let (out, _) = set_range_in_text(SAMPLE, "wiping.wiper_x", Some(0.0), Some(150.0), None)
            .expect("这一改应当被接受");
        assert!(out.contains("max = 150.0"), "整数值丢了小数点：\n{out}");
    }

    #[test]
    fn an_empty_bound_removes_the_key() {
        let (out, edit) = set_range_in_text(SAMPLE, "wiping.wiper_x", None, Some(226.0), None)
            .expect("清空下界是允许的：注册表里没有 min 就是没有下界");
        assert_eq!(edit.changed, ["min"]);
        let entry = out
            .split("[[params]]")
            .find(|s| s.contains("wiping.wiper_x"))
            .expect("那一条还在");
        assert!(!entry.contains("min ="), "min 没被删掉：\n{entry}");
    }

    #[test]
    fn min_above_max_is_refused() {
        let err = set_range_in_text(SAMPLE, "wiping.wiper_x", Some(300.0), Some(10.0), None)
            .expect_err("min > max 必须拒");
        assert!(err.contains("锁死"), "报错没说清后果：{err}");
    }

    #[test]
    fn a_zero_step_is_refused() {
        let err = set_range_in_text(SAMPLE, "wiping.wiper_x", None, None, Some(0.0))
            .expect_err("step = 0 必须拒");
        assert!(err.contains("正数"), "{err}");
    }

    /// 最要紧的那一条：新区间把现有默认值关在外面 ⇒ 拒。
    #[test]
    fn a_default_outside_the_new_range_is_refused() {
        let err = set_range_in_text(SAMPLE, "wiping.wiper_x", Some(50.0), Some(226.0), None)
            .expect_err("默认值 20 落在 [50, 226] 之外，必须拒");
        assert!(err.contains("默认值"), "{err}");
        assert!(
            err.contains("校验"),
            "报错要说清后果（下次读预设就炸）：{err}"
        );
    }

    #[test]
    fn an_unknown_param_is_reported_by_name() {
        let err = set_range_in_text(SAMPLE, "nope.nothing", None, None, None)
            .expect_err("不存在的参数要报错");
        assert!(err.contains("nope.nothing"), "{err}");
    }

    #[test]
    fn no_change_means_nothing_changed() {
        let (out, edit) = set_range_in_text(
            SAMPLE,
            "toolhead.MKP_retract",
            Some(-50.0),
            Some(50.0),
            Some(0.1),
        )
        .expect("同值提交是允许的");
        assert!(edit.changed.is_empty(), "同值不该算改动：{edit:?}");
        assert_eq!(out, SAMPLE, "同值提交不许动文本");
    }

    /// 真源那份文件本身能被这段代码读回来（哨兵：路径与格式都对）。
    #[test]
    fn the_real_registry_parses() {
        let text = std::fs::read_to_string(registry_path()).expect("读得到真的注册表");
        let (out, edit) = set_range_in_text(
            &text,
            "toolhead.MKP_retract",
            Some(-50.0),
            Some(50.0),
            Some(0.1),
        )
        .expect("真注册表要能读能改");
        assert!(
            edit.changed.is_empty(),
            "真源的那三个值应当就是这三个：{edit:?}"
        );
        assert_eq!(out, text, "同值提交不许动真源");
    }
}
