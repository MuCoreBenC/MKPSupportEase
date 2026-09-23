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
//! # 哪些手势不进撤销栈
//!
//! | 手势 | 可撤销 | 兜底 |
//! |---|---|---|
//! | 改值、改名、移动、归档、还原、新建、克隆、套餐、可见性、BBS | ✅ | —— |
//! | [`Patch::PurgeVersion`] | ❌ | 二次确认 + 回收站（`.trash/`） |
//! | [`Patch::MarkBuilt`] | ❌ | 它是生成的记录，不是编辑；撤销一条"生成过"没有意义 |
//!
//! 刻意**不做"有些删除能撤销、有些不能"**：草稿里新建还没保存的版本，删了确实能从
//! 草稿里恢复；已落盘的不能。但"看情况"的撤销比"从来不能"更糟 ——
//! 用户点撤销之前得先想清楚这个版本保存过没有。所以一条规则：**删除不进撤销栈**。
//!
//! # 应用顺序：先结构，再值
//!
//! 反过来的话，同一批里"新建一个版本 + 给它写三个值"会把那三个值丢掉 ——
//! 写值的时候那个版本还不存在。[`tests::values_land_on_versions_created_in_the_same_batch`]
//! 盯着这件事。
//!
//! # 非法 patch 拒绝整批
//!
//! 部分应用会留下一个谁也说不清的中间状态：前三条生效了、第四条没有，
//! 而界面上只看到"保存失败"。所以先全部校验，再全部应用。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::workbench::upstream::Registry;

use super::layer::{Level, Overrides};

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

/// 草稿里新建的版本。没保存之前它只存在于草稿里
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DraftVersion {
    pub uid: String,
    pub machine_id: String,
    /// 落盘用的 id（进文件名，所以只能是 ASCII）。**显示名是 `name`，中文在那里**
    pub version_id: String,
    pub name: String,
}

/// 写操作的载荷（doc §4.1）。
///
/// 两条语义写死：
/// - **`SetValue` 必须带 `level`**，不许由后端猜；
/// - **`value: None` = 删键 = 挂回继承**，不是"值设成空"。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Patch {
    SetValue {
        level: Level,
        /// `Machine` 时是机型 id，`Version` 时是版本 uid
        owner: String,
        key: String,
        /// `None` = 删键 = 挂回继承
        value: Option<Value>,
    },
    CloneVersion {
        from_uid: String,
        name: String,
    },
    NewVersion {
        machine_id: String,
        name: String,
    },
    RenameVersion {
        uid: String,
        name: String,
    },
    MoveVersion {
        uid: String,
        to_machine_id: String,
    },
    ArchiveVersion {
        uid: String,
    },
    RestoreVersion {
        uid: String,
    },
    PurgeVersion {
        uid: String,
    },
    /// `None` = 挂回继承机型默认；`Some` = 本版本独立一份
    SetBbs {
        uid: String,
        list: Option<Vec<String>>,
    },
    SetVisibility {
        file_id: String,
        visibility: Visibility,
    },
    SetBundle {
        bundle_id: String,
        presets: Vec<String>,
        bbs: Vec<String>,
    },
    MarkBuilt {
        uids: Vec<String>,
        stamp: String,
        /// uid → 生成时的配方指纹
        fingerprints: BTreeMap<String, String>,
    },
}

impl Patch {
    /// 这条 patch 能不能进撤销栈。见模块文档那张表
    pub fn is_undoable(&self) -> bool {
        !matches!(self, Patch::PurgeVersion { .. } | Patch::MarkBuilt { .. })
    }

    /// 是不是结构操作。**应用时结构先于值**
    fn is_structural(&self) -> bool {
        !matches!(self, Patch::SetValue { .. })
    }
}

/// 整本草稿：**只存"改了什么"，不存整本**（doc §4.3）。
///
/// 存整本的话，上游数据一变（比如上游改了某个参数的出厂默认），
/// 草稿会把旧值糊回去 —— 而那看起来就像"我的改动莫名其妙回来了"
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Draft {
    /// `"m:A1:toolhead.offset.x"` / `"v:A1/STANDARD:toolhead.offset.x"`。
    /// **值为 `None` 表示"这一层的这个键被删了"**，与"没有这一条草稿"不同
    pub values: BTreeMap<String, Option<Value>>,
    pub added: Vec<DraftVersion>,
    pub renamed: BTreeMap<String, String>,
    pub moved: BTreeMap<String, String>,
    pub archived: BTreeMap<String, bool>,
    pub purged: Vec<String>,
    pub bbs: BTreeMap<String, Option<Vec<String>>>,
    pub visibility: BTreeMap<String, Visibility>,
    pub bundles: BTreeMap<String, BundleEdit>,
    pub built: BTreeMap<String, BuiltRecord>,
    /// 新建版本的序号。只增不减 —— 减了会让新 uid 撞上旧 uid
    pub seq: u64,
}

