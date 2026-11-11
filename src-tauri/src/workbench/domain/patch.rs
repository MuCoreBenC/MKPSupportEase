//! 唯一写入口的载荷、草稿、反向计算（doc §4）。
//!
//! # 一次调用 = 一次手势 = 一条撤销
//!
//! 所有写都走 `wb_apply_draft(label, patches)` 这一个口子。这不是为了好看 ——
//! 第二版有七八个写命令，每个都自己判一遍"该写哪一层"，于是批量改就只能靠调用方猜，
//! 猜错的后果是**静默打破继承**：用户以为改的是机型基底，实际写到了某个版本上。
//!
//! # 反向 patches 由后端算
//!
//! 因为只有后端知道**改之前那一层有没有这个键**。前端自己算反向，会在
//! 「原来是继承来的」这种情形上出错：它看到的是有效值（比如 `4`），
//! 于是反向写成「设回 4」—— 而正确的反向是**删键**，让它重新继承。
//! 两者的区别在下一次改机型基底时才会暴露：前者不跟着变，后者跟着变。
//!
//! # 这一层只管值（b04 Task 12 / REPORT §7）
//!
//! 曾经这里有 12 种动作，其中 8 种是**清单类**的：新建 / 克隆 / 改名 / 移动 /
//! 归档 / 还原 / 删除版本 / 挑 BBS。它们现在全部没有对象了：
//!
//! | 动作 | 去了哪里 | 理由 |
//! |---|---|---|
//! | 改名 | 「机型与版本」页 | 版本名就是机型文件里的 `name`，那一页已经能改 |
//! | 归档 / 还原 | **不做** | 源数据里没有这个概念，是我们自己发明的 SOP |
//! | BBS | 套餐那一层 | 版本已经有 `recommendedBundle`，再存一份 asset id 是两处真相 |
//! | 新建 / 克隆 / 移动 / 删除版本 | 「机型与版本」页 | 清单就是 `presets/machines/*.toml`，那一页直接写它 |
//!
//! 剩下 [`Patch::SetValue`]、[`Patch::SetVisibility`]、[`Patch::SetBundle`] 三种。
//! **撤销栈因此只服务值编辑** ——
//! 低频的结构操作一律即时落盘、没有一个蓄了半天才生效的第二副本。
//!
//! 曾经还有第四种 [`Patch::MarkBuilt`]（生成记录回填草稿）。2026-10-06 状态机修正把它
//! **整个删除**：生成记录是**台账**（`workbench/built.json`），由生成事务在写交付产物的
//! 同一批直接落盘 —— 它不是编辑，不该住在草稿里。住在草稿里的后果是「产物已写盘、
//! 台账在草稿」的三角状态：本机显示已生成，干净检出（CI）却判待生成（真机踩过，见
//! `app::build` 的生成事务文档）。
//!
//! # 哪些手势不进撤销栈
//!
//! 没有。三种 patch 全部可撤销 —— 撤销栈的取舍只剩「inverse 是不是空」
//! （无变化的编辑不产反向）。
//!
//! # 非法 patch 拒绝整批
//!
//! 部分应用会留下一个谁也说不清的中间状态：前三条生效了、第四条没有，
//! 而界面上只看到"保存失败"。所以先全部校验，再全部应用。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::workbench::presets::{ParamRegistry as Registry, ValueType};

use super::layer::Level;
use super::wording as w;

/// 交付物对客户可见吗（doc 的「菜单」与「仅归档」）。
///
/// 「仅归档」= 文件在仓库里、但客户端看不到也下载不到。它**不是删除** ——
/// 删了的东西进回收站，仅归档的东西还在菜单之外正常存在
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Visibility {
    /// 上菜单：客户看得到、能下载
    Menu,
    /// 仅归档：仓库里有，客户看不到
    ArchiveOnly,
}

/// 套餐的一次改动
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BundleEdit {
    /// MKP 产物的 asset id
    pub presets: Vec<String>,
    /// BBS 曲线的 asset id
    pub bbs: Vec<String>,
}

/// 「上次生成时长什么样」。`wb_diff_built` 拿它和当前配方比
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltRecord {
    pub stamp: String,
    /// 生成时那一份有效配方的指纹。产物过不过期看它，**不看文件时间**
    pub fingerprint: String,
}

/// 写操作的载荷（doc §4.1）。
///
/// 两条语义写死：
/// - **`SetValue` 必须带 `level`**，不许由后端猜；
/// - **`value: None` = 「跟随基底」（版本层）/ 删键（机型层）**，不是"值设成空"。
///
/// `value: None` 按层分家（2026-11-03，作者：「值虽然一直存在，但状态是跟随」）：
///
/// ```text
/// Version 层：值留着、同步成当前基底的有效值，跟随状态置 true
/// Machine 层：删掉那一行（＝回到出厂默认）
/// ```
///
/// 前端因此**一个字段都不用改**：右键「跟随默认基底」与抽屉里的 `↶` 本来就发
/// `value: null`，变的是后端怎么理解它。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Patch {
    SetValue {
        level: Level,
        /// `Machine` 时是机型 id，`Version` 时是版本 uid
        owner: String,
        key: String,
        /// `None` = 版本层「跟随基底」/ 机型层「删键回出厂」
        value: Option<Value>,
    },
    /// **「固定为当前值」/「取消固定」**（2026-11-02 作者裁定）。
    ///
    /// 与 [`Patch::SetValue`] 的唯一差别：**写在存的值等于"这一层不钉时会继承到的值"
    /// 时，也不许归一化成删键**。
    ///
    /// 蛇形字段要用 `rename` 逐个指到 camelCase —— 枚举上的 `rename_all`
    /// 只作用于**变体名**，管不到变体的字段（前端发 `fileId`，漏了它就是
    /// 「missing field file_id」，资产库的设归档整个是坏的）
    SetVisibility {
        #[serde(rename = "fileId")]
        file_id: String,
        visibility: Visibility,
    },
    SetBundle {
        #[serde(rename = "bundleId")]
        bundle_id: String,
        presets: Vec<String>,
        bbs: Vec<String>,
    },
}

/// 整本草稿：**只存"改了什么"，不存整本**（doc §4.3）。
///
/// 存整本的话，上游数据一变（比如上游改了某个参数的出厂默认），
/// 草稿会把旧值糊回去 —— 而那看起来就像"本地改动莫名其妙回来了"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Draft {
    /// `"m:A1:toolhead.offset.x"` / `"v:A1/STANDARD:toolhead.offset.x"`。
    /// **值为 `None` 表示"这一层的这个键被删了"**，与"没有这一条草稿"不同
    pub values: BTreeMap<String, Option<Value>>,

    /// **跟随状态**的草稿（2026-11-03）。键与 `values` **同一套**（`v:uid:key`），
    /// 值 = 这一格要变成的跟随状态（`true` = 跟随基底）。
    ///
    /// # 为什么它必须住在草稿里，而不是写值时顺手改注册表
    ///
    /// 值与状态是**同一笔事务**：撤销 / 放弃草稿时两者一起回退，`wb_save` 时
    /// 一起落盘。分开做的话中途失败会留下"值改了、状态还是跟随"，
    /// 下一次改基底就把它覆盖回去 —— 那正是作者报的那个 bug 的形状。
    ///
    /// 表里挂着的键 = **这一格的跟随状态待改**；没挂着 = 不动。
    pub follows: BTreeMap<String, bool>,

    pub visibility: BTreeMap<String, Visibility>,
    pub bundles: BTreeMap<String, BundleEdit>,
    // 没有 `built`：生成记录是台账（`workbench/built.json`），生成事务直接落盘，
    // 不住在草稿里 —— 见模块文档「状态机修正」那一段。
}

