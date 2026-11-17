//! **统一编辑事务**（2026-10-31，作者要求推翻「值走草稿 / 结构操作即时落盘」两套规则）。
//!
//! # 一句话
//!
//! 工作台是一个配置编辑器：**磁盘上的 `presets/` 是已保存状态，影子树是编辑态**。
//! 任何编辑（开关、文本、尺寸、禁区、增删机型/版本、改套餐、换资产）都只动影子树；
//! 点「保存」才把影子树与磁盘的差异一次提交，点「放弃」把影子树拉回磁盘原状。
//!
//! # 为什么是「影子树 + 文件差分」而不是「每种操作写一个 inverse」
//!
//! 上一版草稿层 `Draft` 是「值的最终覆盖表」，结构操作（新建/删除一个文件）**在模型里
//! 无处安放**，所以只能即时落盘。给二十多种操作各写一个 `apply` + `inverse`，等于把
//! 同一套写盘逻辑实现两遍，且反向量（删掉了什么）必须逐字段手写 —— 一处漏了就出现
//! 「撤销之后文件少了一行」。
//!
//! 这里的做法是**以文件字节为准**：
//!
//! ```text
//! 编辑前 snapshot(shadow) ──┐
//!                          ├─ 执行既有写盘代码（它写的是影子树）
//! 编辑后 snapshot(shadow) ──┘
//!                          └→ diff = Op { before, after }
//! ```
//!
//! - **可逆性是构造出来的**，不是手写的：`before` 就是反向；
//! - 新建文件 = `before: None`；删除文件 = `after: None` —— 结构操作天然可撤销；
//! - 撤销/重做不需要知道操作是什么，只需要回放字节；
//! - 落盘只有一条出口（[`commit`]），失败能整体回滚。
//!
//! 代价是每条 op 携带改动文件的完整字节（参数台一次改值 = 56 KB 的 registry 两份）。
//! 人手编辑的频率下这是可接受的，换来的是「不会撤销错」。栈深有上限（[`MAX_OPS`]）。
//!
//! # 与既有骨架的关系
//!
//! `Ctx.presets` 仍然是唯一的内存文档面，只是它的根换成了影子树 —— 于是**所有既有写盘
//! 代码一行不改**：它们写 `Machine::file()` / `Registry::file()`，那些路径本来就来自
//! 「加载时那个根」。换根即换世界。
//!
//! # 崩溃
//!
//! 影子树本身在盘上（`workbench/.draft/presets/`），进程挂了它还在。重开时如果影子树与
//! 磁盘不一致，会以**一条合成的 op**（[`Txn::recovered`]）呈现：能保存、能放弃，
//! 但没有逐条撤销（那些 op 的字节没持久化）。这是刻意的：宁可说「有一次未保存的改动」，
//! 也不假装能撤销一个已经不在内存里的操作。

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write;
use crate::workbench::clock;

/// 撤销栈上限。超了从头丢 —— 与「文件字节」这个代价对齐
pub const MAX_OPS: usize = 200;

/// 一个文件在一次编辑前后的样子（只给界面看，不带字节）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    /// 相对**预设根**的路径（永远用 `/`，跨平台一致）
    pub rel: String,
    /// 编辑前的大小。`None` = 当时这个文件不存在
    #[serde(skip_serializing_if = "Option::is_none")]
    pub before: Option<usize>,
    /// 编辑后的大小。`None` = 这个文件被删掉了
    #[serde(skip_serializing_if = "Option::is_none")]
    pub after: Option<usize>,
}

/// 一条可撤销的编辑（界面视图）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Op {
    pub label: String,
    pub at: String,
    pub files: Vec<FileChange>,
}

/// 一个文件的「改前 / 改后」字节。两侧 `None` 同时成立是不可能的（那说明没变）
type BeforeAfter = (String, Option<Vec<u8>>, Option<Vec<u8>>);

/// **合并窗口**：同一格在这段时间里的连续编辑并成一条（2026-11-02 作者定案）。
///
/// 作者原话：「就像那个撤销堆栈一样，他只记一次……5 秒内的算一条」。
/// 连点步进器改涂胶速度 `65 → 64 → … → 50`，是**改了一次**，不是改了十几次 ——
/// 栈里只留一条，撤一次就回到最初那个值。
pub const MERGE_MS: u64 = 5_000;

/// 一次编辑作用在**哪一格**（只有值编辑有；结构编辑给 `None` ⇒ 永不合并）。
///
/// 三样一起才构成"同一格"：层（机型基底 / 某个版本）+ 归属（机型 id / 版本 uid）+ 参数键。
/// 换个版本改同一个参数是**另一格**，不该合进上一条。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditKey {
    pub level: crate::workbench::domain::Level,
    pub owner: String,
    pub key: String,
}

/// 这次写入**为什么**发生 —— 从日志层 [`crate::workbench::app::logging::Why`] 搬过来？
/// 不，日志层要用它的 `suffix()` / `tag()`，这里只借它的语义标签。
///
/// 直接复用 [`crate::workbench::app::logging::Why`] 会让 `domain` 依赖 `app`
/// （分层是反的）。所以这里**只要一个字符串标签**：产生方（`app` 层）按自己的
/// 枚举填 `tag()`，`domain` 只管把它带下去。
pub type WhyTag = &'static str;

/// **一个字段的变化** —— 「改了什么」列表里的一行，也是「未保存 N」数的那个 1。
///
/// # 为什么是"字段"而不是"文件"或"手势"（2026-11-03 作者定稿）
///
/// 作者：「我改了 A1 基底的 Z，也改了 A1 标准版的 Z，也改了 A1 快拆版的 Z，
/// 那就算三个改动，都显示出来」。而一次 `setValue` 会**归并连带**写出好几格
/// （日志里的"（归并连带）"）—— 所以这个集合必须由**写盘那一刻的事实**产出，
/// 不能由"用户点了哪一格"推断。
///
/// # 去重键 = `(层, 归属, 键)`
///
/// 不同归属**不许合并**：`A1`、`A1:STANDARD`、`A1:FAST` 是三处改动，不是一处。
/// 同一格连续改两次（`0 → 0.05 → 0.1`）只留一条，`before` 保最初、`after` 换最新
/// —— 页头那个数不随操作次数膨胀。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldEntry {
    /// 层（出厂 / 机型 / 版本）。结构编辑没有"层"（改的是定义本身）⇒ `None`
    pub level: Option<String>,
    /// 归属：`A1` / `A1:STANDARD`（机型 id 或版本 uid）
    pub owner: String,
    /// 参数键；结构编辑填字段路径（`label` / `bedSize.depth`）
    pub key: String,
    /// 给人看的名字（参数中文名 / 字段中文名）。查不到时等于 `key`
    pub label: String,
    /// 改前（已格式化）。`None` = 原本没有这一行
    pub before: Option<String>,
    /// 改后（已格式化）。`None` = 这一行被删掉
    pub after: Option<String>,
    /// 来源标签：`user` / `promoted` / `copy` / `empty` / `unknown` / `structural`
    pub why: WhyTag,
}

impl FieldEntry {
    /// 这两条是不是**同一格**（`(层, 归属, 键)` 相同）。**不同归属不许合并** —— 见类型注。
    ///
    /// `pub(crate)` 而不是私有：编辑历史（`domain::history`）用**同一个去重口径**
    /// 数「未保存 N」，两套口径就是两个数。
    pub(crate) fn same_cell(&self, other: &Self) -> bool {
        self.level == other.level && self.owner == other.owner && self.key == other.key
    }

    /// 去重键：`(层, 归属, 键)`。**不同归属不许合并** —— 见类型注。
    fn dedup_key(&self) -> (String, String, String) {
        (
            self.level.clone().unwrap_or_default(),
            self.owner.clone(),
            self.key.clone(),
        )
    }

    /// 真的变了没有（`before == after` 的条目不算改动）
    pub(crate) fn changed(&self) -> bool {
        self.before != self.after
    }
}

/// 一条编辑的**完整载荷**：前后字节都在，撤销/重做靠它
#[derive(Debug, Clone)]
struct Payload {
    label: String,
    at: String,
    files: Vec<BeforeAfter>,
    /// 同一格的连续编辑会**合并进这一条**（见 [`MERGE_MS`]）；`None` = 结构编辑，永不合并
    key: Option<EditKey>,
    /// 合并窗口的计时锚点。`Instant` 而不是系统时钟 —— 改系统时间不该让撤销栈错乱
    last_at: std::time::Instant,
    /// **这一笔真的改了哪些字段**（去重键 = `层, 归属, 键`）。
    ///
    /// 用 `BTreeMap` 而不是 `Vec`：合并语义直接由 `insert` / `entry().or_insert()`
    /// 天然实现 —— 同一格连续改两次只留一条，`before` 保最初、`after` 换最新。
    ///
    /// 这是「未保存 N」与「改了什么」的**唯一数据源**（作者 2026-11-03 定稿：
    /// 「让事务本身成为唯一的数据源」）。不再有 `cells` / `lines` 那两套并存的口径。
    fields: BTreeMap<(String, String, String), FieldEntry>,
}

impl Payload {
    /// **这一笔碰过哪些「归属」**（2026-11-14，撤销作用域用）。
    ///
    /// 与 `history::owners_of` 同一套来源：字段账里的 `owner` ∪ 文件路径反推。
    /// 空 = 说不清（老的合成 op），调用方**不拿它拦**。
    fn owners(&self) -> Vec<String> {
        let mut out: Vec<String> = self.fields.values().map(|f| f.owner.clone()).collect();
        out.extend(
            self.files
                .iter()
                .filter_map(|(rel, _, _)| super::scopes::owner_of_rel(rel)),
        );
        out.sort();
        out.dedup();
        out
    }

    /// 把这一笔并进自己：**`before` 保留最初那份**，`after` 换成最新那份。
    ///
    /// 于是「撤一次」回的是"我开始动手之前"，而不是"上一步"。这也正是合并的意义 ——
    /// 连点十下之后按一下撤销，人期待的是回到原值，不是退一格。
    fn absorb(&mut self, files: Vec<BeforeAfter>, label: String, fields: Vec<FieldEntry>) {
        for (rel, _, after) in files {
            match self.files.iter_mut().find(|(r, _, _)| *r == rel) {
                /* 同一份文件：只换 `after`，`before` 一个字都不动 */
                Some(slot) => slot.2 = after,
                None => self.files.push((rel, None, after)),
            }
        }
        self.merge_fields(fields);
        /* ★ **改回原值 ⇒ 这一份不差了，别留在栈里**（2026-11-02）。
        没有这一句，`70 → 80 → 70` 之后这条 op 的 `before` 与 `after` 是同一份字节，
        ⇒ 界面上「未保存 (1)」永远挂着，点开「查看改动」却说「影子与盘上一模一样」。
        两个数从此对不上。

        `merge_fields` 里已经做过净零剔除，这里再剔一次是为了 `files` 本身
        （字段账与文件字节是两本账，一边归零不代表另一边也归零 —— 例如只改注释）。 */
        self.files.retain(|(_, before, after)| before != after);
        /* 标签取**最新**那一笔的：连点过程中人看的最后一格才是他印象里改的东西 */
        self.label = label;
        self.last_at = std::time::Instant::now();
    }

