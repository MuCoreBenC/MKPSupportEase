//! `content/fallback_registry.json` —— 回退登记表（厨房的应急规矩）。
//!
//! 登记的是「后处理运行时改写用户配置」的行为：用户给的配置在某种情况下做不出来时，
//! 客户端会自动改成别的。每条说清楚**什么情况触发**、**从什么改成什么**、
//! **改了之后算不算事故**。
//!
//! # 这一页是只读的（doc §17）
//!
//! `guide` 长文里上游自己写着：「禁止手改 content/*.json —— 改动请在 Panel
//! 「回退登记表」页或本 TOML 里做，然后构建 + 同步」。真正的源是
//! `source/registry/fallback_registry.toml`，在另一个仓库。
//!
//! 所以工作台这一页**只展示**。参考实现给每条挂了一个开关，点了只改本机内存 ——
//! 那是一个看得见、按得动、什么也不影响的开关，比没有开关更糟。我们不放开关，
//! 放一行「这张表由上游维护，改动请回 mkppanel」。
//!
//! # `enabled` 实测 20/20 全是 true
//!
//! 但**照样读**，而且不给它默认值：这个字段有真实语义（false = 触发即报错中止，
//! 不再静默改写），只是当前没有一条关掉。跟 `mergeable`/`selectable` 那种
//! "永远为真且没有语义"的字段不是一回事，所以不能同样处理。
//!
//! # `guide` 要原样保留
//!
//! 它是带换行与 Markdown 标题的长文（约 1.5KB）。压成一行、或者只显示第一句，
//! 就把"怎么改"这件事弄丢了 —— 而那正是看这一页的人要找的。

use std::collections::BTreeSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::workbench::paths;
use crate::workbench::store::read_json;

const FALLBACK_REL: &str = "content/fallback_registry.json";

/// 回退的动机分类。实测五种
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    /// 用户没给，补一个默认
    Default,
    /// 从别的字段推断出来
    Infer,
    /// 老版本配置迁移到新写法
    Migration,
    /// 用户给了，但当前情况下做不出来，强行改写
    Override,
    /// 出错后的补救
    Recovery,
}

impl Category {
    /// 界面上的分类名。**一个词一处来源**（doc §14）——
    /// 这里是唯一的一处，前端不再自己拼中文
    pub fn label(self) -> &'static str {
        match self {
            Self::Default => "补默认",
            Self::Infer => "推断",
            Self::Migration => "迁移",
            Self::Override => "强制改写",
            Self::Recovery => "补救",
        }
    }
}

/// 触发之后算不算事故。实测只有这两级 —— **没有 error**，
/// 因为"改写失败"不走这张表，走的是 `enabled: false` 时的直接中止
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    /// 正常行为，留个痕
    Info,
    /// 值得看一眼
    Warn,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Info => "留痕",
            Self::Warn => "提醒",
        }
    }
}

/// 一条回退规矩
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Rule {
    /// 稳定 id，如 `support_fallback_to_tower`
    pub id: String,
    pub category: Category,
    /// 什么情况触发。中文短句，如「仅有支撑面无支撑体（无支撑体可擦）」
    pub trigger: String,
    /// 从什么值改过来，如 `"disk/none"`
    pub from: String,
    /// 改成什么值，如 `"tower"`
    pub to: String,
    /// **`false` = 触发即报错中止**（fail fast），不再静默改写。
    /// 实测 20/20 都是 true，但这个字段有真语义，所以不给默认值 ——
    /// 上游漏写时应该报错，而不是被当成 true
    pub enabled: bool,
    pub severity: Severity,
    /// 为什么这么改。这是这张表里唯一解释"改了会怎样"的字段
    pub desc: String,
    /// 触发后写进运行报告的哪个字段，如 `supportFallback`
    pub report_field: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawFile {
    version: u32,
    #[serde(default)]
    updated: String,
    #[serde(default)]
    guide: String,
    #[serde(default)]
    fallbacks: Vec<Rule>,
}

pub struct FallbackRegistry {
    rules: Vec<Rule>,
    /// 表结构自己的版本。实测 1
    pub version: u32,
    pub updated: String,
    /// 上游写的「这是什么 / 怎么改」长文。**带换行，原样保留**
    pub guide: String,
}

impl std::fmt::Debug for FallbackRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "FallbackRegistry({} 条 / v{} / updated {})",
            self.rules.len(),
            self.version,
            self.updated
        )
    }
}

impl FallbackRegistry {
    pub fn load() -> Result<Self, AppError> {
        let root = paths::upstream_root().ok_or_else(|| {
            AppError::not_found("找不到上游预设仓库 mkpse-presets")
                .with_detail(format!("试过：{}", paths::upstream_candidates().join("；")))
        })?;
        Self::load_from(&root)
    }

    pub fn load_from(root: &Path) -> Result<Self, AppError> {
        let raw: RawFile = read_json(&root.join(FALLBACK_REL), "回退登记表")?;
        let out = Self {
            rules: raw.fallbacks,
            version: raw.version,
            updated: raw.updated,
            guide: raw.guide,
        };
        out.check_consistency()?;
        Ok(out)
    }