impl Draft {
    /// 草稿层里有几处改动 —— **一「处」= 一格字段**（2026-11-03）。
    ///
    /// 口径必须与 [`crate::workbench::domain::txn::Txn::dirty_count`] 一致
    /// （那个数的是"改了几个字段"），因为界面把两者**相加**成页头那个 N。
    /// 相加就要求同量纲：草稿层的一格（`m:A1:toolhead.offset.z`）落进文件
    /// 就是一个字段，与事务记的那一条正好对应。
    ///
    /// `visibility` / `bundles` 各是一条记录、各对应一处改动，也同量纲。
    pub fn dirty_count(&self) -> usize {
        /* 值与跟随状态挂**同一格的两本账**：`70 → 75` 又点了「跟随」时，
        `values` 与 `follows` 各有一条 —— 那是**一处**改动，不是两处。
        所以按去重后的键数算，不按表长相加。 */
        let cells: BTreeSet<&String> = self.values.keys().chain(self.follows.keys()).collect();
        cells.len() + self.visibility.len() + self.bundles.len()
    }

    pub fn is_clean(&self) -> bool {
        self.dirty_count() == 0
    }

    /// 把草稿里指向旧 uid（`机型/版本`）的键**重指到新 uid**（改版本 id 时用）。
    ///
    /// 键的形状是 `v:{uid}:{key}`。不重指的话，保存那一刻这批格子会按旧 uid 写进
    /// `machineVariants` —— 而那个版本已经没了，下一次加载判 Corrupted（与
    /// [`super::txn::Txn`] 里 `A1:FAST` 那些键同一条事故）。
    ///
    /// 只动版本层的键；机型层（`m:A1:key`）与 uid 无关，一个字节都不碰。
    pub fn retarget_version_uid(&mut self, old_uid: &str, new_uid: &str) {
        fn move_keys<V>(map: &mut BTreeMap<String, V>, old: &str, new: &str) {
            if !map.keys().any(|k| k.starts_with(old)) {
                return;
            }
            let taken = std::mem::take(map);
            *map = taken
                .into_iter()
                .map(|(k, v)| match k.strip_prefix(old) {
                    Some(rest) => (format!("{new}{rest}"), v),
                    None => (k, v),
                })
                .collect();
        }
        let old = format!("v:{old_uid}:");
        let new = format!("v:{new_uid}:");
        move_keys(&mut self.values, &old, &new);
        move_keys(&mut self.follows, &old, &new);
    }

    /// 某一层在草稿里挂着的值改动
    pub fn pending(&self, level: Level, owner: &str, key: &str) -> Option<&Option<Value>> {
        self.values.get(&value_key(level, owner, key))
    }

    /// 某一层在草稿里挂着的**跟随状态**改动。`None` = 这一笔不动状态。
    pub fn pending_follows(&self, level: Level, owner: &str, key: &str) -> Option<bool> {
        self.follows.get(&value_key(level, owner, key)).copied()
    }

    /// 丢掉指向已经不存在的版本的草稿条目（doc §15 那条）。
    ///
    /// **不让整本草稿失效**：上游删掉一个版本之后，用户在别的九个版本上的改动
    /// 没有任何理由跟着一起没了。返回给界面的提示句
    pub fn prune(&mut self, committed: &Committed) -> Vec<String> {
        let mut gone: BTreeSet<String> = BTreeSet::new();

        /* 值与跟随状态**同一套键**、同一套存活判据 —— 一条判据只写一处，
        免得哪天只清了一本账，另一本还挂着一个已经不存在的版本 */
        let alive = |k: &String, gone: &mut BTreeSet<String>| -> bool {
            match parse_value_key(k) {
                Some((Level::Version, owner, _)) => {
                    let ok = committed.has_version(owner);
                    if !ok {
                        gone.insert(owner.to_owned());
                    }
                    ok
                }
                Some((Level::Machine, owner, _)) => {
                    let ok = committed.has_machine(owner);
                    if !ok {
                        gone.insert(owner.to_owned());
                    }
                    ok
                }
                // 解析不出来的键：丢掉。留着它既进不了任何一层，也会让脏计数虚高
                None => {
                    gone.insert(k.clone());
                    false
                }
            }
        };

        self.values.retain(|k, _| alive(k, &mut gone));
        self.follows.retain(|k, _| alive(k, &mut gone));

        // （生成记录曾经也在这里清——它现在不住在草稿里了，见模块文档「状态机修正」。）
        gone.into_iter()
            .map(|uid| format!("草稿里有一条改动指向已经不存在的 {uid}，已丢弃"))
            .collect()
    }
}

/// 已落盘的那一份（不含草稿）。这一层只读它。
///
/// 为什么要显式传进来而不是在这里读文件：patch 的规则全是纯逻辑，
/// 掺进文件 IO 之后就只能靠建临时目录来测，而真正容易错的是「反向该写什么」
#[derive(Debug, Clone, Default)]
pub struct Committed {
    /// 版本 uid → **身份**（属于哪台机型、叫什么、什么角标）。
    /// 只有清单层面的东西，**一个值都没有** —— 值在 registry 的 `machineVariants` 里
    pub versions: BTreeMap<String, CommittedVersion>,
    pub visibility: BTreeMap<String, Visibility>,
    pub bundles: BTreeMap<String, BundleEdit>,
    pub built: BTreeMap<String, BuiltRecord>,
    /// **清单**：有哪些机型、每台有哪些版本。来源是 `presets/machines/*.toml`
    /// （b04 Task 8）—— 在那之前它是 `machine_ids`，来源是上游那份构建产物。
    ///
    /// 换源的后果一句话：**清单是我们自己的数据，可写**。
    /// 「加一个版本」保存之后它真的会出现在树上，而不是只留下一个孤儿文件
    pub catalog: Vec<CatalogMachine>,
}

impl Committed {
    pub fn machine(&self, id: &str) -> Option<&CatalogMachine> {
        self.catalog.iter().find(|m| m.id == id)
    }

    pub fn has_machine(&self, id: &str) -> bool {
        self.machine(id).is_some()
    }

    pub fn version(&self, uid: &str) -> Option<&CommittedVersion> {
        self.versions.get(uid)
    }

    pub fn has_version(&self, uid: &str) -> bool {
        self.versions.contains_key(uid)
    }
}

/// 清单里的一个版本。uid 的形状是 `"{机型}/{版本 id}"`（doc §3.5）
#[derive(Debug, Clone, Default)]
pub struct CommittedVersion {
    pub machine_id: String,
    pub version_id: String,
    /// 显示名（中文）。来源是机型文件里的 `[[versions]].name`
    pub name: String,
    /// 版本卡上那个角标（`推荐` / `热门`）。来源同上
    pub tag: Option<String>,
}

/// 清单里的一台机型。**只有清单，不含参数值** ——
/// 值那三层还在 `machines` / `versions` 里（Task 8 后面几步才搬）
#[derive(Debug, Clone, Default)]
pub struct CatalogMachine {
    pub id: String,
    /// 界面上显示的名字。实测有机型的 `name` 是空串而 `display` 才是给人看的
    pub display: String,
    pub icon: Option<String>,
    /// `defaultBundle`：这台机型默认那一套 BBS 曲线
    pub default_bundle: Option<String>,
    /// 机型文件里有没有 `[dimensions]`。A2L 实测没有 —— 界面上要能看出"这台还没配尺寸"
    pub has_dimensions: bool,
    /// 版本 id，**照机型文件里的顺序**。
    /// 不用 `BTreeSet` 是因为顺序会直接进界面，而字典序不是作者写下的顺序
    pub version_ids: Vec<String>,
}

