//! **编辑历史**（2026-11-03）：撤销 / 重做记的是**用户操作**，不是文件字节。
//!
//! # 为什么换掉 `txn` 那条栈
//!
//! 上一代把每一笔编辑都写成影子树文件，撤销 = 把那一笔改过的**整个文件**的 before 写回去。
//! 三个后果叠在一起，就是作者报的那个形状：
//!
//! 1. 每敲一下都真的在跑一次文件事务（快照 → 序列化 → `atomic_write`）；
//! 2. 一次撤销退掉的是"那一笔请求碰过的整个文件"，而不是"那个参数"；
//! 3. 撤销之后还得 `reload()` 整份文档、再让前端重取一屏才能看见。
//!
//! 现在：**一条命令 = 一次用户操作**，执行与撤销都只动内存里的 [`Draft`]。
//!
//! # 逆命令不在这里写
//!
//! [`Patch`] 的反向由 [`super::patch::apply`] 现算（那里是唯一知道
//! 「版本层原来跟随 ⇒ 反向要发 `None` 而不是写回旧数」的地方）。
//! 这里只负责**把正向和反向一起收进一条**，别的一概不做。
//!
//! # 合并（粒度）
//!
//! 作者原话：「人家成熟的是按时间的，就不会很频繁，也不会说是一次撤销太多」。
//! 判据：
//!
//! ```text
//! 同一条 EditKey（同一格）
//!   且 距上一条 ≤ COALESCE_MS
//!   且 上一条不是「一次性动作」（跟随 / 保存值 / 出厂，见 Entry::mergable = false）
//!   ⇒ 就地合并：保留最早的"改前"、最新的"改后"，不新增条目
//! ```
//!
//! 其余情况一律**新起一条**：换格、停顿超过窗口、保存、放弃、撤销、重做后的第一笔。
//! 合并只改 `inverse` —— 正向 `patch` 换成最新的那条，所以重做回到的是**最终值**。

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use serde_json::Value;

use crate::workbench::presets::ParamRegistry as Registry;

use crate::error::AppError;

use super::patch::{self, Committed, Draft, Patch};

/// 同一条 `EditKey` 的两笔之间，超过这个间隔就**另起一条**。
///
/// 上一代是 5000ms（`txn::MERGE_MS`）—— 太长：按住步进器连点十几下，中间停 3 秒再接上
/// 也被并进去，一次 Cmd+Z 退掉一长串。500ms 是"一次连续输入"的自然节奏，
/// 停下来看一眼再改就算两次。
///
/// ★ 唯一出处：前端不许自己定合并窗口（它只发命令）。
pub const COALESCE_MS: u64 = 500;

/// 一条命令：**一次用户操作**。
#[derive(Debug, Clone)]
pub struct Entry {
    /// 给撤销栈 / 日志看的一句人话（「A1 · 标准版 · Z 轴偏移」）
    pub label: String,
    /// 这一条怎么执行 / 怎么撤销
    pub action: Action,
    /// **合并判据**：改的是哪一格。`None` = 一次性动作，永不与别人合并。
    ///
    /// 值编辑 = `v:A1/STANDARD:toolhead.offset.z`；结构编辑 = `A1.toml|display` 这类
    /// 「对象 | 字段」。同一个键、500ms 内 ⇒ 并进上一条。
    pub merge_key: Option<String>,
    /// 什么时候发生的（合并窗口用）
    pub at: Instant,
    /// **这一条真的改了哪些字段**（去重键 = `层, 归属, 键`）。
    ///
    /// 与 [`super::txn::FieldEntry`] 同形状、同口径（那边是"改了什么"的唯一定义）：
    /// 一次改基底会连带写出 3 个版本键 ⇒ 这里是 4 条 ⇒ 界面上 4 处。
    /// 合并时由 `merge_fields` 去重：同一格 `before` 保最初、`after` 换最新。
    pub fields: Vec<super::txn::FieldEntry>,
}

/// 一条命令的正/反向。
#[derive(Debug, Clone)]
pub enum Action {
    /// 值编辑：正向一条 patch + 反向若干条（已倒序，由 `patch::apply` 现算）
    Value { patch: Patch, inverse: Vec<Patch> },
    /// 结构编辑（2026-11-03 阶段 3）：**若干文件的前后字节**。
    ///
    /// 结构编辑是"一对象一文件"（`machines/A1.toml` / `bundles.toml` …），
    /// 文件字节就是这一条命令的粒度 —— 撤销把 `before` 写回去、重做写 `after`，
    /// 不碰别的文件。这与值编辑不同：参数全挤在一个 56KB 的注册表里，
    /// 那才必须按"哪个参数"撤。
    Struct(StructCmd),
}

