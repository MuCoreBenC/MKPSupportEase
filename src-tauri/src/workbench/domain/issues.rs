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

use crate::workbench::presets::registry::ValueType;
use crate::workbench::presets::AssetKind;

use super::derive::Book;
use super::visibility::Gate;
use super::wording as w;
use super::wording::BuildState;

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
    /// 与前端 `Severity` 同一套记号（serde 已经把枚举值序列化成这一套）
    pub fn key(self) -> &'static str {
        match self {
            Self::Block => "block",
            Self::Todo => "todo",
            Self::Hint => "hint",
        }
    }

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

/// 这一条**能不能就地修**。由后端说，前端不按 id 猜。
///
/// 有了它，「能不能修」这条判据只有一处；前端不再写 `i.id === 'embedded.catalog_stale'`
/// 那样的特判 —— 那种写法等于把判据抄进视图层，后端一加检查项，按钮给不给就得改前端。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueAction {
    /// 跳到落点看一眼就够，没有可执行的修法
    Goto,
    /// 跑一次生成 / 定稿即可对齐
    Rerun,
    /// 前端能就地调一条命令修掉（按钮名与命令写在 `detail` 里点名）
    Fixable,
    /// 要人裁决 —— 两套数据打架，留哪边是人的决定
    NeedsDecision,
    /// 只是告知，不需要动作
    None,
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
    /// 能不能就地修（见 [`IssueAction`]）。前端据此决定给哪颗按钮
    pub action: IssueAction,
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

/// 把整本过一遍。**纯函数**：同样的输入永远得到同样的输出。
///
/// 这是生成闸门（`wb_generate` 前的那道检查）用的版本。
///
/// 2026-11-12：旧的「清单 ↔ `preset_recipes.toml` 配方」对齐检查已随 `gen-presets`
/// 整条链退役。生成从来不读那份配方，工作台产物由 `presets/*.toml` 单一源渲染。
pub fn inspect(book: &Book<'_>) -> Report {
    let mut issues: Vec<Issue> = Vec::new();
    collect(book, &mut issues);
    summarize(issues)
}

/// **预检全量**（b05 Task 11.8）：[`inspect`] 的全部 + 交付目录的「清单 ↔ 文件」自查
/// + 内嵌目录（编译进安装包那份）的新旧自查。
///
/// 交付自查与内嵌目录的结果**都由调用方给、都收 `Result`** —— 这是刻意的：
/// 它们是磁盘上的事实，domain 的检查保持纯函数；而且坏了（目录对不上）该成为
/// 预检报告里的**一条**，而不是让整个预检命令失败 —— 校验层停摆等于把
/// 「数据坏了」变成「工具坏了」，后者更糟。
pub fn preflight(
    book: &Book<'_>,
    delivery: Result<(), String>,
    embedded: Result<(), String>,
) -> Report {
    let mut issues: Vec<Issue> = Vec::new();
    collect(book, &mut issues);
    match delivery {
        Ok(()) => {}
        Err(detail) => issues.push(Issue {
            id: "delivery.catalog_mismatch".to_owned(),
            action: IssueAction::Rerun,
            // **待办，不是阻断**（作者 2026-10-03 要求把这条升硬 —— 但不能是阻断）：
            // 阻断会压住生成与发布两颗按钮，而「重算清单」恰恰是修复动作 ——
            // 等于把修复的路堵死。待办显眼、进计数、不挡闸。
            severity: Severity::Todo,
            title: "交付目录的清单和文件对不上".to_owned(),
            // 2026-10-19 文案瘦身：检查卡里不再铺整串文件路径；前两条给个印象，
            // 完整清单在①生成卡的「交付明细…」里（那里本来就是逐份列）。
            detail: summarize_mismatch(&detail),
            at: Where::view(View::Build),
        }),
    }
    match embedded {
        Ok(()) => {}
        Err(_) => issues.push(Issue {
            id: "embedded.catalog_stale".to_owned(),
            action: IssueAction::Fixable,
            // 待办而不是阻断：它挡不住工作台生成任何产物 —— 伤的是**客户端首屏**
            // （安装包里编译死的那份 catalog）。重新生成是修复动作，别把路堵死。
            severity: Severity::Todo,
            title: "内嵌目录落后于仓库数据".to_owned(),
            /* 2026-11-05 作者：「不许只剩终端那条路」—— 这条待办右边有「重新生成」按钮，
             * 点它就按当前 `presets/` 重算这份文件（`wb_regen_embedded_catalog`）。
             * 文案里点名那颗按钮，别让人再去翻命令。 */
            detail: "点这一条右边的「重新生成」，按当前 presets/ 重算这份文件。\
                     重新生成之后还要重新编译客户端，新目录才会随安装包走。"
                .to_owned(),
            at: Where::view(View::Build),
        }),
    }
    summarize(issues)
}