/// 从**编辑态**的预设数据里摊出清单 —— [`CatalogMachine`] 构造的唯一实现。
///
/// 两个调用方：
///
/// - `app::storage::load`：开会话 / 保存后把「已落盘那一份」记成 [`Committed`]；
/// - `domain::derive::Book::new`：界面读的那一份，**必须跟着编辑态走**。
///
/// 两处共用一条是因为 `Book` 曾经自己从 `Committed.catalog` 取清单：新建的版本
/// 只写进 `presets` 与影子树，`Committed` 要等保存才更新 —— 于是「新增版本」
/// 只在左侧列表里出现，详情卡、参数台、生成页都看不见它（作者 2026-11-12 报的
/// 「新增的这些展开不了详情」）。
pub fn catalog_machines_of(presets: &crate::workbench::presets::Presets) -> Vec<CatalogMachine> {
    presets
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
        .collect()
}

/// 应用的结果
#[derive(Debug, Clone)]
pub struct Applied {
    /// 撤销这次操作要提交的 patches。空 = 这次没有可撤销的改动
    /// （无变化的编辑不产反向，界面上也就不该有撤销按钮）
    pub inverse: Vec<Patch>,
}

/// 值改动在草稿里的键。`"m:A1:toolhead.offset.x"` / `"v:A1/STANDARD:toolhead.offset.x"`
///
/// 三段用 `:` 分隔是安全的：机型 id 与版本 id 走 `store::validate_id` 的白名单
/// （字母数字 `-` `_` `.`，没有冒号），参数 key 也只有点号
/// 公开的合并键（`domain::history` 用它当"哪一格"）—— 与内部 `value_key` 同一个写法，
/// **只有这一个出口**，别处不许自己拼字符串。
pub fn value_key_public(level: Level, owner: &str, key: &str) -> String {
    value_key(level, owner, key)
}

fn value_key(level: Level, owner: &str, key: &str) -> String {
    let tag = match level {
        Level::Machine => "m",
        Level::Version => "v",
    };
    format!("{tag}:{owner}:{key}")
}

fn parse_value_key(s: &str) -> Option<(Level, &str, &str)> {
    let mut it = s.splitn(3, ':');
    let level = match it.next()? {
        "m" => Level::Machine,
        "v" => Level::Version,
        _ => return None,
    };
    let owner = it.next()?;
    let key = it.next()?;
    if owner.is_empty() || key.is_empty() {
        return None;
    }
    Some((level, owner, key))
}

/// 这条草稿键是不是属于 `(level, owner)`；是就返回参数 key。
///
/// 给 `derive` 合成覆盖表用。**不导出 `value_key` / `parse_value_key` 本身** ——
/// 键的格式是这一层的内部约定，导出了就等于请别的模块自己拼字符串
pub fn split_value_key(raw: &str, level: Level, owner: &str) -> Option<String> {
    match parse_value_key(raw) {
        Some((l, o, k)) if l == level && o == owner => Some(k.to_owned()),
        _ => None,
    }
}

/// 应用一批 patch，并返回反向。
///
/// 流程：**全部校验 → 结构 → 值**。任何一条校验不过就整批拒绝，草稿一个字节不改
pub fn apply(
    draft: &mut Draft,
    committed: &Committed,
    registry: &Registry,
    patches: &[Patch],
) -> Result<Applied, AppError> {
    // 数字先归到注册表声明的那个类型上（见 [`normalize`]）—— 校验、比较、落草稿
    // 三处必须看同一个 `Value`
    let normalized: Vec<Patch> = patches.iter().map(|p| normalize(registry, p)).collect();
    let patches: &[Patch] = &normalized;

    validate(committed, registry, patches)?;

    let mut inverse: Vec<Patch> = Vec::new();

    for p in patches {
        apply_one(draft, committed, registry, p, &mut inverse);
    }

    // 反向要**倒着执行**才能回到原状：正向 A→B→C，反向是 C⁻¹→B⁻¹→A⁻¹
    inverse.reverse();

    Ok(Applied { inverse })
}

/// 把 `SetValue` 的数字归到注册表声明的那个类型上。
///
/// JSON 只有一种 number，而 `serde_json::Value` 分正/负整数与浮点**三种** ——
/// 盘上的 `-1.0`（TOML 里的浮点，见 `param_registry.toml` 的 `'A1:STANDARD' = -1.0`）
/// 和界面发来的 `-1`（JS 的 number 序列化成整数）用 `==` 比是**不相等**的两个 `Value`。
/// 不归一的话，「改了又改回去」永远比不中盘上那个值：
///
///   · [`apply_one`] 里那两步比较都落空 → 草稿里留下一笔多余的改动，
///     界面一直显示「已修改」、保存按钮一直亮（作者实测）；
///   · 草稿里存的是 `-1`、盘上是 `-1.0`，一次无害的编辑也会让生成物的数字换一副长相。
///
/// 归不了（值不是数、类型不是数、注册表里没有这个 key）就原样放行 ——
/// 这一层不是校验器，判能不能写是 [`validate`] 的事。
///
/// 批量预览（[`super::preview`]）也用这个口径把要对齐的那个值归位：
/// 它和写入口看的是同一份盘上数据，归一规则分岔的话「预览说没有变化、写入却记了一笔」
/// 就会同时成立。
pub(super) fn normalize_value(registry: &Registry, key: &str, v: &Value) -> Value {
    let num = match registry.param(key).map(|p| p.value_type) {
        Some(ValueType::Float) => v.as_f64().and_then(serde_json::Number::from_f64),
        Some(ValueType::Int) => as_int(v).map(serde_json::Number::from),
        _ => None,
    };
    match num {
        Some(n) => Value::Number(n),
        None => v.clone(),
    }
}

/// 整数参数那一侧的归一：盘上写成 `1.0` 也要读回 `1`。
///
/// `Value::as_i64()` 只认整数那两种编号，遇到 `1.0`（浮点）返回 `None` ——
/// 真数据里 `wiping.glue_pass_count` 等四个 int 参数的 `defaultValue` 就写成了
/// `1.0` / `30.0` / `50.0` / `0.0`，不补这一步它们仍然比不中
fn as_int(v: &Value) -> Option<i64> {
    if let Some(i) = v.as_i64() {
        return Some(i);
    }
    match v.as_f64() {
        Some(f) if f.is_finite() && f.fract() == 0.0 => Some(f as i64),
        _ => None,
    }
}

fn normalize(registry: &Registry, patch: &Patch) -> Patch {
    match patch {
        Patch::SetValue {
            level,
            owner,
            key,
            value: Some(v),
        } => Patch::SetValue {
            level: *level,
            owner: owner.clone(),
            key: key.clone(),
            value: Some(normalize_value(registry, key, v)),
        },
        _ => patch.clone(),
    }
}

/// 整批校验。这里只判"能不能做"，不改任何东西
fn validate(committed: &Committed, registry: &Registry, patches: &[Patch]) -> Result<(), AppError> {
    let known_uid = |uid: &String| -> Result<(), AppError> {
        if committed.has_version(uid) {
            Ok(())
        } else {
            Err(AppError::not_found(format!("版本 {uid} 不存在"))
                .with_detail("整批改动已拒绝，草稿没有变"))
        }
    };
    let known_machine = |id: &String| -> Result<(), AppError> {
        if committed.has_machine(id) {
            Ok(())
        } else {
            Err(AppError::not_found(format!("机型 {id} 不存在"))
                .with_detail("整批改动已拒绝，草稿没有变"))
        }
    };

    for p in patches {
        match p {
            /* 「固定」与「写值」的目标层、弃用闸、存在性**完全同一套** ——
            合并成一条分支，两条路各写一遍迟早会只改一边 */
            Patch::SetValue {
                level, owner, key, ..
            } => {
                if registry.param(key).is_none() {
                    return Err(AppError::invalid_argument(format!("字段定义里没有 {key}"))
                        .with_detail("整批改动已拒绝，草稿没有变"));
                }
                deprecated_write_gate(committed, registry, key, p)?;
                match level {
                    Level::Machine => known_machine(owner)?,
                    Level::Version => known_uid(owner)?,
                }
            }
            Patch::SetVisibility { file_id, .. } => non_blank(file_id, "文件 id")?,
            Patch::SetBundle { bundle_id, .. } => non_blank(bundle_id, "套餐 id")?,
        }
    }
    Ok(())
}