/// 结构编辑的正/反向：若干文件的前后字节。
#[derive(Debug, Clone, Default)]
pub struct StructCmd {
    /// 同一条命令可能碰多个文件（如新建机型 = 新建 `machines/<id>.toml`）
    pub files: Vec<FileSwap>,
}

/// 一个文件在这一笔里的前后字节。`None` = 这一端"不存在"
/// （`before` 为 `None` = 这一笔**新建**了它；`after` 为 `None` = 这一笔**删掉**了它）。
#[derive(Debug, Clone)]
pub struct FileSwap {
    /// 相对影子树的路径（`machines/A1.toml`）
    pub path: String,
    pub before: Option<Vec<u8>>,
    pub after: Option<Vec<u8>>,
}

impl Entry {
    /// 这一条还允许被下一笔并进来吗（窗口 + 是不是一次性动作）
    pub fn mergable(&self, at: Instant) -> bool {
        if self.merge_key.is_none() {
            return false;
        }
        at.saturating_duration_since(self.at) <= Duration::from_millis(COALESCE_MS)
    }
}

/// 默认显式 `Send + Sync` 一下（`Instant` / `Patch` 都是）。
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<History>();
};

/// 撤销栈 + 重做栈。**只活在内存里** —— 不落盘、不跨会话。
#[derive(Debug, Default)]
pub struct History {
    undo: VecDeque<Entry>,
    redo: Vec<Entry>,
    /// 上一次 `undo` / `redo` 的时刻。它们的**下一笔**一律不合并 ——
    /// 否则新输入会被吸进"已经被撤掉的那一条"，一次 Cmd+Z 直接跨过它。
    break_at: Option<Instant>,
}

/// 记一条**值编辑**的结果（调用方拿它拼日志）
#[derive(Debug, Clone)]
pub struct Recorded {
    /// 合并进了上一条（没有新增条目）
    pub merged: bool,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    /// 还能撤几条
    pub fn undo_depth(&self) -> usize {
        self.undo.len()
    }

    /// **把历史里的命令按顺序重放到文档上**（2026-11-03 迁移期）。
    ///
    /// 谁需要它：结构编辑那条路径的 `reload()`（`ctx.presets = presets.reload()`）
    /// 会从影子树重建内存文档 —— 而值编辑**根本不写影子树**，一重建就没了。
    /// 重放让内存文档回到"磁盘 + 全部命令"的样子。
    ///
    /// 只重放值编辑（`SetValue`）；结构命令不在这个历史里（它们走 `txn`）。
    pub fn replay_into(
        &self,
        presets: &mut crate::workbench::presets::Presets,
        committed: &Committed,
    ) -> Result<(), AppError> {
        for e in self.undo.iter() {
            /* 结构编辑写的是影子文件（`reload` 会读回来），只有值编辑需要重放 */
            let fwd = match &e.action {
                Action::Value { patch, .. } => patch,
                Action::Struct(_) => continue,
            };
            let mut d = Self::baseline_draft();
            patch::apply(
                &mut d,
                committed,
                &presets.registry,
                std::slice::from_ref(fwd),
            )?;
            let staged = crate::workbench::app::storage::stage_value_edits(presets, committed, &d);
            crate::workbench::app::storage::stage_into(presets, &staged)?;
        }
        Ok(())
    }

    /// 栈顶那条**是什么时候**发生的（`None` = 空栈）。
    ///
    /// 迁移期两套栈并存：撤销必须退**最近**的那一笔，不能看"哪条栈先被问" ——
    /// 改了一个参数、又改了机型信息之后按 Cmd+Z，要退的是机型信息那一笔。
    pub fn undo_last_at(&self) -> Option<Instant> {
        self.undo.back().map(|e| e.at)
    }

    /// 重做栈顶那条的时刻（同 [`Self::undo_last_at`] 的理由）
    pub fn redo_last_at(&self) -> Option<Instant> {
        self.redo.last().map(|e| e.at)
    }

    /// 还能重做几条
    pub fn redo_depth(&self) -> usize {
        self.redo.len()
    }