/// 不依赖配方的那几类。生成闸门与预检共用
fn collect(book: &Book<'_>, out: &mut Vec<Issue>) {
    machines(book, out);
    versions(book, out);
    filter_ghosts(book, out);
    delivery(book, out);
}

/// 把 `audit_catalog` 的整串消息压成检查卡能读的一句。
///
/// 原串形如 `3 处与目录登记不一致：a（…）；b（…）；c（…）`。
/// **说几处就要看得到几处**：这里全量摊开，太长由检查卡自己折叠 + 「展开」
/// （原来只留两条再说"等 N 处"，看到的与数出来的不是一件事）。
/// `audit_catalog` 本身不改（发布闸与既有测试仍读原文）。
fn summarize_mismatch(detail: &str) -> String {
    let (head, rest) = detail.split_once('：').unwrap_or((detail, ""));
    let items: Vec<&str> = rest
        .split('；')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if items.is_empty() {
        head.to_owned()
    } else {
        format!("{head}：{}", items.join("；"))
    }
}

fn summarize(mut issues: Vec<Issue>) -> Report {
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

/// 机型级
fn machines(book: &Book<'_>, out: &mut Vec<Issue>) {
    for m in book.machines() {
        // 机型文件里没有 `[dimensions]` —— A2L 现在就是这样。
        //
        // **这是「占位」，不是「缺数据」。** 这台机器还没开始做，先占了个名字。
        // 所以它是 `Hint`（合法但值得知道）而不是 `Todo`（要人去填的空）——
        // 标成待办等于每次打开出货检查都催一次，催的还是一件**刻意**没做的事，
        // 而那种提示看两次就会被整列忽略，连真的待办一起。
        //
        // 没有尺寸的机型不会被生成、也不会进清单（`app::build` 那边按这一条跳过），
        // 消费端因此拿不到它 —— 它自己的内置尺寸表里也没有这台，就算给它预设也会
        // 整份拒掉（b04 P0 审计：`mkp-preset` 的 `load_ir` 第 8 步）。
        // 两边都不交付，所以空着是**闭合的**，不是漏。
        if !m.has_dimensions {
            out.push(Issue {
                id: format!("machine.dimensions.{}", m.id),
                action: IssueAction::Goto,
                severity: Severity::Hint,
                title: format!("{} 是占位机型，不参与交付", m.display),
                detail: "机型文件里没有 [dimensions] —— 这台还没开始做，先占个名字。\
                         没有尺寸的机型不会被生成、也不会进清单，消费端拿不到它，\
                         所以空着是安全的，不是漏了什么。\
                         真要做这台机器的时候把尺寸填上，它就会自动进入交付。"
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
        let Some(layers) = book.version_layers(&v.uid) else {
            continue;
        };
        let gate = Gate::new(&book.presets.registry, &layers);

        // 环：**阻断**。它不成立时可见性算不出稳定结果，产物里那些字段进不进都说不清
        for c in gate.cycles() {
            out.push(Issue {
                id: format!("cycle.{}.{}", v.uid, c.keys.join("+")),
                action: IssueAction::NeedsDecision,
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
                action: IssueAction::NeedsDecision,
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
                action: IssueAction::Goto,
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
            let Some(p) = book.presets.registry.param(key) else {
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
                        action: IssueAction::Goto,
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

            // 枚举值不在选项里：**阻断**。
            //
            // **`choices` 是取值域只有 string 这一档** —— 与 `validate.rs` 的
            // `value_type == "string"` 同一道门。bool / float 参数身上挂着的
            // 是显示文案与「预设档」，不是逼人选的取值域：
            //
            //   · bool：`'off'` / `'on'` 是显示文案，取值域由 bool 类型保证；
            //     不跳过的话每台机器每个版本要误报六条阻断（wiping.* 五条 +
            //     first_pen_revitalization_flag，b05 Task 11.2 实测）；
            //   · float：`wiping.ironing_coverage_threshold` 挂 0/50/90 三条
            //     预设档，而出厂默认就是 10.0 —— 照枚举查会把它判成阻断，
            //     可它真身是一个能填的百分比。
            if p.value_type == ValueType::Text
                && !p.choices.is_empty()
                && !p.choices.iter().any(|c| &c.value == hit.value)
            {
                out.push(Issue {
                    id: format!("choice.{}.{}", v.uid, key),
                    action: IssueAction::Goto,
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

            // 当前值正好是**这台机型上**已弃用的选项：提示。它还能用，只是不该继续用。
            // 判据按机型 —— 别把「A1 上退场」说成「所有机型都退场」
            if let Some(c) = p
                .choices
                .iter()
                .find(|c| c.deprecated_for(&v.machine_id) && &c.value == hit.value)
            {
                out.push(Issue {
                    id: format!("deprecated.{}.{}", v.uid, key),
                    action: IssueAction::Goto,
                    severity: Severity::Hint,
                    title: format!("{} 用的是已废弃的选项", p.label),
                    detail: format!(
                        "当前选的是「{}」，上游把它标成了废弃（{}）。现在还能用，\
                         但上游下一次清理可能就把它删了 —— 那时这一版会变成阻断。",
                        c.label, v.machine_id
                    ),
                    at: Where::field(&v.uid, &v.machine_id, key),
                });
            }
        }
    }
}

/// `machineFilter` 里认不出的机型：**一律提示，不阻断**。
///
/// 过滤里写着一台机型清单里没有的机型，多半是删机型时漏改了过滤。
/// 这一项在那几台（不存在的）机型上会被当成不适用，而在真实机型上不受影响 ——
/// 所以它不挡任何东西，只是留一处固定可见的地方
fn filter_ghosts(book: &Book<'_>, out: &mut Vec<Issue>) {
    let known: Vec<&str> = book.machines().iter().map(|m| m.id.as_str()).collect();

    for p in book.presets.registry.params() {
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
            action: IssueAction::NeedsDecision,
            severity: Severity::Hint,
            title: format!("{} 的机型过滤里有认不出的机型", p.label),
            detail: format!(
                "{} 不在机型清单里。这一项在那几台（不存在的）机型上会被当成不适用，\
                 而在真实机型上不受影响。多半是删机型时漏改了过滤。",
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

/// 交付这一面：登记了、却没进任何套餐的**切片器预设**；有旧版待重新生成的提示。
///
/// **提示，不是待办**：不分配给谁是一种正常的交付身份（doc §10.2）；
/// 旧产物客户端还能下载到，也不是坏数据 —— 但作者点名（2026-10-02）：
/// 「有旧的、新的没生成」这件事该在检查卡里看见，不能只在生成页数出来。
fn delivery(book: &Book<'_>, out: &mut Vec<Issue>) {
    /* 「进套餐」与「这一代交付」是两个问题（作者 2026-11-08：停用不删文件）：
    · `bundled` = 全部套餐的引用（含停用机型）—— 文件有主，不算孤儿；
    · `live`    = 停用机型之外的套餐引用 —— 这一代真会被交付。
    夹在两者之间的那一批既不该报孤儿，也不该一声不吭 —— 下面单独说一句。 */
    let bundled = book.presets.bundled_asset_ids();
    let live: std::collections::BTreeSet<String> = book
        .presets
        .delivery_bundles()
        .iter()
        .flat_map(|b| b.asset_refs.iter().map(|r| r.trim().to_lowercase()))
        .collect();
    let orphan: Vec<&str> = book
        .presets
        .assets
        .items()
        .iter()
        .filter(|a| a.kind == AssetKind::SlicerProfile)
        .filter(|a| !bundled.contains(&a.id.to_lowercase()))
        .map(|a| a.path.as_str())
        .collect();
    if !orphan.is_empty() {
        out.push(Issue {
            id: "bundle.orphan_files".to_owned(),
            action: IssueAction::Goto,
            severity: Severity::Hint,
            title: format!("有 {} 个切片器预设没进任何套餐", orphan.len()),
            detail: format!(
                "{} —— 客户看得到它们（除非标了仅归档），只是没有套餐推荐。\
                 仓库里放一个不分配给谁的 profile 是正常的交付身份，不是待修的事。",
                orphan.join("、")
            ),
            /* 落点在**资产库 + 「可选」筛选**：套餐页里根本没有这些文件，
            跳过去等于没指路。这件事由后端说清，前端不再按 id 特判。 */
            at: Where {
                view: View::Stock,
                machine_id: None,
                uid: None,
                key: Some("optional".to_owned()),
            },
        });
    }

    /* 只被停用机型的套餐引用：文件留在仓库里，但这一代不进交付目录。
    这里必须出现一条 —— 否则"交付面不写它"与"检查面说它已进套餐"两边都不算账，
    作者看到的就是"文件在那儿，客户端却没有"。 */
    let parked: Vec<&str> = book
        .presets
        .assets
        .items()
        .iter()
        .filter(|a| a.kind == AssetKind::SlicerProfile)
        .filter(|a| {
            let id = a.id.to_lowercase();
            bundled.contains(&id) && !live.contains(&id)
        })
        .map(|a| a.path.as_str())
        .collect();
    if !parked.is_empty() {
        out.push(Issue {
            id: "bundle.parked_by_disabled_machine".to_owned(),
            action: IssueAction::None,
            severity: Severity::Hint,
            title: format!("有 {} 个切片器预设只被停用机型的套餐引用", parked.len()),
            detail: format!(
                "{} —— 文件照留在仓库里，这一代不进交付目录（客户端取不到）。\
                 重新启用那台机型，它们就跟着回到交付面。",
                parked.join("、")
            ),
            at: Where::view(View::Stock),
        });
    }

    // 旧版待重新生成的提示：磁盘上的产物是上次生成的，配方已经改过了。
    // 聚合成一条（stale 可能一连十几个版本，逐版一条会把检查卡淹掉），
    // 「去处理」落在生成页 —— 那里能一眼看全、勾一批重新生成。
    let stale: Vec<String> = book
        .versions()
        .iter()
        .filter(|v| book.build_state(&v.uid) == BuildState::Stale)
        .map(|v| v.uid.clone())
        .collect();
    if !stale.is_empty() {
        out.push(Issue {
            id: "build.stale_versions".to_owned(),
            action: IssueAction::Rerun,
            severity: Severity::Hint,
            title: format!("有 {} 个版本的产物是旧的", stale.len()),
            detail: format!(
                "{} —— 配方改过之后还没重新生成，客户端现在下载到的是上一版。\
                 去生成页勾上重新生成，客户端才会拿到新的。",
                stale.join("、")
            ),
            at: Where::view(View::Build),
        });
    }

    /* 「版本自己挑了 BBS 但挑成了空」这条校验撤了（b04 Task 12）：版本不再有自己的
    那份清单（REPORT §7.3），于是那个入口不存在了 ——
    「跟机型默认」和「明确不要任何曲线」也不再会长得像。 */
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::patch::{apply, Committed, CommittedVersion, Draft, Patch};
    use crate::workbench::domain::testkit::{fixture_catalog, Fixture};
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
                    ..Default::default()
                },
            );
        }
        Committed {
            versions,
            catalog: fixture_catalog(),
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
        let r = inspect(&Book::new(&f.presets, &c, &d));

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
            &f.presets.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                // 枚举里没有这个值 → 阻断
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();

        let r = inspect(&Book::new(&f.presets, &c, &d));
        assert!(r.blocked(), "枚举值不存在该是阻断");
        let b = r.first_block().unwrap();
        assert_eq!(b.severity, Severity::Block);
        assert_eq!(b.at.uid.as_deref(), Some("A1/STANDARD"));
        assert_eq!(b.at.key.as_deref(), Some("wiping.mode"));
        assert!(b.detail.contains("擦料塔"), "要列出真的选项：{}", b.detail);

        // **提示（不是待办）：A2L 是占位机型。**
        //
        // 档位本身就是判据：`Todo` 会让出货检查每次都催一遍一件刻意没做的事，
        // 而那种催促看两次就会被整列忽略 —— 连真的待办一起
        let placeholder = r
            .issues
            .iter()
            .find(|i| i.id == "machine.dimensions.A2L")
            .expect("A2L 没有尺寸，这一条该在");
        assert_eq!(
            placeholder.severity,
            Severity::Hint,
            "占位机型是「合法但值得知道」，不是「要人去填的空」"
        );
        assert!(
            placeholder.title.contains("占位"),
            "标题要说清这是占位：{}",
            placeholder.title
        );
        assert!(
            !placeholder.detail.contains("补上尺寸") && !placeholder.detail.contains("先把尺寸"),
            "不该写成催办：{}",
            placeholder.detail
        );

        // 提示：没进任何套餐的切片器预设（夹具里只有一条 BBS 进了套餐）
        let hint = r
            .issues
            .iter()
            .find(|i| i.id == "bundle.orphan_files")
            .expect("该有这一条");
        assert_eq!(hint.severity, Severity::Hint, "不分配给谁是正常的交付身份");

        // 启用状态下一份都不该落进"只被停用机型引用"那一档
        assert!(
            r.issues
                .iter()
                .all(|i| i.id != "bundle.parked_by_disabled_machine"),
            "没有停用机型时不该冒出这一条"
        );
    }

    /// **停用机型的套餐引用着它 ⇒ 文件保留、这一代不交付，但原因要看得见**。
    ///
    /// 作者 2026-11-08 口径：停用不删文件。于是"交付面不写它"与"检查面说它已进套餐"
    /// 会同时对同一份文件成立 —— 那两句都不算错，缺的是把这件事说出来。
    #[test]
    fn assets_kept_alive_only_by_a_disabled_machine_are_named() {
        let mut f = Fixture::load();
        f.presets
            .catalog
            .machine_mut("A1")
            .expect("夹具里有 A1")
            .enabled = false;
        let c = committed();
        let d = Draft::default();
        let r = inspect(&Book::new(&f.presets, &c, &d));

        let parked = r
            .issues
            .iter()
            .find(|i| i.id == "bundle.parked_by_disabled_machine")
            .expect("停用机型的套餐还引用着切片器预设 —— 这一条该在");
        assert_eq!(
            parked.severity,
            Severity::Hint,
            "文件留着是刻意行为，不是待修"
        );
        assert!(
            parked.detail.contains("不进交付目录") && parked.detail.contains("重新启用"),
            "要同时说清「现在取不到」与「怎么让它回来」：{}",
            parked.detail
        );
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
            &f.presets.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();
        let r = inspect(&Book::new(&f.presets, &c, &d));
        assert_eq!(r.issues[0].severity, Severity::Block);
        assert!(r.issues.windows(2).all(|w| w[0].severity <= w[1].severity));
    }

    /// 交付目录的清单与文件对不上 → 预检里是一条**待办**。
    ///
    /// 作者 2026-10-03 要求把这条升硬（「不应该显示提示，而应该是阻断」）——
    /// 但不能真放阻断：阻断会压住生成与发布两颗按钮，而「重算清单」恰恰是
    /// 修复动作，等于把修复的路堵死。待办显眼、进计数、不挡闸。
    #[test]
    fn a_mismatched_delivery_catalog_is_a_todo() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let book = Book::new(&f.presets, &c, &d);

        let r = preflight(
            &book,
            Err("1 份与目录登记不一致：mkp/presets/A1-standard.toml（目录记 4299 字节，实际 4335 字节）".to_owned()),
            Ok(()),
        );
        let todo = r
            .issues
            .iter()
            .find(|i| i.id == "delivery.catalog_mismatch")
            .expect("清单对不上，这条该在");
        assert_eq!(
            todo.severity,
            Severity::Todo,
            "不挡生成也不挡发布 —— 修复动作正是重算清单"
        );
        assert!(
            todo.detail.contains("4299"),
            "要带着自查给的明细：{}",
            todo.detail
        );
        // 自查过了就没有这条
        let r = preflight(&book, Ok(()), Ok(()));
        assert!(r.issues.iter().all(|i| i.id != "delivery.catalog_mismatch"));
    }

    /// 内嵌目录（安装包里编译死的那份 catalog）落后于仓库数据 = 一条待办。
    /// 它挡不住工作台的任何产物，伤的是客户端首屏 —— 重新生成（gen-catalog）
    /// 是修复动作，所以跟交付清单那条同一口径：待办，不是阻断。
    #[test]
    fn a_stale_embedded_catalog_is_a_todo() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let book = Book::new(&f.presets, &c, &d);

        let r = preflight(
            &book,
            Ok(()),
            Err("安装包里编译的那份，与按当前 presets/ 重新构建出来的不一致".to_owned()),
        );
        let todo = r
            .issues
            .iter()
            .find(|i| i.id == "embedded.catalog_stale")
            .expect("内嵌目录过期，这条该在");
        assert_eq!(
            todo.severity,
            Severity::Todo,
            "不挡生成 —— 修复动作是重新生成，别把路堵死"
        );
        assert!(
            todo.detail.contains("重新生成"),
            "要把修复动作（那条待办右边的按钮）写清楚：{}",
            todo.detail
        );

        // 一致就没有这条
        let r = preflight(&book, Ok(()), Ok(()));
        assert!(r.issues.iter().all(|i| i.id != "embedded.catalog_stale"));
    }

    /// **没有渲染器时不许报"产物是旧的"**（2026-11-01 作者截图踩到）。
    ///
    /// `build_state` 判"旧不旧"要拿当前渲染跟盘上字节比；`Book` 没装探针时它只能
    /// 保守说 `Stale`，于是检查卡永远说「有 9 个版本的产物是旧的」，而右边生成页
    /// 同时显示「已生成」—— 同一份数据两处说法相反。
    ///
    /// 现在 `wb_preflight` / `wb_assets` / `wb_bundles` / `audit_with` 都装了探针，
    /// 这条判据盯着"没装探针的 Book 不该被用来出这个结论"这件事本身。
    #[test]
    fn a_book_without_a_renderer_cannot_call_products_stale() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        /* 没有渲染器：盘上没产物时是 `NeverBuilt`（正确），
        盘上**有**产物时只能保守说 `Stale` —— 后者正是要避免拿去出结论的那一档。
        这里用"有产物但没探针"来钉住它，因为那才是作者截图里那一幕。 */
        let product = f
            .presets
            .root()
            .join(crate::workbench::paths::DELIVERY_SUBDIR)
            .join("mkp/presets/A1-standard.toml");
        std::fs::create_dir_all(product.parent().unwrap()).unwrap();
        crate::fsx::atomic::atomic_write(&product, b"# machine: A1\n").unwrap();

        let blind = Book::new(&f.presets, &c, &d);
        assert_eq!(
            blind.build_state("A1/STANDARD"),
            BuildState::Stale,
            "有产物但没探针 → 只能保守说 Stale（这条钉住：为什么必须给 Book 装探针）"
        );

        // 装了探针、探针给出的字节与盘上一致 → 已生成，不是"旧"
        let probe = |_uid: &str| Some("# machine: A1\n".to_owned());
        let seeing = Book::new(&f.presets, &c, &d).with_renderer(&probe);
        assert_eq!(
            seeing.build_state("A1/STANDARD"),
            BuildState::Built,
            "字节对得上就不该再报旧"
        );
    }

    /// 有旧版待重新生成时，检查卡要给一条提示（作者 2026-10-02：
    /// 「有旧的、然后新的未生成」这件事只在生成页数得出来，左侧检查里没有）。
    ///
    /// **提示档不是待办**：旧产物客户端还能下载到，不是坏数据也不是要填的空。
    #[test]
    fn stale_versions_show_up_as_a_hint() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();

        /* 判据是**盘上字节**（2026-11-01 改判，台账不再参与）：
        盘上摆一份与"改前渲染结果"相同的产物 = 产物跟得上源。 */
        let before = |_: &str| Some("machine = A1/STANDARD\n".to_owned());
        {
            let b = Book::new(&f.presets, &c, &d);
            let mkp = b.version("A1/STANDARD").unwrap().mkp_file.clone();
            let target = f
                .presets
                .root()
                .join(crate::workbench::paths::DELIVERY_SUBDIR)
                .join(crate::workbench::paths::MKP_PRESETS_DIR)
                .join(&mkp);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            crate::fsx::atomic::atomic_write(&target, "machine = A1/STANDARD\n".as_bytes())
                .unwrap();
        }

        // 产物跟得上配方 → 没有这条提示
        let r = inspect(&Book::new(&f.presets, &c, &d).with_renderer(&before));
        assert!(r.issues.iter().all(|i| i.id != "build.stale_versions"));

        // 改一个值 → 旧了 → 提示出现：提示档、点得出数量、列得出是哪几版
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.child".to_owned(),
                value: Some(serde_json::json!(33)),
            }],
        )
        .unwrap();
        // 源改了 → 渲染结果变了 → 与盘上那份不同
        let after = |_: &str| Some("machine = A1/STANDARD\nchild = 33\n".to_owned());
        let r = inspect(&Book::new(&f.presets, &c, &d).with_renderer(&after));
        let hint = r
            .issues
            .iter()
            .find(|i| i.id == "build.stale_versions")
            .expect("改了配方没重新生成，这条该在");
        assert_eq!(
            hint.severity,
            Severity::Hint,
            "旧版还能用，是「值得知道」不是「要人去填的空」"
        );
        assert!(hint.title.contains('1'), "要点出数量：{}", hint.title);
        assert!(
            hint.detail.contains("A1/STANDARD"),
            "要列得出是哪几版：{}",
            hint.detail
        );
        assert_eq!(hint.at.view, View::Build, "去处理要落在生成页");
    }

    /// 每条都要说得出「去哪儿」与「怎么办」
    #[test]
    fn every_issue_is_actionable() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let r = inspect(&Book::new(&f.presets, &c, &d));

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
        /* 「能不能就地修」由后端说：整份清单全落成 None 就说明回填漏了 */
        assert!(
            r.issues.iter().any(|i| i.action != IssueAction::None),
            "每一条都是「只是告知」⇒ action 回填漏了"
        );
        /* 纯告知那一档本来就该没有 —— 现在清单里每一条都能去哪儿做点什么 */
        assert!(
            r.issues.iter().all(|i| i.action != IssueAction::None),
            "夹具里的问题都是能处理的，不该有 None：{:?}",
            r.issues
                .iter()
                .filter(|i| i.action == IssueAction::None)
                .map(|i| i.id.clone())
                .collect::<Vec<_>>()
        );
    }

    /// 字段值不合法 → **阻断**，并且说得出来卡在哪一项上
    ///
    /// 以前这一条还要验「归档的版本不参与校验」（归档之后那条阻断会让路）。
    /// 归档这个概念删掉了（REPORT §7.2），于是「不参与校验」没有入口了 ——
    /// 清单上每一版现在都参与交付
    #[test]
    fn an_illegal_value_blocks_and_names_the_field() {
        let f = Fixture::load();
        let c = committed();
        let mut d = Draft::default();
        apply(
            &mut d,
            &c,
            &f.presets.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();

        let r = inspect(&Book::new(&f.presets, &c, &d));
        assert!(r.blocked(), "值不合法必须挡住生成");
        let hit = r
            .issues
            .iter()
            .find(|i| i.at.key.as_deref() == Some("wiping.mode"))
            .expect("没报在具体那一项上");
        assert_eq!(hit.at.uid.as_deref(), Some("A1/STANDARD"));
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