impl Draft {
    /// 未保存的改动有几处。**结构操作各算一处** —— 它们和改一个值一样要被保存
    pub fn dirty_count(&self) -> usize {
        self.values.len()
            + self.added.len()
            + self.renamed.len()
            + self.moved.len()
            + self.archived.len()
            + self.purged.len()
            + self.bbs.len()
            + self.visibility.len()
            + self.bundles.len()
            + self.built.len()
    }

    pub fn is_clean(&self) -> bool {
        self.dirty_count() == 0
    }

    /// 某一层在草稿里挂着的值改动
    pub fn pending(&self, level: Level, owner: &str, key: &str) -> Option<&Option<Value>> {
        self.values.get(&value_key(level, owner, key))
    }

    /// 丢掉指向已经不存在的版本的草稿条目（doc §15 那条）。
    ///
    /// **不让整本草稿失效**：上游删掉一个版本之后，用户在别的九个版本上的改动
    /// 没有任何理由跟着一起没了。返回给界面的提示句
    pub fn prune(&mut self, committed: &Committed) -> Vec<String> {
        let alive: BTreeSet<&str> = committed
            .versions
            .keys()
            .map(String::as_str)
            .chain(self.added.iter().map(|v| v.uid.as_str()))
            .collect();
        // added 里的 uid 也算活的，所以先复制一份再借
        let alive: BTreeSet<String> = alive.into_iter().map(str::to_owned).collect();

        let mut gone: BTreeSet<String> = BTreeSet::new();

        self.values.retain(|k, _| match parse_value_key(k) {
            Some((Level::Version, owner, _)) => {
                let ok = alive.contains(owner);
                if !ok {
                    gone.insert(owner.to_owned());
                }
                ok
            }
            Some((Level::Machine, owner, _)) => {
                let ok = committed.machine_ids.contains(owner);
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
        });

        // 闭包没法对值类型泛化，所以这里用一个自由函数（见本文件末尾的 `retain_alive`）
        retain_alive(&mut self.renamed, &alive, &mut gone);
        retain_alive(&mut self.moved, &alive, &mut gone);
        retain_alive(&mut self.archived, &alive, &mut gone);
        retain_alive(&mut self.bbs, &alive, &mut gone);
        retain_alive(&mut self.built, &alive, &mut gone);
        self.purged.retain(|uid| alive.contains(uid));

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
    /// 机型 id → 机型基底
    pub machines: BTreeMap<String, Overrides>,
    /// 版本 uid → 版本
    pub versions: BTreeMap<String, CommittedVersion>,
    pub visibility: BTreeMap<String, Visibility>,
    pub bundles: BTreeMap<String, BundleEdit>,
    pub built: BTreeMap<String, BuiltRecord>,
    /// 上游有哪些机型。校验 `owner` 与 `to_machine_id` 用
    pub machine_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Default)]
pub struct CommittedVersion {
    pub machine_id: String,
    pub version_id: String,
    pub name: String,
    pub overrides: Overrides,
    pub archived: bool,
    /// `None` = 继承机型默认的那一份 BBS
    pub bbs: Option<Vec<String>>,
    /// **上游机型清单里还有这一版吗。**
    ///
    /// 为 true 时[`Patch::PurgeVersion`]会被拒：版本清单是上游的，删掉我们的文件
    /// 只是"这一版没有自有改动了"，它照样出现在树上。那种情况下该做的是**归档**。
    /// 不拦的话，用户点了删除、版本还在，而且没有任何提示说为什么
    pub declared_upstream: bool,
}

/// 应用的结果
#[derive(Debug, Clone)]
pub struct Applied {
    /// 撤销这次操作要提交的 patches。空 = 这次操作不可撤销
    pub inverse: Vec<Patch>,
    /// 能不能进撤销栈。**为 false 时界面不该给出撤销按钮** ——
    /// 给一个按下去没反应的按钮比没有按钮更糟
    pub undoable: bool,
    /// 给界面的提示句（没改成任何东西的 patch 会在这里说明）
    pub notices: Vec<String>,
}

/// 值改动在草稿里的键。`"m:A1:toolhead.offset.x"` / `"v:A1/STANDARD:toolhead.offset.x"`
///
/// 三段用 `:` 分隔是安全的：机型 id 与版本 id 走 `store::validate_id` 的白名单
/// （字母数字 `-` `_` `.`，没有冒号），参数 key 也只有点号
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
    validate(draft, committed, registry, patches)?;

    let undoable = patches.iter().all(Patch::is_undoable);
    let mut inverse: Vec<Patch> = Vec::new();
    let mut notices: Vec<String> = Vec::new();