    /// 并进一批字段变化：**`before` 保最初、`after` 换最新**，净零的剔掉。
    ///
    /// 这就是作者要的「同一格连续修改两次只显示最终变化」：
    /// `0 → 0.05 → 0.1` 并完之后是 `0 → 0.1`，页头那个数不随操作次数膨胀。
    fn merge_fields(&mut self, incoming: Vec<FieldEntry>) {
        for e in incoming {
            match self.fields.entry(e.dedup_key()) {
                std::collections::btree_map::Entry::Occupied(mut slot) => {
                    /* `before` 一个字都不动（那是"我开始动手之前"），只换 `after`
                    与来源标签（最后一次写入的原因才是这一笔现在的样子） */
                    let cur = slot.get_mut();
                    cur.after = e.after;
                    cur.why = e.why;
                    cur.label = e.label;
                    /*
                     * ★ 2026-11-03：净零时**不删，只记下"回到原点了"**。
                     *
                     * 以前这里 `slot.remove()`。可 `3.9 → 3.95`（脏）再来一笔
                     * `3.95 → 3.9`（净零）之后如果又改回去，条目重新插入时
                     * `before` 变成第二次的那个 `3.95` —— 屏幕上显示
                     * `3.95 → 3.8`，而人记得自己是从 `3.9` 开始的。
                     *
                     * 现在保留原文：`before` 始终是**这一格在事务开始时**的值，
                     * 净零的条目由 `field_changes()` 的 `retain` 过滤掉，
                     * 不进页头那个数。
                     */
                }
                std::collections::btree_map::Entry::Vacant(slot) => {
                    slot.insert(e);
                }
            }
        }
    }
}

impl Payload {
    fn view(&self) -> Op {
        Op {
            label: self.label.clone(),
            at: self.at.clone(),
            files: self
                .files
                .iter()
                .map(|(rel, b, a)| FileChange {
                    rel: rel.clone(),
                    before: b.as_ref().map(Vec::len),
                    after: a.as_ref().map(Vec::len),
                })
                .collect(),
        }
    }
}

/// 快照里一份文件的样子。
///
/// 文本类（`toml`/`json`/`md`/`txt`）直接存字节 —— 总共两百来 KB，比算哈希省事，也不会
/// 因为同一秒内两次编辑而误判。二进制（webp/png 那批，5 MB 量级）只记 `(len, mtime)`：
/// 资产替换是整体换文件，长度或时间必有一项变。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Entry {
    Bytes(Vec<u8>),
    /// 二进制：长度 + 头部指纹。
    ///
    /// **不能用 mtime**：影子树是复制出来的，复制不保留 mtime，于是「刚铺完影子树
    /// 就判脏」。指纹取前 4 KB 的摘要 —— 图片替换是整份换掉，头部必然变。
    Stamp {
        len: u64,
        head: u64,
    },
}

/// 前 4 KB 的稳定指纹（FNV-1a）。只用来判「这份二进制换过没有」，不做安全用途
fn head_fingerprint(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for b in &bytes[..bytes.len().min(4096)] {
        hash ^= u64::from(*b);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

/// 影子 / 真源 / 基准 三方对账的结果（见 [`Snapshot::reconcile`]）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShadowVerdict {
    /// **用户的未保存草稿**（影子相对基准改了、真源没改）
    pub drafts: Vec<String>,
    /// **外部修改**（真源相对基准改了、影子没改）—— 不算草稿，直接读真源即可
    pub external: Vec<String>,
    /// **冲突**（同一个文件两边都改过）—— 不能自动选边，必须让人定
    pub conflicts: Vec<String>,
}

impl ShadowVerdict {
    /// 有没有需要人处理的东西（有草稿或有冲突）。
    pub fn needs_attention(&self) -> bool {
        !self.drafts.is_empty() || !self.conflicts.is_empty()
    }

    /// 能不能安全地按真源重建（没有草稿、没有冲突）。
    ///
    /// `true` 时影子最多是"旧拷贝"，对齐到真源不丢任何人的东西。
    pub fn safe_to_reset(&self) -> bool {
        !self.needs_attention()
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    map: BTreeMap<String, Entry>,
}

impl Snapshot {
    /// **去掉生成产物**（`delivery/`）。
    ///
    /// 影子与真仓库的"是不是同一份"只该看**源**：`delivery/` 每次生成都会变，
    /// 拿它去判"外面改过源"会让横幅在每次生成之后复活（作者 2026-11-01：
    /// 「我点了重新加载，下次打开它还在」）。
    pub fn without_products(self) -> Self {
        Self {
            map: self
                .map
                .into_iter()
                .filter(|(p, _)| !p.starts_with("delivery/"))
                .collect(),
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 这份快照里有、而 `other` 里没有（或内容不同）的**源**文件数。
    ///
    /// 注意：**单靠"内容不同"分不出方向**。影子因为落后而不同、和因为用户编辑
    /// 而不同，这个函数都数成 1。要判"是不是用户的草稿"，用
    /// [`Self::has_drafts_over`]。
    ///
    /// `delivery/` 不参与（它是生成产物，每次生成都会变，不是用户的编辑）。
    pub fn edited_against(&self, other: &Snapshot) -> usize {
        self.map
            .iter()
            .filter(|(rel, _)| !rel.starts_with("delivery/"))
            .filter(|(rel, entry)| other.map.get(*rel) != Some(*entry))
            .count()
    }

    /// 一份文件相对另一份快照**变没变**（只看源，`delivery/` 不算）。
    ///
    /// "变了"包括：内容不同、一边有另一边没有。
    fn changed_between(a: &Snapshot, b: &Snapshot, rel: &str) -> bool {
        a.map.get(rel) != b.map.get(rel)
    }

    /// 所有**源**路径的并集（不看 `delivery/`）。
    fn source_paths_of(&self, other: &Snapshot) -> BTreeSet<String> {
        self.map
            .keys()
            .chain(other.map.keys())
            .filter(|rel| !rel.starts_with("delivery/"))
            .cloned()
            .collect()
    }

    /// 与 `other` 的**源**文件并集（不看 `delivery/`）。
    ///
    /// 基准缺失时用它兜底：把所有可能的路径都当成"要看的"，宁可多问一句。
    pub fn source_paths_union(&self, other: &Snapshot) -> BTreeSet<String> {
        self.source_paths_of(other)
    }

    /// 两份快照的**源**内容是否逐字节相同（不看 `delivery/`）。
    ///
    /// 这是"能安全重建基准"的唯一证明：影子与真源完全一致 ⇒ 影子没有任何独有内容
    /// ⇒ 重建基准不会抹掉任何人的改动。只要有一处不同就不能下这个结论 ——
    /// 不同可能是"用户改了没存"，也可能是"外部改了真源"，靠字节分不出方向，
    /// 而基准的作用恰恰是分出方向，缺了基准就无法证明。
    pub fn identical_sources(&self, other: &Snapshot) -> bool {
        let mine: BTreeSet<&String> = self
            .map
            .keys()
            .filter(|r| !r.starts_with("delivery/"))
            .collect();
        let theirs: BTreeSet<&String> = other
            .map
            .keys()
            .filter(|r| !r.starts_with("delivery/"))
            .collect();
        mine == theirs
            && mine
                .iter()
                .all(|rel| self.map.get(*rel) == other.map.get(*rel))
    }

    /// **影子 / 真源 / 基准 三方对账**：谁改了什么，有没有撞车。
    ///
    /// 这是"有没有用户的草稿"的**完整**判据（2026-11-01 第二轮）。
    /// 上一轮只用"影子独有文件"，认不出"用户改了既有文件"——
    /// 真源也被外部改过时，用户的改动会被静默丢掉。
    pub fn reconcile(&self, real: &Snapshot, base: &Snapshot) -> ShadowVerdict {
        let mut verdict = ShadowVerdict::default();
        for rel in self.source_paths_of(real) {
            let shadow_changed = Self::changed_between(self, base, rel.as_str());
            let real_changed = Self::changed_between(real, base, rel.as_str());
            match (shadow_changed, real_changed) {
                // 只有影子动了 → 用户的未保存草稿
                (true, false) => verdict.drafts.push(rel),
                // 只有真源动了 → 外部修改，影子是旧拷贝，不是草稿
                (false, true) => verdict.external.push(rel),
                // 两边都动过 → **冲突**，不许自动选边
                (true, true) => verdict.conflicts.push(rel),
                (false, false) => {}
            }
        }
        verdict
    }

    /// **`self` 里有没有 `base`（真源）里不存在的编辑** —— 即"用户还没保存的草稿"。
    ///
    /// 为什么不能直接用 [`Self::edited_against`]（2026-11-01 踩过）：
    ///
    /// ```text
    /// 真源 A1 = 0.85（AI 刚改的）   影子 A1 = 0.8（旧拷贝）
    /// edited_against → 1             ← 但这不是用户的草稿！
    /// has_drafts_over → false        ← 这才是事实
    /// ```
    ///
    /// 区别在**方向**：影子落后（外部改了真源）还是影子超前（用户改了没存）。
    ///
    /// 判法：**影子里的内容，在真源里找不到** ⇒ 那是影子里独有的东西 = 用户的编辑。
    /// 影子只是旧版时，它的每一份内容在真源的历史里也都有过 —— 但这里没有历史，
    /// 所以退一步用一条更保守、也更实用的判据：
    ///
    /// · 真源里**没有这个文件**，影子里有 → 草稿（用户新建的）
    /// · 两边都有但内容不同 → **看哪边"更新"**做不到（没有时间轴），
    ///   所以交给调用方：`Ctx::open` 用"影子是否含有真源没有的文件"这一条，
    ///   加上 `txn` 的 dirty 计数（真正在本次会话里改过的）来判断。
    ///
    /// 这里只回答最确定的那一问：**影子里有没有真源完全不知道的文件**。
    pub fn has_files_absent_from(&self, base: &Snapshot) -> Vec<String> {
        self.map
            .keys()
            .filter(|rel| !rel.starts_with("delivery/"))
            .filter(|rel| !base.map.contains_key(*rel))
            .cloned()
            .collect()
    }
}

fn is_text(rel: &str) -> bool {
    matches!(
        rel.rsplit('.').next().unwrap_or_default(),
        "toml" | "json" | "md" | "txt"
    )
}

fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) -> Result<(), AppError> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(
                AppError::io(format!("读不出目录：{}", dir.display())).with_detail(e.to_string())
            )
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        if meta.is_dir() {
            walk(&path, base, out)?;
        } else if meta.is_file() {
            let rel = path
                .strip_prefix(base)
                .map_err(|_| AppError::internal("影子树的路径不在根下".to_owned()))?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push(rel);
        }
    }
    Ok(())
}

/// 相对路径的防穿越：事务里的路径只许在根里。
fn safe_rel(rel: &str) -> Result<&str, AppError> {
    let p = Path::new(rel);
    if p.is_absolute()
        || p.components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(AppError::permission_denied(format!(
            "事务里的路径不许越界：{rel}"
        )));
    }
    Ok(rel)
}

/// 拍一份快照。`root` 是影子树根（或磁盘根）。
pub fn snapshot(root: &Path) -> Result<Snapshot, AppError> {
    let mut rels = Vec::new();
    walk(root, root, &mut rels)?;
    let mut map = BTreeMap::new();
    for rel in rels {
        let path = root.join(&rel);
        let Ok(meta) = std::fs::metadata(&path) else {
            continue;
        };
        let entry = if is_text(&rel) {
            match std::fs::read(&path) {
                Ok(bytes) => Entry::Bytes(bytes),
                Err(_) => continue,
            }
        } else {
            // 只读头部：资产文件 5 MB 量级，整份读进来做指纹不划算
            let head = std::fs::read(&path)
                .map(|b| head_fingerprint(&b))
                .unwrap_or(0);
            Entry::Stamp {
                len: meta.len(),
                head,
            }
        };
        map.insert(rel, entry);
    }
    Ok(Snapshot { map })
}