    pub fn is_empty(&self) -> bool {
        self.undo.is_empty() && self.redo.is_empty()
    }

    /// **断开合并**：下一笔无论多近都另起一条。
    ///
    /// 谁需要它：`undo` / `redo` 之后（自动）、保存 / 放弃之后（`clear` 已含）、
    /// 以及测试里要模拟"停顿了一会儿"的时候 —— 测试跑得比 500ms 快得多，
    /// 不手动断就测不出"两串连续编辑是两个条目"。
    pub fn break_here(&mut self) {
        self.break_at = Some(Instant::now());
    }

    /// 全部清空（保存成功 / 放弃之后）。**不碰 `Draft`** —— 那是调用方的事
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        self.break_at = None;
    }

    /// 栈里所有条目（从旧到新）。给「改了什么」与日志用，**不许拿它当真相**
    pub fn entries(&self) -> impl Iterator<Item = &Entry> {
        self.undo.iter()
    }

    /// **记一条命令**（不碰草稿、不碰文档 —— 调用方已经应用过了）。
    ///
    /// # 为什么逆命令由调用方给
    ///
    /// 真实路径（`wb_apply_draft`）是「`patch::apply` 一次 → 算 → 落文档」：
    /// `apply_patches` 已经在草稿上正向应用并**算出了逆命令**（`Applied::inverse`，
    /// 由 `apply_one` 现算 —— 那里是唯一知道「版本层原来跟随 ⇒ 反向要发 `None`
    /// 而不是写回旧数」的地方）。如果 `record` 自己再 `apply` 一次，第二次是空操作、
    /// 逆命令算成空，撤销栈永远是空的（2026-11-03 实测：`recorded = debug: 0`）。
    ///
    /// 顺序仍是死的：
    ///
    /// ```text
    /// ① inverse 为空 ⇒ 什么都不记（无变化的编辑不该占一条撤销）
    /// ② 能合并就并进上一条（反向沿用最早那条），否则压一条新的
    /// ③ 任何新输入都清掉重做栈（经典编辑器语义）
    /// ```
    pub fn record(
        &mut self,
        label: String,
        merge_key: Option<String>,
        patch: Patch,
        inverse: Vec<Patch>,
        fields: Vec<super::txn::FieldEntry>,
    ) -> Recorded {
        if inverse.is_empty() {
            /* 无变化：不占栈，也不打断合并（连点 +1 再 -1 不该断成两条） */
            return Recorded { merged: false };
        }
        self.push_entry(label, Action::Value { patch, inverse }, merge_key, fields)
    }

    /// 记一条**结构编辑**：若干文件的前后字节（调用方已经改完盘/内存）。
    ///
    /// 没有任何字节差别时不占栈（"改了又改回去"不该留一条撤销）。
    pub fn record_struct(
        &mut self,
        label: String,
        merge_key: Option<String>,
        cmd: StructCmd,
        fields: Vec<super::txn::FieldEntry>,
    ) -> Recorded {
        if cmd.files.iter().all(|f| f.before == f.after) {
            return Recorded { merged: false };
        }
        self.push_entry(label, Action::Struct(cmd), merge_key, fields)
    }

    /// 通用的入栈：合并判据是 `merge_key` + 时间窗口，反向沿用最早那条。
    fn push_entry(
        &mut self,
        label: String,
        action: Action,
        merge_key: Option<String>,
        fields: Vec<super::txn::FieldEntry>,
    ) -> Recorded {
        self.redo.clear();
        let at = Instant::now();
        let break_here = self.break_at.take().is_some();
        let can_merge = !break_here
            && merge_key.is_some()
            && self
                .undo
                .back()
                .map(|e| e.merge_key == merge_key && e.mergable(at))
                .unwrap_or(false);

        if can_merge {
            let top = self.undo.back_mut().expect("刚判过非空");
            /* 合并：**反向沿用栈顶那条**（最早那一笔的"改前"），正向换成最新 ——
            这个组合正好是 `1 → 4` 的一条：撤一次回 1，重做一次回 4。
            中间态（1→2、2→3）不保留，这正是"一次连续编辑"的定义。 */
            merge_action(&mut top.action, action);
            top.at = at;
            top.label = label;
            merge_fields(&mut top.fields, fields);
            Recorded { merged: true }
        } else {
            self.undo.push_back(Entry {
                label,
                action,
                merge_key,
                at,
                fields,
            });
            Recorded { merged: false }
        }
    }

    /// 撤销一条：弹出并转到重做栈，返回这一条（调用方拿 `inverse` 回退文档）。
    pub fn undo(&mut self) -> Option<Entry> {
        let e = self.undo.pop_back()?;
        self.redo.push(e.clone());
        self.break_at = Some(Instant::now());
        Some(e)
    }

    /// 重做一条：返回它。调用方应用 `entry.patch` 的正向。
    pub fn redo(&mut self) -> Option<Entry> {
        let e = self.redo.pop()?;
        self.undo.push_back(e.clone());
        self.break_at = Some(Instant::now());
        Some(e)
    }
}

