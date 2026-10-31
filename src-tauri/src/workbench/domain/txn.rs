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

use std::collections::BTreeMap;
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

/// 一条编辑的**完整载荷**：前后字节都在，撤销/重做靠它
#[derive(Debug, Clone)]
struct Payload {
    label: String,
    at: String,
    files: Vec<BeforeAfter>,
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Snapshot {
    map: BTreeMap<String, Entry>,
}

impl Snapshot {
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
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

/// 把影子树对齐到磁盘。
///
/// - 目标里多出来的文件删掉；
/// - 源里有、目标没有（或内容不同）的写过去。
///
/// **逐份复制，不用硬链接**（2026-10-31 实测）：硬链接只有在「所有写都是
/// `atomic_write`（临时文件 + rename）」时才安全 —— 任何一处 `std::fs::write`
/// 都会顺着链接把**真仓库**改了，事务边界当场失效。5.9 MB 复制几十毫秒，
/// 换一条不会破的边界，值。
pub fn mirror(src: &Path, dst: &Path) -> Result<(), AppError> {
    let mut want = Vec::new();
    walk(src, src, &mut want)?;
    let want_set: std::collections::BTreeSet<&String> = want.iter().collect();

    let mut have = Vec::new();
    walk(dst, dst, &mut have)?;
    for rel in &have {
        if !want_set.contains(rel) {
            let path = dst.join(rel);
            let _ = std::fs::remove_file(&path);
            prune_empty_parents(&path, dst);
        }
    }

    for rel in &want {
        let to = dst.join(rel);
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::io(format!("建不出影子目录：{}", parent.display()))
                    .with_detail(e.to_string())
            })?;
        }
        let from = src.join(rel);
        if same_file(&from, &to, rel) {
            continue;
        }
        let _ = std::fs::remove_file(&to);
        std::fs::copy(&from, &to).map_err(|e| {
            AppError::io(format!("复制到影子树失败：{}", to.display())).with_detail(e.to_string())
        })?;
    }
    Ok(())
}

fn same_file(a: &Path, b: &Path, rel: &str) -> bool {
    let (Ok(ma), Ok(mb)) = (std::fs::metadata(a), std::fs::metadata(b)) else {
        return false;
    };
    if ma.len() != mb.len() {
        return false;
    }
    match (std::fs::read(a), std::fs::read(b)) {
        (Ok(x), Ok(y)) if is_text(rel) => x == y,
        (Ok(x), Ok(y)) => head_fingerprint(&x) == head_fingerprint(&y),
        _ => false,
    }
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
    pub fn record(
        &mut self,
        label: impl Into<String>,
        before: &Snapshot,
        after: &Snapshot,
        root: &Path,
    ) -> Result<Option<Op>, AppError> {
        let files = diff(before, after, root)?;
        if files.is_empty() {
            return Ok(None);
        }
        let payload = Payload {
            label: label.into(),
            at: clock::now_iso8601(),
            files,
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

    #[test]
    fn an_edit_only_touches_the_shadow_tree() {
        let (_d, real, shadow, _trash) = setup();
        let before = snapshot(&shadow).unwrap();
        write(&shadow, "machines/A1.toml", "id = 'A1'\nenabled = false\n");
        let after = snapshot(&shadow).unwrap();

        let mut txn = Txn::new();
        let op = txn.record("停用 A1", &before, &after, &shadow).unwrap();
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
        txn.record("新增版本", &before, &after, &shadow).unwrap();
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
        txn.record("删除套餐文件", &before, &after, &shadow)
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
            txn.record(label, &before, &after, &shadow).unwrap();
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
}