/// 两份快照是不是同一份（开场判「有没有上次遗留的改动」、保存前判「要不要写」）
pub fn equal(a: &Snapshot, b: &Snapshot) -> bool {
    a == b
}

/// 两份树里一份文件的关系（**给界面看的方向**，与撤销栈里的 before/after 不是一个东西）。
///
/// 口径固定成「左边那份有 / 右边那份有」：对 [`changed_files`] 来说
/// 左边 = 盘上的真源、右边 = 影子树，所以 `Added` = **保存会新建它**。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FileState {
    /// 右边有、左边没有
    Added,
    /// 两边都有、字节不同
    Modified,
    /// 左边有、右边没有
    Removed,
}

/// 两份快照之间**有变化的文件**（连同两侧字节）。
///
/// 与 [`diff`] 的差别只有一个：`diff(before, after, root)` 只用**一个**根读二进制，
/// 而这里两侧住在两棵不同的树（盘上 / 影子）。未保存改动的文本 diff 要它 ——
/// 见 [`super::shadow_diff`]。
///
/// `delivery/` 由调用方先用 [`Snapshot::without_products`] 滤掉，这里不重复判。
pub type ChangedFiles = Vec<(String, FileState, Option<Vec<u8>>, Option<Vec<u8>>)>;

pub fn changed_files(
    left: &Snapshot,
    right: &Snapshot,
    left_root: &Path,
    right_root: &Path,
) -> Result<ChangedFiles, AppError> {
    let mut rels: Vec<&String> = left.map.keys().chain(right.map.keys()).collect();
    rels.sort();
    rels.dedup();

    let mut out: ChangedFiles = Vec::new();
    for rel in rels {
        let l = left.map.get(rel);
        let r = right.map.get(rel);
        if l == r {
            continue;
        }
        let state = match (l, r) {
            (None, _) => FileState::Added,
            (_, None) => FileState::Removed,
            _ => FileState::Modified,
        };
        out.push((
            rel.clone(),
            state,
            read_side(l, left_root, rel)?,
            read_side(r, right_root, rel)?,
        ));
    }
    Ok(out)
}

/// 从某一侧读出这一份的字节（`None` = 那一侧没有这个文件）。
///
/// 二进制（`Entry::Stamp`）要落地去读 —— 快照里只有长度与头部指纹。
fn read_side(entry: Option<&Entry>, root: &Path, rel: &str) -> Result<Option<Vec<u8>>, AppError> {
    match entry {
        None => Ok(None),
        Some(Entry::Bytes(bytes)) => Ok(Some(bytes.clone())),
        Some(Entry::Stamp { .. }) => std::fs::read(root.join(rel))
            .map(Some)
            .map_err(|e| AppError::io(format!("读不出改动文件：{rel}")).with_detail(e.to_string())),
    }
}

/// 两次快照的差集。`root` 用来在**变化时**才把二进制读进来（没变的不读）。
type Changes = Vec<BeforeAfter>;

fn diff(before: &Snapshot, after: &Snapshot, root: &Path) -> Result<Changes, AppError> {
    let mut rels: Vec<&String> = before.map.keys().chain(after.map.keys()).collect();
    rels.sort();
    rels.dedup();

    let mut out = Vec::new();
    for rel in rels {
        let b = before.map.get(rel);
        let a = after.map.get(rel);
        if b == a {
            continue;
        }
        let read = |entry: Option<&Entry>| -> Result<Option<Vec<u8>>, AppError> {
            match entry {
                None => Ok(None),
                Some(Entry::Bytes(bytes)) => Ok(Some(bytes.clone())),
                Some(Entry::Stamp { .. }) => std::fs::read(root.join(rel)).map(Some).map_err(|e| {
                    AppError::io(format!("读不出改动文件：{rel}")).with_detail(e.to_string())
                }),
            }
        };
        out.push((rel.clone(), read(b)?, read(a)?));
    }
    Ok(out)
}

/// 把 `dst` 整份对齐到 `src`（放弃 / 对齐 / 换根都走这一条）。
///
/// # 为什么不再逐份复制（2026-11-02 改）
///
/// 老实现是「先删 `dst` 多出来的 → 再逐份 `remove_file` + `copy`」。它在两处不安全：
///
/// 1. **非原子**：中途失败 ⇒ `dst` 半新半旧，而调用方（放弃）没有第二份快照可回退；
/// 2. **`remove_file` 成功、`copy` 失败 ⇒ 那份文件直接没了**（删除错误还被 `let _ =` 吞掉）。
///
/// 而「放弃 = 把编辑树拉回基准」正是这个函数 —— 用户最需要可靠的一步，
/// 不该建在最不可靠的机制上。
///
/// # 四步：让旧目录整体让位，而不是一件件改
///
/// ```text
/// ① 整份拷到 <dst>.reset/        dst 一个字节没动
/// ② 校验 <dst>.reset/ ≡ src      （快照逐份比对，不只看长度）
/// ③ rename(<dst> → <dst>.old)    原子
/// ④ rename(<dst>.reset → <dst>)  原子
/// ⑤ 删 <dst>.old                 删不掉不算失败（下次启动回收）
/// ```
///
/// ①② 失败 ⇒ `dst` 还是原来那一份（只多一个 `.reset` 残片，见 [`recover_dir_replace`]）。
/// ③④ 是 `rename`，同卷内原子 ⇒ 「半新半旧」这个状态在磁盘上**观察不到**。
///
/// **不用硬链接**（2026-10-31 实测）：硬链接只有在「所有写都是 `atomic_write`」时才安全 ——
/// 任何一处 `std::fs::write` 都会顺着链接把真仓库改了。整份复制几十毫秒，换一条不会破的边界，值。
///
/// `src` / `dst` 必须是两个**平级**目录（真实调用都是 `presets/` ↔ `.draft/presets/`
/// ↔ `.draft/base/`）。一个是另一个的祖先时 rename 语义会变，这里显式拒绝。
pub fn mirror(src: &Path, dst: &Path) -> Result<(), AppError> {
    if dst.starts_with(src) || src.starts_with(dst) {
        return Err(AppError::internal(format!(
            "目录替换要求两个平级目录：{} / {}",
            src.display(),
            dst.display()
        )));
    }
    // 已经一致就一个字节都不写 —— 启动时 `settle_shadow` 会走这条，
    // 不能每次开工作台都整份拷一遍
    let same = matches!((snapshot(src), snapshot(dst)), (Ok(a), Ok(b)) if equal(&a, &b));
    if same {
        return Ok(());
    }

    let reset = sidecar(dst, ".reset")?;
    let old = sidecar(dst, ".old")?;

    // 上一轮失败留下的残片：dst 还在，说明那次替换没生效，残片无价值
    remove_dir_all_if_exists(&reset)?;

    // ① 整份拷（dst 一个字节没动）
    copy_tree(src, &reset)?;

    // ② 校验：拷坏了就不许往下走（此时 dst 完好，等于什么都没发生）
    let want = snapshot(src)?;
    let got = snapshot(&reset)?;
    if !equal(&want, &got) {
        let _ = std::fs::remove_dir_all(&reset);
        return Err(AppError::io(format!(
            "目录替换校验失败，已停在原状：{}",
            dst.display()
        )));
    }

    // ③ 旧的整体让开（先清上一轮留下的 .old）
    remove_dir_all_if_exists(&old)?;
    if dst.exists() {
        std::fs::rename(dst, &old).map_err(|e| {
            let _ = std::fs::remove_dir_all(&reset);
            AppError::io(format!("旧目录让位失败：{}", dst.display())).with_detail(e.to_string())
        })?;
    }

    // ④ 新的就位
    if let Err(e) = std::fs::rename(&reset, dst) {
        // 让位之后没换成 ⇒ 必须把旧的搬回来，否则 dst 就没了
        let _ = std::fs::rename(&old, dst);
        let _ = std::fs::remove_dir_all(&reset);
        return Err(
            AppError::io(format!("新目录就位失败：{}", dst.display())).with_detail(e.to_string())
        );
    }

    // ⑤ 回收旧的（失败不算错：`recover_dir_replace` 下次会清）
    let _ = std::fs::remove_dir_all(&old);
    Ok(())
}

/// `<dst>.reset` / `<dst>.old` 这种同级旁路名（必须与 `dst` **同卷**才谈得上 rename）。
fn sidecar(dst: &Path, suffix: &str) -> Result<PathBuf, AppError> {
    let parent = dst
        .parent()
        .ok_or_else(|| AppError::internal(format!("目录没有父级：{}", dst.display())))?;
    let name = dst
        .file_name()
        .ok_or_else(|| AppError::internal(format!("目录没有名字：{}", dst.display())))?
        .to_string_lossy()
        .into_owned();
    Ok(parent.join(format!("{name}{suffix}")))
}

fn copy_tree(src: &Path, dst: &Path) -> Result<(), AppError> {
    std::fs::create_dir_all(dst).map_err(|e| {
        AppError::io(format!("建不出目录：{}", dst.display())).with_detail(e.to_string())
    })?;
    let entries = match std::fs::read_dir(src) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => {
            return Err(
                AppError::io(format!("读不出目录：{}", src.display())).with_detail(e.to_string())
            )
        }
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(meta) = entry.metadata() else { continue };
        let to = dst.join(entry.file_name());
        if meta.is_dir() {
            copy_tree(&path, &to)?;
        } else if meta.is_file() {
            std::fs::copy(&path, &to).map_err(|e| {
                AppError::io(format!("复制失败：{}", to.display())).with_detail(e.to_string())
            })?;
        }
    }
    Ok(())
}

fn remove_dir_all_if_exists(path: &Path) -> Result<(), AppError> {
    match std::fs::remove_dir_all(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => {
            Err(AppError::io(format!("删不掉目录：{}", path.display())).with_detail(e.to_string()))
        }
    }
}

/* ---------- 保存的提交阶段（刀 5，2026-11-02） ---------- */

/// 一次保存要走四步，**不是一步**：
///
/// ```text
/// ① commit(WORK → presets/)        磁盘换成新的
/// ② BASE := WORK                    基准跟上
/// ③ 重读内存
/// ④ 清草稿 / 清事务
/// ```
///
/// 每份文件都是 `atomic_write`，但**四次原子不等于一次原子**：① 成功、② 之前崩，
/// 磁盘已经是新的、`BASE` 还是旧的 ⇒ 下次打开会判「有未保存草稿」，
/// 用户点「放弃」就把刚保存成功的东西退回去。
///
/// 所以在 `session.json` 里记阶段，启动时按阶段补完（见 [`recover_save_stage`]）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveStage {
    /// 没有正在进行的保存
    Idle,
    /// 已决定要提交，磁盘还没动 ⇒ 崩在这一步等于什么都没发生
    Prepared,
    /// `commit` 返回 Ok —— **磁盘已经是新的，基准还没搬**
    RealCommitted,
    /// 基准也搬完了，只差清内存
    BaselineAdvanced,
}

