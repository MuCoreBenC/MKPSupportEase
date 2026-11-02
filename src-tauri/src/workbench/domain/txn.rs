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
}

impl Payload {
    /// 把这一笔并进自己：**`before` 保留最初那份**，`after` 换成最新那份。
    ///
    /// 于是「撤一次」回的是"我开始动手之前"，而不是"上一步"。这也正是合并的意义 ——
    /// 连点十下之后按一下撤销，人期待的是回到原值，不是退一格。
    fn absorb(&mut self, files: Vec<BeforeAfter>, label: String) {
        for (rel, _, after) in files {
            match self.files.iter_mut().find(|(r, _, _)| *r == rel) {
                /* 同一份文件：只换 `after`，`before` 一个字都不动 */
                Some(slot) => slot.2 = after,
                None => self.files.push((rel, None, after)),
            }
        }
        /* 标签取**最新**那一笔的：连点过程中人看的最后一格才是他印象里改的东西 */
        self.label = label;
        self.last_at = std::time::Instant::now();
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

    /// 未保存的改动条数（界面上那个 N）
    pub fn dirty_count(&self) -> usize {
        self.payloads.len() + usize::from(self.recovered)
    }

    pub fn is_dirty(&self) -> bool {
        self.dirty_count() > 0
    }

    /// 能逐条撤销的深度（崩溃恢复那一条不算）
    pub fn undo_depth(&self) -> usize {
        self.payloads.len()
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
                top.absorb(files, label);
                self.redo.clear();
                self.recovered = false;
                return Ok(Some(top.view()));
            }
        }

        let payload = Payload {
            label,
            at: clock::now_iso8601(),
            files,
            key,
            last_at: std::time::Instant::now(),
        };
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
    workbench_root.join(".draft").join("presets")
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
    workbench_root.join(".draft").join("base")
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
            .record("停用 A1", None, &before, &after, &shadow)
            .unwrap();
        assert!(op.is_some());
        assert_eq!(txn.dirty_count(), 1);

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
        let step = |txn: &mut Txn, label: &str, edit: &dyn Fn(&Path)| {
            let before = snapshot(&shadow).unwrap();
            edit(&shadow);
            let after = snapshot(&shadow).unwrap();
            txn.record(label, None, &before, &after, &shadow).unwrap();
        };

        // ① 关掉 enabled
        step(&mut txn, "停用 A1", &|s: &Path| {
            write(
                s,
                "machines/A1.toml",
                "id = 'A1'\nname = 'A1'\nenabled = false\n",
            );
        });
        // ② 改显示名
        step(&mut txn, "改显示名", &|s: &Path| {
            write(
                s,
                "machines/A1.toml",
                "id = 'A1'\nname = 'A1 新版'\nenabled = false\n",
            );
        });
        // ③ 改床身尺寸
        step(&mut txn, "改尺寸", &|s: &Path| {
            write(
                s,
                "machines/A1.toml",
                "id = 'A1'\nname = 'A1 新版'\nenabled = false\n[dimensions.bedSize]\nwidth = 256.0\ndepth = 256.0\n",
            );
        });
        // ④ 删一个禁区文件
        step(&mut txn, "删禁区", &|s: &Path| {
            std::fs::remove_file(s.join("forbidden_zones/A1.toml")).unwrap();
        });
        // ⑤ 新增一个版本（同一份机型文件里多一节）
        step(&mut txn, "新增版本", &|s: &Path| {
            write(
                s,
                "machines/A1.toml",
                "id = 'A1'\nname = 'A1 新版'\nenabled = false\n[dimensions.bedSize]\nwidth = 256.0\ndepth = 256.0\n[[versions]]\nid = 'STANDARD'\nname = '标准版'\n[[versions]]\nid = 'FAST'\nname = '快拆版'\n",
            );
        });
        // ⑥ 改新版本参数
        step(&mut txn, "改新版本参数", &|s: &Path| {
            write(
                s,
                "registry/param_registry.toml",
                "params = [{ key = 'wiping.ironing_mode', machineVariants = { 'A1:FAST' = 'auto' } }]\n",
            );
        });

        assert_eq!(txn.dirty_count(), 6);
        assert_eq!(
            snapshot(&real).unwrap(),
            real_before,
            "六步编辑之后，真仓库必须一个字节都没变"
        );

        // 撤销三次：跨「参数 / 新增版本 / 删文件」三种类型，同一条栈
        txn.undo(&shadow).unwrap();
        txn.undo(&shadow).unwrap();
        txn.undo(&shadow).unwrap();
        assert_eq!(txn.dirty_count(), 3);
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
        assert_eq!(txn.dirty_count(), 6);
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
}
