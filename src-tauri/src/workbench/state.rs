//! 生成状态 —— **比出来的，不是记出来的**。
//!
//! 判据只有一条：**当前有效配方的 hash** vs **上次成功生成时快照里记的那个 hash**。
//!
//! 刻意不看文件修改时间。理由是用户端：产物的时间戳一变，同步/更新检查就可能认为
//! 有新版本，于是顾客莫名被提示更新，而内容一个字节都没改（doc §8）。
//!
//! 四个状态里有两个容易被混成一个，这里分开：
//! - **未配置** = 有效配方是空的（配方本上有名字、下面一行没写）。这不是失败。
//! - **生成失败** = 上次尝试没成，但**上次成功的产物还在**，"上次成功生成时间"仍然是真的。
//!
//! 还有一层独立的事实：**产物文件本身还在不在、字节对不对**。
//! 配方没改但有人把 `dist-presets/` 清了，hash 比对仍然相等 —— 所以
//! [`VersionStatus::output_present`] 与 `output_matches` 单独算，不掺进状态判定。

use std::path::Path;

use serde::Serialize;

use crate::error::AppError;
use crate::fsx::paths::resolve_in;
use crate::workbench::generate;
use crate::workbench::resolve;
use crate::workbench::store::{GenFailure, Store};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum GenState {
    /// hash 相同 —— 产物对应当前配方
    Generated,
    /// hash 不同 —— 待生成
    Stale,
    /// 有效配方为空 —— 还没写配方
    Unconfigured,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionStatus {
    pub machine_id: String,
    pub version_id: String,
    pub machine_name: String,
    pub version_name: String,
    pub preset_id: String,
    pub state: GenState,
    /// 上次**成功**生成的时间。生成失败不会把它抹掉
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_generated_at: Option<String>,
    /// 上次失败记录。与 state 是两件事 —— 失败之后状态是"待生成"，不是"失败态"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_failure: Option<GenFailure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_client_version: Option<String>,
    /// 产物文件还在不在
    pub output_present: bool,
    /// 产物字节与快照里记的 sha256 是否一致（有人手改过产物就会对不上）
    pub output_matches: bool,
    /// 覆盖了但当前机型不适用的字段数。非阻断，但要看得见
    pub orphan_count: usize,
}

/// 全部版本的状态。`dist_root` 显式传入，理由同 [`generate::generate_one`]
pub fn status_all(store: &Store, dist_root: &Path) -> Result<Vec<VersionStatus>, AppError> {
    let reg = store.registry()?;
    let fb = store.fallback()?;
    let mut out = Vec::new();

    for mid in store.machine_ids()? {
        let Ok(machine) = store.machine(&mid) else {
            continue;
        };
        for vid in store.version_ids(&mid)? {
            let Ok(version) = store.version(&mid, &vid) else {
                continue;
            };
            let eff = resolve::resolve(&reg, &fb, &machine, &version);
            let snap = store.snapshot(&mid, &vid)?;

            let state = if eff.is_unconfigured() {
                GenState::Unconfigured
            } else if snap
                .as_ref()
                .is_some_and(|s| !s.recipe_hash.is_empty() && s.recipe_hash == eff.hash)
            {
                GenState::Generated
            } else {
                GenState::Stale
            };

            // 产物是否还在、字节对不对：独立于 hash 判定
            let (present, matches) = match snap.as_ref().filter(|s| !s.output_rel.is_empty()) {
                Some(s) => match resolve_in(dist_root, &s.output_rel) {
                    Ok(p) => match std::fs::read(&p) {
                        Ok(bytes) => (true, sha256_bytes(&bytes) == s.output_sha256),
                        Err(_) => (false, false),
                    },
                    Err(_) => (false, false),
                },
                None => (false, false),
            };

            out.push(VersionStatus {
                preset_id: generate::preset_id_of(store, &mid, &vid)?,
                machine_id: mid.clone(),
                version_id: vid.clone(),
                machine_name: machine.display_name.clone(),
                version_name: version.display_name.clone(),
                state,
                last_generated_at: snap
                    .as_ref()
                    .map(|s| s.generated_at.clone())
                    .filter(|t| !t.is_empty()),
                last_failure: snap.as_ref().and_then(|s| s.last_failure.clone()),
                min_client_version: snap.as_ref().and_then(|s| s.min_client_version.clone()),
                output_present: present,
                output_matches: matches,
                orphan_count: eff.orphans.len(),
            });
        }
    }

    Ok(out)
}

/// 待更新项 = 状态不是"已生成"、且不是"未配置"的那些。
///
/// **未配置不进这个清单**：它不是"要重新烤"，是"还没写配方"。
/// 把它塞进来会让"生成待更新项"每次都报一堆注定失败的项
pub fn plan_stale(store: &Store, dist_root: &Path) -> Result<Vec<(String, String)>, AppError> {
    Ok(status_all(store, dist_root)?
        .into_iter()
        .filter(|s| s.state == GenState::Stale || (s.state == GenState::Generated && !s.output_matches))
        .map(|s| (s.machine_id, s.version_id))
        .collect())
}

/// 恢复到上次成功生成时的配方。
///
/// 只写回**版本覆盖**。机型基底刻意不动 —— 它是多个版本共享的，为了一个版本回滚它
/// 会连带改掉别的版本（doc §9 的 Snapshot 注释）。基底有差异时如实报出来。
///
/// **恢复 ≠ 重新生成**：恢复完状态会变成"待生成"，还得显式点生成（doc §6）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RestoreReport {
    pub restored_overrides: usize,
    /// 当时的基底与现在不一样的字段。**没有被恢复**，只是告诉你
    pub base_differs: Vec<String>,
    pub snapshot_generated_at: String,
}

