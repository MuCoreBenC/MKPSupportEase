//! **「这一版跟不跟基底」的唯一判据**（2026-11-03）。
//!
//! # 它取代的是什么
//!
//! 以前这件事没有名字：徽章问"文件里有没有 `机型:版本` 那一行"，
//! 写值那条路问"值等不等于基底"，两个答案在"值恰好等于基底"时**分岔** ——
//! 界面上写着「不跟随」，改基底却照样跟着变。作者报的就是这一条。
//!
//! 现在它是一个**真实存在、与值分离、随文件持久化**的状态：
//!
//! ```toml
//! [[params]]
//! key = "toolhead.offset.z"
//!
//! [params.machineVariants]
//! "A1:STANDARD" = 3.85
//!
//! [params.follows]
//! "A1:STANDARD" = false
//! ```
//!
//! # 编码：**表里没有这个键 = 跟随**
//!
//! | 文件里 | 意思 |
//! |---|---|
//! | 没有 `[params.follows]` / 表里没有这个 uid | **跟随基底**（默认） |
//! | `"A1:STANDARD" = false` | 不跟随：这一版的值归自己 |
//!
//! 只有 `false` 会落盘。理由是**文件大小**：跟随是绝大多数格子的常态
//! （75 个参数 × 每台机型的每个版本），把"跟随"写进文件会凭空多出几百行；
//! 反过来只记"哪几版自己定了"，与今天文件里那 78 条版本键正好一一对应。
//!
//! 这与旧判据（`machine_variants` 里有没有那一行）在**存量数据上完全等价**：
//! 迁移就是把那 78 条版本键各补一句 `false`，一个格子都不换边。
//! 唯一的区别是值可以留着 —— 「跟随」不再意味着"必须删掉那一行"。
//!
//! # 不许拿值反推
//!
//! `值 == 基底` **不参与判断**。写过的、点了「跟随」的，值是同一个数，
//! 状态却不同 —— 这正是这一层存在的全部理由。

use std::collections::BTreeMap;

use super::registry::{ParamDef, ParamRegistry};

/// 版本 uid 在 `follows` 表里的形状：`机型:版本id`（与 `machineVariants` 同一套键名）。
///
/// **只对版本有意义**：机型基底是这条链的根，谈不上"跟不跟谁"
/// （`derive::badge_at` 也从不给基底列挂徽章）。
pub fn uid_of(machine_id: &str, version_id: &str) -> String {
    format!("{machine_id}:{version_id}")
}

/// 这一版**现在跟随基底吗**。唯一判据，三处（徽章 / 写值 / 日志）共用。
pub fn is_following(p: &ParamDef, uid: &str) -> bool {
    p.follows.get(uid).copied().unwrap_or(true)
}

/// 改内存里的跟随状态。`true` = 跟随（表里删掉这个键，恢复"默认"形状）。
///
/// **只动 `ParamDef`**（内存文档）；落盘由 [`ParamRegistry::set_following`] 走
/// —— 那里是唯一写文件的地方，且与值共用同一次 `write_back`。
///
/// [`ParamRegistry::set_following`]: super::registry::ParamRegistry::set_following
pub fn set_following(p: &mut ParamDef, uid: &str, on: bool) {
    if on {
        p.follows.remove(uid);
    } else {
        p.follows.insert(uid.to_owned(), false);
    }
}

/// **迁移判定的唯一实现**：一处存量数据该怎么定状态。
///
/// ```text
/// 有 `机型:版本` 那一行  ⇒ 不跟随（旧徽章也是这么说的）
/// 没有那一行           ⇒ 跟随（没有独立值可言）
/// ```
///
/// 与旧判据 `machine_variants.contains_key` 等价 —— 迁移**不改变任何一格的表现**，
/// 它只是把"文件里有没有那一行"这句推断，换成一句写在文件里的状态。
///
/// 「值恰好等于基底」的那些格**不特殊对待**：按旧口径它们本来就显示「不跟随」
/// （有那一行），迁移保持原样；它们会进台账（[`MigrationItem`]），
/// 因为"当初是自己定在 4.0，还是跟随过来的"已经无法证明 —— 要跟随就点一次
/// 「跟随默认基底」，那是一次有记录的用户动作。
pub fn migration_state(has_variant_row: bool) -> bool {
    !has_variant_row
}

/// **一次性迁移**：给存量版本键各补一句「不跟随」，把推断变成写下来的状态。
///
/// 只碰**没有状态记录**的版本键（已经有记录的一律不动），所以跑第二遍是空操作。
/// 返回台账；写盘由调用方决定（`--dry-run` 时一个字节都不写）。
pub fn migrate(
    reg: &mut ParamRegistry,
    at: &str,
) -> Result<MigrationLedger, crate::error::AppError> {
    /* 先把活算完再动文档：`set_following` 要 `&mut self`，而这一趟在读 `params()` */
    let mut work: Vec<(String, String, serde_json::Value, serde_json::Value)> = Vec::new();
    for p in reg.params() {
        for (uid, value) in &p.machine_variants {
            let Some((machine, _version)) = uid.split_once(':') else {
                continue; // 裸键是机型基底，跟"跟不跟"无关
            };
            if p.follows.contains_key(uid) {
                continue; // 已经有状态记录：幂等
            }
            /* 有版本行 ⇒ 不跟随；这一句只是把判据写在调用点上 */
            debug_assert!(!migration_state(true));
            work.push((
                p.key.clone(),
                uid.clone(),
                value.clone(),
                base_value_of(p, machine),
            ));
        }
    }

    let mut ledger = MigrationLedger {
        at: at.to_owned(),
        marked: work.len(),
        ambiguous: Vec::new(),
    };
    for (key, uid, value, base) in work {
        reg.set_following(&key, &uid, migration_state(true))?;
        if value == base {
            ledger.ambiguous.push(MigrationItem {
                param_key: key,
                uid,
                value,
                base_value: base,
                reason: AMBIGUOUS_REASON.to_owned(),
            });
        }
    }
    Ok(ledger)
}