impl SaveStage {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Prepared => "prepared",
            Self::RealCommitted => "real_committed",
            Self::BaselineAdvanced => "baseline_advanced",
        }
    }

    fn parse(s: &str) -> Self {
        match s {
            "prepared" => Self::Prepared,
            "real_committed" => Self::RealCommitted,
            "baseline_advanced" => Self::BaselineAdvanced,
            _ => Self::Idle,
        }
    }
}

fn stage_path(wb: &Path) -> PathBuf {
    wb.join(".draft").join("session.json")
}

/// 会话账本。两个字段各管一件事，**不许互相顶替**：
///
/// | 字段 | 回答什么 | 谁写 |
/// |---|---|---|
/// | `stage` | 上次保存走到第几步（见 [`SaveStage`]） | 保存过程 / 启动补完 |
/// | `disk_stamp` | **打开会话时** `presets/` 是什么样 | 打开（无草稿时）/ 保存成功后 |
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionState {
    pub stage: SaveStage,
    /// 空 = 老账本 / 测试夹具没写 ⇒ 不做「外部改过」校验。
    pub disk_stamp: String,
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            stage: SaveStage::Idle,
            disk_stamp: String::new(),
        }
    }
}

/// 读会话账本。读不出 / 坏档 / 没这文件一律当空 —— 宁可少补一次，
/// 也不能因为一个记账文件把工作台卡在打不开的状态。
pub fn read_session(wb: &Path) -> SessionState {
    let Ok(text) = std::fs::read_to_string(stage_path(wb)) else {
        return SessionState::default();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else {
        return SessionState::default();
    };
    SessionState {
        stage: v
            .get("stage")
            .and_then(|x| x.as_str())
            .map(SaveStage::parse)
            .unwrap_or(SaveStage::Idle),
        disk_stamp: v
            .get("diskStamp")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_owned(),
    }
}

pub fn write_session(wb: &Path, s: &SessionState) -> Result<(), AppError> {
    let path = stage_path(wb);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            AppError::io(format!("建不出目录：{}", parent.display())).with_detail(e.to_string())
        })?;
    }
    let body = serde_json::json!({ "stage": s.stage.as_str(), "diskStamp": s.disk_stamp });
    atomic_write(&path, body.to_string().as_bytes())
}

pub fn read_save_stage(wb: &Path) -> SaveStage {
    read_session(wb).stage
}

/// 只改阶段，**保留 `disk_stamp`** —— 两个字段各说一件事，整份覆盖会把另一件抹掉。
pub fn write_save_stage(wb: &Path, stage: SaveStage) -> Result<(), AppError> {
    let mut s = read_session(wb);
    s.stage = stage;
    write_session(wb, &s)
}

/// 记下「这一刻 `presets/` 是什么样」。只有**打开会话（无草稿）**与**保存成功**
/// 该调 —— 编辑中途调，等于把外部改动洗白。
pub fn write_disk_stamp(wb: &Path, stamp: &str) -> Result<(), AppError> {
    let mut s = read_session(wb);
    s.disk_stamp = stamp.to_owned();
    write_session(wb, &s)
}

/// 启动时按阶段补完上一次没走完的保存。返回做了哪些动作（给日志用）。
///
/// | 读到 | 磁盘 | BASE | 动作 |
/// |---|---|---|---|
/// | `prepared` | 未动 | 旧 | 什么都不用补（零写入，安全），只清阶段 |
/// | `real_committed` | **已换** | 旧 | **补完** `mirror(real → base)` ⇒ 基准跟上 |
/// | `baseline_advanced` | 已换 | 新 | 只清阶段 |
///
/// 这条是「保存中断不会让已保存与基准分裂」的全部保证。
pub fn recover_save_stage(wb: &Path, real: &Path, base: &Path) -> Result<Vec<String>, AppError> {
    let mut log = Vec::new();
    match read_save_stage(wb) {
        SaveStage::Idle => {}
        SaveStage::Prepared => {
            log.push("上次保存停在提交之前（磁盘一个字节没动）".to_owned());
        }
        SaveStage::RealCommitted => {
            /* 磁盘已经是新的、基准还是旧的 ⇒ 补搬。幂等：再跑一次也是同一结果。 */
            mirror(real, base)?;
            log.push("补完上次保存的基准推进（磁盘已提交、基准未搬）".to_owned());
        }
        SaveStage::BaselineAdvanced => {
            log.push("上次保存已完成，只差清理".to_owned());
        }
    }
    if !log.is_empty() {
        write_save_stage(wb, SaveStage::Idle)?;
    }
    Ok(log)
}

/// 启动时收拾「上次目录替换没做完」的现场。返回做了哪些动作（给日志用，空 `Vec` = 很干净）。
///
/// 判据来自四步的**可观察中间态**：
///
/// | 磁盘上看到 | 崩在哪 | 动作 |
/// |---|---|---|
/// | `dst` + `.reset` | ①② | 删 `.reset`（编辑态完整，替换未生效） |
/// | `dst` + `.old` | ④之后⑤之前 | 删 `.old` ⇒ 完成 |
/// | `.reset` + `.old`，没有 `dst` | ③之后④之前 | `rename(.reset → dst)` ⇒ **完成替换** |
/// | 只有 `.old` | `dst` 被外部删了 | `rename(.old → dst)` ⇒ **回滚** |
/// | 只有 `.reset` | 理论上不会出现 | `rename(.reset → dst)` ⇒ 恢复 |
///
/// **为什么③④之间选「完成」而不是回滚**：`.reset` 已经校验过（与 `src` 逐份相同），
/// 完成是用户点下那个动作的本意；`.old` 里那一份仍留在盘上可人工捞回。
pub fn recover_dir_replace(dst: &Path) -> Result<Vec<String>, AppError> {
    let reset = sidecar(dst, ".reset")?;
    let old = sidecar(dst, ".old")?;
    let mut log = Vec::new();

    match (dst.exists(), reset.exists(), old.exists()) {
        // 崩在 ③④ 之间：旧的整体让开了、新的还没进来
        (false, true, true) => {
            std::fs::rename(&reset, dst).map_err(|e| {
                AppError::io(format!("完成上次替换失败：{}", dst.display()))
                    .with_detail(e.to_string())
            })?;
            let _ = std::fs::remove_dir_all(&old);
            log.push(format!("完成上次未做完的目录替换：{}", dst.display()));
        }
        // dst 被外部删了，只剩旧的一份 ⇒ 回滚
        (false, false, true) => {
            std::fs::rename(&old, dst).map_err(|e| {
                AppError::io(format!("回滚上次替换失败：{}", dst.display()))
                    .with_detail(e.to_string())
            })?;
            log.push(format!("回滚上次未完成的目录替换：{}", dst.display()));
        }
        // 只有 .reset（④ 已把 reset 换进来却又消失？保守：搬回来）
        (false, true, false) => {
            std::fs::rename(&reset, dst).map_err(|e| {
                AppError::io(format!("恢复目录失败：{}", dst.display())).with_detail(e.to_string())
            })?;
            log.push(format!("恢复目录：{}", dst.display()));
        }
        // 正常 / 崩在 ①② / 崩在 ④⑤ 之间
        (true, has_reset, has_old) => {
            if has_reset || has_old {
                log.push(format!("清理目录替换残片：{}", dst.display()));
            }
            remove_dir_all_if_exists(&reset)?;
            remove_dir_all_if_exists(&old)?;
        }
        (false, false, false) => {}
    }
    Ok(log)
}

fn prune_empty_parents(path: &Path, stop: &Path) {
    let mut cur = path.parent();
    while let Some(dir) = cur {
        if dir == stop || !dir.starts_with(stop) {
            return;
        }
        if std::fs::remove_dir(dir).is_err() {
            return;
        }
        cur = dir.parent();
    }
}

/// 把一组「字节状态」写进某个根（撤销/重做、回滚用）。`None` = 删掉这个文件。
pub fn apply_bytes(root: &Path, files: &[(String, Option<Vec<u8>>)]) -> Result<(), AppError> {
    for (rel, bytes) in files {
        safe_rel(rel)?;
        let path = root.join(rel);
        match bytes {
            None => {
                let _ = std::fs::remove_file(&path);
                prune_empty_parents(&path, root);
            }
            Some(data) => {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        AppError::io(format!("建不出目录：{}", parent.display()))
                            .with_detail(e.to_string())
                    })?;
                }
                atomic_write(&path, data)?;
            }
        }
    }
    Ok(())
}

/// 保存结果（给界面看的那一版）
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitReport {
    pub written: usize,
    pub deleted: usize,
    pub files: Vec<String>,
}

/// **唯一落盘出口**：影子树 → 真实 `presets/`。
///
/// 逐份原子写；删除的文件不直接抹掉，而是移进回收站 —— 「保存之后才发现删错了」还有救。
/// 任一文件写失败 → 把已经写过的按原字节回滚，**不留半份**。
pub fn commit(
    real: &Path,
    shadow: &Path,
    trash: &Path,
    stamp: &str,
) -> Result<CommitReport, AppError> {
    let a = snapshot(real)?;
    let b = snapshot(shadow)?;
    let changes = diff(&a, &b, shadow)?;

    let mut report = CommitReport::default();
    let mut undo: Vec<(String, Option<Vec<u8>>)> = Vec::new();

    for (rel, _before, after) in &changes {
        safe_rel(rel)?;
        let target = real.join(rel);
        let original = std::fs::read(&target).ok();
        match after {
            None => {
                if original.is_some() {
                    let keep = trash.join(stamp).join(rel);
                    if let Some(parent) = keep.parent() {
                        let _ = std::fs::create_dir_all(parent);
                    }
                    if std::fs::rename(&target, &keep).is_err() {
                        std::fs::copy(&target, &keep).map_err(|e| {
                            AppError::io(format!("回收站留底失败：{rel}"))
                                .with_detail(e.to_string())
                        })?;
                        let _ = std::fs::remove_file(&target);
                    }
                    report.deleted += 1;
                    report.files.push(rel.clone());
                }
            }
            Some(data) => {
                if let Some(parent) = target.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| {
                        AppError::io(format!("建不出目录：{}", parent.display()))
                            .with_detail(e.to_string())
                    })?;
                }
                if let Err(e) = atomic_write(&target, data) {
                    let _ = apply_bytes(real, &undo);
                    return Err(e);
                }
                report.written += 1;
                report.files.push(rel.clone());
            }
        }
        undo.push((rel.clone(), original));
    }

    report.files.sort();
    report.files.dedup();
    tracing::info!(
        written = report.written,
        deleted = report.deleted,
        "事务已提交"
    );
    Ok(report)
}

/// 事务本体。**只有内存里的 op 栈**，文件在影子树里。
#[derive(Debug, Default)]
pub struct Txn {
    payloads: Vec<Payload>,
    redo: Vec<Payload>,
    /// 崩溃恢复：影子树里有一次没有逐条 op 的改动（见模块头注）
    recovered: bool,
}

impl Txn {
    pub fn new() -> Self {
        Self::default()
    }

    /// 从「影子树与磁盘不一致」的开场进来：一条合成 op，不能逐条撤销
    pub fn recovered() -> Self {
        Self {
            payloads: Vec::new(),
            redo: Vec::new(),
            recovered: true,
        }
    }