fn non_blank(s: &str, what: &str) -> Result<(), AppError> {
    if s.trim().is_empty() {
        return Err(AppError::invalid_argument(format!("{what}不能为空"))
            .with_detail("整批改动已拒绝，草稿没有变"));
    }
    Ok(())
}

/// 弃用写闸（C14 §五）：**写值的唯一出口就是这里**，所以闸只设这一处 ——
/// 行上的控件、右栏各层、以后的批量，谁想写都得从这儿过。
///
/// 两档：
/// 1. 参数自己在这台机型上被弃用 —— 值照旧读得到，但「不要再改它」（`null` = 挂回继承不拦：
///    删键不是写值，老配方里钉着的弃用值也得有一条退路）；
/// 2. 写进来的值是一个**在这台机型上**已弃用的选项（「外围结构 = 护套」那一档 ——
///    选项点得动，作者要的正是「我可以选择但是不能保存」）。
///
/// 「在哪台机型上」由 patch 的目标层决定：机型层直接就是 owner；
/// 版本层查清单一遍拿所属机型。版本 uid 不存在时这里放行 —— 随后 `known_uid`
/// 会报「版本不存在」，那才是这一笔真正的问题。
///
/// 界面在手势前就拦了（toast 一句人话）；这里是后端的闸，拦的是绕过界面的路。
fn deprecated_write_gate(
    committed: &Committed,
    registry: &Registry,
    key: &str,
    patch: &Patch,
) -> Result<(), AppError> {
    let Patch::SetValue {
        level,
        owner,
        value,
        ..
    } = patch
    else {
        return Ok(());
    };
    // `None`（或显式 null）= 删键 = 挂回继承：删键不是写值，不拦
    let Some(value) = value else {
        return Ok(());
    };
    if value.is_null() {
        return Ok(());
    }
    let Some(p) = registry.param(key) else {
        return Ok(());
    };
    let machine = match level {
        Level::Machine => owner.as_str(),
        Level::Version => match committed.version(owner) {
            Some(v) => v.machine_id.as_str(),
            None => return Ok(()),
        },
    };
    let refused = |what: &str| {
        AppError::invalid_argument(format!("{what}{}", w::disabled::DEPRECATED_WRITE_BLOCKED))
            .with_detail(format!(
                "整批改动已拒绝，草稿没有变 —— 「{machine}」上这一项已弃用"
            ))
    };
    if p.deprecated_for(machine) {
        return Err(refused(&p.label));
    }
    if let Some(choice) = registry.deprecated_choice_hit(key, value, machine) {
        return Err(refused(&format!("{0} 的「{choice}」", p.label)));
    }
    Ok(())
}

fn apply_one(
    draft: &mut Draft,
    committed: &Committed,
    registry: &Registry,
    patch: &Patch,
    inverse: &mut Vec<Patch>,
) {
    match patch {
        Patch::SetValue {
            level,
            owner,
            key,
            value,
        } => {
            let vk = value_key(*level, owner, key);
            /* ---------- 现在这一格是什么样（草稿优先，其次盘上） ---------- */
            let at_rest_value = at_rest(registry, *level, owner, key);
            let cur_value = draft
                .values
                .get(&vk)
                .cloned()
                .unwrap_or_else(|| at_rest_value.clone());
            let at_rest_follow = following_at_rest(registry, *level, owner, key);
            let cur_follow = draft.follows.get(&vk).copied().or(at_rest_follow);

            /* ---------- 这一笔要把它变成什么样 ---------- */
            /*
             * ★ 2026-11-03：`value: None` 的语义按层分开。
             *
             * **版本层**：`None` 不再是"删键"，而是**「跟随基底」**——
             *   值留着（同步成当前基底的有效值），状态置 `follows = true`。
             *   作者的原话：「值虽然一直存在，但状态是跟随」。删键那条老路会把
             *   "这一版的值"抹掉，而跟随不该等于"没有值"。
             *
             * **机型层**：`None` 仍是**删键 = 回到出厂默认**。基底是这条链的根，
             *   没有"跟随"可言（`badge_at` 也从不给基底列挂徽章）。
             */
            let (tgt_value, tgt_follow) = match (level, value) {
                (Level::Version, None) => {
                    if cur_follow == Some(true) {
                        /* **已经跟随**：值本来就该等于基底（跟随的格子由写值那条路
                        守恒），点一下不该在文件里凭空多写一行 —— 否则"点跟随"
                        这个动作在已经跟随的格子上也算一次改动，未保存数会无故 +1 */
                        (cur_value.clone(), Some(true))
                    } else {
                        (base_value_of(registry, owner, key), Some(true))
                    }
                }
                (Level::Version, Some(v)) => (
                    Some(normalize_value(registry, key, v)),
                    /* 用户直接改这一版的值 ⇒ 它归自己（「改过就不跟随」） */
                    Some(false),
                ),
                (Level::Machine, v) => (v.clone(), None),
            };

            if cur_value == tgt_value && cur_follow == tgt_follow {
                return; // 没变化：不记草稿、不产反向，脏计数也不该涨
            }

            /* ---------- 值与状态**同时**写进草稿 ----------
             *
             * 判"还算不算改动"的口径：**这一层和它原本那一行是否一致**。
             * 一致就从草稿里拿掉（否则"改了又改回来"会让保存按钮一直亮），
             * 不一致就记下来。值与状态各判各的：只点了「跟随」（值本来就等于基底）
             * 时，`values` 里什么都不该留，`follows` 里留一条。
             */
            if at_rest_value == tgt_value {
                draft.values.remove(&vk);
            } else {
                draft.values.insert(vk.clone(), tgt_value);
            }
            if let Some(on) = tgt_follow {
                if at_rest_follow == Some(on) {
                    draft.follows.remove(&vk);
                } else {
                    draft.follows.insert(vk, on);
                }
            }

            /* ---------- 反向：把**值和状态**一起送回原样 ----------
             *
             * 版本层原来处于"跟随"时，反向发 `None`（= 跟随 + 值同步成基底），
             * 而不是"写回那个数" —— 写回一个数的意思是不跟随，状态就丢了。
             * 原来"不跟随"时反向只能写回它那一行的值（`value: None` 在这里
             * 表达不了"不跟随但没值"，而那个状态在数据里不存在：不跟随的格子
             * 一定有一行值，见 `apply_one` 的所有入口）。
             */
            let inverse_value = match (level, cur_follow) {
                (Level::Version, Some(true)) => None,
                _ => cur_value,
            };
            inverse.push(Patch::SetValue {
                level: *level,
                owner: owner.clone(),
                key: key.clone(),
                value: inverse_value,
            });
        }

        Patch::SetVisibility {
            file_id,
            visibility,
        } => {
            let saved = committed
                .visibility
                .get(file_id)
                .copied()
                .unwrap_or(Visibility::Menu);
            let before = draft.visibility.get(file_id).copied().unwrap_or(saved);
            if before == *visibility {
                return;
            }
            if saved == *visibility {
                draft.visibility.remove(file_id);
            } else {
                draft.visibility.insert(file_id.clone(), *visibility);
            }
            inverse.push(Patch::SetVisibility {
                file_id: file_id.clone(),
                visibility: before,
            });
        }

        Patch::SetBundle {
            bundle_id,
            presets,
            bbs,
        } => {
            let saved = committed
                .bundles
                .get(bundle_id)
                .cloned()
                .unwrap_or_default();
            let before = draft
                .bundles
                .get(bundle_id)
                .cloned()
                .unwrap_or_else(|| saved.clone());
            let next = BundleEdit {
                presets: presets.clone(),
                bbs: bbs.clone(),
            };
            if before == next {
                return;
            }
            if saved == next {
                draft.bundles.remove(bundle_id);
            } else {
                draft.bundles.insert(bundle_id.clone(), next);
            }
            inverse.push(Patch::SetBundle {
                bundle_id: bundle_id.clone(),
                presets: before.presets,
                bbs: before.bbs,
            });
        }
    }
}