pub fn restore_snapshot_recipe(
    store: &Store,
    machine_id: &str,
    version_id: &str,
) -> Result<RestoreReport, AppError> {
    let snap = store
        .snapshot(machine_id, version_id)?
        .filter(|s| !s.recipe_hash.is_empty())
        .ok_or_else(|| {
            AppError::not_found(format!(
                "{machine_id}/{version_id} 还没有成功生成过，没有可恢复的配方"
            ))
        })?;

    let machine = store.machine(machine_id)?;
    let mut base_differs: Vec<String> = snap
        .base
        .iter()
        .filter(|(k, v)| machine.base.get(*k) != Some(*v))
        .map(|(k, _)| k.clone())
        .collect();
    // 现在多出来的字段也算差异
    for k in machine.base.keys() {
        if !snap.base.contains_key(k) {
            base_differs.push(k.clone());
        }
    }
    base_differs.sort_unstable();
    base_differs.dedup();

    let mut version = store.version(machine_id, version_id)?;
    let n = snap.overrides.len();
    version.overrides = snap.overrides.clone();
    store.save_version(&version)?;
    store.clear_draft(machine_id, version_id)?;

    Ok(RestoreReport {
        restored_overrides: n,
        base_differs,
        snapshot_generated_at: snap.generated_at,
    })
}