    pub fn ops(&self) -> Vec<Op> {
        let mut out: Vec<Op> = self.payloads.iter().map(Payload::view).collect();
        if self.recovered {
            out.insert(
                0,
                Op {
                    label: "上次会话未保存的改动".to_owned(),
                    at: String::new(),
                    files: Vec::new(),
                },
            );
        }
        out
    }

    /// **撤销栈顶那一条**：标签 + 它改了几处字段（与 `history::History::top_op` 同一形状）。
    ///
    /// `recovered` 那条合成 op 没有逐条字段（它是"影子树里有一笔说不清的改动"），
    /// 记 0 处照实说不出来。
    pub fn undo_top(&self) -> Option<(String, usize, Vec<String>)> {
        if self.recovered {
            /* 合成 op：没有逐条字段 ⇒ 归属也说不清（空 = 不拦，见 `scopes` 头注） */
            return Some(("上次会话未保存的改动".to_owned(), 0, Vec::new()));
        }
        let p = self.payloads.last()?;
        Some((p.label.clone(), p.fields.len(), p.owners()))
    }

    /// 重做栈顶那一条（形状同 [`Self::undo_top`]）—— 重做同样过作用域闸。
    pub fn redo_top(&self) -> Option<(String, usize, Vec<String>)> {
        let p = self.redo.last()?;
        Some((p.label.clone(), p.fields.len(), p.owners()))
    }

    /// **唯一统计来源**：所有 payload 的字段变化并集（去重键 `层, 归属, 键`）。
    ///
    /// 作者 2026-11-03：「你要是做这个，就把事务本身当唯一数据源」——
    /// 页头那个 N、「改了什么」那张表，都读这一份，物理上不可能各说各话。
    ///
    /// # 只收"现在仍与盘上不同"的 payload
    ///
    /// 合并只发生在栈顶，非栈顶那条仍可能是 `70 → 80 → 70` 之后净零的样子
    /// （`files` 已剔但字段账在另一条 payload 里）。数它会让「未保存 (1)」与
    /// 「查看改动：一模一样」同时出现在一屏上。
    pub fn field_changes(&self) -> Vec<FieldEntry> {
        let mut merged: BTreeMap<(String, String, String), FieldEntry> = BTreeMap::new();
        for p in self.payloads.iter() {
            /* ★ 2026-11-03 修：**不再按 payload 的字节差整条跳过**。
            以前这里写着 `if !p.files.any(before != after) { continue }` ——
            而 `files` 是**整份文件**的前后字节：改一格又改回来之后，`absorb`
            会把这一份从 `files` 里剔掉，于是**整条 payload 的字段全被跳过**。
            作者连点几下之后 `fields=3 → 2 → 1` 越改越少，就是这么来的
            （日志：`ops=4 fields=2`、`ops=5 fields=1`）。

            正确的过滤在**字段自己**身上：`changed()` 已经排除净零的条目，
            下面 `retain` 再兜一次。payload 有没有字节差与"它的字段还算不算数"
            是两件事。 */
            for (k, e) in &p.fields {
                match merged.get_mut(k) {
                    /* 同一格在**另一条 payload** 里又改了一次（两条之间隔了别的编辑，
                    或都不是单个 `SetValue` ⇒ 合并不生效）——`before` 保最初、
                    `after` 换最新，与 `Payload::merge_fields` 同一套语义。
                    只留 `or_insert` 的话，`0 → 0.05 → 0.1` 会停在 `0 → 0.05`。 */
                    Some(cur) => {
                        cur.after = e.after.clone();
                        cur.why = e.why;
                        cur.label = e.label.clone();
                    }
                    None => {
                        merged.insert(k.clone(), e.clone());
                    }
                }
            }
        }
        /* 净零的剔掉（跨 payload 合并之后才可能出现的 `0 → 0.05 → 0`） */
        merged.retain(|_, e| e.changed());
        merged.into_values().collect()
    }

    /// 未保存的改动**处数**（界面上那个 N）= [`Self::field_changes`] 的条数。
    ///
    /// 这就是「页头 N 与框里条数严格相等」的实现 —— 同一个函数，不是两套算法。
    pub fn dirty_count(&self) -> usize {
        self.field_changes().len() + usize::from(self.recovered)
    }

    /// 影子树与磁盘**有没有字节差别**（`dirty_count()` 答的是"改了几处字段"）。
    ///
    /// 两个问题必须分开：`dirty_count()` 是**显示用**（数得出来给人看，
    /// 没填字段账的命令不虚构处数）；`is_dirty()` 是**判定用**
    /// （能不能保存、要不要弹「未保存」）—— 它只认字节，认字段账会把
    /// "改了但没记字段"的那一笔当成没改，保存按钮就点不动了。
    pub fn is_dirty(&self) -> bool {
        self.recovered
            || self
                .payloads
                .iter()
                .any(|p| p.files.iter().any(|(_, before, after)| before != after))
    }

    /// 能逐条撤销的深度（崩溃恢复那一条不算）
    pub fn undo_depth(&self) -> usize {
        self.payloads.len()
    }

    /// 栈顶那条的时刻（`None` = 空栈）—— 迁移期与 `domain::history` 的同名方法
    /// 一起决定"撤销该退哪条栈"，见那边的注。
    pub fn undo_last_at(&self) -> Option<std::time::Instant> {
        self.payloads.last().map(|p| p.last_at)
    }

    /// 重做栈顶那条的时刻
    pub fn redo_last_at(&self) -> Option<std::time::Instant> {
        self.redo.last().map(|p| p.last_at)
    }

    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    pub fn clear(&mut self) {
        self.payloads.clear();
        self.redo.clear();
        self.recovered = false;
    }

    /// 记一条编辑：调用方已经**在影子树上**执行完写盘，这里只收差集。
    /// 没有实际变化时返回 `None`（不占撤销栈）。
    ///
    /// # 合并（2026-11-02）
    ///
    /// `key: Some(k)` 且**栈顶同 `k`、距今不到 [`MERGE_MS`]** ⇒ 并进栈顶，不新增一条。
    /// 这是作者报的那一幕的解药：连点涂胶速度十几次，栈里只该有一条，
    /// 「未保存」也不该跟着涨十几次。
    ///
    /// 只判栈顶，不往下找：中途改过别的地方（`A1` 改完去改 `A1_MINI`，再回来改 `A1`）
    /// 就是**两次**编辑 —— 后一次必须能单独撤销，否则撤一次会把中间那个也带走。
    pub fn record(
        &mut self,
        label: impl Into<String>,
        key: Option<EditKey>,
        before: &Snapshot,
        after: &Snapshot,
        root: &Path,
    ) -> Result<Option<Op>, AppError> {
        self.record_with_fields(label, key, before, after, root, Vec::new())
    }

    /// 同上，另外收下**这一笔真的改了哪些字段**。
    ///
    /// 结构编辑（改说明 / 改尺寸 / 改机型信息）走这一条 —— 它们没有"格"，
    /// 但每一处字段变化都要进账，否则「改了什么」对那一类改动仍然是空的。
    ///
    /// `fields` 由**写盘那一刻的事实**产出（值类来自 `apply_draft_now_traced` 的
    /// `KeyChange`，结构类来自 `ParamDef::summary` / `Dimensions::summary` 的
    /// `FieldChange`），不是事后从影子树反推 —— 作者 2026-11-03 定的第 8 条硬约束。
    pub fn record_with_fields(
        &mut self,
        label: impl Into<String>,
        key: Option<EditKey>,
        before: &Snapshot,
        after: &Snapshot,
        root: &Path,
        fields: Vec<FieldEntry>,
    ) -> Result<Option<Op>, AppError> {
        let files = diff(before, after, root)?;
        if files.is_empty() {
            return Ok(None);
        }
        let label = label.into();

        /* ★ 合并：栈顶还是同一格、还在窗口里 ⇒ 只更新它 */
        if let (Some(k), Some(top)) = (&key, self.payloads.last_mut()) {
            if top.key.as_ref() == Some(k)
                && top.last_at.elapsed().as_millis() < u128::from(MERGE_MS)
            {
                top.absorb(files, label, fields);
                self.redo.clear();
                self.recovered = false;
                /* 合并完这一条已经**不差任何字节**（改回原值）⇒ 整条丢掉，
                否则它会以"一条改动"的身份一直留在栈里（见 `absorb` 的注释） */
                if top.files.is_empty() {
                    self.payloads.pop();
                    return Ok(None);
                }
                return Ok(Some(top.view()));
            }
        }

        let mut payload = Payload {
            label,
            at: clock::now_iso8601(),
            files,
            key,
            last_at: std::time::Instant::now(),
            fields: BTreeMap::new(),
        };
        payload.merge_fields(fields);
        let view = payload.view();
        self.payloads.push(payload);
        if self.payloads.len() > MAX_OPS {
            self.payloads.remove(0);
        }
        self.redo.clear();
        self.recovered = false;
        Ok(Some(view))
    }

    /// 栈里所有编辑碰过的文件（去重、排序）。**与 [`changed_files`] 口径分开**：
    /// 这个只回答"撤销栈碰过哪些文件"，不回答"现在差几份"（前者不会因撤销变少）。
    pub fn touched_files(&self) -> BTreeSet<String> {
        self.payloads
            .iter()
            .flat_map(|p| p.files.iter().map(|(rel, _, _)| rel.clone()))
            .collect()
    }

    /// 撤销一条：把 `before` 写回影子树。返回被撤销那条的标签
    pub fn undo(&mut self, shadow: &Path) -> Result<Option<String>, AppError> {
        if self.recovered {
            // 合成 op 没有字节：拒绝而不是假装撤销成功
            return Ok(None);
        }
        let Some(top) = self.payloads.pop() else {
            return Ok(None);
        };
        let back: Vec<(String, Option<Vec<u8>>)> = top
            .files
            .iter()
            .map(|(rel, before, _)| (rel.clone(), before.clone()))
            .collect();
        apply_bytes(shadow, &back)?;
        let label = top.label.clone();
        self.redo.push(top);
        Ok(Some(label))
    }

    /// 重做一条：把 `after` 写回影子树
    pub fn redo(&mut self, shadow: &Path) -> Result<Option<String>, AppError> {
        let Some(top) = self.redo.pop() else {
            return Ok(None);
        };
        let forward: Vec<(String, Option<Vec<u8>>)> = top
            .files
            .iter()
            .map(|(rel, _, after)| (rel.clone(), after.clone()))
            .collect();
        apply_bytes(shadow, &forward)?;
        let label = top.label.clone();
        self.payloads.push(top);
        Ok(Some(label))
    }

    /// 保存清单（给状态栏 / 保存确认框看一眼）
    pub fn labels(&self) -> Vec<String> {
        self.ops().into_iter().map(|o| o.label).collect()
    }

    /// 影子树与磁盘的差异文件数（保存时真会动的份数）
    pub fn pending_files(&self, real: &Path, shadow: &Path) -> Result<usize, AppError> {
        let a = snapshot(real)?;
        let b = snapshot(shadow)?;
        Ok(diff(&a, &b, shadow)?.len())
    }
}

/// 影子树在会话目录下的固定落点（`workbench/.draft/presets/`）
pub fn shadow_root(workbench_root: &Path) -> PathBuf {
    /* ★ 工作区**不跟数据根走**（2026-11-02 作者裁定）：注入了就落应用数据目录，
    仓库里只剩正式预设。没注入（`cargo test` / CLI）才退回数据根下的 `.draft`,
    测试因此照旧用临时目录。 */
    match crate::fsx::paths::workspace_root() {
        Some(w) => w.join("presets"),
        None => workbench_root.join(".draft").join("presets"),
    }
}