impl History {
    /// **基线草稿**：`Base`（打开会话 / 上次保存那份文档）与它自己比，草稿自然是空的。
    ///
    /// 撤销 / 重做要算"那一条写了哪些行"，靠的是把**反向 / 正向**应用到一个草稿上，
    /// 再用**正向同一条管线**（`stage_value_edits`）算出结果。而"当前的草稿"
    /// 在这条路上已经清空了（值编辑一落内存就把草稿层清掉），所以起点永远是空草稿。
    fn baseline_draft() -> Draft {
        Draft::default()
    }

    /// **撤销栈顶那条**要写的行（在真的撤销之前算 —— `changes` 要的是"改前"的值）。
    ///
    /// 走的是与正向**同一条管线**：把那条的反向应用到一个空草稿上，
    /// 再 `stage_value_edits`。于是撤销不会长出第二套"归并连带 / 跟随状态"的规则。
    pub fn inverse_staged(
        &self,
        presets: &crate::workbench::presets::Presets,
        committed: &Committed,
    ) -> Option<crate::workbench::app::storage::StagedEdits> {
        let e = self.undo.back()?;
        let inverse = match &e.action {
            Action::Value { inverse, .. } => inverse,
            /* 结构编辑的撤销是"把 before 字节写回文件"，不走值管线 */
            Action::Struct(_) => return None,
        };
        let mut d = Self::baseline_draft();
        if !inverse.is_empty() {
            patch::apply(&mut d, committed, &presets.registry, inverse).ok()?;
        }
        Some(crate::workbench::app::storage::stage_value_edits(
            presets, committed, &d,
        ))
    }

    /// **重做栈顶那条**要写的行（正向）
    pub fn redo_staged(
        &self,
        presets: &crate::workbench::presets::Presets,
        committed: &Committed,
    ) -> Option<crate::workbench::app::storage::StagedEdits> {
        let e = self.redo.last()?;
        let fwd = match &e.action {
            Action::Value { patch, .. } => patch,
            Action::Struct(_) => return None,
        };
        let mut d = Self::baseline_draft();
        patch::apply(
            &mut d,
            committed,
            &presets.registry,
            std::slice::from_ref(fwd),
        )
        .ok()?;
        Some(crate::workbench::app::storage::stage_value_edits(
            presets, committed, &d,
        ))
    }
}

/// 合并两份字段账：**同一格（层, 归属, 键）只留一条，`before` 保最初、`after` 换最新**。
///
/// 与 `txn` 那边 `Payload::fields` 的合并语义逐字相同 —— 两套撤销栈的「未保存 N」
/// 必须是同一个数，否则迁移期两个数字会打架。
fn merge_fields(into: &mut Vec<super::txn::FieldEntry>, more: Vec<super::txn::FieldEntry>) {
    for e in more {
        if !e.changed() {
            continue;
        }
        match into.iter_mut().find(|x| x.same_cell(&e)) {
            Some(old) => old.after = e.after,
            None => into.push(e),
        }
    }
    /* 合并之后"改回原样"的条目要剔掉 —— 否则「未保存 N」会留下一个虚假的 1 */
    into.retain(|x| x.changed());
}

/// 一条历史条目**改了几处**（去处重后还剩几个字段）。就是页头那个 N 的原料。
pub fn dirty_fields(entries: &[Entry]) -> usize {
    let mut out: Vec<&super::txn::FieldEntry> = Vec::new();
    for e in entries {
        for f in e.fields.iter().filter(|f| f.changed()) {
            if !out.iter().any(|x| x.same_cell(f)) {
                out.push(f);
            }
        }
    }
    out.len()
}

