//! 校验三档（doc §10.2）。
//!
//! ```text
//! 阻断   数据自相矛盾，生成一定出错。**全程唯一的硬闸门**
//! 待办   要人去填的空。不挡
//! 提示   合法但值得知道。不挡
//! ```
//!
//! # 每条必须说得出「去哪儿处理」
//!
//! 一条说不清去哪儿的问题**等于没报**：用户看到「有 3 个待办」，然后在六台机型十个版本
//! 七十四个字段里找。所以 [`Issue::at`] 带视角 + 机型 + 版本 + 字段，前端据此跳过去
//! 并把主选中落上。
//!
//! # 分档的判据是「挡不挡生成」，不是「严不严重」
//!
//! 按严重程度分档会让人纠结「这个算中还是算高」。按后果分就没得纠结：
//!
//! - **阻断** = 照这份数据生成，产出来的东西一定是坏的（值越界、枚举值不存在、条件成环）。
//! - **待办** = 有个空该人填（上游没声明最低客户端版本、A2L 没登记尺寸、
//!   我们写着一个再也进不了产物的键）。生成照做。
//! - **提示** = 合法，但换了个人看会想问一句（这个文件没进任何套餐、当前值是废弃选项）。
//!
//! 「有 N 个文件没进任何套餐」**刻意放提示档**：仓库里放一个 0.2mm 的 profile
//! 不分配给谁，是一种正常的交付身份，不是待修的事。
//!
//! # 零问题不留白
//!
//! 明确显示「都过了」。空白会被读成「还没校验」，而那两件事的后果完全不同。

use serde::Serialize;

use super::derive::Book;
use super::visibility::Gate;
use super::wording as w;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    /// 生成一定出错。**唯一会挡住生成的一档**
    Block,
    /// 要人去填的空。不挡
    Todo,
    /// 合法但值得知道。不挡
    Hint,
}

impl Severity {
    pub fn label(self) -> &'static str {
        match self {
            Self::Block => "阻断",
            Self::Todo => "待办",
            Self::Hint => "提示",
        }
    }

    pub fn explain(self) -> &'static str {
        match self {
            Self::Block => "照这份数据生成，产出来的东西一定是坏的",
            Self::Todo => "有个空该人填。不挡生成",
            Self::Hint => "合法，但值得看一眼。不挡生成",
        }
    }
}