/// **基准快照**：影子树是"从哪一份真源铺出来的"。
///
/// 为什么必须有它（2026-11-01）：判"影子有没有用户的草稿"不能只看"影子里有没有
/// 真源没有的文件" —— 那只认得**新增**，认不出**改既有文件**：
///
/// ```text
/// 基准（铺影子时）  0.8
/// 影子（用户改）    0.9   ← 未保存的草稿
/// 真源（AI 改）    0.85  ← 外部修改
/// ```
///
/// 只看"影子独有文件"会得出"没草稿"，于是切回真源、用户的 0.9 就没了。
/// 有基准才能同时回答两个问题，并识别"两边都改了同一个文件"的冲突：
///
/// | 影子 vs 基准 | 真源 vs 基准 | 判定 |
/// |---|---|---|
/// | 变了 | 没变 | 用户的草稿 |
/// | 没变 | 变了 | 外部修改（直接读真源） |
/// | 变了 | 变了 | **冲突**，不能自动选边 |
/// | 都没变 | — | 干净 |
///
/// 它住 `<workbench>/.draft/base/`（与影子树同一族，不进 git）。
pub fn base_root(workbench_root: &Path) -> PathBuf {
    /* 与 `shadow_root` 同一条：工作区在应用数据目录，不跟数据根走 */
    match crate::fsx::paths::workspace_root() {
        Some(w) => w.join("base"),
        None => workbench_root.join(".draft").join("base"),
    }
}

#[cfg(test)]
// 测试要造夹具：`std::fs::write` 在这里是正当用法（写盘纪律管的是生产代码）
#[allow(clippy::disallowed_methods)]
mod tests {
    use super::*;

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// 造一条字段变化（生产链路由 `KeyChange` / `FieldChange` 转过来）
    fn field(owner: &str, key: &str, before: &str, after: &str) -> FieldEntry {
        FieldEntry {
            level: Some("Machine".to_owned()),
            owner: owner.to_owned(),
            key: key.to_owned(),
            label: key.to_owned(),
            before: Some(before.to_owned()).filter(|s| !s.is_empty()),
            after: Some(after.to_owned()).filter(|s| !s.is_empty()),
            why: "user",
        }
    }