/// 值编辑的合并判据：**哪一格**（`v:A1/STANDARD:toolhead.offset.z` 这样的字符串键）。
///
/// 只认 `Patch::SetValue`；其余（可见性 / 套餐）是"一次性动作"，`None` ⇒ 永不合并。
pub fn merge_key_of(patch: &Patch) -> Option<String> {
    match patch {
        Patch::SetValue {
            level, owner, key, ..
        } => Some(super::patch::value_key_public(*level, owner, key)),
        _ => None,
    }
}

/// 结构编辑的合并判据：**哪个对象的哪个字段**（`machines/A1.toml|display`）。
///
/// 增删对象（新建版本 / 删套餐…）没有"字段"可并 ⇒ `None`。
pub fn struct_merge_key(path: &str, field: &str) -> Option<String> {
    if field.is_empty() {
        None
    } else {
        Some(format!("{path}|{field}"))
    }
}

/// 合并一条命令：**反向沿用最早（`before` / `inverse`），正向换最新**。
fn merge_action(into: &mut Action, more: Action) {
    match (into, more) {
        (Action::Value { patch, .. }, Action::Value { patch: new, .. }) => {
            *patch = new;
        }
        (Action::Struct(old), Action::Struct(new)) => {
            for f in new.files {
                match old.files.iter_mut().find(|x| x.path == f.path) {
                    Some(slot) => slot.after = f.after,
                    None => old.files.push(f),
                }
            }
        }
        /* 同一个 `merge_key` 不该跨类别撞上；真撞上了就以新的一条为准 */
        (slot, more) => *slot = more,
    }
}

/// 一条命令在界面上怎么说（撤销栈标题）。
///
/// 参数那一格用「谁 · 哪个参数」；拿不到定义时退回 key 本身（不编一个假名字）。
pub fn label_of(registry: &Registry, target: &str, patch: &Patch) -> String {
    let key = match patch {
        Patch::SetValue { key, .. } => key.as_str(),
        Patch::SetVisibility { file_id, .. } => file_id.as_str(),
        Patch::SetBundle { bundle_id, .. } => bundle_id.as_str(),
    };
    let what = registry
        .param(key)
        .map(|p| p.label.clone())
        .unwrap_or_else(|| key.to_owned());
    format!("{target} · {what}")
}

/// 一格里现在这个值（草稿优先，其次盘上）—— 日志与「改了什么」用。
///
/// 与 `patch::apply_one` 里的 `at_rest` 同口径：**草稿挂着的那条优先**，
/// 否则读注册表里那一行（并且只有版本层才有"跟随"这回事）。
pub fn current_value(
    draft: &Draft,
    registry: &Registry,
    level: super::Level,
    owner: &str,
    key: &str,
) -> Option<Value> {
    if let Some(v) = draft.pending(level, owner, key) {
        return v.clone();
    }
    let p = registry.param(key)?;
    match level {
        super::Level::Version => p.machine_variants.get(&variant_owner(owner)).cloned(),
        super::Level::Machine => p.machine_variants.get(owner).cloned(),
    }
}

