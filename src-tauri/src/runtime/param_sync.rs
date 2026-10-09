//! **逐参数「官方更新」的比对**——一份用户预设 vs 隐藏 baseline vs 官方当前版。
//!
//! # 三方是什么
//!
//! ```text
//!   我      presets-mine/… 里现在写着什么          （用户只有一个可见对象）
//!   旧      我「上次处理到的那一版官方」的默认值    （藏在 baseline 里，用户看不见）
//!   新      官方当前最新版里这一项是什么           （目录里那一版）
//! ```
//!
//! 「我上次处理到的那一版」由 **逐参数决定账**（[`ParamDecision`]）回答：
//! 某一项处理过就把它的水位推到当时的官方版摘要；没处理过的项退回整份的血统
//! `based_on_sha256`。于是：
//!
//! ```text
//! 参考版(key) = decisions[key].sha256   ??  based_on_sha256
//! 待处理(key) = 官方新值 ≠ 官方旧值        （两边都得有这一项）
//! ```
//!
//! 这个写法同时满足三件必须成立的事（规则文档 §14/§15）：
//!
//! - **连续发多版**：水位只记"我处理到哪一版"，比对的是「那一版 → 当前版」，
//!   云端不需要提供任何中间版本（场景 E）；
//! - **保持之后不再自动跳变**：保持 = 水位推到当前版、文件一个字不动 ——
//!   下一次官方再改它才会重新进待处理（场景 C）；
//! - **采用之后还能再进待处理**：采用 = 文件写成官方新值 + 水位推到当前版（场景 D）。
//!
//! # 这里只算，不读盘
//!
//! 参数值、baseline 正文、血统、决定账全部由调用方（`ipc::param_sync`）读好之后交进来。
//! 纯函数才好摆判据：三方各摆一份 map，断言直接落在结果上。
//!
//! 一份预设**没有血统**（导入的 / 手工拷的）时 `based_on_sha256` 是 `None`：
//! 那就没有"官方旧值"可比，任何一项都不进待处理 —— **不猜它从哪一版来**。

use std::collections::BTreeMap;

use crate::presetdata::registry::ParamDef;

use super::app_state::{ParamDecision, ParamDecisionKind};

/// 一个参数上的四方账（给界面用的那一份形状）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParamSyncEntry {
    /// 参数 key（注册表的键）
    pub key: String,
    /// 我这份文件里现在写着的值。文件里没有这一项 ⇒ `None`
    /// （"不含这个参数"与"这个参数是空值"是两件事）
    pub mine: Option<String>,
    /// 官方**旧值**（我上次处理到的那一版）。没有血统 / 拿不到那一版 ⇒ `None`
    pub official_old: Option<String>,
    /// 官方**当前最新值**。官方当前版字节不在本机 ⇒ `None`
    pub official_new: Option<String>,
    /// 官方改过这一项、而我还没处理
    pub pending: bool,
    /// 我已经对**当前官方版**处理过这一项（采用 / 保持）
    pub decided: Option<ParamDecisionKind>,
}

/// 一次比对要的全部输入。字段各自独立，摆判据时只填用得上那几个。
pub struct SyncInput<'a> {
    /// 参数定义（顺序就是结果里的顺序 —— 页面按注册表顺序列）
    pub defs: &'a [ParamDef],
    /// 我这份文件里真的有的那些参数值（`presetdata::params::read_param_values` 的读数）
    pub mine: &'a BTreeMap<String, String>,
    /// 逐参数决定账（这本是某一份预设的）
    pub decisions: &'a BTreeMap<String, ParamDecision>,
    /// 这份预设的血统里记的"从哪一版官方来"
    pub based_on_sha256: Option<&'a str>,
    /// 官方当前版的正文摘要（目录登记的那一版）
    pub current_sha256: Option<&'a str>,
    /// 摘要 → 那一版官方的参数值（baseline 里读出来的）。
    /// **至少要含** `current_sha256` 与各个 `decisions[*].sha256`；缺谁那一项就算不出来 —— 如实给 `None`，不编。
    pub by_sha: &'a BTreeMap<String, BTreeMap<String, String>>,
}