/// 草稿里的 owner → `machineVariants` 的键。
///
/// 机型层两边同形（`A1`）；版本层草稿用 uid（`A1/STANDARD`），
/// registry 用冒号（`A1:STANDARD`）。**换算只在这里做一处** —— 散出去的话，
/// 迟早有一个地方忘了替换那个斜杠，于是写出来的键 registry 永远认不出来
pub fn variant_key(level: Level, owner: &str) -> String {
    match level {
        Level::Machine => owner.to_owned(),
        Level::Version => owner.replace('/', ":"),
    }
}

/// 这一格**盘上**的跟随状态。`None` = 与跟随无关（机型基底那一条链的根）。
///
/// 只有版本层有意义：`[params.follows]` 里没有这个 uid = 跟随（见
/// [`crate::presetdata::follows::is_following`]）。
fn following_at_rest(registry: &Registry, level: Level, owner: &str, key: &str) -> Option<bool> {
    if level == Level::Machine {
        return None;
    }
    let p = registry.param(key)?;
    Some(crate::presetdata::follows::is_following(
        p,
        &variant_key(level, owner),
    ))
}

/// 这一版**跟随的那个基底值**：机型裸键 → 出厂默认。
///
/// 点「跟随基底」时用它把值同步过去（值要留着，且等于基底）。
fn base_value_of(registry: &Registry, owner: &str, key: &str) -> Option<Value> {
    let p = registry.param(key)?;
    /* ⚠ 版本 owner 到这一层还是 `A1/STANDARD`（`machineVariants` 里才写成
    `A1:STANDARD`），所以**两种分隔符都要认** —— 只认 `:` 会把机型名取成
    `A1/STANDARD`，于是"跟随"同步出来的是出厂默认而不是机型基底 */
    let machine = owner.split(['/', ':']).next().unwrap_or(owner);
    Some(
        p.machine_variants
            .get(machine)
            .cloned()
            .unwrap_or_else(|| p.default_value.clone()),
    )
}

/// `machineVariants` 里**此刻写着**的那一个值 —— 这是这一层唯一的盘上真相。
///
/// **读出来就归一到声明类型**（[`normalize_value`]）：真数据里有几处声明是浮点、
/// 盘上却写成了整数（`wiping.glue_pass_count` 的 `defaultValue = 1.0` 反过来也有）。
/// 只归一界面发来的那一侧的话，这两档仍然比不中，「改回原值」照样留下一笔多余改动
fn at_rest(registry: &Registry, level: Level, owner: &str, key: &str) -> Option<Value> {
    let variant = variant_key(level, owner);
    let raw = registry.param(key)?.machine_variants.get(&variant)?;
    Some(normalize_value(registry, key, raw))
}

/// 只留下 uid 还活着的条目，被丢掉的记进 `gone`。
///
#[cfg(test)]
mod tests {
    use super::*;

