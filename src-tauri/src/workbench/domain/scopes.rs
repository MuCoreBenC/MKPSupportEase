//! **撤销的作用域**（2026-11-14）。
//!
//! 作者的原话：「最大的问题还是它可以撤销其他页面的，就比如说我点了我当前这个版本，
//! 我撤销它可以撤销到我其他版本的」。
//!
//! 撤销栈是全局一条（历史如此，也是对的：它记的是"我这一趟改了什么"）。缺的是
//! **每一条属于哪一份**。这一份判据就两件事：
//!
//! 1. 一条命令碰过哪些「归属」——值编辑有 `FieldEntry.owner`（`A1` / `A1:STANDARD`），
//!    结构编辑没有逐格字段的就从文件路径反推（`machines/A1.toml` → `A1`）；
//! 2. 当前页面说得出自己认哪几个 token（`A1`、`A1:FAST261111`、`bundles`…），
//!    **两边有交集才允许撤**。
//!
//! 交集而不是相等：一版上的改动既属于这台机型（`A1`）也属于这一版
//! （`A1:FAST261111`），页面在哪个层级看都得认。
//!
//! 判不出来时（归属为空，如崩溃恢复那条合成 op）**不拦** —— 拦了会让那一条永远撤不掉，
//! 而"说不清"不是"不属于你"。

/// 文件路径 → 归属。`machines/A1.toml` → `A1`；`bundles.toml` → `bundles`。
///
/// 只认这三个顶层位置，其它（`registry/`、`layout_schema.toml`）返回 `None` ——
/// 它们属于"参数台/全局定义"，页面自己会给 token。
pub fn owner_of_rel(rel: &str) -> Option<String> {
    let rel = rel.trim_start_matches("./");
    if let Some(rest) = rel.strip_prefix("machines/") {
        return rest
            .strip_suffix(".toml")
            .filter(|s| !s.is_empty())
            .map(str::to_owned);
    }
    let top = rel.split('/').next()?;
    match top {
        "bundles.toml" | "assets.toml" | "brands.toml" | "plates.toml" => {
            Some(top.trim_end_matches(".toml").to_owned())
        }
        _ => None,
    }
}

/// 两个 token 是不是说同一份。
///
/// 相等就算；否则**按分隔符分段**比前缀：`A1` 与 `A1:FAST261111` 是同一台机器，
/// `A1` 与 `A1_MINI` 不是（分段之后前者是 `["A1"]`、后者是 `["A1_MINI"]`）。
fn same_entity(a: &str, b: &str) -> bool {
    if a == b {
        return true;
    }
    let segs = |s: &str| {
        s.split([':', '/', '.'])
            .filter(|x| !x.is_empty())
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };
    let (x, y) = (segs(a), segs(b));
    if x.is_empty() || y.is_empty() {
        return false;
    }
    let (short, long) = if x.len() <= y.len() { (x, y) } else { (y, x) };
    short.iter().zip(long.iter()).all(|(p, q)| p == q)
}

/// 这一条碰过的归属里，有没有一个属于当前作用域。
///
/// - `owners` 为空 ⇒ **放行**（说不清的不拦，见模块头注）；
/// - `scope` 为空 ⇒ 放行（命令行 / 没有页面上下文的老调用）；
/// - 否则要求有交集。
pub fn allows(scope: &[String], owners: &[String]) -> bool {
    if scope.is_empty() || owners.is_empty() {
        return true;
    }
    scope
        .iter()
        .any(|s| owners.iter().any(|o| same_entity(s, o)))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 文件路径反推归属：只认三个顶层位置，其余交给页面自己给 token
    #[test]
    fn owner_of_rel_knows_the_three_shapes() {
        assert_eq!(owner_of_rel("machines/A1.toml").as_deref(), Some("A1"));
        assert_eq!(
            owner_of_rel("./machines/A1_MINI.toml").as_deref(),
            Some("A1_MINI")
        );
        assert_eq!(owner_of_rel("bundles.toml").as_deref(), Some("bundles"));
        assert_eq!(owner_of_rel("assets.toml").as_deref(), Some("assets"));
        assert_eq!(owner_of_rel("registry/param_registry.toml"), None);
        assert_eq!(owner_of_rel("layout_schema.toml"), None);
    }

    /// 一台机器的改动，在"机型页"和"版本页"都该认
    #[test]
    fn a_machine_change_belongs_to_both_levels() {
        let owners = vec!["A1".to_owned()];
        assert!(allows(&["A1".to_owned()], &owners));
        assert!(allows(&["A1:FAST261111".to_owned()], &owners));
        /* 但另一台机器不行 —— 前缀是按分隔符分段的 */
        assert!(!allows(&["A1_MINI".to_owned()], &owners));
        assert!(!allows(&["P1S".to_owned()], &owners));
    }

    /// 一条只碰了某一版的改动，不该被另一版认领
    #[test]
    fn a_version_change_does_not_bleed_into_the_sibling_version() {
        let owners = vec!["A1:FAST261111".to_owned()];
        assert!(allows(&["A1:FAST261111".to_owned()], &owners));
        assert!(allows(&["A1".to_owned()], &owners));
        assert!(!allows(&["A1:FAST260326".to_owned()], &owners));
    }

    /// 说不清（归属为空）与没有上下文都不拦 —— 拦了等于把那条永久锁死
    #[test]
    fn unknown_owners_and_empty_scope_are_not_blocked() {
        assert!(allows(&["A1".to_owned()], &[]));
        assert!(allows(&[], &["P1S".to_owned()]));
        assert!(allows(&[], &[]));
    }
}
