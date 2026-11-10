//! 落笔/收笔模板骨架的渲染与结构校验。
//!
//! 字段是唯一真相，骨架里只留占位符 —— `{wait}` / `{wait_y}` / `{target}` / `{speed}`。
//! 渲染是一次线性替换：**骨架里没出现的占位符不替换、也不补默认值**。A 系列的骨架
//! 没有 `{wait_y}`（它只动 X），若在这里"顺手补个 0"就会渲染出 `Y0`，而这是绝对坐标 ——
//! 工具头会直接被甩到 Y 零点。所以两个函数都只认骨架里实际出现的东西。

use std::fmt;

/// 骨架里允许出现的占位符。
pub const PLACEHOLDERS: &[&str] = &["wait", "wait_y", "target", "speed"];

/// 渲染用的值。`wait` / `wait_y` / `target` 是 mm，`speed` 是 mm/min。
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PenVars {
    pub wait: f64,
    pub wait_y: f64,
    pub target: f64,
    pub speed: f64,
}

/// 结构错误 —— 阻断保存；数值越界（速度离谱、位置超行程）不进这里，由界面警告。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PenTemplateError {
    /// 出现了白名单以外的占位符。
    UnknownPlaceholder(String),
    /// `{` 没有配对的 `}`（附带它在骨架里的字节位置）。
    UnclosedPlaceholder(usize),
    /// 某行首 token 既不是 G/M/L，也不是注释或空行。
    MalformedLine { line_no: usize, text: String },
}

impl fmt::Display for PenTemplateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownPlaceholder(name) => {
                write!(
                    f,
                    "未知占位符 {{{name}}}，只支持 {{wait}}/{{wait_y}}/{{target}}/{{speed}}"
                )
            }
            Self::UnclosedPlaceholder(pos) => {
                write!(f, "占位符没有闭合：第 {pos} 个字节处有一个 `{{` 缺少 `}}`")
            }
            Self::MalformedLine { line_no, text } => {
                write!(f, "第 {line_no} 行不是 G/M/L 指令：{text}")
            }
        }
    }
}

impl std::error::Error for PenTemplateError {}

/// G-code 习惯：整数不带 `.0`。
fn fmt_num(v: f64) -> String {
    if v.fract().abs() < 1e-9 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// 把骨架 + 字段拼成最终 G-code。
///
/// 先 [`validate`] 再替换：结构不对就报错，不产出半截代码。
pub fn render(skeleton: &str, vars: &PenVars) -> Result<String, PenTemplateError> {
    validate(skeleton)?;
    let mut out = String::with_capacity(skeleton.len() + 16);
    let mut rest = skeleton;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return Err(PenTemplateError::UnclosedPlaceholder(open));
        };
        let value = match &after[..close] {
            "wait" => vars.wait,
            "wait_y" => vars.wait_y,
            "target" => vars.target,
            "speed" => vars.speed,
            other => return Err(PenTemplateError::UnknownPlaceholder(other.to_string())),
        };
        out.push_str(&fmt_num(value));
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    Ok(out)
}

/// 结构校验：占位符白名单 + 每行首 token。
pub fn validate(skeleton: &str) -> Result<(), PenTemplateError> {
    let mut rest = skeleton;
    while let Some(open) = rest.find('{') {
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            return Err(PenTemplateError::UnclosedPlaceholder(open));
        };
        let name = &after[..close];
        if !PLACEHOLDERS.contains(&name) {
            return Err(PenTemplateError::UnknownPlaceholder(name.to_string()));
        }
        rest = &after[close + 1..];
    }
    for (i, line) in skeleton.lines().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with(';') {
            continue;
        }
        let first = t.chars().next().unwrap_or(' ');
        if !matches!(first, 'G' | 'M' | 'L') {
            return Err(PenTemplateError::MalformedLine {
                line_no: i + 1,
                text: t.to_string(),
            });
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn a1_mount_skeleton() -> &'static str {
        "G92 E0\nG1 E-5 F1800\nG92 E0\nL801\nM204 S10000\nG1 X{wait} F42000\nG1 X261 F10000\nG1 X{target} F{speed}\nG1 X{wait} F42000\nG1 F42000"
    }

    fn p1_mount_skeleton() -> &'static str {
        "L803\nG92 E0\nG1 E-10\nG92 E0\nL801\nM204 S10000\nG1 X{wait} Y{wait_y} F30000\nG1 X{target} F{speed}\nG1 X{wait} Y{wait_y} F30000\nG1 F30000"
    }

    fn vars() -> PenVars {
        PenVars {
            wait: 256.0,
            wait_y: 7.0,
            target: 267.0,
            speed: 10000.0,
        }
    }

    /// 整数不带 `.0`，与手写的 G-code 逐字一致。
    #[test]
    fn renders_integers_without_decimal_point() {
        let out = render(a1_mount_skeleton(), &vars()).unwrap();
        assert!(out.contains("G1 X256 F42000"));
        assert!(out.contains("G1 X267 F10000"));
        assert!(!out.contains(".0"));
    }

    /// A1 骨架里没有 `{wait_y}`，渲染结果**不许出现任何 Y**。
    #[test]
    fn a1_skeleton_never_emits_y() {
        let out = render(a1_mount_skeleton(), &vars()).unwrap();
        assert!(!out.contains('Y'), "A1 只动 X，出现 Y 就是撞机：{out}");
    }

    /// P1 骨架有 `{wait_y}`，两处都替换成同一个值。
    #[test]
    fn p1_skeleton_renders_both_y_occurrences() {
        let out = render(p1_mount_skeleton(), &vars()).unwrap();
        assert_eq!(out.matches("G1 X256 Y7 F30000").count(), 2, "{out}");
    }

    /// 没有占位符的老骨架是恒等变换（迁移期间两种数据共存）。
    #[test]
    fn skeleton_without_placeholder_passes_through() {
        let raw = "G92 E0\nG1 X1 F300\n";
        assert_eq!(render(raw, &vars()).unwrap(), raw);
    }

    #[test]
    fn unknown_placeholder_is_rejected() {
        let err = render("G1 X{foo} F300", &vars()).unwrap_err();
        assert_eq!(err, PenTemplateError::UnknownPlaceholder("foo".into()));
    }

    #[test]
    fn unclosed_placeholder_is_rejected() {
        assert!(matches!(
            render("G1 X{wait F300", &vars()),
            Err(PenTemplateError::UnclosedPlaceholder(_))
        ));
    }

    #[test]
    fn malformed_line_is_rejected() {
        let err = validate("G92 E0\nX100\n").unwrap_err();
        assert_eq!(
            err,
            PenTemplateError::MalformedLine {
                line_no: 2,
                text: "X100".into()
            }
        );
    }

    #[test]
    fn comments_and_blank_lines_are_allowed() {
        validate("; 注释\n\nG1 X{wait} F300\n").unwrap();
    }
}