/// 去哪儿处理。前端据此切视角 + 落主选中
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum View {
    Params,
    Menu,
    Build,
    Fields,
    Stock,
    Fallback,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Where {
    pub view: View,
    pub machine_id: Option<String>,
    pub uid: Option<String>,
    pub key: Option<String>,
}

impl Where {
    fn view(view: View) -> Self {
        Self {
            view,
            machine_id: None,
            uid: None,
            key: None,
        }
    }
    fn version(uid: &str, machine_id: &str) -> Self {
        Self {
            view: View::Params,
            machine_id: Some(machine_id.to_owned()),
            uid: Some(uid.to_owned()),
            key: None,
        }
    }
    fn field(uid: &str, machine_id: &str, key: &str) -> Self {
        Self {
            view: View::Params,
            machine_id: Some(machine_id.to_owned()),
            uid: Some(uid.to_owned()),
            key: Some(key.to_owned()),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    /// 稳定 id。同一条问题反复出现时前端能 key 住，列表不会每次重排
    pub id: String,
    pub severity: Severity,
    pub title: String,
    /// 「怎么办」。不是把 title 换个说法重复一遍
    pub detail: String,
    pub at: Where,
}

/// 一次校验的全部结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Report {
    pub issues: Vec<Issue>,
    pub blocks: usize,
    pub todos: usize,
    pub hints: usize,
    /// 零问题时的那一句。**不留白**
    pub empty_hint: &'static str,
}

impl Report {
    /// 有阻断吗 = 生成该不该全禁用
    pub fn blocked(&self) -> bool {
        self.blocks > 0
    }

    /// 第一条阻断的标题。生成按钮下面要写出「是哪一条」
    pub fn first_block(&self) -> Option<&Issue> {
        self.issues.iter().find(|i| i.severity == Severity::Block)
    }
}

/// 把整本过一遍。**纯函数**：同样的输入永远得到同样的输出
pub fn inspect(book: &Book<'_>) -> Report {
    let mut issues: Vec<Issue> = Vec::new();

    compat(book, &mut issues);
    machines(book, &mut issues);
    versions(book, &mut issues);
    upstream_data(book, &mut issues);
    delivery(book, &mut issues);

    // 阻断在前。列表顺序就是处理顺序 —— 待办排在阻断前面会让人先去填空，
    // 而那些空填完了照样生成不出来
    issues.sort_by(|a, b| a.severity.cmp(&b.severity).then(a.id.cmp(&b.id)));

    let count = |s: Severity| issues.iter().filter(|i| i.severity == s).count();
    Report {
        blocks: count(Severity::Block),
        todos: count(Severity::Todo),
        hints: count(Severity::Hint),
        issues,
        empty_hint: w::NO_ISSUES,
    }
}

/// 兼容声明（doc §12）
fn compat(book: &Book<'_>, out: &mut Vec<Issue>) {
    if book.up.manifest.compat.minimum_client.is_none() {
        out.push(Issue {
            id: "compat.minimum_client".to_owned(),
            severity: Severity::Todo,
            title: format!("最低客户端版本{}", w::UNDECLARED),
            // doc §12 那三条实测事实原样写进说明 —— 不写的话，看到这一条的人
            // 第一反应会是「去哪儿填」，而答案是「不在我们这儿」
            detail: "上游 manifest.json 的 minimumClient 是空串，version 也是空串，\
                     全局只有 manifestVersion: 2 与 fallback_registry.version: 1 两个版本号。\
                     也就是说上游现在**没有**声明「客户端要多新才能用这份数据」。\
                     这不是我们该填的空，而是发布时要知道的事：\
                     老客户端拿到新字段会静默忽略，而不是报错。"
                .to_owned(),
            at: Where::view(View::Build),
        });
    }
}

/// 机型级
fn machines(book: &Book<'_>, out: &mut Vec<Issue>) {
    for m in book.up.catalog.machines() {
        // A2L：上游没登记床身尺寸。**待办，不是阻断** —— 参数照样能改、产物照样能烤
        if m.dimensions.is_none() {
            out.push(Issue {
                id: format!("machine.dimensions.{}", m.id),
                severity: Severity::Todo,
                title: format!("{} 的床身尺寸{}", m.display, w::UNCONFIGURED),
                detail: "上游 machine_catalog.json 的 dimensions 字典里没有这台机型，\
                         manifest 里那份也是 null。于是越界检查与禁区都判不了。\
                         参数与产物不受影响 —— 这两件是分开的。要补请回 mkppanel。"
                    .to_owned(),
                at: Where {
                    view: View::Params,
                    machine_id: Some(m.id.clone()),
                    uid: None,
                    key: None,
                },
            });
        }
    }
}

/// 版本级：值越界、枚举不存在、条件成环、孤儿键
fn versions(book: &Book<'_>, out: &mut Vec<Issue>) {
    for v in book.versions() {
        if v.archived {
            continue; // 归档的不参与交付，也就不参与校验
        }
        let Some(layers) = book.version_layers(&v.uid) else {
            continue;
        };
        let gate = Gate::new(&book.up.registry, &layers);

        // 环：**阻断**。它不成立时可见性算不出稳定结果，产物里那些字段进不进都说不清
        for c in gate.cycles() {
            out.push(Issue {
                id: format!("cycle.{}.{}", v.uid, c.keys.join("+")),
                severity: Severity::Block,
                title: "字段的显示条件成环".to_owned(),
                detail: format!(
                    "{} 互相作为对方的显示条件。这几项该不该进产物算不出稳定结果。\
                     这是上游 param_registry 的数据问题，要回 mkppanel 改。",
                    c.keys.join(" → ")
                ),
                at: Where::field(&v.uid, &v.machine_id, &c.keys[0]),
            });
        }

        // 永远不可能满足的条件：**提示**。那个字段永久隐藏，但它有值、照样进产物 ——
        // 所以生成不会出错，只是界面上永远看不到它
        for u in gate.unsatisfiable() {
            out.push(Issue {
                id: format!("unsatisfiable.{}.{}", v.uid, u.key),
                severity: Severity::Hint,
                title: "有字段永久隐藏".to_owned(),
                detail: format!(
                    "{} 的显示条件要求 {} 等于 {}，而那个字段没有这个选项。\
                     它的值照样会进产物，只是界面上永远看不到、也改不了。",
                    u.key, u.depends_on, u.wants
                ),
                at: Where::field(&v.uid, &v.machine_id, &u.key),
            });
        }

        // 我们写着、却再也进不了产物的键：**待办**
        let orphans = layers.orphan_keys();
        if !orphans.is_empty() {
            out.push(Issue {
                id: format!("orphan.{}", v.uid),
                severity: Severity::Todo,
                title: format!("{} 有 {} 项改动已经失效", v.name, orphans.len()),
                detail: format!(
                    "{} —— 这些键还在文件里，但上游已经删掉它们、或者把这台机型从 \
                     machineFilter 里摘掉了，所以再也进不了产物。\
                     「我明明改过」这件事在它们身上是错的。确认之后删掉即可。",
                    orphans.join("、")
                ),
                at: Where::version(&v.uid, &v.machine_id),
            });
        }

        for key in layers.keys() {
            let Some(p) = book.up.registry.param(key) else {
                continue;
            };
            let Some(hit) = layers.effective(key) else {
                continue;
            };

            // 值越界：**阻断**。47 个字段有 min/max，越界的 TOML 客户端会拒
            if let Some(n) = hit.value.as_f64() {
                let low = p.min.filter(|m| n < *m);
                let high = p.max.filter(|m| n > *m);
                if low.is_some() || high.is_some() {
                    out.push(Issue {
                        id: format!("range.{}.{}", v.uid, key),
                        severity: Severity::Block,
                        title: format!("{} 超出范围", p.label),
                        detail: format!(
                            "当前是 {n}，允许的范围是 {}~{}。超范围的值写进 TOML，\
                             客户端会拒掉整份配方 —— 不是只忽略这一项。",
                            p.min.map(|x| x.to_string()).unwrap_or_else(|| "-∞".into()),
                            p.max.map(|x| x.to_string()).unwrap_or_else(|| "+∞".into()),
                        ),
                        at: Where::field(&v.uid, &v.machine_id, key),
                    });
                }
            }

            // 枚举值不在选项里：**阻断**
            if !p.choices.is_empty() && !p.choices.iter().any(|c| &c.value == hit.value) {
                out.push(Issue {
                    id: format!("choice.{}.{}", v.uid, key),
                    severity: Severity::Block,
                    title: format!("{} 的值不在选项里", p.label),
                    detail: format!(
                        "当前是 {}，而选项只有 {}。客户端按枚举解析，认不出的值会让整份配方失败。",
                        hit.value,
                        p.choices
                            .iter()
                            .map(|c| c.label.as_str())
                            .collect::<Vec<_>>()
                            .join(" / ")
                    ),
                    at: Where::field(&v.uid, &v.machine_id, key),
                });
            }

            // 当前值正好是废弃选项：**提示**。它还能用，只是不该继续用
            if let Some(c) = p
                .choices
                .iter()
                .find(|c| c.deprecated && &c.value == hit.value)
            {
                out.push(Issue {
                    id: format!("deprecated.{}.{}", v.uid, key),
                    severity: Severity::Hint,
                    title: format!("{} 用的是已废弃的选项", p.label),
                    detail: format!(
                        "当前选的是「{}」，上游把它标成了废弃。现在还能用，\
                         但上游下一次清理可能就把它删了 —— 那时这一版会变成阻断。",
                        c.label
                    ),
                    at: Where::field(&v.uid, &v.machine_id, key),
                });
            }
        }
    }
}

/// 上游数据本身的问题：**一律提示，不阻断** ——
/// 它们不是我们能修的，报成阻断会让工作台变成一个打不开的软件
fn upstream_data(book: &Book<'_>, out: &mut Vec<Issue>) {
    let known: Vec<&str> = book
        .up
        .catalog
        .machines()
        .iter()
        .map(|m| m.id.as_str())
        .collect();

    for p in book.up.registry.params() {
        let ghosts: Vec<&str> = p
            .machine_filter
            .iter()
            .map(String::as_str)
            .filter(|m| !known.contains(m))
            .collect();
        if ghosts.is_empty() {
            continue;
        }
        out.push(Issue {
            id: format!("filter_ghost.{}", p.key),
            severity: Severity::Hint,
            title: format!("{} 的机型过滤里有认不出的机型", p.label),
            detail: format!(
                "{} 不在机型清单里。这一项在那几台（不存在的）机型上会被当成不适用，\
                 而在真实机型上不受影响。多半是上游删机型时漏改了过滤。",
                ghosts.join("、")
            ),
            at: Where {
                view: View::Fields,
                machine_id: None,
                uid: None,
                key: Some(p.key.clone()),
            },
        });
    }
}

/// 交付这一面
fn delivery(book: &Book<'_>, out: &mut Vec<Issue>) {
    let orphan: Vec<String> = book
        .stock_rows()
        .into_iter()
        .filter(|r| !r.in_any_bundle)
        .map(|r| r.file_name)
        .collect();
    if !orphan.is_empty() {
        // **提示，不是待办**：不分配给谁是一种正常的交付身份（doc §10.2）
        out.push(Issue {
            id: "bundle.orphan_files".to_owned(),
            severity: Severity::Hint,
            title: format!("有 {} 个文件没进任何套餐", orphan.len()),
            detail: format!(
                "{} —— 客户看得到它们（除非标了仅归档），只是没有套餐推荐。\
                 仓库里放一个不分配给谁的 profile 是正常的交付身份，不是待修的事。",
                orphan.join("、")
            ),
            at: Where::view(View::Menu),
        });
    }

    // 版本自己挑了 BBS 但挑成了空：**待办** ——
    // 「跟机型默认」和「明确不要任何曲线」在界面上长得像，但后者需要有人确认过
    for v in book.versions() {
        if v.archived {
            continue;
        }
        if v.own_bbs.as_ref().is_some_and(Vec::is_empty) {
            out.push(Issue {
                id: format!("bbs.empty.{}", v.uid),
                severity: Severity::Todo,
                title: format!("{} 明确不带任何 BBS 曲线", v.name),
                detail: "这一版自己挑了一份 BBS 清单，而清单是空的 —— 与「跟机型默认」不是\
                         同一件事。确认是有意的话，这一条可以一直留着。"
                    .to_owned(),
                at: Where {
                    view: View::Menu,
                    machine_id: Some(v.machine_id.clone()),
                    uid: Some(v.uid.clone()),
                    key: None,
                },
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, Committed, CommittedVersion, Draft, Patch};
    use crate::workbench::domain::testkit::Fixture;
    use crate::workbench::domain::Overrides;
    use std::collections::BTreeMap;

    fn committed() -> Committed {
        let mut versions = BTreeMap::new();
        for (uid, machine, vid, name) in [
            ("A1/STANDARD", "A1", "STANDARD", "标准版"),
            ("A1/FAST", "A1", "FAST", "高速版"),
            ("A2L/STANDARD", "A2L", "STANDARD", "标准版"),
            ("P1S/LITE", "P1S", "LITE", "精简版"),
        ] {
            versions.insert(
                uid.to_owned(),
                CommittedVersion {
                    machine_id: machine.to_owned(),
                    version_id: vid.to_owned(),
                    name: name.to_owned(),
                    declared_upstream: true,
                    ..Default::default()
                },
            );
        }
        Committed {
            machines: ["A1", "A2L", "P1S"]
                .into_iter()
                .map(|m| (m.to_owned(), Overrides::new()))
                .collect(),
            versions,
            machine_ids: ["A1", "A2L", "P1S"].into_iter().map(str::to_owned).collect(),
            ..Default::default()
        }
    }

    /// 夹具是健康数据：**一条阻断都不该有**。
    /// 这条同时是反空转判据 —— 如果它报出阻断，说明规则太紧
    #[test]
    fn healthy_data_has_no_blocks() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let r = inspect(&Book::new(&f.up, &c, &d));

        assert_eq!(r.blocks, 0, "健康数据被判成阻断了：{:#?}", r.first_block());
        assert!(!r.blocked());
        assert_eq!(r.empty_hint, w::NO_ISSUES);
    }

    /// 三档各要有真实样本（tasks 9.8）
    #[test]
    fn all_three_tiers_have_real_samples() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();

        // 阻断：把偏移改到范围外（夹具里 offset.x 没有 min/max，所以用有范围的那个）
        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                // 枚举里没有这个值 → 阻断
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();

        let r = inspect(&Book::new(&f.up, &c, &d));
        assert!(r.blocked(), "枚举值不存在该是阻断");
        let b = r.first_block().unwrap();
        assert_eq!(b.severity, Severity::Block);
        assert_eq!(b.at.uid.as_deref(), Some("A1/STANDARD"));
        assert_eq!(b.at.key.as_deref(), Some("wiping.mode"));
        assert!(b.detail.contains("擦料塔"), "要列出真的选项：{}", b.detail);

        // 待办：上游没声明最低客户端版本（夹具照真上游做的，minimumClient 是空串）
        assert!(r
            .issues
            .iter()
            .any(|i| i.id == "compat.minimum_client" && i.severity == Severity::Todo));
        // 待办：A2L 没登记尺寸
        assert!(r
            .issues
            .iter()
            .any(|i| i.id == "machine.dimensions.A2L" && i.severity == Severity::Todo));

        // 提示：没进任何套餐的文件（夹具里只有一条 BBS 进了套餐，三个 MKP 都没进）
        let hint = r
            .issues
            .iter()
            .find(|i| i.id == "bundle.orphan_files")
            .expect("该有这一条");
        assert_eq!(hint.severity, Severity::Hint, "不分配给谁是正常的交付身份");
    }

    /// **阻断排在最前面**：待办排前面会让人先去填空，而填完照样生成不出来
    #[test]
    fn blocks_come_first() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();
        let r = inspect(&Book::new(&f.up, &c, &d));
        assert_eq!(r.issues[0].severity, Severity::Block);
        assert!(r.issues.windows(2).all(|w| w[0].severity <= w[1].severity));
    }

    /// 每条都要说得出「去哪儿」与「怎么办」
    #[test]
    fn every_issue_is_actionable() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let r = inspect(&Book::new(&f.up, &c, &d));

        assert!(!r.issues.is_empty(), "一条都没有，下面的判据在空转");
        for i in &r.issues {
            assert!(!i.title.trim().is_empty(), "{} 没有标题", i.id);
            assert!(!i.detail.trim().is_empty(), "{} 没说怎么办", i.id);
            assert_ne!(i.detail, i.title, "{} 的说明只是把标题重复一遍", i.id);
            // 落在字段上的必须带机型与版本，否则「去处理」跳不过去
            if i.at.key.is_some() && i.at.view == View::Params {
                assert!(i.at.machine_id.is_some(), "{} 的落点缺机型", i.id);
                assert!(i.at.uid.is_some(), "{} 的落点缺版本", i.id);
            }
        }
    }

    /// 归档的版本不参与校验 —— 它不交付，报它的问题是噪音
    #[test]
    fn archived_versions_are_skipped() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();
        assert!(inspect(&Book::new(&f.up, &c, &d)).blocked());

        apply(
            &mut d,
            &c,
            &f.up.registry,
            &[Patch::ArchiveVersion {
                uid: "A1/STANDARD".to_owned(),
            }],
        )
        .unwrap();
        assert!(
            !inspect(&Book::new(&f.up, &c, &d)).blocked(),
            "归档之后它不交付了，那条阻断也就不该挡着别人生成"
        );
    }

    /// 三档的词互不相同，且解释句不是把词重复一遍
    #[test]
    fn severity_words_are_distinct() {
        let all = [Severity::Block, Severity::Todo, Severity::Hint];
        let mut seen: Vec<&str> = Vec::new();
        for s in all {
            assert!(!s.label().is_empty());
            assert_ne!(s.explain(), s.label());
            assert!(!seen.contains(&s.label()));
            seen.push(s.label());
        }
    }
}