/// 比一遍：逐项的四方账 + 待处理条数。
///
/// 待处理条数就地从结果里数（同一份数据算两遍必漂）—— 调用方不需要自己过滤。
pub fn diff(input: &SyncInput<'_>) -> (Vec<ParamSyncEntry>, usize) {
    let current = input.current_sha256.and_then(|sha| input.by_sha.get(sha));

    let mut entries = Vec::with_capacity(input.defs.len());
    let mut pending = 0usize;

    for def in input.defs {
        let key = def.key.as_str();
        /*
         * 参考版 = **这一项**处理到的那一版（有决定账）否则整份的血统。
         * 决定账里那串摘要来自盘上的状态文件 —— 拿不到那一版就把这一项当"比不出来"，
         * 不报错（宁可少报一项，也不许瞎猜一个值）。
         */
        let decision = input.decisions.get(key);
        let ref_sha = decision
            .map(|d| d.sha256.as_str())
            .or(input.based_on_sha256);
        let old = ref_sha
            .and_then(|sha| input.by_sha.get(sha))
            .and_then(|m| m.get(key));
        let new = current.and_then(|m| m.get(key));

        /* 待处理：官方把这一项改了（旧值新值都在、且不同） */
        let is_pending = matches!((old, new), (Some(o), Some(n)) if o != n);
        if is_pending {
            pending += 1;
        }

        /* "我已经对当前版处理过"：决定账上的水位就是当前版 */
        let decided = decision
            .filter(|d| Some(d.sha256.as_str()) == input.current_sha256)
            .map(|d| d.kind);

        entries.push(ParamSyncEntry {
            key: def.key.clone(),
            mine: input.mine.get(key).cloned(),
            official_old: old.cloned(),
            official_new: new.cloned(),
            pending: is_pending,
            decided,
        });
    }

    (entries, pending)
}