    /// 两个参数的最小字段定义。
    ///
    /// patch 层除了判"这个 key 存在吗"，还要知道"它此刻钉着什么"（[`at_rest`]）——
    /// 而现在这一层的真相就在 `machineVariants` 里，所以夹具也要带上它：
    /// `offset.z` 在机型层有值（`A1`），`offset.x` 在版本层有值（`A1:STANDARD`）
    ///
    /// **数值一律写成浮点**（`4.0` / `-1.0`），照抄 `presets/registry/param_registry.toml`
    /// 里那个样子：写成 `4` / `-1` 会让夹具里所有数字都成了整数，而界面发来的正好也是
    /// 整数 —— 两边碰巧同型，`serde_json` 那个「浮点 ≠ 整数」的坑就永远测不出来
    /// （真出过一次，见 [`normalize`]）
    fn registry() -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        /* 夹具里**写了版本行 ⇒ 那一版不跟随** —— 与 `presets follows-migrate`
        同一口径（`machineVariants` 里那一行就是"这一版自己定的值"）。
        不带上这句，版本会被当成"跟随基底"（文件里没有状态 = 跟随），
        于是"改一个值"顺带改了状态，下面一半的断言就测不到原本那件事了。 */
        let p = |key: &str, order: f64, variants: serde_json::Value| {
            let follows: serde_json::Map<String, serde_json::Value> = variants
                .as_object()
                .map(|m| {
                    m.keys()
                        .filter(|k| k.contains(':'))
                        .map(|k| (k.clone(), serde_json::json!(false)))
                        .collect()
                })
                .unwrap_or_default();
            serde_json::json!({
                "key": key, "configKey": "X", "tomlKey": key, "jsonKey": key,
                "label": key, "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 4.0,
                "scope": "universal", "section": "toolhead",
                "layout": { "order": order, "sectionId": "s1" },
                "machineVariants": variants,
                "follows": follows
            })
        };
        let params = serde_json::json!({
            "params": [
                p("toolhead.offset.x", 1.0, serde_json::json!({ "A1:STANDARD": -1.0 })),
                p("toolhead.offset.z", 2.0, serde_json::json!({ "A1": 1.1 }))
            ],
            "tabs": [{ "id": "t1", "label": "偏移", "order": 10,
                       "sections": [{ "id": "s1", "label": "空间偏移", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "toolhead.offset.x" },
                { "id": "i1", "paramKey": "toolhead.offset.z" }
            ] }] }]
        });
        let r =
            crate::workbench::presets::registry::load_from_json_fixture(d.path(), &params, &layout)
                .unwrap();
        (d, r)
    }

    /// 只有身份，一个值都没有 —— 值在 [`registry`] 那份 `machineVariants` 里
    fn committed() -> Committed {
        let versions = BTreeMap::from([(
            "A1/STANDARD".to_owned(),
            CommittedVersion {
                machine_id: "A1".to_owned(),
                version_id: "STANDARD".to_owned(),
                name: "标准版".to_owned(),
                tag: None,
            },
        )]);

        Committed {
            versions,
            catalog: vec![
                CatalogMachine {
                    id: "A1".to_owned(),
                    display: "A1".to_owned(),
                    version_ids: vec!["STANDARD".to_owned()],
                    ..Default::default()
                },
                CatalogMachine {
                    id: "P1S".to_owned(),
                    display: "P1S".to_owned(),
                    ..Default::default()
                },
            ],
            ..Default::default()
        }
    }

    fn set(level: Level, owner: &str, key: &str, value: Option<serde_json::Value>) -> Patch {
        Patch::SetValue {
            level,
            owner: owner.to_owned(),
            key: key.to_owned(),
            value,
        }
    }

    /// **反向的重点**：原来是继承来的，反向必须是**删键**，不是写回有效值。
    ///
    /// 写回有效值的话，下一次改机型基底，这一项不会跟着变 —— 而它本该跟着变
    #[test]
    fn inverse_of_a_first_write_is_a_delete_not_the_old_effective_value() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        // offset.z 在版本上没有自有值（它继承机型基底的 1.1）
        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(2)),
            )],
        )
        .unwrap();

        assert_eq!(
            out.inverse,
            vec![set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                None
            )],
            "反向该是删键，不是「设回 1.1」"
        );
    }

    /// 原来有自有值时，反向才是写回那个旧值
    #[test]
    fn inverse_of_overwriting_an_own_value_restores_it() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(7)),
            )],
        )
        .unwrap();
        assert_eq!(
            out.inverse,
            vec![set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(-1.0))
            )]
        );
    }

    /// **`value: None` 在版本层 = 「跟随基底」**（2026-11-03 改判）：值留着，
    /// 同步成当前基底；状态置跟随。**与"写空串"是两回事**（空串是一个值）。
    ///
    /// 机型层的 `None` 仍是删键（回到出厂默认）—— 基底是这条链的根，没有"跟谁"。
    #[test]
    fn none_means_follow_on_a_version_and_delete_on_a_machine() {
        let (_d, reg) = registry();
        let c = committed();

        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                None,
            )],
        )
        .unwrap();
        assert_eq!(
            draft.pending(Level::Version, "A1/STANDARD", "toolhead.offset.x"),
            Some(&Some(serde_json::json!(4.0))),
            "跟随：那一行值留着，并同步成当前基底（offset.x 没有 A1 裸键 ⇒ 出厂 4.0）"
        );
        assert_eq!(
            draft.pending_follows(Level::Version, "A1/STANDARD", "toolhead.offset.x"),
            Some(true),
            "值与状态是同一笔草稿：这一版从此跟随基底"
        );

        /* 「同步成基底」同步的是**机型那一层**，不是出厂默认：
        `offset.z` 在 A1 有裸键 1.1，而版本 owner 到这一层还写着 `A1/STANDARD` */
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(9)),
            )],
        )
        .unwrap();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                None,
            )],
        )
        .unwrap();
        assert_eq!(
            draft.pending(Level::Version, "A1/STANDARD", "toolhead.offset.z"),
            Some(&Some(serde_json::json!(1.1))),
            "跟随同步的是机型基底（A1 = 1.1），不是出厂默认 4.0"
        );

        /* 机型层：`None` 还是删键，且不挂状态 */
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(Level::Machine, "A1", "toolhead.offset.z", None)],
        )
        .unwrap();
        assert_eq!(
            draft.pending(Level::Machine, "A1", "toolhead.offset.z"),
            Some(&None),
            "机型层的 None 是删掉那一行 = 回到出厂默认"
        );
        assert_eq!(
            draft.pending_follows(Level::Machine, "A1", "toolhead.offset.z"),
            None,
            "基底没有「跟不跟」可言"
        );

        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!("")),
            )],
        )
        .unwrap();
        assert_eq!(
            draft.pending(Level::Version, "A1/STANDARD", "toolhead.offset.x"),
            Some(&Some(serde_json::json!(""))),
            "写空串是写了一个值"
        );
    }

    /// 改成和现在一样的值 → 不记草稿、不产反向、脏计数不涨
    #[test]
    fn a_no_op_write_does_not_dirty_the_draft() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();
        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(-1)),
            )],
        )
        .unwrap();
        assert!(out.inverse.is_empty());
        assert_eq!(draft.dirty_count(), 0);
    }

    /// 改了又改回落盘的那个值 → 这一处不再是改动，保存按钮该灭
    #[test]
    fn changing_back_to_the_saved_value_clears_the_dirty_mark() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(7)),
            )],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 1);

        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(-1)),
            )],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 0, "改回去就不该再算一处未保存改动");
    }

    /// **改回「与继承值相同」仍然是一笔改动**（2026-11-02 换口径后）。
    ///
    /// 旧口径把"值等于继承值"当成"改回原样"，于是这一笔被吞掉，界面回到「跟随」。
    /// 作者定稿是反过来的：只要这一层从"没有"变成"有"，就是「不跟随」——
    /// 哪怕写的就是基底那个数。想回到跟随，是**删键**（右键「跟随默认基底」）。
    #[test]
    fn changing_back_to_the_inherited_value_is_still_a_change() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        // `toolhead.offset.z` 只有机型层钉着（`A1` = 1.1），版本层没有任何条目
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(2)),
            )],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 1);

        /* ★ 2026-11-02 换口径：改回"与继承值相同"**仍然是一笔改动**。
        作者的原话是「改过就不跟随」—— 这一层从"没有"变成"有"了，
        哪怕值恰好等于基底，界面也该显示「不跟随」。想回到跟随是删键，不是写同值。 */
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(1.1)),
            )],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 1, "写过就是写过：值等于继承值也不例外");
        assert_eq!(
            draft.pending(Level::Version, "A1/STANDARD", "toolhead.offset.z"),
            Some(&Some(serde_json::json!(1.1))),
            "草稿里留着这一笔，保存后那一行才会落到文件里"
        );
    }

    /// **界面发来整数、盘上写的是浮点** —— `serde_json::Value` 里 `-1` 与 `-1.0`
    /// 是两个**不相等**的 `Value`。不归型的话「改回去」永远比不中盘上那个值，
    /// 草稿里留下多余的一笔（作者在真后端上实测到的那条）。
    #[test]
    fn an_integer_from_the_ui_matches_the_float_on_disk() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();
        let vk = value_key(Level::Version, "A1/STANDARD", "toolhead.offset.x");

        // 盘上 `A1:STANDARD` = -1.0；界面把输入框里的 "-1" 发成整数
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(5)),
            )],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 1);
        assert_eq!(
            draft.values.get(&vk),
            Some(&Some(serde_json::json!(5.0))),
            "落进草稿的也必须是浮点，和盘上同型"
        );

        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(-1)),
            )],
        )
        .unwrap();
        assert_eq!(
            draft.dirty_count(),
            0,
            "整数 -1 就是盘上那个浮点 -1.0，改回去不该再算改动"
        );
    }

    /// 一份只有 `toolhead.offset.x` 的最小字段定义，**盘上那一份故意写成
    /// 与声明类型不同的数字写法**（`disk` 参数说了算）。
    ///
    /// 真数据里两种写法都有：几个 float 参数的 `machineVariants` 写成整数、
    /// 几个 int 参数的 `defaultValue` 写成 `1.0`。夹具照抄这个形状才测得到那个坑
    fn registry_with_offtype_disk(
        declared: &str,
        disk: serde_json::Value,
    ) -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        let params = serde_json::json!({
            "params": [{
                "key": "toolhead.offset.x", "configKey": "X", "tomlKey": "x", "jsonKey": "x",
                "label": "X 轴偏移", "desc": "", "tomlComment": "",
                "valueType": declared, "uiComponent": "number", "defaultValue": 0,
                "scope": "universal", "section": "toolhead",
                "layout": { "order": 1.0, "sectionId": "s1" },
                "machineVariants": { "A1:STANDARD": disk },
                /* 有版本行 ⇒ 那一版不跟随（与 `presets follows-migrate` 同一口径） */
                "follows": { "A1:STANDARD": false }
            }],
            "tabs": [{ "id": "t1", "label": "偏移", "order": 10,
                       "sections": [{ "id": "s1", "label": "空间偏移", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "toolhead.offset.x" }
            ] }] }]
        });
        let r =
            crate::workbench::presets::registry::load_from_json_fixture(d.path(), &params, &layout)
                .unwrap();
        (d, r)
    }

    /// 声明是**浮点**、盘上却写成整数：界面发来的整数仍要能比中。
    ///
    /// 只归一界面发来的那一侧不够 —— 盘上读出来的那份也得归一（见 [`at_rest`]）
    #[test]
    fn an_integer_written_on_disk_still_matches_the_declared_float_type() {
        let (_d, reg) = registry_with_offtype_disk("float", serde_json::json!(-1));
        let c = committed();
        let mut draft = Draft::default();
        let write = |draft: &mut Draft, v: serde_json::Value| {
            apply(
                draft,
                &c,
                &reg,
                &[set(
                    Level::Version,
                    "A1/STANDARD",
                    "toolhead.offset.x",
                    Some(v),
                )],
            )
            .unwrap();
        };

        write(&mut draft, serde_json::json!(5));
        assert_eq!(draft.dirty_count(), 1);

        write(&mut draft, serde_json::json!(-1));
        assert_eq!(
            draft.dirty_count(),
            0,
            "盘上写成 -1 也还是那个 -1.0，改回去不该再算改动"
        );
    }

    /// 反过来的那一档：声明是**整数**、盘上写成 `-1.0`。
    ///
    /// `Value::as_i64()` 认不出 `-1.0`，所以归一里补了 [`as_int`] 那一手
    #[test]
    fn a_float_written_on_disk_still_matches_the_declared_int_type() {
        let (_d, reg) = registry_with_offtype_disk("int", serde_json::json!(-1.0));
        let c = committed();
        let mut draft = Draft::default();
        let write = |draft: &mut Draft, v: serde_json::Value| {
            apply(
                draft,
                &c,
                &reg,
                &[set(
                    Level::Version,
                    "A1/STANDARD",
                    "toolhead.offset.x",
                    Some(v),
                )],
            )
            .unwrap();
        };

        write(&mut draft, serde_json::json!(5));
        assert_eq!(draft.dirty_count(), 1);

        write(&mut draft, serde_json::json!(-1));
        assert_eq!(
            draft.dirty_count(),
            0,
            "盘上写成 -1.0 也还是那个 -1，改回去不该再算改动"
        );
    }

    /// 机型层：写回**它本来那一行**的值不算改动；写一个等于出厂值的新行**算**。
    ///
    /// 两段的差别正是新口径的落点：判断依据是「本层有没有那一行」，
    /// 不是「值等不等于继承/出厂」。
    #[test]
    fn writing_back_the_same_row_is_not_a_change_but_creating_one_is() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        // 机型层本来就钉着 1.1 ⇒ 写 1.1 是"改回原样"
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "A1",
                "toolhead.offset.z",
                Some(serde_json::json!(1.1)),
            )],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 0, "本来就是这一行的值，不是改动");

        // `toolhead.offset.x` 在机型层没有条目，出厂值是 4
        // ⇒ 写一个 4 —— 本层从"没有"变成"有"，**算改动**（界面显示「不跟随」）
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "A1",
                "toolhead.offset.x",
                Some(serde_json::json!(4)),
            )],
        )
        .unwrap();
        assert_eq!(
            draft.dirty_count(),
            1,
            "值等于出厂也算改过：这一层现在有自己的一行了"
        );
    }

    /// 删键（`None`）的判据仍是**本层**：本层本来就没有它 → 删了也没变
    #[test]
    fn deleting_a_key_the_layer_never_had_is_not_a_change() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                None,
            )],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 0, "本层没有这个键，删键就是没变化");

        // 本层钉着 1.1 → 删键是一次真实的改动（挂回继承）
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(Level::Machine, "A1", "toolhead.offset.z", None)],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 1, "删掉本层钉着的键要记一笔");
        assert_eq!(
            draft.values.get("m:A1:toolhead.offset.z"),
            Some(&None),
            "草稿里要写「这一层的这个键被删了」"
        );
    }

    /// 非法 patch **拒绝整批**：前面几条也不许生效
    #[test]
    fn an_invalid_patch_rejects_the_whole_batch() {
        let (_d, reg) = registry();
        let c = committed();

        for bad in [
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.made_up",
                Some(serde_json::json!(1)),
            ),
            set(
                Level::Version,
                "A1/NOPE",
                "toolhead.offset.x",
                Some(serde_json::json!(1)),
            ),
            set(
                Level::Machine,
                "KOBRA",
                "toolhead.offset.x",
                Some(serde_json::json!(1)),
            ),
        ] {
            let mut draft = Draft::default();
            let good = set(
                Level::Machine,
                "A1",
                "toolhead.offset.x",
                Some(serde_json::json!(5)),
            );
            let err = apply(&mut draft, &c, &reg, &[good, bad.clone()]).unwrap_err();
            assert!(
                matches!(
                    err.code,
                    crate::error::ErrorCode::NotFound | crate::error::ErrorCode::InvalidArgument
                ),
                "{bad:?} 应该被拒，实际 {:?}",
                err.code
            );
            assert_eq!(draft.dirty_count(), 0, "整批拒绝就不许留下前半截");
        }
    }

    /// 弃用写闸（C14 §五）：**写值的唯一出口就是 patch 校验**，所以闸设在这里 ——
    /// 弃用的参数写不进去、弃用的选项档写不进去；`null`（挂回继承）不拦，
    /// 删键不是写值。整批拒绝，草稿一个字节不动。
    #[test]
    fn deprecated_writes_are_refused_at_the_only_write_gate() {
        let d = tempfile::tempdir().unwrap();
        let params = serde_json::json!({
            "params": [
                {
                    "key": "toolhead.legacy", "configKey": "Legacy", "tomlKey": "legacy",
                    "jsonKey": "legacy", "label": "旧版擦料计数", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 7,
                    "scope": "universal", "section": "toolhead",
                    "layout": { "order": 1, "sectionId": "s1" },
                    "deprecated": true
                },
                {
                    "key": "wiping.mode", "configKey": "Mode", "tomlKey": "mode",
                    "jsonKey": "mode", "label": "擦拭部件", "desc": "", "tomlComment": "",
                    "valueType": "string", "uiComponent": "segmented", "defaultValue": "tower",
                    "scope": "universal", "section": "toolhead",
                    "layout": { "order": 2, "sectionId": "s1" },
                    "choices": [
                        { "label": "擦料塔", "value": "tower" },
                        { "label": "圆盘擦拭", "value": "disk" }
                    ]
                },
                {
                    "key": "wiping.disk_only", "configKey": "D", "tomlKey": "d", "jsonKey": "d",
                    "label": "圆盘专属", "desc": "", "tomlComment": "",
                    "valueType": "float", "uiComponent": "number", "defaultValue": 0,
                    "scope": "universal", "section": "toolhead",
                    "layout": { "order": 3, "sectionId": "s1" },
                    "deprecated": true,
                    "showWhen": { "key": "wiping.mode", "op": "eq", "value": "disk" }
                }
            ],
            "tabs": [{ "id": "t1", "label": "偏移", "order": 10,
                       "sections": [{ "id": "s1", "label": "空间偏移", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "toolhead.legacy" },
                { "id": "i1", "paramKey": "wiping.mode" },
                { "id": "i2", "paramKey": "wiping.disk_only" }
            ] }] }]
        });
        let reg =
            crate::workbench::presets::registry::load_from_json_fixture(d.path(), &params, &layout)
                .unwrap();
        let c = committed();

        // ① 弃用的参数：写值被拒，草稿不动
        let mut draft = Draft::default();
        let err = apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "A1",
                "toolhead.legacy",
                Some(serde_json::json!(3)),
            )],
        )
        .unwrap_err();
        assert!(matches!(err.code, crate::error::ErrorCode::InvalidArgument));
        assert!(err.message.contains("旧版擦料计数"), "{}", err.message);
        assert!(err.message.contains("已弃用"), "{}", err.message);
        assert_eq!(draft.dirty_count(), 0);

        // ② 弃用的选项档：参数没弃用，但「圆盘擦拭」放开的参数全弃用 → 写它被拒
        let mut draft = Draft::default();
        let err = apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "A1",
                "wiping.mode",
                Some(serde_json::json!("disk")),
            )],
        )
        .unwrap_err();
        assert!(err.message.contains("「圆盘擦拭」"), "{}", err.message);
        assert_eq!(draft.dirty_count(), 0);

        // ③ 没弃用的档照旧能写
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "A1",
                "wiping.mode",
                Some(serde_json::json!("tower")),
            )],
        )
        .unwrap();

        // ④ 挂回继承（null）不拦：删键不是写值 —— 老配方里钉着的弃用值也得有退路
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[
                set(
                    Level::Machine,
                    "A1",
                    "toolhead.legacy",
                    Some(serde_json::json!(3)),
                ),
                set(Level::Machine, "A1", "wiping.mode", None),
            ],
        )
        .expect_err("第一批里有弃用写值，整批照旧要拒");
    }

    /// 弃用**按机型**：只点名的机型被拦，没点名的照写；
    /// 版本层先把 uid 换成机型再判（别把「A1 上退场」当成「所有机型都退场」）
    #[test]
    fn deprecated_machines_block_only_the_named_machines() {
        let d = tempfile::tempdir().unwrap();
        let params = serde_json::json!({
            "params": [{
                "key": "wiping.mode", "configKey": "Mode", "tomlKey": "mode",
                "jsonKey": "mode", "label": "擦拭部件", "desc": "", "tomlComment": "",
                "valueType": "string", "uiComponent": "segmented", "defaultValue": "tower",
                "scope": "universal", "section": "toolhead",
                "layout": { "order": 1, "sectionId": "s1" },
                "deprecatedMachines": "A1",
                "choices": [
                    { "label": "擦料塔", "value": "tower" },
                    { "label": "圆盘擦拭", "value": "disk", "deprecatedMachines": "A1" }
                ]
            }],
            "tabs": [{ "id": "t1", "label": "偏移", "order": 10,
                       "sections": [{ "id": "s1", "label": "空间偏移", "order": 0 }] }],
            "updated": "2026-01-01 00:00:00"
        });
        let layout = serde_json::json!({
            "tabs": [{ "id": "t1", "sections": [{ "id": "s1", "items": [
                { "id": "i0", "paramKey": "wiping.mode" }
            ] }] }]
        });
        let reg =
            crate::workbench::presets::registry::load_from_json_fixture(d.path(), &params, &layout)
                .unwrap();
        let c = committed();

        // A1 机型层：这条在 A1 上点名弃用 → 拒，且这句话里要念得出机型
        let mut draft = Draft::default();
        let err = apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "A1",
                "wiping.mode",
                Some(serde_json::json!("tower")),
            )],
        )
        .unwrap_err();
        assert!(err.message.contains("已弃用"), "{}", err.message);
        assert!(
            err.detail.as_deref().is_some_and(|d| d.contains("A1")),
            "错句里没说是哪台机型：{:?}",
            err.detail
        );
        assert_eq!(draft.dirty_count(), 0, "整批拒绝就不许留下前半截");

        // P1S 机型层：没点名 → 照写；连「圆盘擦拭」这一档也能写
        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "P1S",
                "wiping.mode",
                Some(serde_json::json!("disk")),
            )],
        )
        .expect("P1S 上这一项还能写");

        // 版本层：A1/STANDARD 的 uid 要先换成机型 A1，判据才落得上
        let mut draft = Draft::default();
        let err = apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Version,
                "A1/STANDARD",
                "wiping.mode",
                Some(serde_json::json!("disk")),
            )],
        )
        .unwrap_err();
        assert!(err.message.contains("已弃用"), "{}", err.message);
        assert_eq!(draft.dirty_count(), 0);
    }

    /// **apply 反向能回到原状**：机型层与版本层的值、脏计数都回。
    ///
    /// 以前这一条还要回结构与身份（改名 / 归档），那些 layer 之外的东西现在不在这一层了
    #[test]
    fn applying_the_inverse_returns_to_the_original_state() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let forward = vec![
            set(
                Level::Machine,
                "A1",
                "toolhead.offset.x",
                Some(serde_json::json!(5)),
            ),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(7)),
            ),
            set(Level::Version, "A1/STANDARD", "toolhead.offset.z", None),
        ];
        let out = apply(&mut draft, &c, &reg, &forward).unwrap();
        assert!(draft.dirty_count() > 0);

        apply(&mut draft, &c, &reg, &out.inverse).unwrap();
        assert_eq!(draft.dirty_count(), 0, "脏计数要回到 0");
        assert!(
            draft.values.is_empty(),
            "值要回到没改过：{:?}",
            draft.values
        );
    }

    /// 反向要**倒着**执行才对：同一个键连改两次，反向顺序错了就回不去
    #[test]
    fn the_inverse_is_ordered_back_to_front() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[
                set(
                    Level::Machine,
                    "A1",
                    "toolhead.offset.x",
                    Some(serde_json::json!(1)),
                ),
                set(
                    Level::Machine,
                    "A1",
                    "toolhead.offset.x",
                    Some(serde_json::json!(2)),
                ),
            ],
        )
        .unwrap();

        // 正向 无→1→2，反向必须是 2→1、1→无
        assert_eq!(
            out.inverse,
            vec![
                set(
                    Level::Machine,
                    "A1",
                    "toolhead.offset.x",
                    Some(serde_json::json!(1.0))
                ),
                set(Level::Machine, "A1", "toolhead.offset.x", None),
            ]
        );
        apply(&mut draft, &c, &reg, &out.inverse).unwrap();
        assert_eq!(draft.dirty_count(), 0);
    }

    /// 草稿里指向已不存在的版本的条目要被丢掉，**别的改动照样留着**
    #[test]
    fn pruning_drops_only_the_entries_that_point_at_the_missing_version() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &reg,
            &[set(
                Level::Machine,
                "A1",
                "toolhead.offset.x",
                Some(serde_json::json!(5)),
            )],
        )
        .unwrap();
        // 手工塞一条指向已消失版本的草稿（模拟上游删掉了那个版本）
        draft.values.insert(
            "v:A1/GONE:toolhead.offset.x".to_owned(),
            Some(serde_json::json!(1)),
        );
        draft.values.insert("乱码键".to_owned(), None);
        assert_eq!(draft.dirty_count(), 3);

        let notices = draft.prune(&c);
        assert_eq!(draft.dirty_count(), 1, "只该剩 A1 基底那一处");
        assert!(draft.values.contains_key("m:A1:toolhead.offset.x"));
        assert_eq!(notices.len(), 2, "消失的版本与解析不出的键各一条提示");
    }

    /// 值键三段能来回互转。**版本 id 带点**（`FASTV3.3`）这件事要真被覆盖到
    #[test]
    fn value_keys_round_trip_including_dotted_version_ids() {
        for (level, owner, key) in [
            (Level::Machine, "A1", "toolhead.offset.x"),
            (Level::Version, "A1/FASTV3.3", "wiping.glue_z_lift_height"),
        ] {
            let s = value_key(level, owner, key);
            assert_eq!(
                parse_value_key(&s),
                Some((level, owner, key)),
                "来回转不回去：{s}"
            );
        }
        assert!(parse_value_key("x:A1:k").is_none());
        assert!(parse_value_key("m:A1").is_none());
        assert!(parse_value_key("m::k").is_none());
    }

    /// 草稿里的 owner → `machineVariants` 的键：版本层那道斜杠要换成冒号。
    ///
    /// 这两处的形状只差一个字符，写错了不会报错 —— `machineVariants` 里会多出一个
    /// 永远查不到的键，而跨文件校验下一次加载才拦得住
    #[test]
    fn variant_key_turns_the_uid_into_a_machine_variant_key() {
        assert_eq!(variant_key(Level::Machine, "A1"), "A1");
        assert_eq!(variant_key(Level::Version, "A1/STANDARD"), "A1:STANDARD");
        assert_eq!(variant_key(Level::Version, "A1/FASTV3.3"), "A1:FASTV3.3");
    }
}