    // 先结构 —— 否则同一批里"新建版本 + 给它写值"的值会掉在地上
    for p in patches.iter().filter(|p| p.is_structural()) {
        apply_one(draft, committed, p, &mut inverse, &mut notices);
    }
    for p in patches.iter().filter(|p| !p.is_structural()) {
        apply_one(draft, committed, p, &mut inverse, &mut notices);
    }

    // 反向要**倒着执行**才能回到原状：正向 A→B→C，反向是 C⁻¹→B⁻¹→A⁻¹
    inverse.reverse();

    Ok(Applied {
        inverse: if undoable { inverse } else { Vec::new() },
        undoable,
        notices,
    })
}

/// 整批校验。这里只判"能不能做"，不改任何东西
fn validate(
    draft: &Draft,
    committed: &Committed,
    registry: &Registry,
    patches: &[Patch],
) -> Result<(), AppError> {
    // 同一批里新建的版本也算存在 —— 否则"新建 + 立刻改名"会被自己拒掉
    let mut will_exist: BTreeSet<String> = committed
        .versions
        .keys()
        .cloned()
        .chain(draft.added.iter().map(|v| v.uid.clone()))
        .collect();
    let mut seq = draft.seq;
    for p in patches {
        if matches!(p, Patch::NewVersion { .. } | Patch::CloneVersion { .. }) {
            seq += 1;
            will_exist.insert(new_uid(seq));
        }
    }

    let known_uid = |uid: &String| -> Result<(), AppError> {
        if will_exist.contains(uid) {
            Ok(())
        } else {
            Err(AppError::not_found(format!("版本 {uid} 不存在"))
                .with_detail("整批改动已拒绝，草稿没有变"))
        }
    };
    let known_machine = |id: &String| -> Result<(), AppError> {
        if committed.machine_ids.contains(id) {
            Ok(())
        } else {
            Err(AppError::not_found(format!("机型 {id} 不存在"))
                .with_detail("整批改动已拒绝，草稿没有变"))
        }
    };

    for p in patches {
        match p {
            Patch::SetValue {
                level, owner, key, ..
            } => {
                if registry.param(key).is_none() {
                    return Err(AppError::invalid_argument(format!("字段定义里没有 {key}"))
                        .with_detail("整批改动已拒绝，草稿没有变"));
                }
                match level {
                    Level::Machine => known_machine(owner)?,
                    Level::Version => known_uid(owner)?,
                }
            }
            Patch::NewVersion { machine_id, name } => {
                known_machine(machine_id)?;
                non_blank(name, "版本名")?;
            }
            Patch::CloneVersion { from_uid, name } => {
                known_uid(from_uid)?;
                non_blank(name, "版本名")?;
            }
            Patch::RenameVersion { uid, name } => {
                known_uid(uid)?;
                non_blank(name, "版本名")?;
            }
            // **移动到不存在的机型当没移**（doc §15）：那会让这个版本从树上整个消失，
            // 比不动更糟。所以这里不拒绝整批，留给 apply_one 记一条提示
            Patch::MoveVersion { uid, .. } => known_uid(uid)?,
            // **删除只对「上游不再有的版本」与「草稿里新建的」开放。**
            // 版本清单是上游的：删掉我们的文件只是"这一版没有自有改动了"，
            // 它照样出现在树上。那种情况下该做的是归档
            Patch::PurgeVersion { uid } => {
                known_uid(uid)?;
                if committed
                    .versions
                    .get(uid)
                    .is_some_and(|v| v.declared_upstream)
                {
                    return Err(AppError::invalid_argument(format!(
                        "{uid} 是上游机型清单里的版本，删不掉"
                    ))
                    .with_detail(
                        "版本清单由上游维护，删掉我们的文件它照样在树上。\
                         不想交付这一版的话用「归档」",
                    ));
                }
            }
            Patch::ArchiveVersion { uid }
            | Patch::RestoreVersion { uid }
            | Patch::SetBbs { uid, .. } => known_uid(uid)?,
            Patch::SetVisibility { file_id, .. } => non_blank(file_id, "文件 id")?,
            Patch::SetBundle { bundle_id, .. } => non_blank(bundle_id, "套餐 id")?,
            Patch::MarkBuilt { uids, .. } => {
                for uid in uids {
                    known_uid(uid)?;
                }
            }
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

fn new_uid(seq: u64) -> String {
    format!("new-{seq}")
}

fn apply_one(
    draft: &mut Draft,
    committed: &Committed,
    patch: &Patch,
    inverse: &mut Vec<Patch>,
    notices: &mut Vec<String>,
) {
    match patch {
        Patch::SetValue {
            level,
            owner,
            key,
            value,
        } => {
            let before = own_value(draft, committed, *level, owner, key);
            if &before == value {
                return; // 没变化：不记草稿、不产反向，脏计数也不该涨
            }
            let vk = value_key(*level, owner, key);
            let saved = committed_own(committed, *level, owner, key);
            if &saved == value {
                // 改回了已落盘的那个值 = 这一处不再是改动。**从草稿里拿掉而不是记一条**，
                // 否则"改了又改回来"会让保存按钮一直亮着
                draft.values.remove(&vk);
            } else {
                draft.values.insert(vk, value.clone());
            }
            inverse.push(Patch::SetValue {
                level: *level,
                owner: owner.clone(),
                key: key.clone(),
                value: before,
            });
        }

        Patch::NewVersion { machine_id, name } => {
            draft.seq += 1;
            let uid = new_uid(draft.seq);
            draft.added.push(DraftVersion {
                uid: uid.clone(),
                machine_id: machine_id.clone(),
                // 进文件名，所以是 ASCII；中文显示名在 name 里
                version_id: format!("NEW{}", draft.seq),
                name: name.clone(),
            });
            inverse.push(Patch::PurgeVersion { uid });
        }

        Patch::CloneVersion { from_uid, name } => {
            draft.seq += 1;
            let uid = new_uid(draft.seq);
            let machine_id = machine_of(draft, committed, from_uid).unwrap_or_default();
            draft.added.push(DraftVersion {
                uid: uid.clone(),
                machine_id,
                version_id: format!("NEW{}", draft.seq),
                name: name.clone(),
            });
            // 复制自有覆盖。**只复制"自有的"**，继承来的不复制 ——
            // 复制了就等于把新版本和源版本一起从继承里摘出来
            let src: Vec<(String, Value)> = own_overrides(draft, committed, from_uid);
            for (k, v) in src {
                draft
                    .values
                    .insert(value_key(Level::Version, &uid, &k), Some(v));
            }
            if let Some(list) = own_bbs(draft, committed, from_uid) {
                draft.bbs.insert(uid.clone(), Some(list));
            }
            inverse.push(Patch::PurgeVersion { uid });
        }

        Patch::RenameVersion { uid, name } => {
            let before = current_name(draft, committed, uid);
            if before.as_deref() == Some(name.as_str()) {
                return;
            }
            if let Some(v) = draft.added.iter_mut().find(|v| &v.uid == uid) {
                v.name = name.clone();
            } else if committed.versions.get(uid).map(|v| v.name.as_str()) == Some(name.as_str()) {
                draft.renamed.remove(uid);
            } else {
                draft.renamed.insert(uid.clone(), name.clone());
            }
            if let Some(old) = before {
                inverse.push(Patch::RenameVersion {
                    uid: uid.clone(),
                    name: old,
                });
            }
        }

        Patch::MoveVersion { uid, to_machine_id } => {
            if !committed.machine_ids.contains(to_machine_id) {
                // doc §15：当没移。整个版本从树上消失比不动更糟
                notices.push(format!(
                    "机型 {to_machine_id} 不存在，{uid} 没有移动"
                ));
                return;
            }
            let before = machine_of(draft, committed, uid);
            if before.as_deref() == Some(to_machine_id.as_str()) {
                return;
            }
            if let Some(v) = draft.added.iter_mut().find(|v| &v.uid == uid) {
                v.machine_id = to_machine_id.clone();
            } else if committed.versions.get(uid).map(|v| v.machine_id.as_str())
                == Some(to_machine_id.as_str())
            {
                draft.moved.remove(uid);
            } else {
                draft.moved.insert(uid.clone(), to_machine_id.clone());
            }
            // **uid 不跟着变**（doc §3.5）—— 所以草稿里那些
            // `v:{uid}:{key}` 一条都不用改名。上一版靠事后搬运，这一版从根上避免
            if let Some(old) = before {
                inverse.push(Patch::MoveVersion {
                    uid: uid.clone(),
                    to_machine_id: old,
                });
            }
        }

        Patch::ArchiveVersion { uid } => set_archived(draft, committed, uid, true, inverse),
        Patch::RestoreVersion { uid } => set_archived(draft, committed, uid, false, inverse),

        Patch::PurgeVersion { uid } => {
            // 草稿里新建的：直接从草稿里拿掉，连它的值一起
            if let Some(at) = draft.added.iter().position(|v| &v.uid == uid) {
                draft.added.remove(at);
                let prefix = format!("v:{uid}:");
                draft.values.retain(|k, _| !k.starts_with(&prefix));
                draft.bbs.remove(uid);
                draft.renamed.remove(uid);
                draft.moved.remove(uid);
                draft.archived.remove(uid);
                return;
            }
            if !draft.purged.contains(uid) {
                draft.purged.push(uid.clone());
            }
        }

        Patch::SetBbs { uid, list } => {
            let before = own_bbs_slot(draft, committed, uid);
            if &before == list {
                return;
            }
            let saved = committed.versions.get(uid).and_then(|v| v.bbs.clone());
            if &saved == list {
                draft.bbs.remove(uid);
            } else {
                draft.bbs.insert(uid.clone(), list.clone());
            }
            inverse.push(Patch::SetBbs {
                uid: uid.clone(),
                list: before,
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
            let saved = committed.bundles.get(bundle_id).cloned().unwrap_or_default();
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

        Patch::MarkBuilt {
            uids,
            stamp,
            fingerprints,
        } => {
            for uid in uids {
                draft.built.insert(
                    uid.clone(),
                    BuiltRecord {
                        stamp: stamp.clone(),
                        fingerprint: fingerprints.get(uid).cloned().unwrap_or_default(),
                    },
                );
            }
            // 不产反向：撤销一条「生成过」没有意义（见模块文档那张表）
        }
    }
}

fn set_archived(
    draft: &mut Draft,
    committed: &Committed,
    uid: &str,
    want: bool,
    inverse: &mut Vec<Patch>,
) {
    let saved = committed.versions.get(uid).map(|v| v.archived).unwrap_or(false);
    let before = draft.archived.get(uid).copied().unwrap_or(saved);
    if before == want {
        return;
    }
    if saved == want {
        draft.archived.remove(uid);
    } else {
        draft.archived.insert(uid.to_owned(), want);
    }
    inverse.push(if before {
        Patch::ArchiveVersion {
            uid: uid.to_owned(),
        }
    } else {
        Patch::RestoreVersion {
            uid: uid.to_owned(),
        }
    });
}

/// 这一层**现在**自己写着什么（草稿优先）。`None` = 这一层没有这个键 = 继承
fn own_value(
    draft: &Draft,
    committed: &Committed,
    level: Level,
    owner: &str,
    key: &str,
) -> Option<Value> {
    if let Some(pending) = draft.values.get(&value_key(level, owner, key)) {
        return pending.clone();
    }
    committed_own(committed, level, owner, key)
}

/// 这一层**已落盘**的那一份自有值
fn committed_own(
    committed: &Committed,
    level: Level,
    owner: &str,
    key: &str,
) -> Option<Value> {
    match level {
        Level::Machine => committed.machines.get(owner)?.get(key).cloned(),
        Level::Version => committed.versions.get(owner)?.overrides.get(key).cloned(),
    }
}

/// 一个版本现在的全部自有覆盖（草稿叠在落盘之上）
fn own_overrides(draft: &Draft, committed: &Committed, uid: &str) -> Vec<(String, Value)> {
    let mut out: Overrides = committed
        .versions
        .get(uid)
        .map(|v| v.overrides.clone())
        .unwrap_or_default();
    let prefix = format!("v:{uid}:");
    for (k, v) in &draft.values {
        if let Some(key) = k.strip_prefix(&prefix) {
            match v {
                Some(val) => {
                    out.insert(key.to_owned(), val.clone());
                }
                None => {
                    out.remove(key);
                }
            }
        }
    }
    out.into_iter().collect()
}

/// `Some(list)` = 这个版本自己有一份 BBS；`None` = 继承机型默认
fn own_bbs(draft: &Draft, committed: &Committed, uid: &str) -> Option<Vec<String>> {
    own_bbs_slot(draft, committed, uid)
}

fn own_bbs_slot(draft: &Draft, committed: &Committed, uid: &str) -> Option<Vec<String>> {
    if let Some(pending) = draft.bbs.get(uid) {
        return pending.clone();
    }
    committed.versions.get(uid).and_then(|v| v.bbs.clone())
}

fn current_name(draft: &Draft, committed: &Committed, uid: &str) -> Option<String> {
    if let Some(v) = draft.added.iter().find(|v| v.uid == uid) {
        return Some(v.name.clone());
    }
    if let Some(n) = draft.renamed.get(uid) {
        return Some(n.clone());
    }
    committed.versions.get(uid).map(|v| v.name.clone())
}

fn machine_of(draft: &Draft, committed: &Committed, uid: &str) -> Option<String> {
    if let Some(v) = draft.added.iter().find(|v| v.uid == uid) {
        return Some(v.machine_id.clone());
    }
    if let Some(m) = draft.moved.get(uid) {
        return Some(m.clone());
    }
    committed.versions.get(uid).map(|v| v.machine_id.clone())
}

/// 只留下 uid 还活着的条目，被丢掉的记进 `gone`。
///
/// 写成自由函数而不是闭包：`Draft` 那几张表的值类型各不相同
/// （`String` / `bool` / `Option<Vec<String>>` / `BuiltRecord`），闭包没法对值类型泛化
fn retain_alive<V>(
    map: &mut BTreeMap<String, V>,
    alive: &BTreeSet<String>,
    gone: &mut BTreeSet<String>,
) {
    map.retain(|uid, _| {
        let ok = alive.contains(uid);
        if !ok {
            gone.insert(uid.clone());
        }
        ok
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 两个参数的最小字段定义。patch 层只用它判"这个 key 存在吗"
    fn registry() -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        let p = |key: &str, order: f64| {
            serde_json::json!({
                "key": key, "configKey": "X", "tomlKey": key, "jsonKey": key,
                "label": key, "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 4,
                "scope": "universal", "section": "toolhead",
                "layout": { "order": order, "sectionId": "s1" }
            })
        };
        let params = serde_json::json!({
            "params": [p("toolhead.offset.x", 1.0), p("toolhead.offset.z", 2.0)],
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
        let w = |rel: &str, v: &serde_json::Value| {
            crate::fsx::atomic::atomic_write_json(&d.path().join(rel), v).unwrap()
        };
        w("content/param_registry.json", &params);
        w("content/layout_schema.json", &layout);
        let r = Registry::load_from(d.path()).unwrap();
        (d, r)
    }

    /// A1 有基底（offset.z = 1.1）与两个版本：
    /// `A1/STANDARD` 上游还声明着，`A1/OLD` 是上游已经删掉的孤儿（只有它能被删）
    fn committed() -> Committed {
        let mut machines = BTreeMap::new();
        machines.insert(
            "A1".to_owned(),
            [("toolhead.offset.z".to_owned(), serde_json::json!(1.1))]
                .into_iter()
                .collect::<Overrides>(),
        );
        machines.insert("P1S".to_owned(), Overrides::new());

        let mut versions = BTreeMap::new();
        versions.insert(
            "A1/STANDARD".to_owned(),
            CommittedVersion {
                machine_id: "A1".to_owned(),
                version_id: "STANDARD".to_owned(),
                name: "标准版".to_owned(),
                overrides: [("toolhead.offset.x".to_owned(), serde_json::json!(-1))]
                    .into_iter()
                    .collect(),
                archived: false,
                bbs: None,
                declared_upstream: true,
            },
        );
        versions.insert(
            "A1/OLD".to_owned(),
            CommittedVersion {
                machine_id: "A1".to_owned(),
                version_id: "OLD".to_owned(),
                name: "上游已删的老版本".to_owned(),
                declared_upstream: false,
                ..Default::default()
            },
        );

        Committed {
            machines,
            versions,
            machine_ids: ["A1", "P1S"].into_iter().map(str::to_owned).collect(),
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
            &[set(Level::Version, "A1/STANDARD", "toolhead.offset.z", Some(serde_json::json!(2)))],
        )
        .unwrap();

        assert_eq!(
            out.inverse,
            vec![set(Level::Version, "A1/STANDARD", "toolhead.offset.z", None)],
            "反向该是删键，不是「设回 1.1」"
        );
        assert!(out.undoable);
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
            &[set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!(7)))],
        )
        .unwrap();
        assert_eq!(
            out.inverse,
            vec![set(
                Level::Version,
                "A1/STANDARD",
                "toolhead.offset.x",
                Some(serde_json::json!(-1))
            )]
        );
    }

    /// `value: None` = 删键 = 挂回继承。**与"写空串"是两回事**
    #[test]
    fn none_deletes_the_key_while_empty_string_is_a_value() {
        let (_d, reg) = registry();
        let c = committed();

        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(Level::Version, "A1/STANDARD", "toolhead.offset.x", None)],
        )
        .unwrap();
        assert_eq!(
            draft.pending(Level::Version, "A1/STANDARD", "toolhead.offset.x"),
            Some(&None),
            "草稿里记的是「这个键被删了」"
        );

        let mut draft = Draft::default();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!("")))],
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
            &[set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!(-1)))],
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
            &[set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!(7)))],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 1);

        apply(
            &mut draft,
            &c,
            &reg,
            &[set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!(-1)))],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 0, "改回去就不该再算一处未保存改动");
    }

    /// **先结构再值**：同一批里新建版本 + 给它写值，值不能掉在地上
    #[test]
    fn values_land_on_versions_created_in_the_same_batch() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        // 刻意把 SetValue 放在 NewVersion **前面**，证明顺序由实现保证，不靠调用方
        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[
                set(Level::Version, "new-1", "toolhead.offset.x", Some(serde_json::json!(3))),
                Patch::NewVersion {
                    machine_id: "A1".to_owned(),
                    name: "快速版".to_owned(),
                },
            ],
        )
        .unwrap();

        assert_eq!(draft.added.len(), 1);
        assert_eq!(draft.added[0].uid, "new-1");
        assert_eq!(
            draft.pending(Level::Version, "new-1", "toolhead.offset.x"),
            Some(&Some(serde_json::json!(3))),
            "值要落在同一批里刚建出来的版本上"
        );
        assert_eq!(out.inverse.len(), 2);
    }

    /// 克隆**只复制自有覆盖**。继承来的不复制 ——
    /// 复制了就等于把新版本从继承里摘出来，以后改机型基底它不跟着变
    #[test]
    fn cloning_copies_only_the_own_overrides() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &reg,
            &[Patch::CloneVersion {
                from_uid: "A1/STANDARD".to_owned(),
                name: "标准版 副本".to_owned(),
            }],
        )
        .unwrap();

        let uid = draft.added[0].uid.clone();
        assert_eq!(
            draft.pending(Level::Version, &uid, "toolhead.offset.x"),
            Some(&Some(serde_json::json!(-1))),
            "自有的那一项要复制"
        );
        assert!(
            draft
                .pending(Level::Version, &uid, "toolhead.offset.z")
                .is_none(),
            "继承来的 offset.z 不该被复制成自有"
        );
    }

    /// 移动之后 **uid 不变**，所以草稿里那些值一条都不用改名（doc §3.5）
    #[test]
    fn moving_keeps_the_uid_so_draft_values_stay_put() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &reg,
            &[
                set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!(9))),
                Patch::MoveVersion {
                    uid: "A1/STANDARD".to_owned(),
                    to_machine_id: "P1S".to_owned(),
                },
            ],
        )
        .unwrap();

        assert_eq!(draft.moved.get("A1/STANDARD"), Some(&"P1S".to_owned()));
        assert_eq!(
            draft.pending(Level::Version, "A1/STANDARD", "toolhead.offset.x"),
            Some(&Some(serde_json::json!(9))),
            "值还挂在同一个 uid 上"
        );
    }

    /// 移到不存在的机型 = 当没移 + 一条提示（doc §15）。
    /// 拒绝整批也不对 —— 那会让同一批里别的改动一起失败
    #[test]
    fn moving_to_a_missing_machine_is_a_no_op_with_a_notice() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[Patch::MoveVersion {
                uid: "A1/STANDARD".to_owned(),
                to_machine_id: "KOBRA".to_owned(),
            }],
        )
        .unwrap();

        assert!(draft.moved.is_empty());
        assert_eq!(out.notices.len(), 1);
        assert!(out.notices[0].contains("KOBRA"));
    }

    /// 归档 / 还原互为反向
    #[test]
    fn archive_and_restore_are_inverses() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[Patch::ArchiveVersion {
                uid: "A1/STANDARD".to_owned(),
            }],
        )
        .unwrap();
        assert_eq!(draft.archived.get("A1/STANDARD"), Some(&true));
        assert_eq!(
            out.inverse,
            vec![Patch::RestoreVersion {
                uid: "A1/STANDARD".to_owned()
            }]
        );

        apply(&mut draft, &c, &reg, &out.inverse).unwrap();
        assert_eq!(draft.dirty_count(), 0, "还原之后回到干净");
    }

    /// **删除不可撤销**，而且不可撤销时不给出半截反向。
    /// 只有上游已经删掉的那一版能被删（见 `committed()` 的 `A1/OLD`）
    #[test]
    fn purge_is_not_undoable_and_returns_no_inverse() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[Patch::PurgeVersion {
                uid: "A1/OLD".to_owned(),
            }],
        )
        .unwrap();
        assert!(!out.undoable);
        assert!(out.inverse.is_empty(), "不该给一个按下去没反应的撤销");
        assert_eq!(draft.purged, vec!["A1/OLD"]);
    }

    /// 上游还声明着的版本**删不掉**：版本清单是上游的，
    /// 删掉我们的文件它照样在树上 —— 那种情况下该做的是归档
    #[test]
    fn purging_an_upstream_declared_version_is_refused() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();
        let e = apply(
            &mut draft,
            &c,
            &reg,
            &[Patch::PurgeVersion {
                uid: "A1/STANDARD".to_owned(),
            }],
        )
        .unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::InvalidArgument);
        assert!(e.detail.unwrap_or_default().contains("归档"));
        assert!(draft.is_clean(), "拒绝了就不许留下痕迹");
    }

    /// 删掉草稿里新建的版本，连它的值一起走干净 —— 留下孤儿值会让脏计数永远降不下来
    #[test]
    fn purging_a_draft_version_takes_its_values_with_it() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &reg,
            &[Patch::NewVersion {
                machine_id: "A1".to_owned(),
                name: "新版".to_owned(),
            }],
        )
        .unwrap();
        let uid = draft.added[0].uid.clone();
        apply(
            &mut draft,
            &c,
            &reg,
            &[set(Level::Version, &uid, "toolhead.offset.x", Some(serde_json::json!(3)))],
        )
        .unwrap();
        assert_eq!(draft.dirty_count(), 2);

        apply(&mut draft, &c, &reg, &[Patch::PurgeVersion { uid }]).unwrap();
        assert_eq!(draft.dirty_count(), 0, "版本和它的值都该走干净");
        assert!(draft.purged.is_empty(), "它从没落盘，不用记进待删清单");
    }

    /// 生成记录不产反向 —— 撤销一条「生成过」没有意义
    #[test]
    fn mark_built_is_recorded_but_not_undoable() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let out = apply(
            &mut draft,
            &c,
            &reg,
            &[Patch::MarkBuilt {
                uids: vec!["A1/STANDARD".to_owned()],
                stamp: "2026-01-01T00:00:00Z".to_owned(),
                fingerprints: [("A1/STANDARD".to_owned(), "abc".to_owned())]
                    .into_iter()
                    .collect(),
            }],
        )
        .unwrap();
        assert!(!out.undoable);
        assert_eq!(draft.built["A1/STANDARD"].fingerprint, "abc");
    }

    /// 非法 patch **拒绝整批**：前面几条也不许生效
    #[test]
    fn an_invalid_patch_rejects_the_whole_batch() {
        let (_d, reg) = registry();
        let c = committed();

        for bad in [
            set(Level::Version, "A1/STANDARD", "toolhead.made_up", Some(serde_json::json!(1))),
            set(Level::Version, "A1/NOPE", "toolhead.offset.x", Some(serde_json::json!(1))),
            set(Level::Machine, "KOBRA", "toolhead.offset.x", Some(serde_json::json!(1))),
            Patch::RenameVersion {
                uid: "A1/STANDARD".to_owned(),
                name: "  ".to_owned(),
            },
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

    /// 同一批里"新建 + 立刻改名"不该被自己的校验拒掉
    #[test]
    fn a_version_created_in_this_batch_can_be_referenced_by_later_patches() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        apply(
            &mut draft,
            &c,
            &reg,
            &[
                Patch::NewVersion {
                    machine_id: "A1".to_owned(),
                    name: "新版".to_owned(),
                },
                Patch::RenameVersion {
                    uid: "new-1".to_owned(),
                    name: "改过名的新版".to_owned(),
                },
            ],
        )
        .unwrap();
        assert_eq!(draft.added[0].name, "改过名的新版");
    }

    /// **apply 反向能回到原状**：值、结构、脏计数三项都回
    #[test]
    fn applying_the_inverse_returns_to_the_original_state() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let forward = vec![
            set(Level::Machine, "A1", "toolhead.offset.x", Some(serde_json::json!(5))),
            set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!(7))),
            set(Level::Version, "A1/STANDARD", "toolhead.offset.z", None),
            Patch::RenameVersion {
                uid: "A1/STANDARD".to_owned(),
                name: "改了名".to_owned(),
            },
            Patch::ArchiveVersion {
                uid: "A1/STANDARD".to_owned(),
            },
        ];
        let out = apply(&mut draft, &c, &reg, &forward).unwrap();
        assert!(out.undoable);
        assert!(draft.dirty_count() > 0);

        apply(&mut draft, &c, &reg, &out.inverse).unwrap();
        assert_eq!(draft.dirty_count(), 0, "脏计数要回到 0");
        assert!(draft.values.is_empty(), "值要回到没改过：{:?}", draft.values);
        assert!(draft.renamed.is_empty());
        assert!(draft.archived.is_empty());
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
                set(Level::Machine, "A1", "toolhead.offset.x", Some(serde_json::json!(1))),
                set(Level::Machine, "A1", "toolhead.offset.x", Some(serde_json::json!(2))),
            ],
        )
        .unwrap();

        // 正向 无→1→2，反向必须是 2→1、1→无
        assert_eq!(
            out.inverse,
            vec![
                set(Level::Machine, "A1", "toolhead.offset.x", Some(serde_json::json!(1))),
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
            &[set(Level::Machine, "A1", "toolhead.offset.x", Some(serde_json::json!(5)))],
        )
        .unwrap();
        // 手工塞一条指向已消失版本的草稿（模拟上游删掉了那个版本）
        draft.values.insert(
            "v:A1/GONE:toolhead.offset.x".to_owned(),
            Some(serde_json::json!(1)),
        );
        draft
            .renamed
            .insert("A1/GONE".to_owned(), "改过名".to_owned());
        draft.values.insert("乱码键".to_owned(), None);
        assert_eq!(draft.dirty_count(), 4);

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
            (Level::Version, "new-7", "toolhead.custom_mount_gcode"),
        ] {
            let s = value_key(level, owner, key);
            assert_eq!(parse_value_key(&s), Some((level, owner, key)), "来回转不回去：{s}");
        }
        assert!(parse_value_key("x:A1:k").is_none());
        assert!(parse_value_key("m:A1").is_none());
        assert!(parse_value_key("m::k").is_none());
    }
}