    /// 两条断言
    pub fn check_consistency(&self) -> Result<(), AppError> {
        // ① id 不重复 —— 运行报告按 id 对账，重复 id 会让"是哪条触发的"说不清
        let ids: BTreeSet<&str> = self.rules.iter().map(|r| r.id.as_str()).collect();
        if ids.len() != self.rules.len() {
            let mut seen = BTreeSet::new();
            let dup: Vec<&str> = self
                .rules
                .iter()
                .map(|r| r.id.as_str())
                .filter(|id| !seen.insert(*id))
                .collect();
            return Err(AppError::corrupted("回退登记表里有重复的 id")
                .with_detail(format!("重复的：{}", dup.join("、"))));
        }

        // ② 每条都要有 desc —— 这张表唯一解释"改了会怎样"的字段。
        //    空的话界面上就是一行"A 改成 B"，看的人不知道为什么，也不知道该不该管
        let blank: Vec<&str> = self
            .rules
            .iter()
            .filter(|r| r.desc.trim().is_empty())
            .map(|r| r.id.as_str())
            .collect();
        if !blank.is_empty() {
            return Err(
                AppError::corrupted("有回退规则没写说明").with_detail(format!(
                    "{} —— 没有说明，界面上只剩一行「A 改成 B」",
                    blank.join("、")
                )),
            );
        }

        Ok(())
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn rule(&self, id: &str) -> Option<&Rule> {
        self.rules.iter().find(|r| r.id == id)
    }

    /// 按分类分组，分类按 [`Category`] 的声明顺序。
    /// **只返回真有规则的分类** —— 空分类会在界面上留一个永远没有内容的小标题
    pub fn by_category(&self) -> Vec<(Category, Vec<&Rule>)> {
        let mut out: Vec<(Category, Vec<&Rule>)> = Vec::new();
        for cat in [
            Category::Default,
            Category::Infer,
            Category::Migration,
            Category::Override,
            Category::Recovery,
        ] {
            let hit: Vec<&Rule> = self.rules.iter().filter(|r| r.category == cat).collect();
            if !hit.is_empty() {
                out.push((cat, hit));
            }
        }
        out
    }

    /// 被关掉的规则：它们触发时会**直接报错中止**，不再静默改写。
    /// 实测当前一条都没有，所以这个列表为空是正常状态，界面上写「当前无」而不是留白
    pub fn disabled(&self) -> Vec<&Rule> {
        self.rules.iter().filter(|r| !r.enabled).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule(id: &str, cat: &str, sev: &str) -> serde_json::Value {
        serde_json::json!({
            "id": id, "category": cat, "trigger": "某种情况",
            "from": "a", "to": "b", "enabled": true, "severity": sev,
            "desc": "改了之后会怎样", "reportField": "someField"
        })
    }

    fn good() -> serde_json::Value {
        serde_json::json!({
            "version": 1,
            "updated": "2026-08-19 17:20:00",
            "guide": "# 这是什么？\n第一段。\n\n# 怎么改？\n- 关掉某条：把该条 enabled 改为 false。\n",
            "fallbacks": [
                rule("support_fallback_to_tower", "override", "info"),
                rule("wipe_default", "default", "warn"),
                rule("legacy_key_migration", "migration", "info")
            ]
        })
    }

    fn load(v: serde_json::Value) -> (tempfile::TempDir, Result<FallbackRegistry, AppError>) {
        let d = tempfile::tempdir().unwrap();
        crate::fsx::atomic::atomic_write_json(&d.path().join(FALLBACK_REL), &v).unwrap();
        let r = FallbackRegistry::load_from(d.path());
        (d, r)
    }

    #[test]
    fn loads_rules_and_meta() {
        let (_d, r) = load(good());
        let f = r.unwrap();
        assert_eq!(f.rules().len(), 3);
        assert_eq!(f.version, 1);
        assert_eq!(f.updated, "2026-08-19 17:20:00");
        assert_eq!(
            f.rule("support_fallback_to_tower").unwrap().category,
            Category::Override
        );
        assert_eq!(f.rule("wipe_default").unwrap().severity, Severity::Warn);
    }

    /// `guide` 的换行要在。压成一行就把"怎么改"这件事弄丢了
    #[test]
    fn guide_keeps_its_line_breaks() {
        let (_d, r) = load(good());
        let f = r.unwrap();
        assert!(f.guide.contains('\n'), "换行被吃了");
        assert!(f.guide.starts_with("# 这是什么？"));
        assert!(f.guide.contains("# 怎么改？"), "第二段整段丢了");
    }

    /// 分组按声明顺序，且**不返回空分类**
    #[test]
    fn grouping_skips_empty_categories() {
        let (_d, r) = load(good());
        let f = r.unwrap();
        let groups = f.by_category();
        assert_eq!(
            groups.iter().map(|(c, _)| *c).collect::<Vec<_>>(),
            vec![Category::Default, Category::Migration, Category::Override],
            "infer 与 recovery 没有规则，不该出现"
        );
        assert_eq!(groups[0].1.len(), 1);
    }

    /// 全开启时 `disabled()` 为空 —— 这是正常状态，界面写「当前无」
    #[test]
    fn nothing_disabled_is_a_normal_state() {
        let (_d, r) = load(good());
        assert!(r.unwrap().disabled().is_empty());
    }

    /// 关掉一条能被认出来：它触发时会直接中止，不是静默改写
    #[test]
    fn disabled_rule_is_reported() {
        let mut v = good();
        v["fallbacks"][0]["enabled"] = serde_json::json!(false);
        let (_d, r) = load(v);
        let f = r.unwrap();
        assert_eq!(f.disabled().len(), 1);
        assert_eq!(f.disabled()[0].id, "support_fallback_to_tower");
    }

    /// **`enabled` 不给默认值**：上游漏写时应该报错，不该被当成 true。
    /// 实测 20/20 全是 true，但"当前没有一条关掉"和"这个字段可以省略"是两件事
    #[test]
    fn missing_enabled_is_an_error_not_a_silent_true() {
        let mut v = good();
        v["fallbacks"][0].as_object_mut().unwrap().remove("enabled");
        let (_d, r) = load(v);
        let e = r.unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::Corrupted);
    }

    #[test]
    fn duplicate_rule_id_is_rejected() {
        let mut v = good();
        v["fallbacks"]
            .as_array_mut()
            .unwrap()
            .push(rule("wipe_default", "infer", "info"));
        let (_d, r) = load(v);
        assert!(r
            .unwrap_err()
            .detail
            .unwrap_or_default()
            .contains("wipe_default"));
    }

    /// 没有说明的规则要拦 —— 界面上只剩一行「a 改成 b」，看的人不知道该不该管
    #[test]
    fn rule_without_desc_is_rejected() {
        let mut v = good();
        v["fallbacks"][1]["desc"] = serde_json::json!("   ");
        let (_d, r) = load(v);
        let e = r.unwrap_err();
        assert!(e.detail.unwrap_or_default().contains("wipe_default"));
    }

    /// 没见过的分类不能被静默丢掉 —— 那条规则会从界面上消失
    #[test]
    fn unknown_category_is_rejected() {
        let mut v = good();
        v["fallbacks"][0]["category"] = serde_json::json!("teleport");
        let (_d, r) = load(v);
        assert_eq!(r.unwrap_err().code, crate::error::ErrorCode::Corrupted);
    }

    #[test]
    fn missing_file_is_not_found() {
        let d = tempfile::tempdir().unwrap();
        let e = FallbackRegistry::load_from(d.path()).unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }

    /* ---------- 真上游对齐 ---------- */

    /// 同前几条的边界：**没上游时不执行**。验不变式与漏读字段，不钉条数
    #[test]
    fn real_upstream_loads_and_satisfies_the_invariants() {
        let Some(root) = paths::upstream_root() else {
            eprintln!("没定位到上游 mkpse-presets，这条对齐检查未执行（不是通过）");
            return;
        };
        let f = FallbackRegistry::load_from(&root).expect("真上游的回退登记表读不出来或断言不过");

        assert!(!f.rules().is_empty(), "读出来 0 条，判据已空转");
        assert!(!f.by_category().is_empty());
        // guide 是这一页的主要内容，空了整页就只剩一张没有上下文的表
        assert!(f.guide.contains('\n'), "guide 的换行丢了或者本来是空的");
        assert!(
            f.guide.contains("禁止手改"),
            "上游那句「禁止手改 content/*.json」不在了 —— 这一页只读的理由就是它"
        );
        // reportField 是运行报告的对账键，空了就对不上
        for r in f.rules() {
            assert!(
                !r.report_field.trim().is_empty(),
                "{} 没有 reportField",
                r.id
            );
        }

        let raw: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(root.join(FALLBACK_REL)).unwrap())
                .unwrap();
        let top: BTreeSet<&str> = ["version", "updated", "guide", "fallbacks"]
            .into_iter()
            .collect();
        let unknown: Vec<&String> = raw
            .as_object()
            .unwrap()
            .keys()
            .filter(|k| !top.contains(k.as_str()))
            .collect();
        assert!(
            unknown.is_empty(),
            "回退登记表顶层有我没读的键：{unknown:?}"
        );

        let known: BTreeSet<&str> = [
            "id",
            "category",
            "trigger",
            "from",
            "to",
            "enabled",
            "severity",
            "desc",
            "reportField",
        ]
        .into_iter()
        .collect();
        let mut unknown_field: BTreeSet<String> = BTreeSet::new();
        for r in raw["fallbacks"].as_array().unwrap() {
            for k in r.as_object().unwrap().keys() {
                if !known.contains(k.as_str()) {
                    unknown_field.insert(k.clone());
                }
            }
        }
        assert!(
            unknown_field.is_empty(),
            "回退规则有我没读的字段：{unknown_field:?}"
        );
    }
}
