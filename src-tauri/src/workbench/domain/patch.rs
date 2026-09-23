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
//! 剩下 [`Patch::SetValue`]、[`Patch::SetVisibility`]、[`Patch::SetBundle`]、
//! [`Patch::MarkBuilt`] 四种。**撤销栈因此只服务值编辑** ——
//! 低频的结构操作一律即时落盘、没有一个蓄了半天才生效的第二副本。
//!
//! # 哪些手势不进撤销栈
//!
//! | 手势 | 可撤销 | 兜底 |
//! |---|---|---|
//! | 改值、套餐、可见性 | ✅ | —— |
//! | [`Patch::MarkBuilt`] | ❌ | 它是生成的记录，不是编辑；撤销一条"生成过"没有意义 |
//!
//! # 非法 patch 拒绝整批
//!
//! 部分应用会留下一个谁也说不清的中间状态：前三条生效了、第四条没有，
//! 而界面上只看到"保存失败"。所以先全部校验，再全部应用。

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::AppError;
use crate::workbench::presets::ParamRegistry as Registry;

use super::layer::Level;

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
        !matches!(self, Patch::MarkBuilt { .. })
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
    pub visibility: BTreeMap<String, Visibility>,
    pub bundles: BTreeMap<String, BundleEdit>,
    pub built: BTreeMap<String, BuiltRecord>,
}

impl Draft {
    /// 未保存的改动有几处
    pub fn dirty_count(&self) -> usize {
        self.values.len() + self.visibility.len() + self.bundles.len() + self.built.len()
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
        let mut gone: BTreeSet<String> = BTreeSet::new();

        self.values.retain(|k, _| match parse_value_key(k) {
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
        });

        // 闭包没法对值类型泛化，所以这里用一个自由函数（见本文件末尾的 `retain_alive`）
        let alive: BTreeSet<String> = committed.versions.keys().cloned().collect();
        retain_alive(&mut self.built, &alive, &mut gone);

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

/// 应用的结果
#[derive(Debug, Clone)]
pub struct Applied {
    /// 撤销这次操作要提交的 patches。空 = 这次操作不可撤销
    pub inverse: Vec<Patch>,
    /// 能不能进撤销栈。**为 false 时界面不该给出撤销按钮** ——
    /// 给一个按下去没反应的按钮比没有按钮更糟
    pub undoable: bool,
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
    validate(committed, registry, patches)?;

    let undoable = patches.iter().all(Patch::is_undoable);
    let mut inverse: Vec<Patch> = Vec::new();

    for p in patches {
        apply_one(draft, committed, registry, p, &mut inverse);
    }

    // 反向要**倒着执行**才能回到原状：正向 A→B→C，反向是 C⁻¹→B⁻¹→A⁻¹
    inverse.reverse();

    Ok(Applied {
        inverse: if undoable { inverse } else { Vec::new() },
        undoable,
    })
}

/// 整批校验。这里只判"能不能做"，不改任何东西
fn validate(
    committed: &Committed,
    registry: &Registry,
    patches: &[Patch],
) -> Result<(), AppError> {
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
            let before = own_value(draft, registry, *level, owner, key);
            if &before == value {
                return; // 没变化：不记草稿、不产反向，脏计数也不该涨
            }
            let vk = value_key(*level, owner, key);
            if &at_rest(registry, *level, owner, key) == value {
                // 改回了盘上那个值 = 这一处不再是改动。**从草稿里拿掉而不是记一条**，
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

/// 这一层**现在**这一项是什么值（草稿优先）。`None` = 这一层没钉着它 = 继承
fn own_value(
    draft: &Draft,
    registry: &Registry,
    level: Level,
    owner: &str,
    key: &str,
) -> Option<Value> {
    if let Some(pending) = draft.values.get(&value_key(level, owner, key)) {
        return pending.clone();
    }
    at_rest(registry, level, owner, key)
}

/// `machineVariants` 里**此刻写着**的那一个值 —— 这是这一层唯一的盘上真相
fn at_rest(registry: &Registry, level: Level, owner: &str, key: &str) -> Option<Value> {
    let variant = variant_key(level, owner);
    registry.param(key)?.machine_variants.get(&variant).cloned()
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

    /// 两个参数的最小字段定义。
    ///
    /// patch 层除了判"这个 key 存在吗"，还要知道"它此刻钉着什么"（[`at_rest`]）——
    /// 而现在这一层的真相就在 `machineVariants` 里，所以夹具也要带上它：
    /// `offset.z` 在机型层有值（`A1`），`offset.x` 在版本层有值（`A1:STANDARD`）
    fn registry() -> (tempfile::TempDir, Registry) {
        let d = tempfile::tempdir().unwrap();
        let p = |key: &str, order: f64, variants: serde_json::Value| {
            serde_json::json!({
                "key": key, "configKey": "X", "tomlKey": key, "jsonKey": key,
                "label": key, "desc": "", "tomlComment": "",
                "valueType": "float", "uiComponent": "number", "defaultValue": 4,
                "scope": "universal", "section": "toolhead",
                "layout": { "order": order, "sectionId": "s1" },
                "machineVariants": variants
            })
        };
        let params = serde_json::json!({
            "params": [
                p("toolhead.offset.x", 1.0, serde_json::json!({ "A1:STANDARD": -1 })),
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
        let r = crate::workbench::presets::registry::load_from_json_fixture(
            d.path(),
            &params,
            &layout,
        )
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

    /// **apply 反向能回到原状**：机型层与版本层的值、脏计数都回。
    ///
    /// 以前这一条还要回结构与身份（改名 / 归档），那些 layer 之外的东西现在不在这一层了
    #[test]
    fn applying_the_inverse_returns_to_the_original_state() {
        let (_d, reg) = registry();
        let c = committed();
        let mut draft = Draft::default();

        let forward = vec![
            set(Level::Machine, "A1", "toolhead.offset.x", Some(serde_json::json!(5))),
            set(Level::Version, "A1/STANDARD", "toolhead.offset.x", Some(serde_json::json!(7))),
            set(Level::Version, "A1/STANDARD", "toolhead.offset.z", None),
        ];
        let out = apply(&mut draft, &c, &reg, &forward).unwrap();
        assert!(out.undoable);
        assert!(draft.dirty_count() > 0);

        apply(&mut draft, &c, &reg, &out.inverse).unwrap();
        assert_eq!(draft.dirty_count(), 0, "脏计数要回到 0");
        assert!(draft.values.is_empty(), "值要回到没改过：{:?}", draft.values);
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
            assert_eq!(parse_value_key(&s), Some((level, owner, key)), "来回转不回去：{s}");
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