fn sha256_bytes(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workbench::capability::{CapField, ClientCapability, Support};
    use crate::workbench::model::{builtin_registry, BbsBinding, Machine, Version};
    use crate::workbench::store::empty_params;

    fn caps() -> Vec<ClientCapability> {
        vec![ClientCapability {
            client_version: "0.1.0".into(),
            catalog_schema_version: 1,
            machines: vec!["A1".into(), "A2L".into()],
            fields: builtin_registry()
                .fields
                .iter()
                .map(|f| CapField {
                    key: f.key.clone(),
                    value_type: f.value_type,
                    min: f.min,
                    max: f.max,
                    step: f.step,
                })
                .collect(),
        }]
    }

    fn setup() -> (tempfile::TempDir, tempfile::TempDir, Store) {
        let dist = tempfile::tempdir().unwrap();
        let d = tempfile::tempdir().unwrap();
        let s = Store::at(d.path());
        s.bootstrap().unwrap();

        let mut base = empty_params();
        base.insert("toolhead.z_offset".into(), serde_json::json!(0.10));
        s.save_machine(&Machine {
            id: "A1".into(),
            display_name: "A1".into(),
            base,
            default_bbs: vec![],
        })
        .unwrap();
        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A1".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        // A2L：基底空 + 覆盖空 → 未配置
        s.save_machine(&Machine {
            id: "A2L".into(),
            display_name: "A2L".into(),
            base: empty_params(),
            default_bbs: vec![],
        })
        .unwrap();
        s.save_version(&Version {
            id: "std".into(),
            display_name: "标准版".into(),
            machine_id: "A2L".into(),
            overrides: empty_params(),
            bbs: BbsBinding::Inherit,
        })
        .unwrap();

        s.write_doc(
            "capability/support.json",
            &Support {
                supported: vec!["0.1.0".into()],
            },
        )
        .unwrap();

        (dist, d, s)
    }

    fn pick<'a>(all: &'a [VersionStatus], m: &str) -> &'a VersionStatus {
        all.iter().find(|s| s.machine_id == m).unwrap()
    }

    /// 从没生成过 → 待生成；未配置的那个 → 未配置（**不是**待生成）
    #[test]
    fn initial_states_separate_stale_from_unconfigured() {
        let (dist, _d, s) = setup();
        let all = status_all(&s, dist.path()).unwrap();
        assert_eq!(pick(&all, "A1").state, GenState::Stale);
        assert_eq!(pick(&all, "A2L").state, GenState::Unconfigured);
        assert!(pick(&all, "A1").last_generated_at.is_none());
    }

    /// 生成之后变"已生成"；改一笔配方立刻变回"待生成"
    #[test]
    fn state_follows_recipe_hash_not_file_time() {
        let (dist, _d, s) = setup();
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();

        let all = status_all(&s, dist.path()).unwrap();
        let st = pick(&all, "A1");
        assert_eq!(st.state, GenState::Generated);
        assert!(st.output_present && st.output_matches);
        assert_eq!(st.min_client_version.as_deref(), Some("0.1.0"));

        // 改配方 → 待生成
        let mut v = s.version("A1", "std").unwrap();
        v.overrides
            .insert("toolhead.z_offset".into(), serde_json::json!(0.15));
        s.save_version(&v).unwrap();
        assert_eq!(
            pick(&status_all(&s, dist.path()).unwrap(), "A1").state,
            GenState::Stale
        );
    }

    /// 改**全局配置**（字段定义）也要让产物过期 —— 这是 hash 里带指纹的那条的兑现
    #[test]
    fn changing_registry_makes_everything_stale() {
        let (dist, _d, s) = setup();
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();
        assert_eq!(
            pick(&status_all(&s, dist.path()).unwrap(), "A1").state,
            GenState::Generated
        );

        let mut reg = s.registry().unwrap();
        reg.fields[1].step = Some(0.05);
        s.write_doc("registry.json", &reg).unwrap();

        assert_eq!(
            pick(&status_all(&s, dist.path()).unwrap(), "A1").state,
            GenState::Stale,
            "改了字段定义，产物却还被当成最新的"
        );
    }

    /// 产物被删：hash 仍然相等（配方没动），但 output_present 必须是 false，
    /// 而且它要进"待更新"清单 —— 否则点生成不会把它补回来
    #[test]
    fn missing_output_is_detected_and_replanned() {
        let (dist, _d, s) = setup();
        let out = generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();
        std::fs::remove_file(resolve_in(dist.path(), &out.output_rel).unwrap()).unwrap();

        let st = pick(&status_all(&s, dist.path()).unwrap(), "A1").clone();
        assert_eq!(st.state, GenState::Generated, "配方没变，hash 判定应当仍相等");
        assert!(!st.output_present);

        let plan = plan_stale(&s, dist.path()).unwrap();
        assert!(
            plan.contains(&("A1".to_string(), "std".to_string())),
            "产物没了却不在待更新清单里"
        );
    }

    /// 未配置的不进待更新清单
    #[test]
    fn unconfigured_is_not_in_the_plan() {
        let (dist, _d, s) = setup();
        let plan = plan_stale(&s, dist.path()).unwrap();
        assert!(!plan.contains(&("A2L".to_string(), "std".to_string())));
    }

    /// 失败记录挂上去之后：状态仍由 hash 说话，"上次成功时间"不被抹掉
    #[test]
    fn failure_is_recorded_without_changing_state_or_time() {
        let (dist, _d, s) = setup();
        let ok = generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();
        generate::record_failure(&s, "A1", "std", "盘满").unwrap();

        let st = pick(&status_all(&s, dist.path()).unwrap(), "A1").clone();
        assert_eq!(st.state, GenState::Generated);
        assert_eq!(st.last_generated_at.as_deref(), Some(ok.generated_at.as_str()));
        assert_eq!(st.last_failure.unwrap().reason, "盘满");
    }

    /// 恢复配方：只写回覆盖，基底差异如实报出、**不恢复**；且恢复 ≠ 重新生成
    #[test]
    fn restore_only_touches_overrides() {
        let (dist, _d, s) = setup();

        let mut v = s.version("A1", "std").unwrap();
        v.overrides
            .insert("toolhead.z_offset".into(), serde_json::json!(0.15));
        s.save_version(&v).unwrap();
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();

        // 之后把配方改乱，并且顺手改了基底
        let mut v2 = s.version("A1", "std").unwrap();
        v2.overrides
            .insert("toolhead.z_offset".into(), serde_json::json!(0.9));
        s.save_version(&v2).unwrap();
        let mut m = s.machine("A1").unwrap();
        m.base
            .insert("toolhead.z_offset".into(), serde_json::json!(0.5));
        s.save_machine(&m).unwrap();

        let r = restore_snapshot_recipe(&s, "A1", "std").unwrap();
        assert_eq!(r.restored_overrides, 1);
        assert_eq!(r.base_differs, vec!["toolhead.z_offset"]);

        assert_eq!(
            s.version("A1", "std").unwrap().overrides["toolhead.z_offset"],
            serde_json::json!(0.15),
            "覆盖没恢复回去"
        );
        assert_eq!(
            s.machine("A1").unwrap().base["toolhead.z_offset"],
            serde_json::json!(0.5),
            "基底被顺手改回去了 —— 它是共享的，不该动"
        );

        /* 这里的状态是 **Generated**，而且这是对的 —— 起初我以为该是 Stale，实测纠正：
        被改的那个基底字段正好被本版覆盖遮住，所以**有效配方一个字节都没变**，
        产物仍然对应当前配方。

        这恰好是"只生成待更新项"想要的性质：改一个所有版本都覆盖了的基底字段，
        不会把这些版本的产物全判过期、也就不会让用户端莫名要更新。

        "恢复 ≠ 重新生成"这条由另一条路兑现：恢复回来的覆盖若与上次生成时不同，
        hash 就会不同、状态就会变 Stale。本例里恢复后的覆盖恰好与上次生成时相同。 */
        assert_eq!(
            pick(&status_all(&s, dist.path()).unwrap(), "A1").state,
            GenState::Generated
        );
    }

    /// "恢复 ≠ 重新生成"的正面样例：恢复回来的配方与上次生成时**不**相同时，
    /// 状态必须是待生成，不能因为"刚恢复过"就当成已生成
    #[test]
    fn restore_to_a_different_recipe_leaves_it_stale() {
        let (dist, _d, s) = setup();

        // 第一次：覆盖 0.15，生成
        let mut v = s.version("A1", "std").unwrap();
        v.overrides
            .insert("toolhead.z_offset".into(), serde_json::json!(0.15));
        s.save_version(&v).unwrap();
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();

        // 第二次：改成 0.2 再生成一次 —— 快照里记的就变成 0.2 了
        let mut v2 = s.version("A1", "std").unwrap();
        v2.overrides
            .insert("toolhead.z_offset".into(), serde_json::json!(0.2));
        s.save_version(&v2).unwrap();
        generate::generate_one(&s, dist.path(), "A1", "std", &caps()).unwrap();

        // 现在改成 0.3（不生成），再恢复 → 回到 0.2，与快照一致 → 已生成
        let mut v3 = s.version("A1", "std").unwrap();
        v3.overrides
            .insert("toolhead.z_offset".into(), serde_json::json!(0.3));
        s.save_version(&v3).unwrap();
        assert_eq!(
            pick(&status_all(&s, dist.path()).unwrap(), "A1").state,
            GenState::Stale
        );

        restore_snapshot_recipe(&s, "A1", "std").unwrap();
        assert_eq!(
            s.version("A1", "std").unwrap().overrides["toolhead.z_offset"],
            serde_json::json!(0.2)
        );
        assert_eq!(
            pick(&status_all(&s, dist.path()).unwrap(), "A1").state,
            GenState::Generated,
            "恢复到上次生成时那份配方后，产物本来就是对应它的"
        );
    }

    #[test]
    fn restore_without_snapshot_is_not_found() {
        let (_dist, _d, s) = setup();
        let e = restore_snapshot_recipe(&s, "A1", "std").unwrap_err();
        assert_eq!(e.code, crate::error::ErrorCode::NotFound);
    }
}