/// `A1/STANDARD` → `A1:STANDARD`（`machineVariants` 的键写法）
fn variant_owner(owner: &str) -> String {
    owner.replacen('/', ":", 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::domain::testkit::Fixture;
    use crate::workbench::domain::Level;

    /// 夹具 + 一份**认得 `A1/STANDARD` 的已落盘清单**。
    ///
    /// `Committed::default()` 里一个版本都没有，`patch::validate` 会以
    /// 「版本 A1/STANDARD 不存在」整批拒绝 —— 与 `patch` 那边的 `committed()` 同源。
    fn setup() -> (Fixture, Committed) {
        (Fixture::load(), committed())
    }

    fn committed() -> Committed {
        use super::super::patch::{CatalogMachine, CommittedVersion};
        use std::collections::BTreeMap;

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
            catalog: vec![CatalogMachine {
                id: "A1".to_owned(),
                display: "A1".to_owned(),
                version_ids: vec!["STANDARD".to_owned()],
                ..Default::default()
            }],
            ..Default::default()
        }
    }

    /// 一次编辑 = **应用到草稿 + 记一条命令**（真实路径是 `wb_apply_draft` 做这两步）。
    ///
    /// 注意顺序：先 `patch::apply` 拿逆命令，再 `history.record` 记账 ——
    /// `record` 自己不再 `apply`（第二次是空操作、逆命令算成空，撤销栈永远是空的）。
    fn edit(
        h: &mut History,
        d: &mut Draft,
        c: &Committed,
        reg: &Registry,
        label: &str,
        cell: Option<String>,
        patch: Patch,
    ) {
        let applied = patch::apply(d, c, reg, std::slice::from_ref(&patch)).unwrap();
        h.record(label.to_owned(), cell, patch, applied.inverse, Vec::new());
    }

    /// 撤销一条并把反向应用回草稿
    fn undo(h: &mut History, d: &mut Draft, c: &Committed, reg: &Registry) {
        let e = h.undo().expect("有得撤");
        let inverse = match &e.action {
            Action::Value { inverse, .. } => inverse,
            Action::Struct(_) => panic!("测试里只用值编辑"),
        };
        if !inverse.is_empty() {
            patch::apply(d, c, reg, inverse).unwrap();
        }
    }

    /// 重做一条
    fn redo(h: &mut History, d: &mut Draft, c: &Committed, reg: &Registry) {
        let e = h.redo().expect("有得重做");
        let fwd = match &e.action {
            Action::Value { patch, .. } => patch,
            Action::Struct(_) => panic!("测试里只用值编辑"),
        };
        patch::apply(d, c, reg, std::slice::from_ref(fwd)).unwrap();
    }

    fn set(level: Level, owner: &str, key: &str, v: Option<Value>) -> Patch {
        Patch::SetValue {
            level,
            owner: owner.to_owned(),
            key: key.to_owned(),
            value: v,
        }
    }

    fn key(level: Level, owner: &str, k: &str) -> String {
        super::super::patch::value_key_public(level, owner, k)
    }

    /// **1 → 2 → 3 是一条还是三条**：同一格、都在窗口内 ⇒ 一条，撤一次回 1。
    #[test]
    fn a_flurry_on_one_cell_is_one_entry() {
        let (f, c) = setup();
        let reg = &f.presets.registry;
        let mut d = Draft::default();
        let mut h = History::new();
        let cell = key(Level::Version, "A1/STANDARD", "toolhead.offset.z");

        /* 先把这一格写成"自己钉着"的（初值 2）：夹具里 A1 没有裸键，
        不先钉住的话「撤完回起始态」要退到出厂默认，跟合并没关系的断言会先炸。 */
        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "钉住",
            Some(cell.clone()),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(2.0)),
            ),
        );
        assert_eq!(h.undo_depth(), 1, "钉住那一笔进了栈");
        /*
         * ★ 合并的**边界**在这里显式断开一笔：真实世界里"停顿超过窗口"会让下一串
         * 另起一条，而测试跑得比 500ms 快得多，不手动断的话钉住与 3/4/5 会被并成一条。
         * `break_here()` 就是那个断点（与撤销后第一笔同一个入口）。
         */
        h.break_here();
        let base = d.clone();

        for v in [3.0, 4.0, 5.0] {
            edit(
                &mut h,
                &mut d,
                &c,
                reg,
                "A1 · 标准版 · Z 轴偏移",
                Some(cell.clone()),
                set(
                    Level::Version,
                    "A1/STANDARD",
                    "toolhead.offset.z",
                    Some(serde_json::json!(v)),
                ),
            );
        }
        /* 3/4/5 都在同一格、都在窗口内 ⇒ **整串只占一条**：
        这是"一次连续编辑"的定义（一次撤销退回 2，而不是退回 5→4→3）。 */
        assert_eq!(h.undo_depth(), 2, "钉住 1 条 + 这一串 1 条");
        assert_eq!(
            current_value(&d, reg, Level::Version, "A1/STANDARD", "toolhead.offset.z"),
            Some(serde_json::json!(5.0)),
            "草稿里是最后一次的值"
        );

        /* 撤销一次 ⇒ 回到这一串的"改前"（2），不是中间态 */
        undo(&mut h, &mut d, &c, reg);
        assert_eq!(
            d.values, base.values,
            "撤一次应当退回这一串的起点（2），而不是 3 或 4"
        );
    }

    /// 换格 ⇒ 另一条；撤一次只退掉最近那次
    #[test]
    fn switching_cells_breaks_the_entry() {
        let (f, c) = setup();
        let reg = &f.presets.registry;
        let mut d = Draft::default();
        let mut h = History::new();

        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "A · Z",
            Some(key(Level::Version, "A1/STANDARD", "toolhead.offset.z")),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(9)),
            ),
        );
        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "A · X",
            Some(key(Level::Version, "A1/STANDARD", "toolhead.offset.x")),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(5)),
            ),
        );

        assert_eq!(h.undo_depth(), 2, "两个参数是两次操作");

        undo(&mut h, &mut d, &c, reg);
        assert_eq!(
            d.pending(Level::Version, "A1/STANDARD", "toolhead.offset.x"),
            None,
            "X 那一笔被退掉了（回到它没被改过的样子）"
        );
        assert!(
            d.pending(Level::Version, "A1/STANDARD", "toolhead.offset.z")
                .is_some(),
            "Z 那一笔还在 —— 撤一次只退最近一次操作"
        );
    }

    /// 撤销之后的**下一笔不许并进那条已经被撤掉的**（否则一次 Cmd+Z 跨过它）
    #[test]
    fn an_edit_after_undo_starts_a_new_entry() {
        let (f, c) = setup();
        let reg = &f.presets.registry;
        let mut d = Draft::default();
        let mut h = History::new();
        let cell = key(Level::Version, "A1/STANDARD", "toolhead.offset.z");

        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "第一笔",
            Some(cell.clone()),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(9)),
            ),
        );
        undo(&mut h, &mut d, &c, reg);

        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "撤销后再改",
            Some(cell),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(7)),
            ),
        );
        assert_eq!(h.undo_depth(), 1, "只剩新那一笔（旧那条已经被撤走了）");
        assert_eq!(h.redo_depth(), 0, "新输入必须清掉重做栈 —— 经典编辑器语义");
    }

    /// 一次性动作（`cell: None`，例如点「跟随基底」）不与前后任何一笔合并
    #[test]
    fn a_one_shot_action_never_merges() {
        let (f, c) = setup();
        let reg = &f.presets.registry;
        let mut d = Draft::default();
        let mut h = History::new();

        /* 一次性动作在这里用"写一个值 + `cell: None`"代表（真实里是点「跟随基底」/
        「保存值」/「出厂」）。要点不是它写什么，而是**它永不与别人合并**。 */
        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "一次性 1",
            None,
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(6.0)),
            ),
        );
        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "一次性 2",
            None,
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(7.0)),
            ),
        );
        assert_eq!(h.undo_depth(), 2, "两次一次性动作 → 两条");
    }

    /// 没有实际变化的编辑**不占撤销栈**
    #[test]
    fn a_no_op_edit_takes_no_entry() {
        let (f, c) = setup();
        let reg = &f.presets.registry;
        let mut d = Draft::default();
        let mut h = History::new();

        /* 写回"它本来就是的那个值" —— 注册表里 A1:STANDARD 的 offset.x 是 -1 */
        let before = current_value(
            &Draft::default(),
            reg,
            Level::Version,
            "A1/STANDARD",
            "toolhead.offset.x",
        )
        .unwrap();
        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "同值",
            Some(key(Level::Version, "A1/STANDARD", "toolhead.offset.x")),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(before),
            ),
        );
        assert_eq!(h.undo_depth(), 0, "无变化不许占一条");
        assert!(d.is_clean());
    }

    /// 重做把正向重新走一遍；撤 → 重做 仍然对称
    #[test]
    fn undo_and_redo_are_symmetric() {
        let (f, c) = setup();
        let reg = &f.presets.registry;
        let mut d = Draft::default();
        let mut h = History::new();

        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "钉住",
            Some(key(Level::Version, "A1/STANDARD", "toolhead.offset.z")),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(8.0)),
            ),
        );
        h.break_here();
        let start = d.clone();

        edit(
            &mut h,
            &mut d,
            &c,
            reg,
            "改值",
            Some(key(Level::Version, "A1/STANDARD", "toolhead.offset.z")),
            set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.z",
                Some(serde_json::json!(12.0)),
            ),
        );

        undo(&mut h, &mut d, &c, reg);
        assert_eq!(
            d.values, start.values,
            "撤完回到这一条的起点：{:?}",
            d.values
        );

        redo(&mut h, &mut d, &c, reg);
        assert_eq!(
            d.pending(Level::Version, "A1/STANDARD", "toolhead.offset.z"),
            Some(&Some(serde_json::json!(12.0))),
            "重做之后回到改完的那个值"
        );
    }
}