/// 这一份的参数里，**要用到的官方版本摘要**都有哪些 —— 调用方据此决定从 baseline 读哪几份。
///
/// 含义与 [`diff`] 里那条一致：逐项决定账上的水位，加上整份血统里那一版。
/// 去重；**不含当前版**（当前版由调用方单独保证：它可能是刚按需取回来的那一份）。
pub fn needed_shas(
    defs: &[ParamDef],
    decisions: &BTreeMap<String, ParamDecision>,
    based_on_sha256: Option<&str>,
) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut push = |sha: &str| {
        if !out.iter().any(|x| x == sha) {
            out.push(sha.to_owned());
        }
    };
    for def in defs {
        if let Some(d) = decisions.get(def.key.as_str()) {
            push(&d.sha256);
        }
    }
    if let Some(sha) = based_on_sha256 {
        push(sha);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::presetdata::registry::ParamRegistry;

    /// 比对逻辑与"字段有多少项"无关，但字段定义得是**真的**那一份：
    /// 这里只按 key 挑出要用的几条（手工拼 ParamDef 等于自己出题）。
    fn defs_of(keys: &[&str]) -> Vec<ParamDef> {
        let root = crate::presetdata::repo_presets_root().expect("仓库预设根（测试专用）");
        let all = ParamRegistry::load_from(&root).expect("读字段定义");
        keys.iter()
            .map(|k| {
                all.params()
                    .iter()
                    .find(|p| p.key == *k)
                    .unwrap_or_else(|| panic!("注册表里没有 {k}"))
                    .clone()
            })
            .collect()
    }

    /// 真注册表里的三条（本模块的判据要摆多键）
    const K1: &str = "toolhead.offset.x";
    const K2: &str = "toolhead.offset.y";
    const K3: &str = "toolhead.speed_limit";

    fn map(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect()
    }

    fn decisions_of(pairs: &[(&str, &str, ParamDecisionKind)]) -> BTreeMap<String, ParamDecision> {
        pairs
            .iter()
            .map(|(k, sha, kind)| {
                (
                    (*k).to_owned(),
                    ParamDecision {
                        sha256: (*sha).to_owned(),
                        kind: *kind,
                    },
                )
            })
            .collect()
    }

    /// 三方都摆齐的一次比对（判据里反复用，省得每个用例写一遍装订）
    fn run(
        keys: &[&str],
        mine: BTreeMap<String, String>,
        by_sha: BTreeMap<String, BTreeMap<String, String>>,
        decisions: BTreeMap<String, ParamDecision>,
        based: Option<&str>,
        current: Option<&str>,
    ) -> (Vec<ParamSyncEntry>, usize) {
        let defs = defs_of(keys);
        diff(&SyncInput {
            defs: &defs,
            mine: &mine,
            decisions: &decisions,
            based_on_sha256: based,
            current_sha256: current,
            by_sha: &by_sha,
        })
    }

    /// 场景 A：用户没改过、官方改了 → 这一项待处理，三方是「我 = 旧 → 新」
    #[test]
    fn an_untouched_param_comes_back_as_pending() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("old".to_owned(), map(&[(K1, "-1")]));
        by_sha.insert("new".to_owned(), map(&[(K1, "-1.5")]));

        let (out, pending) = run(
            &[K1],
            map(&[(K1, "-1")]),
            by_sha,
            BTreeMap::new(),
            Some("old"),
            Some("new"),
        );

        assert_eq!(pending, 1);
        assert_eq!(out[0].key, K1);
        assert_eq!(out[0].mine.as_deref(), Some("-1"));
        assert_eq!(out[0].official_old.as_deref(), Some("-1"));
        assert_eq!(out[0].official_new.as_deref(), Some("-1.5"));
        assert!(out[0].pending);
        assert_eq!(out[0].decided, None);
    }

    /// 场景 B：用户改过的那一项，官方也改了 —— 照样待处理，但「我」那一栏是他的值
    #[test]
    fn a_user_edited_param_keeps_the_users_value() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("old".to_owned(), map(&[(K1, "-1")]));
        by_sha.insert("new".to_owned(), map(&[(K1, "-1.5")]));

        let (out, pending) = run(
            &[K1],
            map(&[(K1, "-2")]),
            by_sha,
            BTreeMap::new(),
            Some("old"),
            Some("new"),
        );

        assert_eq!(pending, 1, "官方动过就要让人看得见");
        assert_eq!(out[0].mine.as_deref(), Some("-2"), "我的值不许被官方顶掉");
    }

    /// 场景 C：保持过之后水位推到当前版 —— 同一版不再待处理
    #[test]
    fn holding_retires_the_pending_flag_for_that_version() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("old".to_owned(), map(&[(K1, "-1")]));
        by_sha.insert("new".to_owned(), map(&[(K1, "-1.5")]));

        let (out, pending) = run(
            &[K1],
            map(&[(K1, "-1")]),
            by_sha,
            decisions_of(&[(K1, "new", ParamDecisionKind::Hold)]),
            Some("old"),
            Some("new"),
        );

        assert_eq!(pending, 0, "处理过就不该再挂着");
        assert_eq!(out[0].decided, Some(ParamDecisionKind::Hold));
        assert_eq!(out[0].mine.as_deref(), Some("-1"), "保持 = 文件一个字没动");
    }

    /// 场景 C（续）/ E：官方又发一版 —— 旧值取的是「我处理到的那一版」，不是更早的血统
    #[test]
    fn after_another_release_the_old_value_follows_the_watermark() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("based".to_owned(), map(&[(K1, "-1")]));
        by_sha.insert("held".to_owned(), map(&[(K1, "-1.5")]));
        by_sha.insert("newer".to_owned(), map(&[(K1, "-2")]));

        let (out, pending) = run(
            &[K1],
            map(&[(K1, "-1")]),
            by_sha,
            decisions_of(&[(K1, "held", ParamDecisionKind::Hold)]),
            Some("based"),
            Some("newer"),
        );

        assert_eq!(pending, 1, "官方又改了这一项");
        assert_eq!(
            out[0].official_old.as_deref(),
            Some("-1.5"),
            "旧值是「我上次处理到的那一版」，不是血统里那一版"
        );
        assert_eq!(out[0].official_new.as_deref(), Some("-2"));
        assert_eq!(out[0].mine.as_deref(), Some("-1"), "我的值还是我保持的那个");
    }

    /// 场景 D：采用之后水位也推走；再发新版时它还能重新进待处理
    #[test]
    fn adopting_moves_the_watermark_too() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("adopted".to_owned(), map(&[(K1, "-1.5")]));
        by_sha.insert("newer".to_owned(), map(&[(K1, "-2")]));

        let (out, pending) = run(
            &[K1],
            map(&[(K1, "-1.5")]),
            by_sha,
            decisions_of(&[(K1, "adopted", ParamDecisionKind::Adopt)]),
            Some("based"),
            Some("newer"),
        );

        assert_eq!(pending, 1, "后续官方发布时它可以再次进入待处理");
        assert_eq!(out[0].decided, None, "这是新的一版，决定账不是针对它的");
    }

    /// 只有一边有这一项 / 值没变 —— 一律不算待处理，三方的缺格如实给 None
    #[test]
    fn missing_sides_and_unchanged_values_are_never_pending() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("old".to_owned(), map(&[(K1, "1"), (K2, "2")]));
        by_sha.insert("new".to_owned(), map(&[(K1, "1"), (K3, "3")]));

        let (out, pending) = run(
            &[K1, K2, K3],
            map(&[(K1, "1")]),
            by_sha,
            BTreeMap::new(),
            Some("old"),
            Some("new"),
        );

        assert_eq!(pending, 0, "只有一边有这一项时比不出「改了」：{out:?}");
        assert_eq!(
            out[0].official_new.as_deref(),
            Some("1"),
            "值没变就不算更新"
        );
        assert_eq!(out[1].official_new, None, "官方新版里没有它");
        assert_eq!(out[2].official_old, None, "官方旧版里没有它");
    }

    /// 没有血统（导入的 / 手工拷的）⇒ 没有官方旧值可比，任何一项都不进待处理
    #[test]
    fn a_preset_without_lineage_has_no_updates() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("new".to_owned(), map(&[(K1, "-1.5")]));

        let (out, pending) = run(
            &[K1],
            map(&[(K1, "-2")]),
            by_sha,
            BTreeMap::new(),
            None,
            Some("new"),
        );

        assert_eq!(pending, 0);
        assert_eq!(out[0].official_old, None);
        assert_eq!(
            out[0].official_new.as_deref(),
            Some("-1.5"),
            "官方值照旧看得见"
        );
    }

    /// 官方当前版的字节还没到本机（`by_sha` 里没有它）⇒ 一项都算不出来，如实给 None
    #[test]
    fn an_unavailable_official_version_yields_no_entries_values() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("old".to_owned(), map(&[(K1, "-1")]));

        let (out, pending) = run(
            &[K1],
            map(&[(K1, "-1")]),
            by_sha,
            BTreeMap::new(),
            Some("old"),
            Some("new"),
        );

        assert_eq!(pending, 0, "拿不到官方新值就不许报一个凭空来的待处理");
        assert_eq!(out[0].official_new, None);
    }

    /// 决定账上的水位**不等于**当前版时，decided 要报 `None`（那是上一版的事）
    #[test]
    fn a_decision_on_an_older_version_is_not_reported_as_decided() {
        let mut by_sha = BTreeMap::new();
        by_sha.insert("held".to_owned(), map(&[(K1, "-1.5")]));
        by_sha.insert("newer".to_owned(), map(&[(K1, "-2")]));

        let (out, _) = run(
            &[K1],
            map(&[(K1, "-1")]),
            by_sha,
            decisions_of(&[(K1, "held", ParamDecisionKind::Hold)]),
            None,
            Some("newer"),
        );

        assert_eq!(out[0].decided, None);
    }

    /// 要读哪几份 baseline：逐项水位 + 血统，去重
    #[test]
    fn needed_shas_collects_watermarks_and_lineage() {
        let defs = defs_of(&[K1, K2, K3]);
        let decisions = decisions_of(&[
            (K1, "v2", ParamDecisionKind::Adopt),
            (K2, "v2", ParamDecisionKind::Hold),
            (K3, "v3", ParamDecisionKind::Hold),
        ]);

        assert_eq!(
            needed_shas(&defs, &decisions, Some("v1")),
            vec!["v2".to_owned(), "v3".to_owned(), "v1".to_owned()]
        );
        assert_eq!(
            needed_shas(&defs, &BTreeMap::new(), None),
            Vec::<String>::new()
        );
    }
}
