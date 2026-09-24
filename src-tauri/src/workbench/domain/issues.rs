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

use preset::recipe::Recipe;

use crate::workbench::presets::registry::ValueType;

use super::derive::Book;
use super::patch::CatalogMachine;
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

/// 把整本过一遍。**纯函数**：同样的输入永远得到同样的输出。
///
/// 这是生成闸门（`wb_generate` 前的那道检查）用的版本：它不含「清单 ↔ 配方」对齐
/// —— 因为生成**不读** `preset_recipes.toml`，配方对不上不影响工作台的任何产物。
/// 预检（`wb_preflight`）要用 [`preflight`]，那里多查配方这一面。
pub fn inspect(book: &Book<'_>) -> Report {
    let mut issues: Vec<Issue> = Vec::new();
    collect(book, &mut issues);
    summarize(issues)
}

/// **预检全量**（b05 Task 11.8）：[`inspect`] 的全部 + 清单 ↔ 配方对齐
/// （b05 Task 11.4 / 11.5 / 11.6）。
///
/// 配方文本由调用方给（真数据是 `preset::PRESET_RECIPES_TOML`）。**收 `Result`**
/// 是刻意的：配方本身坏掉（读不回来 / 校验不过）是 gen-presets 那条链的问题，
/// 它该成为预检报告里的**一条**，而不是让整个预检命令失败 —— 校验层停摆
/// 等于把「数据坏了」变成「工具坏了」，后者更糟。
pub fn preflight(book: &Book<'_>, recipe: Result<&Recipe, &str>) -> Report {
    let mut issues: Vec<Issue> = Vec::new();
    // **空 registry = 阻断**（b05 Task 15 裁定③）：空集通过会把"尚未建立参数体系"
    // 误报成"预检成功"。数据还没初始化完，就照实说 —— 其余检查照跑（空的
    // 机型/套餐清单本来就没有可查的，不会重复刷屏）
    if book.presets.registry.params().is_empty() {
        issues.push(Issue {
            id: "registry.empty".to_owned(),
            severity: Severity::Block,
            title: "缺少可用参数注册数据".to_owned(),
            detail: "param_registry.toml 里没有任何参数定义 —— 还没建立参数体系。\
                     先初始化工作台数据并定义参数（或接入上游），在此之前参数编辑\
                     与生成没有可用的字段。"
                .to_owned(),
            at: Where::view(View::Fields),
        });
    }
    collect(book, &mut issues);
    match recipe {
        Ok(r) => recipe_alignment(book, r, &mut issues),
        Err(e) => issues.push(Issue {
            id: "recipe.unreadable".to_owned(),
            severity: Severity::Hint,
            title: "配方 preset_recipes.toml 读不回来".to_owned(),
            detail: format!(
                "{e}\n内置预设那条链（gen-presets）眼下用不了它，但工作台的其余检查\
                 与生成照常 —— 这一轮先把别的看完。"
            ),
            at: Where::view(View::Build),
        }),
    }
    summarize(issues)
}