    fn setup() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf) {
        let d = tempfile::tempdir().unwrap();
        let real = d.path().join("presets");
        let shadow = shadow_root(&d.path().join("workbench"));
        let trash = d.path().join("workbench").join(".trash");
        write(&real, "machines/A1.toml", "id = 'A1'\nname = 'A1'\n");
        write(&real, "bundles.toml", "bundles = []\n");
        std::fs::create_dir_all(&shadow).unwrap();
        mirror(&real, &shadow).unwrap();
        (d, real, shadow, trash)
    }

    /* ---------- 目录替换：故障注入（刀 2，2026-11-02） ----------
     *
     * 这一组钉的不是「能不能换」，而是**换到一半崩了会怎样**。
     * 老实现（逐份 remove_file + copy）在这里一条都过不了：
     * 它没有任何可观察的中间态，崩了就是半新半旧。
     */

    /// 正常路径：整份对齐 + 不留残片
    #[test]
    fn mirror_replaces_the_whole_directory_and_leaves_no_residue() {
        let (_d, real, shadow, _trash) = setup();
        write(&shadow, "only-in-shadow.toml", "x = 1\n");
        write(&shadow, "machines/A1.toml", "id = 'A1'\nname = '改过的'\n");
        std::fs::remove_file(shadow.join("bundles.toml")).unwrap();

        mirror(&real, &shadow).unwrap();

        assert!(
            equal(&snapshot(&shadow).unwrap(), &snapshot(&real).unwrap()),
            "整份对齐"
        );
        assert!(
            !shadow.join("only-in-shadow.toml").exists(),
            "多出来的要删掉"
        );
        assert!(shadow.join("bundles.toml").exists(), "缺的要补回");
        assert!(
            !sidecar(&shadow, ".reset").unwrap().exists(),
            "不许留 .reset 残片"
        );
        assert!(
            !sidecar(&shadow, ".old").unwrap().exists(),
            "不许留 .old 残片"
        );
    }

    /// 崩在 ①②（拷/校验阶段）：`.reset` 是半份、`dst` 完好 ⇒ 恢复后 dst 一个字节没变
    #[test]
    fn a_crash_before_the_swap_leaves_the_old_directory_intact() {
        let (_d, _real, shadow, _trash) = setup();
        let before = snapshot(&shadow).unwrap();
        let reset = sidecar(&shadow, ".reset").unwrap();
        write(&reset, "half.toml", "拷了一半\n");

        let log = recover_dir_replace(&shadow).unwrap();
        assert!(!log.is_empty(), "该报告清了一次残片");
        assert!(!reset.exists(), "残片要删掉");
        assert!(
            equal(&snapshot(&shadow).unwrap(), &before),
            "dst 必须一个字节没变 —— 替换未生效时用户的编辑态不能被动"
        );
    }

    /// 崩在 ③ 之后 ④ 之前：`dst` 不在，`.reset` + `.old` 都在 ⇒ 恢复后**完成替换**
    #[test]
    fn a_crash_between_the_two_renames_finishes_the_replacement() {
        let (_d, real, shadow, _trash) = setup();
        let reset = sidecar(&shadow, ".reset").unwrap();
        let old = sidecar(&shadow, ".old").unwrap();
        // 新版（= 真源）落在 .reset，旧编辑态落在 .old，dst 不存在 —— 正是 ③④ 之间的形状
        mirror(&real, &reset).unwrap();
        std::fs::rename(&shadow, &old).unwrap();

        let log = recover_dir_replace(&shadow).unwrap();
        assert!(
            log.iter().any(|l| l.contains("完成")),
            "该报告完成了一次替换：{log:?}"
        );
        assert!(shadow.exists(), "dst 必须回来");
        assert!(
            equal(&snapshot(&shadow).unwrap(), &snapshot(&real).unwrap()),
            "进来的是 .reset 那一份（已校验过）"
        );
        assert!(!old.exists(), ".old 要回收");
        assert!(!reset.exists(), ".reset 已经换进来了");
    }

    /// 崩在 ④ 之后 ⑤ 之前：`dst` 已是新版，`.old` 还在 ⇒ 只清 `.old`
    #[test]
    fn a_crash_after_the_swap_only_reclaims_the_old_copy() {
        let (_d, real, shadow, _trash) = setup();
        let old = sidecar(&shadow, ".old").unwrap();
        write(&old, "旧版.toml", "旧\n");

        let log = recover_dir_replace(&shadow).unwrap();
        assert!(!old.exists(), ".old 要回收");
        assert!(
            equal(&snapshot(&shadow).unwrap(), &snapshot(&real).unwrap()),
            "dst 不受影响"
        );
        assert!(log.iter().any(|l| l.contains("残片")), "{log:?}");
    }

    /// 只有 `.old`（dst 被外部删了）⇒ 回滚，**不许把编辑态丢掉**
    #[test]
    fn a_lost_directory_is_rolled_back_from_the_old_copy() {
        let (_d, _real, shadow, _trash) = setup();
        let old = sidecar(&shadow, ".old").unwrap();
        std::fs::rename(&shadow, &old).unwrap();

        let log = recover_dir_replace(&shadow).unwrap();
        assert!(
            log.iter().any(|l| l.contains("回滚")),
            "该报告回滚：{log:?}"
        );
        assert!(shadow.exists(), "编辑态必须搬回来");
        assert!(!old.exists());
    }

    /// 什么都没发生 ⇒ 不报、不建目录（全新会话走这条）
    #[test]
    fn a_clean_directory_reports_nothing() {
        let (d, _real, _shadow, _trash) = setup();
        let nowhere = d.path().join("nothing-here");
        let log = recover_dir_replace(&nowhere).unwrap();
        assert!(log.is_empty(), "{log:?}");
        assert!(!nowhere.exists(), "不许顺手把目录建出来");
    }

    /// 自己套自己 / 祖先关系必须拒绝 —— rename 在那两种形状下语义会变
    #[test]
    fn replacing_a_directory_into_itself_is_refused() {
        let (_d, real, _shadow, _trash) = setup();
        assert!(mirror(&real, &real.join("sub")).is_err());
        assert!(mirror(&real, &real).is_err());
    }

    #[test]
    fn an_edit_only_touches_the_shadow_tree() {
        let (_d, real, shadow, _trash) = setup();
        let before = snapshot(&shadow).unwrap();
        write(&shadow, "machines/A1.toml", "id = 'A1'\nenabled = false\n");
        let after = snapshot(&shadow).unwrap();

        let mut txn = Txn::new();
        let op = txn
            .record_with_fields(
                "停用 A1",
                None,
                &before,
                &after,
                &shadow,
                vec![field("A1", "enabled", "true", "false")],
            )
            .unwrap();
        assert!(op.is_some());
        /* **一处**（2026-11-03 定稿：按字段计）——`enabled` 这一格从 true 变 false，
        就是一处在界面上。按行数会写成 2（旧行没了 + 新行来了），那是 diff 框的口径，
        不是"改了几处"。 */
        assert_eq!(txn.dirty_count(), 1, "改了一个字段就是一处");

        // 真实树一个字节都没变
        let real_text = std::fs::read_to_string(real.join("machines/A1.toml")).unwrap();
        assert!(
            !real_text.contains("enabled"),
            "编辑不该碰磁盘：{real_text}"
        );
    }

    #[test]
    fn undo_and_redo_are_symmetric_even_for_file_creation() {
        let (_d, _real, shadow, _trash) = setup();
        let before = snapshot(&shadow).unwrap();
        // 新建一个版本（= 新文件）
        write(&shadow, "machines/A1-NEW.toml", "id = 'X'\n");
        let after = snapshot(&shadow).unwrap();

        let mut txn = Txn::new();
        txn.record("新增版本", None, &before, &after, &shadow)
            .unwrap();
        assert!(shadow.join("machines/A1-NEW.toml").is_file());

        txn.undo(&shadow).unwrap();
        assert!(
            !shadow.join("machines/A1-NEW.toml").exists(),
            "撤销后文件该消失"
        );

        txn.redo(&shadow).unwrap();
        assert!(
            shadow.join("machines/A1-NEW.toml").is_file(),
            "重做后文件该回来"
        );
    }

    #[test]
    fn undo_brings_a_deleted_file_back_byte_for_byte() {
        let (_d, _real, shadow, _trash) = setup();
        let original = std::fs::read(shadow.join("bundles.toml")).unwrap();
        let before = snapshot(&shadow).unwrap();
        std::fs::remove_file(shadow.join("bundles.toml")).unwrap();
        let after = snapshot(&shadow).unwrap();

        let mut txn = Txn::new();
        txn.record("删除套餐文件", None, &before, &after, &shadow)
            .unwrap();
        assert!(!shadow.join("bundles.toml").exists());

        txn.undo(&shadow).unwrap();
        assert_eq!(
            std::fs::read(shadow.join("bundles.toml")).unwrap(),
            original
        );
    }

    #[test]
    fn commit_writes_diffs_and_keeps_deletions_in_the_trash() {
        let (_d, real, shadow, trash) = setup();
        write(&shadow, "machines/A1.toml", "id = 'A1'\nenabled = false\n");
        std::fs::remove_file(shadow.join("bundles.toml")).unwrap();

        let report = commit(&real, &shadow, &trash, "2026-10-31T00-00-00").unwrap();
        assert_eq!(report.written, 1);
        assert_eq!(report.deleted, 1);
        assert!(std::fs::read_to_string(real.join("machines/A1.toml"))
            .unwrap()
            .contains("enabled = false"));
        assert!(!real.join("bundles.toml").exists());
        assert!(
            trash.join("2026-10-31T00-00-00/bundles.toml").is_file(),
            "删除的文件必须进回收站留底"
        );
    }

    #[test]
    fn discard_mirrors_the_disk_back_over_the_shadow() {
        let (_d, real, shadow, _trash) = setup();
        write(&shadow, "machines/A1.toml", "id = 'A1'\nenabled = false\n");
        write(&shadow, "machines/GHOST.toml", "id = 'GHOST'\n");

        mirror(&real, &shadow).unwrap();
        assert!(!std::fs::read_to_string(shadow.join("machines/A1.toml"))
            .unwrap()
            .contains("enabled"));
        assert!(
            !shadow.join("machines/GHOST.toml").exists(),
            "放弃后多余文件要清掉"
        );
    }

    /// **作者给的六步验收场景**（2026-10-31）：关开关 → 改显示名 → 改尺寸 → 删禁区 →
    /// 新增版本 → 改新版本参数。磁盘必须一个字节不动；撤销要跨类型逐步回退；
    /// 保存才统一提交。
    #[test]
    fn the_six_step_scenario_stays_in_memory_until_saved() {
        let (_d, real, shadow, trash) = setup();
        write(&real, "registry/param_registry.toml", "params = []\n");
        write(&real, "forbidden_zones/A1.toml", "zones = ['keep-out']\n");
        mirror(&real, &shadow).unwrap();
        let real_before = snapshot(&real).unwrap();

        let mut txn = Txn::new();
        /* `fields` 在真实链路里由写盘那一刻产出（`KeyChange` / `FieldChange`）；
        这里手工给，模拟"这一步真的改了哪几个字段"。 */
        let step = |txn: &mut Txn, label: &str, fields: Vec<FieldEntry>, edit: &dyn Fn(&Path)| {
            let before = snapshot(&shadow).unwrap();
            edit(&shadow);
            let after = snapshot(&shadow).unwrap();
            txn.record_with_fields(label, None, &before, &after, &shadow, fields)
                .unwrap();
        };

        // ① 关掉 enabled
        step(
            &mut txn,
            "停用 A1",
            vec![field("A1", "enabled", "true", "false")],
            &|s: &Path| {
                write(
                    s,
                    "machines/A1.toml",
                    "id = 'A1'\nname = 'A1'\nenabled = false\n",
                );
            },
        );
        // ② 改显示名
        step(
            &mut txn,
            "改显示名",
            vec![field("A1", "name", "A1", "A1 新版")],
            &|s: &Path| {
                write(
                    s,
                    "machines/A1.toml",
                    "id = 'A1'\nname = 'A1 新版'\nenabled = false\n",
                );
            },
        );
        // ③ 改床身尺寸
        step(
            &mut txn,
            "改尺寸",
            vec![
                field("A1", "bedSize.width", "0", "256.0"),
                field("A1", "bedSize.depth", "0", "256.0"),
            ],
            &|s: &Path| {
                write(
                s,
                "machines/A1.toml",
                "id = 'A1'\nname = 'A1 新版'\nenabled = false\n[dimensions.bedSize]\nwidth = 256.0\ndepth = 256.0\n",
            );
            },
        );
        // ④ 删一个禁区文件
        step(
            &mut txn,
            "删禁区",
            vec![field("A1", "zones", "['keep-out']", "")],
            &|s: &Path| {
                std::fs::remove_file(s.join("forbidden_zones/A1.toml")).unwrap();
            },
        );
        // ⑤ 新增一个版本（同一份机型文件里多一节）
        step(
            &mut txn,
            "新增版本",
            vec![
                field("A1:STANDARD", "version", "", "标准版"),
                field("A1:FAST", "version", "", "快拆版"),
            ],
            &|s: &Path| {
                write(
                s,
                "machines/A1.toml",
                "id = 'A1'\nname = 'A1 新版'\nenabled = false\n[dimensions.bedSize]\nwidth = 256.0\ndepth = 256.0\n[[versions]]\nid = 'STANDARD'\nname = '标准版'\n[[versions]]\nid = 'FAST'\nname = '快拆版'\n",
            );
            },
        );
        // ⑥ 改新版本参数
        step(
            &mut txn,
            "改新版本参数",
            vec![field("A1:FAST", "wiping.ironing_mode", "", "auto")],
            &|s: &Path| {
                write(
                s,
                "registry/param_registry.toml",
                "params = [{ key = 'wiping.ironing_mode', machineVariants = { 'A1:FAST' = 'auto' } }]\n",
            );
            },
        );

        /* **六步 = 8 处字段**（2026-11-03 定稿：按字段计）：停用 1、改显示名 1、
        改尺寸 2（width + depth）、删禁区 1、新增两个版本 2、改参数 1。
        这个数是"改了哪几处"，不是步数（看下一句），也不是 diff 行数。 */
        assert_eq!(txn.dirty_count(), 8, "六步编辑改了 8 处字段");
        assert_eq!(txn.undo_depth(), 6, "撤销栈仍然按步（6 步）");
        assert_eq!(
            snapshot(&real).unwrap(),
            real_before,
            "六步编辑之后，真仓库必须一个字节都没变"
        );

        // 撤销三次：跨「参数 / 新增版本 / 删文件」三种类型，同一条栈
        txn.undo(&shadow).unwrap();
        txn.undo(&shadow).unwrap();
        txn.undo(&shadow).unwrap();
        /* 撤销三步之后剩前三步：停用 1 + 改显示名 1 + 改尺寸 2 = 4 处。
         **别在这里凭印象写数** —— 口径换一次就得重数一遍。 */
        assert_eq!(txn.dirty_count(), 4, "撤销三步之后剩 4 处");
        assert!(
            shadow.join("forbidden_zones/A1.toml").is_file(),
            "撤销到第 3 步之后，被删的禁区文件必须回来"
        );
        assert!(
            !std::fs::read_to_string(shadow.join("machines/A1.toml"))
                .unwrap()
                .contains("FAST"),
            "撤销到第 3 步之后，新增的版本必须消失"
        );

        // 重做三次：回到六步之后
        txn.redo(&shadow).unwrap();
        txn.redo(&shadow).unwrap();
        txn.redo(&shadow).unwrap();
        assert_eq!(txn.dirty_count(), 8, "重做回到六步的 8 处");
        assert!(!shadow.join("forbidden_zones/A1.toml").exists());

        // 保存：这时才动磁盘
        let report = commit(&real, &shadow, &trash, "2026-10-31T12-00-00").unwrap();
        assert!(report.written >= 2, "至少机型与注册表两份要写");
        assert_eq!(report.deleted, 1, "删掉的禁区文件按一份算");
        assert!(std::fs::read_to_string(real.join("machines/A1.toml"))
            .unwrap()
            .contains("A1 新版"));
        assert!(trash
            .join("2026-10-31T12-00-00/forbidden_zones/A1.toml")
            .is_file());
    }

    #[test]
    fn a_path_escape_is_refused() {
        let d = tempfile::tempdir().unwrap();
        assert!(safe_rel("../outside.toml").is_err());
        assert!(safe_rel("/etc/passwd").is_err());
        assert!(safe_rel("machines/A1.toml").is_ok());
        let _ = d;
    }

    /// 同一格 5 秒内连改 ⇒ **只留一条**，且撤一次回到最初那个值。
    ///
    /// 作者 2026-11-02：「就像那个撤销堆栈一样，他只记一次……5 秒内的算一条」。
    /// 连点步进器 `65 → 64 → … → 60` 之后按一下撤销，人期待的是回到 `65`，不是退到 `61`。
    #[test]
    fn the_same_cell_within_the_window_collapses_into_one_op() {
        let (_d, _real, shadow, _trash) = setup();
        let key = EditKey {
            level: crate::workbench::domain::Level::Version,
            owner: "A1/STANDARD".to_owned(),
            key: "glue.speed_limit".to_owned(),
        };
        let mut txn = Txn::new();

        /* 改五次，每次都是一整轮「拍快照 → 写盘 → 记事务」 */
        for value in ["64\n", "63\n", "62\n", "61\n", "60\n"] {
            let before = snapshot(&shadow).unwrap();
            write(&shadow, "bundles.toml", value);
            let after = snapshot(&shadow).unwrap();
            txn.record("改参数值", Some(key.clone()), &before, &after, &shadow)
                .unwrap();
        }

        assert_eq!(txn.undo_depth(), 1, "同一格连改五次只该留一条撤销");
        assert_eq!(txn.ops().len(), 1);

        txn.undo(&shadow).unwrap();
        assert_eq!(
            std::fs::read_to_string(shadow.join("bundles.toml")).unwrap(),
            "bundles = []\n",
            "撤一次必须回到**最初**那一份，不是上一步"
        );
        assert_eq!(txn.undo_depth(), 0);

        // 重做把五次的结果一次补回来
        txn.redo(&shadow).unwrap();
        assert_eq!(
            std::fs::read_to_string(shadow.join("bundles.toml")).unwrap(),
            "60\n"
        );
    }

    /// **改回原值 ⇒ 「未保存」必须归零**（2026-11-02 作者：「上面显示的不正常，
    /// 还会出现没有未保存的改动这个提示一直不消失」）。
    ///
    /// 现象是一屏上三个说法：页头「未保存 (0)」、状态栏「未保存改动 (1)」、
    /// 「查看改动」框里「影子与盘上一模一样」。根子就是这里 ——
    /// `70 → 80 → 70` 合并之后这条 op 的 `before` 与 `after` 是同一份字节，
    /// 而 `dirty_count` 数的是**条数**。
    #[test]
    fn editing_back_to_the_original_value_clears_the_unsaved_count() {
        let (_d, _real, shadow, _trash) = setup();
        let key = EditKey {
            level: crate::workbench::domain::Level::Version,
            owner: "A1/STANDARD".to_owned(),
            key: "glue.speed_limit".to_owned(),
        };
        let mut txn = Txn::new();

        /* 70 → 80：一条 */
        let before = snapshot(&shadow).unwrap();
        write(&shadow, "bundles.toml", "80\n");
        let after = snapshot(&shadow).unwrap();
        txn.record_with_fields(
            "改参数值",
            Some(key.clone()),
            &before,
            &after,
            &shadow,
            vec![field("A1:STANDARD", "glue.speed_limit", "70", "80")],
        )
        .unwrap();
        assert_eq!(txn.dirty_count(), 1, "一个字段变了一次就是一处");

        /* 80 → 70（改回原值）：净零，整条丢掉 */
        let before = snapshot(&shadow).unwrap();
        write(&shadow, "bundles.toml", "bundles = []\n");
        let after = snapshot(&shadow).unwrap();
        let op = txn
            .record_with_fields(
                "改参数值",
                Some(key),
                &before,
                &after,
                &shadow,
                vec![field("A1:STANDARD", "glue.speed_limit", "80", "70")],
            )
            .unwrap();
        assert!(op.is_none(), "净零的这一笔不该记成一条改动");
        assert_eq!(txn.dirty_count(), 0, "改回原值之后不该还有未保存改动");
        assert!(!txn.is_dirty());
        assert_eq!(txn.undo_depth(), 0, "净零的那条也不该留在撤销栈里");
    }

    /// **不同归属算不同处**（2026-11-03 作者定稿）：
    ///
    /// > 「我改了 A1 基底的 Z，也改了 A1 标准版的 Z，也改了 A1 快拆版的 Z，
    /// > 那就算三个改动，都显示出来」
    ///
    /// 一次手势连带写出三格 ⇒ 页头就是 3，`ops()` 仍然只有 1 条（撤销一次全回去）。
    #[test]
    fn three_owners_of_the_same_key_are_three_changes() {
        let (_d, _real, shadow, _trash) = setup();
        write(
            &shadow,
            "bundles.toml",
            "a = 1
",
        );
        let before = snapshot(&shadow).unwrap();
        write(
            &shadow,
            "bundles.toml",
            "a = 2
",
        );
        let after = snapshot(&shadow).unwrap();

        let mut txn = Txn::new();
        txn.record_with_fields(
            "改参数值",
            None,
            &before,
            &after,
            &shadow,
            vec![
                field("A1", "toolhead.offset.z", "3.9", "3.95"),
                field("A1:STANDARD", "toolhead.offset.z", "3.9", "3.95"),
                field("A1:FAST", "toolhead.offset.z", "3.9", "3.95"),
            ],
        )
        .unwrap();

        assert_eq!(txn.ops().len(), 1, "一次手势只有一条撤销");
        assert_eq!(txn.dirty_count(), 3, "但三处归属都算改动");

        /* 条目也要原样给出来（界面上那三行），顺序按 (层, 归属, 键) */
        let got: Vec<(String, String)> = txn
            .field_changes()
            .into_iter()
            .map(|e| (e.owner, e.key))
            .collect();
        assert_eq!(
            got,
            vec![
                ("A1".to_owned(), "toolhead.offset.z".to_owned()),
                ("A1:FAST".to_owned(), "toolhead.offset.z".to_owned()),
                ("A1:STANDARD".to_owned(), "toolhead.offset.z".to_owned()),
            ]
        );
    }

    /// **同一格连改两次只显示最终变化**（作者定稿：`0 → 0.05 → 0.1` 只显示 `0 → 0.1`）
    #[test]
    fn editing_the_same_field_twice_keeps_one_entry_with_first_before_and_last_after() {
        let (_d, _real, shadow, _trash) = setup();
        write(
            &shadow,
            "bundles.toml",
            "a = 0
",
        );

        let mut txn = Txn::new();
        /* 第一次：0 → 0.05 */
        let before = snapshot(&shadow).unwrap();
        write(
            &shadow,
            "bundles.toml",
            "a = 0.05
",
        );
        let after = snapshot(&shadow).unwrap();
        txn.record_with_fields(
            "改参数值",
            None,
            &before,
            &after,
            &shadow,
            vec![field("A1", "k", "0", "0.05")],
        )
        .unwrap();
        /* 第二次：0.05 → 0.1（同一格，同一笔事务里继续改） */
        let before = snapshot(&shadow).unwrap();
        write(
            &shadow,
            "bundles.toml",
            "a = 0.1
",
        );
        let after = snapshot(&shadow).unwrap();
        txn.record_with_fields(
            "改参数值",
            None,
            &before,
            &after,
            &shadow,
            vec![field("A1", "k", "0.05", "0.1")],
        )
        .unwrap();

        let entries = txn.field_changes();
        assert_eq!(entries.len(), 1, "同一格还是 1 处，不随操作次数膨胀");
        assert_eq!(entries[0].before.as_deref(), Some("0"), "before 保最初");
        assert_eq!(entries[0].after.as_deref(), Some("0.1"), "after 换最新");
    }

    /// 换了格（另一个 owner / 另一个键）⇒ 不许合并，那是两件事
    #[test]
    fn a_different_cell_is_a_separate_op() {
        let (_d, _real, shadow, _trash) = setup();
        let a = EditKey {
            level: crate::workbench::domain::Level::Version,
            owner: "A1/STANDARD".to_owned(),
            key: "glue.speed_limit".to_owned(),
        };
        let b = EditKey {
            owner: "A1/FAST".to_owned(),
            ..a.clone()
        };
        let mut txn = Txn::new();
        /* 写不同的内容：同一份文件写两遍同样的字节，第二遍的差集是空的 ——
        那不会记成一条（`record` 见到空差集直接返回 `None`），测的就不是合并了 */
        for (i, k) in [a, b].into_iter().enumerate() {
            let before = snapshot(&shadow).unwrap();
            write(&shadow, "bundles.toml", &format!("x = {}\n", i + 1));
            let after = snapshot(&shadow).unwrap();
            txn.record("改参数值", Some(k), &before, &after, &shadow)
                .unwrap();
        }
        assert_eq!(txn.undo_depth(), 2, "两格就是两条，撤销要能分开回退");
    }

    /// 结构编辑（`key: None`）永不合并 —— 建一个版本和删一个版本不是"同一件事的延续"
    #[test]
    fn structural_edits_never_merge() {
        let (_d, _real, shadow, _trash) = setup();
        let mut txn = Txn::new();
        for name in ["X-A.toml", "X-B.toml"] {
            let before = snapshot(&shadow).unwrap();
            write(&shadow, &format!("machines/{name}"), "id = 'X'\n");
            let after = snapshot(&shadow).unwrap();
            txn.record("新增版本", None, &before, &after, &shadow)
                .unwrap();
        }
        assert_eq!(txn.undo_depth(), 2);
    }

    /// **改着改着「未保存」越来越少**（2026-11-03 作者报：`ops=5 fields=1`）。
    ///
    /// 现场日志：
    /// ```text
    /// ops=3 fields=3
    /// ops=4 fields=2    ← 又改了一格，反而变少
    /// ops=5 fields=1
    /// ```
    ///
    /// 根因在 `field_changes` 开头那句 `if !p.files.any(before != after) { continue }`：
    /// `files` 是**整份文件**的前后字节，用户把某一格改回原值之后 `absorb` 会把这一份
    /// 从 `files` 里剔掉，于是**整条 payload 的字段全被跳过** —— 哪怕里面还有别的格是脏的。
    #[test]
    fn one_cell_returning_to_its_old_value_does_not_erase_the_others() {
        let (_d, _real, shadow, _trash) = setup();
        write(&shadow, "registry/param_registry.toml", "a = 1\nb = 2\n");
        let s0 = snapshot(&shadow).unwrap();
        write(&shadow, "registry/param_registry.toml", "a = 9\nb = 8\n");
        let s1 = snapshot(&shadow).unwrap();
        write(&shadow, "registry/param_registry.toml", "a = 9\nb = 2\n");
        let s2 = snapshot(&shadow).unwrap();

        let f = |key: &str, before: &str, after: &str| FieldEntry {
            level: Some("Machine".to_owned()),
            owner: "A1".to_owned(),
            key: key.to_owned(),
            label: key.to_owned(),
            before: Some(before.to_owned()),
            after: Some(after.to_owned()),
            why: "user",
        };

        let mut txn = Txn::new();
        txn.record_with_fields(
            "改两格",
            None,
            &s0,
            &s1,
            &shadow,
            vec![f("a", "1", "9"), f("b", "2", "8")],
        )
        .unwrap();
        assert_eq!(txn.dirty_count(), 2, "两格都是脏的");

        txn.record_with_fields("b 改回去", None, &s1, &s2, &shadow, vec![f("b", "8", "2")])
            .unwrap();

        let keys: Vec<String> = txn.field_changes().into_iter().map(|e| e.key).collect();
        assert_eq!(keys, vec!["a"], "只剩 a 一处，b 已回到原值：实际 {keys:?}");
    }

    /// **净零之后又改脏，`before` 必须还是最初那个值**（2026-11-03）。
    ///
    /// `3.9 → 3.95`（脏）→ `3.95 → 3.9`（净零）→ `3.9 → 3.8`。
    /// 净零那一刻条目必须消失（页头判 0），再改脏时要重新出现且 `before = 3.9`。
    #[test]
    fn a_cell_that_goes_back_and_forth_keeps_the_original_before() {
        let (_d, _real, shadow, _trash) = setup();
        write(&shadow, "registry/param_registry.toml", "z = 3.9\n");
        let s0 = snapshot(&shadow).unwrap();
        write(&shadow, "registry/param_registry.toml", "z = 3.95\n");
        let s1 = snapshot(&shadow).unwrap();
        write(&shadow, "registry/param_registry.toml", "z = 3.9\n");
        let s2 = snapshot(&shadow).unwrap();
        write(&shadow, "registry/param_registry.toml", "z = 3.8\n");
        let s3 = snapshot(&shadow).unwrap();

        let f = |before: &str, after: &str| FieldEntry {
            level: Some("Machine".to_owned()),
            owner: "A1".to_owned(),
            key: "z".to_owned(),
            label: "Z".to_owned(),
            before: Some(before.to_owned()),
            after: Some(after.to_owned()),
            why: "user",
        };

        let mut txn = Txn::new();
        txn.record_with_fields("z=3.95", None, &s0, &s1, &shadow, vec![f("3.9", "3.95")])
            .unwrap();
        txn.record_with_fields("z=3.9", None, &s1, &s2, &shadow, vec![f("3.95", "3.9")])
            .unwrap();
        assert_eq!(txn.dirty_count(), 0, "回到原值 ⇒ 没有未保存改动");

        txn.record_with_fields("z=3.8", None, &s2, &s3, &shadow, vec![f("3.9", "3.8")])
            .unwrap();
        let got = txn.field_changes();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].before.as_deref(), Some("3.9"), "改之前是 3.9");
        assert_eq!(got[0].after.as_deref(), Some("3.8"));
    }

    /// **同一台机型的基底改动，按继承展开成"受影响的每一格"**
    /// （2026-11-03 作者：「修改 A1 基底 Z，基底和三个继承版本都能看到变化」）。
    ///
    /// 关键：这是**展开**，不是复制。判据是 `machine_variants` 里有没有
    /// `机型:版本` 这个键 —— 有独立值的不受影响，所以不该被算进来。
    /// 展开后的条目 `why = "inherited"`，与真写了版本键的 `"promoted"` 分开。
    #[test]
    fn a_base_edit_expands_into_the_versions_that_inherit_it() {
        /* 这条只看 `inherited_fields` 的规则，不碰真数据 —— 放在 app 层测更合适，
        这里用 domain 层的纯逻辑验算 dedup_key 不冲突即可。 */
        let e = FieldEntry {
            level: Some("Machine".to_owned()),
            owner: "A1".to_owned(),
            key: "toolhead.offset.z".to_owned(),
            label: "Z 轴偏移".to_owned(),
            before: Some("3.85".to_owned()),
            after: Some("3.9".to_owned()),
            why: "user",
        };
        let inherited = FieldEntry {
            level: Some("Version".to_owned()),
            owner: "A1:STANDARD".to_owned(),
            key: "toolhead.offset.z".to_owned(),
            label: "Z 轴偏移".to_owned(),
            before: e.before.clone(),
            after: e.after.clone(),
            why: "inherited",
        };
        assert_ne!(
            e.dedup_key(),
            inherited.dedup_key(),
            "基底那一格与继承的版本格是两个归属，不许合并成一条"
        );
    }
}