/// 迁移台账里的一条：`值恰好等于基底` 的那些格。
///
/// 它们的历史状态**不可证明**，被保守判成"不跟随"。记下来是为了将来能回答
/// 「为什么这一版不跟随」—— 不改判定，只留证据。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationItem {
    pub param_key: String,
    /// `机型:版本`
    pub uid: String,
    /// 这一版自己那一行的值
    pub value: serde_json::Value,
    /// 同一台机器的基底值（`machineVariants` 的裸键，没有就用出厂默认）
    pub base_value: serde_json::Value,
    /// 为什么进台账（人话，给日后核查用）
    pub reason: String,
}

/// 迁移台账整份文件（`presets/registry/follows-migration.json`）。
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationLedger {
    /// 迁移跑完的时刻（ISO 8601）
    pub at: String,
    /// 一共给多少格写了"不跟随"
    pub marked: usize,
    /// 其中**值恰好等于基底**、历史状态不可确认的那些
    pub ambiguous: Vec<MigrationItem>,
}

/// 台账里 `reason` 的那句话。**只此一处**，不留两套说法。
pub const AMBIGUOUS_REASON: &str =
    "值恰好等于基底：历史状态无法确认（可能是跟随，也可能自己定过），按保守口径判「不跟随」";

/// 这一版的基底值：机型裸键 → 出厂默认。迁移判"值等不等于基底"用它。
pub fn base_value_of(p: &ParamDef, machine_id: &str) -> serde_json::Value {
    p.machine_variants
        .get(machine_id)
        .cloned()
        .unwrap_or_else(|| p.default_value.clone())
}

/// 把 `follows` 表里"认不出来的 uid"挑出来（跨文件校验用）。
///
/// 与 `machine_variants` 同一口径：写进一个不存在的版本，下一次加载就该报
/// `Corrupted`，而不是静静地留一条谁都不读的键。
pub fn unknown_uids(follows: &BTreeMap<String, bool>, known: impl Fn(&str) -> bool) -> Vec<&str> {
    follows
        .keys()
        .map(String::as_str)
        .filter(|uid| !known(uid))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn param() -> ParamDef {
        let json = serde_json::json!({
            "key": "toolhead.offset.z", "configKey": "X", "tomlKey": "z", "jsonKey": "z",
            "label": "Z", "desc": "", "tomlComment": "",
            "valueType": "float", "uiComponent": "number", "defaultValue": 4.0,
            "scope": "universal", "section": "toolhead",
            "layout": { "order": 1.0, "sectionId": "s1" },
            "machineVariants": { "A1": 3.85, "A1:STANDARD": 3.85 }
        });
        serde_json::from_value(json).unwrap()
    }

    /// **默认是跟随**：表里没有这个键就是跟随 —— 这是"新参数 / 新版本"的起点。
    #[test]
    fn absence_means_following() {
        let p = param();
        assert!(is_following(&p, "A1:STANDARD"));
        assert!(is_following(&p, "A1:FAST"), "没有任何记录的版本也是跟随");
    }

    /// **值与状态无关**：值等于基底仍然可以是"不跟随"，也可以是"跟随"。
    #[test]
    fn the_value_never_decides_the_state() {
        let mut p = param();
        assert_eq!(p.machine_variants["A1:STANDARD"], serde_json::json!(3.85));
        assert!(
            is_following(&p, "A1:STANDARD"),
            "值等于基底，但没记录 ⇒ 跟随"
        );

        set_following(&mut p, "A1:STANDARD", false);
        assert!(!is_following(&p, "A1:STANDARD"));
        assert_eq!(
            p.machine_variants["A1:STANDARD"],
            serde_json::json!(3.85),
            "改状态不许碰值"
        );
    }

    /// 切回跟随 = **把键删掉**（恢复文件里的默认形状），不是写 `true`
    #[test]
    fn following_is_encoded_by_absence() {
        let mut p = param();
        set_following(&mut p, "A1:STANDARD", false);
        assert_eq!(p.follows.len(), 1);
        set_following(&mut p, "A1:STANDARD", true);
        assert!(p.follows.is_empty(), "跟随 = 表里没有这个键");
        assert!(is_following(&p, "A1:STANDARD"));
    }

    /// 迁移判据与旧判据（有没有版本行）**等价** —— 迁移不换任何一格的表现
    #[test]
    fn migration_keeps_todays_badges() {
        assert!(
            !migration_state(true),
            "有版本行 ⇒ 不跟随（旧徽章也是如此）"
        );
        assert!(migration_state(false), "没有版本行 ⇒ 跟随");
    }

    #[test]
    fn unknown_uids_are_reported() {
        let mut p = param();
        set_following(&mut p, "A1:GONE", false);
        let bad = unknown_uids(&p.follows, |uid| uid != "A1:GONE");
        assert_eq!(bad, vec!["A1:GONE"]);
    }
}