/// 不依赖配方的那几类。生成闸门与预检共用
fn collect(book: &Book<'_>, out: &mut Vec<Issue>) {
    compat(book, out);
    machines(book, out);
    versions(book, out);
    upstream_data(book, out);
    upstream_drift(book, out);
    delivery(book, out);
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

/// 清单 ↔ 配方（`preset_recipes.toml`，G-1 方案甲）的三个方向
/// （b05 Task 11.4 / 11.5 / 11.6）。
///
/// 为什么**全档待办**（warning）而不是阻断：工作台的 `render()` 读参数注册表，
/// 不读配方 —— 这些不一致不影响工作台生成任何一份产物。它们伤的是 gen-presets
/// 那条链（内置预设），而那条链构建时自己会报错（`gen-presets --check`）——
/// 预检在这里报出来，正是 11.6 说的「参数源缺失报 warning，构建时才升为 error」。
/// 三个方向：
///
/// 1. **机型存在、配方缺它**（11.5）：那台机器的内置预设链还没跟上；
/// 2. **版本存在、配方缺变体**（11.6：「允许版本先存在，参数源后补」）；
/// 3. **配方里有、清单不认**（11.5 反方向 + 11.4 的孤儿）：机型级（改了名/删了之后
///    配方没跟上）与变体级（`机型:变体` 没有任何版本定义指向 —— 产物文件名由
///    机型 id + 版本 id 小写算出，清单里没有那一版就没有那个名字）。
///
/// **占位机型（没有 `[dimensions]`）整台跳过**：不参与交付，两条链都没有它是闭合的
/// —— 与 [`machines`] 里 A2L 那条「占位机型」提示同一口径。
fn recipe_alignment(book: &Book<'_>, recipe: &Recipe, out: &mut Vec<Issue>) {
    let delivered: Vec<&CatalogMachine> = book
        .machines()
        .iter()
        .filter(|m| m.has_dimensions)
        .collect();
    let combos: std::collections::BTreeSet<(String, String)> =
        recipe.combos().into_iter().collect();

    // 方向一 + 二：清单这边（参与交付的）每台机型、每个版本，配方都要认
    for m in &delivered {
        if !recipe.machines.iter().any(|r| r.name == m.id) {
            out.push(Issue {
                id: format!("recipe.missing_machine.{}", m.id),
                severity: Severity::Todo,
                title: format!("机型 {} 在配方 preset_recipes.toml 里没有条目", m.id),
                detail: "内置预设那条链（gen-presets）生成不出这台的任何一份预设。\
                         机型定义可以先建、配方正文后补，但补上之前它进不了内置预设，\
                         那条链构建时会报错（11.6：预检 warning，构建时 error）。"
                    .to_owned(),
                at: Where::view(View::Build),
            });
        }
        for vid in &m.version_ids {
            if !combos.contains(&(m.id.clone(), vid.to_lowercase())) {
                out.push(Issue {
                    id: format!("recipe.missing_variant.{}.{}", m.id, vid),
                    severity: Severity::Todo,
                    title: format!("版本 {}:{} 在配方里没有对应的变体", m.id, vid),
                    detail: "版本定义可以先存在，参数源后补（doc §9）—— 但补上之前 \
                             内置预设那条链生成不出这一版的产物。产物文件名是 \
                             机型 id + 版本 id 小写，配方变体按这个名字对上。"
                        .to_owned(),
                    at: Where::view(View::Build),
                });
            }
        }
    }

    // 方向三：配方那边每台机型、每个变体，清单都要有对应的版本定义
    for r in &recipe.machines {
        let Some(m) = delivered.iter().find(|m| m.id == r.name) else {
            out.push(Issue {
                id: format!("recipe.unknown_machine.{}", r.name),
                severity: Severity::Todo,
                title: format!("配方 preset_recipes.toml 里的 {} 不在机型清单里", r.name),
                detail: "这份正文连它的全部变体，再也进不了任何清单认的产物 —— \
                         机型改了名或删了之后配方没跟上，就会留下这种孤儿（11.4）。\
                         确认之后从配方里删掉这台，或者把机型清单补回来。"
                    .to_owned(),
                at: Where::view(View::Build),
            });
            continue;
        };
        for v in &r.variants {
            if !m.version_ids.iter().any(|x| x.to_lowercase() == *v) {
                out.push(Issue {
                    id: format!("recipe.orphan_variant.{}.{}", r.name, v),
                    severity: Severity::Todo,
                    title: format!("配方里的变体 {}:{} 没有任何版本定义指向", r.name, v),
                    detail: "它永远进不了产物：清单里没有那台机型的那一版。\
                             gen-presets 会为它生成一份谁也不引用的内置预设 —— \
                             生成不报错，所以只能在这里看见。确认之后从配方里删掉，\
                             或把版本定义补回来。"
                        .to_owned(),
                    at: Where::view(View::Build),
                });
            }
        }
    }
}

/// 清单里有、上游不认的机型与版本（b05 Task 11.9）。**提示档**。
///
/// 背景：`render()` 改用我们自己的清单之后（b05 Task 3.3c 的残余风险收尾），
/// 「上游清单里没有这台」不再让渲染失败 —— 也就是说这类不一致从此不再以
/// 「机型不存在」的形式暴露，只能靠主动对表才能看见。它不挡任何东西：
/// 上游今天是搬数据的来源，不是运行时的依赖。
fn upstream_drift(book: &Book<'_>, out: &mut Vec<Issue>) {
    // 上游未配置 → 无从谈起「漂移」：这条检查是相对上游的对表
    let Some(up_stream) = book.up else {
        return;
    };
    for m in book.machines() {
        let Some(up) = up_stream.catalog.machine(&m.id) else {
            out.push(Issue {
                id: format!("upstream.unknown_machine.{}", m.id),
                severity: Severity::Hint,
                title: format!("上游不认机型 {}", m.id),
                detail: "我们清单里有、上游清单里没有。渲染用我们自己的清单，\
                         所以这不影响任何产物；但发布侧的数据仍以上游为准时，\
                         这台在上游那边是不存在的。上游退役（Task 8 之后只读）\
                         之前，每次加机型都会先见到这一条。"
                    .to_owned(),
                at: Where::view(View::Build),
            });
            continue;
        };
        for v in &m.version_ids {
            if !up.versions.iter().any(|x| x.id == *v) {
                out.push(Issue {
                    id: format!("upstream.unknown_version.{}.{}", m.id, v),
                    severity: Severity::Hint,
                    title: format!("上游不认版本 {}:{}", m.id, v),
                    detail: "同上：清单是我们说了算，上游只是不再对齐的参照。\
                             保留这条是为了在「发布侧还看着上游」的过渡期里，\
                             两边清单的出入有一处固定可见的地方。"
                        .to_owned(),
                    at: Where::view(View::Build),
                });
            }
        }
    }
}

/// 兼容声明（doc §12）
fn compat(book: &Book<'_>, out: &mut Vec<Issue>) {
    // 上游未配置 → 没有声明可查，这条检查跳过（「不可用原因」由 boot 说）
    let Some(up) = book.up else {
        return;
    };
    if up.manifest.compat.minimum_client.is_none() {
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
            // **bool（switch）字段跳过**：真数据里它们挂着的 `choices`
            // （`'off'` / `'on'`）是显示文案，不是取值域 —— 取值域由 bool 类型保证，
            // 值永远进不了那两个字符串。不跳过的话，每台机器每个版本要误报六条阻断
            // （wiping.* 五条 + first_pen_revitalization_flag，b05 Task 11.2 实测）
            if !p.choices.is_empty()
                && p.value_type != ValueType::Bool
                && !p.choices.iter().any(|c| &c.value == hit.value)
            {
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
    // 上游未配置 → 没有参照清单，ghost 过滤无从判定，整条跳过
    let Some(up) = book.up else {
        return;
    };
    let known: Vec<&str> = up
        .catalog
        .machines()
        .iter()
        .map(|m| m.id.as_str())
        .collect();

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

    /* ---------- 清单 ↔ 配方（b05 Task 11.4 / 11.5 / 11.6） ---------- */

    /// **三个方向各抓一条**（外加占位机型跳过的反例）。
    ///
    /// 配方用内联 TOML 而不是真数据：`Recipe::parse` 只保证**配方内部**自洽
    /// （机型重名、覆盖键不存在这些它自己会拦），「配方 vs 清单」的不一致
    /// 只能靠 recipe_alignment —— 所以这个测试能喂真配方永远写不出来的形状
    #[test]
    fn recipe_alignment_catches_every_direction() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let book = Book::new(Some(&f.up), &f.presets, &c, &d);

        // 夹具清单：A1(STANDARD, FAST)、A2L(占位，无尺寸)、P1S(LITE)。
        // body 用 TOML 双引号 + `\n` 转义（字面量串不支持跨行，见 recipe.rs minimal()）
        let text = r##"
release_time = '2026-01-01 00:00:00'

[[machines]]
name = 'A1'
variants = ['standard']
body = "# uuid: x\n# machine: A1\n[toolhead]\nspeed_limit = 70\n"

[[machines]]
name = 'P1S'
variants = ['ghost_var', 'lite']
body = "# uuid: x\n# machine: P1S\n[toolhead]\nspeed_limit = 70\n"

[[machines]]
name = 'GHOST_MACHINE'
variants = ['x']
body = "# uuid: x\n# machine: GHOST\n[toolhead]\nspeed_limit = 70\n"

[overrides.'A1:standard']
uuid = '11111111-1111-1111-1111-111111111111'

[overrides.'P1S:ghost_var']
uuid = '22222222-2222-2222-2222-222222222222'

[overrides.'P1S:lite']
uuid = '33333333-3333-3333-3333-333333333333'

[overrides.'GHOST_MACHINE:x']
uuid = '44444444-4444-4444-4444-444444444444'
"##;
        let recipe = Recipe::parse(text).expect("内联配方应该 parse 得过");
        let mut out = Vec::new();
        recipe_alignment(&book, &recipe, &mut out);

        let ids: Vec<&str> = out.iter().map(|i| i.id.as_str()).collect();
        // 方向三（配方有、清单不认的变体，11.4 孤儿）：P1S 认得，ghost_var 没人指向
        assert!(
            ids.contains(&"recipe.orphan_variant.P1S.ghost_var"),
            "配方有、清单不认的变体（11.4 孤儿）：{ids:?}"
        );
        // 方向二（版本在、配方缺变体，11.6 的「允许先存在」）：A1 的 FAST
        assert!(
            ids.contains(&"recipe.missing_variant.A1.FAST"),
            "清单版本在配方里没有对应变体：{ids:?}"
        );
        // 方向三（配方有、清单不认，11.5 反方向）：GHOST_MACHINE
        assert!(
            ids.contains(&"recipe.unknown_machine.GHOST_MACHINE"),
            "配方里的机型清单不认：{ids:?}"
        );
        // 占位机型必须整台跳过：A2L 不在配方里，也不该因为「配方缺它」被报
        assert!(
            !ids.iter().any(|i| i.contains("A2L")),
            "占位机型不参与交付，两条链都没有它是闭合的：{ids:?}"
        );
        // 「机型存在、配方缺机型条目」这一方向换个形状验：把 P1S 从配方里拿掉
        let text2 = r##"
release_time = '2026-01-01 00:00:00'

[[machines]]
name = 'A1'
variants = ['standard', 'fast']
body = "# uuid: x\n# machine: A1\n[toolhead]\nspeed_limit = 70\n"

[overrides.'A1:standard']
uuid = '11111111-1111-1111-1111-111111111111'

[overrides.'A1:fast']
uuid = '22222222-2222-2222-2222-222222222222'
"##;
        let recipe2 = Recipe::parse(text2).expect("内联配方应该 parse 得过");
        let mut out2 = Vec::new();
        recipe_alignment(&book, &recipe2, &mut out2);
        assert!(
            out2.iter().any(|i| i.id == "recipe.missing_machine.P1S"),
            "清单机型在配方里没有条目：{:?}",
            out2.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()
        );
        // A1 全对齐，不该再报
        assert!(
            !out2.iter().any(|i| i.id.contains("A1")),
            "对齐的部分不该报：{:?}",
            out2.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()
        );
    }

    /// 全对齐的配方**一条都不该报** —— 少了这条，上面那条分不清「查过了没报」
    /// 和「根本没查」
    #[test]
    fn recipe_alignment_stays_silent_when_everything_lines_up() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let book = Book::new(Some(&f.up), &f.presets, &c, &d);

        let text = r##"
release_time = '2026-01-01 00:00:00'

[[machines]]
name = 'A1'
variants = ['standard', 'fast']
body = "# uuid: x\n# machine: A1\n[toolhead]\nspeed_limit = 70\n"

[[machines]]
name = 'P1S'
variants = ['lite']
body = "# uuid: x\n# machine: P1S\n[toolhead]\nspeed_limit = 70\n"

[overrides.'A1:standard']
uuid = '11111111-1111-1111-1111-111111111111'

[overrides.'A1:fast']
uuid = '22222222-2222-2222-2222-222222222222'

[overrides.'P1S:lite']
uuid = '33333333-3333-3333-3333-333333333333'
"##;
        let recipe = Recipe::parse(text).expect("内联配方应该 parse 得过");
        let mut out = Vec::new();
        recipe_alignment(&book, &recipe, &mut out);
        assert!(
            out.is_empty(),
            "全对齐却报了：{:?}",
            out.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()
        );
    }

    /// `preflight` 的 Err 分支：配方读不回来是**报告里的一条**（Hint），
    /// 其余检查照跑 —— 校验层停摆比数据坏了更糟
    #[test]
    fn an_unreadable_recipe_becomes_an_issue_not_a_failure() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let book = Book::new(Some(&f.up), &f.presets, &c, &d);

        let r = preflight(&book, Err("配方读不回来（测试）"));
        assert!(
            r.issues
                .iter()
                .any(|i| i.id == "recipe.unreadable" && i.severity == Severity::Hint),
            "实测：{:?}",
            r.issues.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()
        );
        // 其余检查照跑：夹具上游没声明最低客户端版本，那条待办该在
        assert!(
            r.issues.iter().any(|i| i.id == "compat.minimum_client"),
            "配方坏了不该把别的检查一起带停"
        );
    }

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
        let r = inspect(&Book::new(Some(&f.up), &f.presets, &c, &d));

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

        let r = inspect(&Book::new(Some(&f.up), &f.presets, &c, &d));
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
            &f.presets.registry,
            &[Patch::SetValue {
                level: crate::workbench::domain::Level::Version,
                owner: "A1/STANDARD".to_owned(),
                key: "wiping.mode".to_owned(),
                value: Some(serde_json::json!("teleport")),
            }],
        )
        .unwrap();
        let r = inspect(&Book::new(Some(&f.up), &f.presets, &c, &d));
        assert_eq!(r.issues[0].severity, Severity::Block);
        assert!(r.issues.windows(2).all(|w| w[0].severity <= w[1].severity));
    }

    /// 每条都要说得出「去哪儿」与「怎么办」
    #[test]
    fn every_issue_is_actionable() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let r = inspect(&Book::new(Some(&f.up), &f.presets, &c, &d));

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

        let r = inspect(&Book::new(Some(&f.up), &f.presets, &c, &d));
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

    /* ---------- 上游漂移（b05 Task 11.9） ---------- */

    /// 夹具上游与清单**完全对齐** → 一条漂移都不该有（healthy 对照；
    /// 没有这条，产出侧那条分不清「对齐了」和「没查」）
    #[test]
    fn upstream_drift_stays_silent_when_the_catalogs_agree() {
        let f = Fixture::load();
        let c = committed();
        let d = Draft::default();
        let r = inspect(&Book::new(Some(&f.up), &f.presets, &c, &d));
        assert!(
            !r.issues.iter().any(|i| i.id.starts_with("upstream.")),
            "对齐的夹具不该报上游漂移：{:?}",
            r.issues.iter().map(|i| i.id.as_str()).collect::<Vec<_>>()
        );
    }

    /// **真数据上的预检全量**（b05 Task 11.4–11.6 / 11.9，防空转的主判据）。
    ///
    /// 夹具上游 + 真机型清单 + 真配方：
    /// - 清单侧（真 presets）交付机型与版本同真配方**完全对齐** → 配方类一条不该报，
    ///   A2L 占位跳过（不参与交付）；
    /// - 夹具上游只认 A1 / A2L / P1S → 真清单里的 A1_MINI / P2S / X1C
    ///   正好当上游漂移的产出侧样本（11.9）
    #[test]
    fn the_real_recipe_and_catalog_line_up() {
        let Some(root) = crate::workbench::paths::presets_root() else {
            eprintln!("没定位到 <repo>/presets，这条检查未执行（不是通过）");
            return;
        };
        let f = Fixture::load();
        let real =
            crate::workbench::presets::Presets::load_from(&root).expect("真 presets 必须读得通");
        // 清单照 Book 的吃法走 Committed.catalog：从真 Catalog 转出 CatalogMachine
        let catalog: Vec<CatalogMachine> = real
            .catalog
            .machines()
            .iter()
            .map(|m| CatalogMachine {
                id: m.id.clone(),
                display: m.display.clone(),
                icon: m.icon.clone(),
                default_bundle: m.default_bundle.clone(),
                has_dimensions: m.has_dimensions,
                version_ids: m.versions.iter().map(|v| v.id.clone()).collect(),
            })
            .collect();
        let delivered = catalog.iter().filter(|m| m.has_dimensions).count();
        let versions: usize = catalog
            .iter()
            .filter(|m| m.has_dimensions)
            .map(|m| m.version_ids.len())
            .sum();
        // 反空转前置：交付机型 5 台（A2L 占位在外）、版本 9 个 —— 少了说明清单变了
        assert_eq!(
            delivered, 5,
            "参与交付的机型数变了（A2L 占位不算）—— 说清为什么再改判据"
        );
        assert_eq!(versions, 9, "交付版本数变了 —— 真配方那边是 9 个变体");

        let committed = Committed {
            catalog,
            ..Default::default()
        };
        let draft = Draft::default();
        let book = Book::new(Some(&f.up), &real, &committed, &draft);
        let recipe =
            Recipe::parse(preset::PRESET_RECIPES_TOML).expect("仓库里的配方真源必须 parse 得过");
        assert_eq!(
            recipe.combos().len(),
            9,
            "真配方的变体数变了 —— 清单与配方得一起动，这条判据逼着两边对表"
        );

        let mut out = Vec::new();
        recipe_alignment(&book, &recipe, &mut out);
        assert!(
            out.is_empty(),
            "真清单与真配方应当完全对齐，却报了：{:?}",
            out.iter()
                .map(|i| (i.id.as_str(), i.title.as_str()))
                .collect::<Vec<_>>()
        );

        // 11.9：夹具上游不认 A1_MINI / P2S / X1C —— 漂移要说得出是谁（Hint 档）
        let r = inspect(&book);
        let drift: Vec<&str> = r
            .issues
            .iter()
            .filter(|i| i.id.starts_with("upstream.unknown_machine."))
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(
            drift,
            vec![
                "upstream.unknown_machine.A1_MINI",
                "upstream.unknown_machine.P2S",
                "upstream.unknown_machine.X1C",
            ],
            "上游漂移的机型集合变了 —— 夹具上游或真清单动了，说清为什么"
        );
        // 机型级报过就 continue：那三台的版本不该逐条再报一遍。
        // A1 的 FASTV3.3 是**机型认得、版本不认**的合法漂移（夹具上游只有
        // STANDARD / FAST 两版）—— 这一条该在，而且是版本级的样本
        let version_drift: Vec<&str> = r
            .issues
            .iter()
            .filter(|i| i.id.starts_with("upstream.unknown_version."))
            .map(|i| i.id.as_str())
            .collect();
        assert_eq!(
            version_drift,
            vec!["upstream.unknown_version.A1.FASTV3.3"],
            "版本级漂移的集合变了 —— 夹具上游或真清单动了，说清为什么"
        );
    }
}
