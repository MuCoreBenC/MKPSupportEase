//! **软件更新的唯一事实源**（`run/update-state.json`）。
//!
//! # 它是什么
//!
//! 一次检查的**全部结论**落这一个文件 —— 查到了哪几版（`candidates`）、
//! 上一次检查成没成（`checked`）、忽略的是哪一版（`ignored`）、上次装成没成
//! （`last_result`）。标题栏、设置页、更新面板、通知都读它，不各自联网、不各自判版本。
//!
//! # 与进程态的分界
//!
//! 下载进度（[`super::updater::UpdateState`]）**不进这个文件**：它只活在本次进程里，
//! 重启即 `Idle`。快照只记业务事实；两者在 `ipc::update` 的 DTO 里合成一份给界面。
//!
//! # 失效只有一个出口
//!
//! [`reconcile`]：装上新版之后（`current` 变了），低于/等于当前版本的候选、安装结果、
//! 忽略记录一次性作废。从前这件事藏在"读的时候顺手清"里，于是装到 0.0.14 之后
//! 标题栏还挂着"发现 0.0.13"。

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;
use crate::fsx;

use super::release_info::{self, UpdateCandidate, UpdateChannel};
use super::structure::APP_VERSION;
use super::updater::{UpdateResult, UpdateState};

/// 快照协议代次。**加字段不升号，改语义才升**（与 `release.json` / `catalog.json` 同一条）。
pub const UPDATE_STATE_SCHEMA: u32 = 1;

const STATE_FILE: &str = "run/update-state.json";

fn schema_now() -> u32 {
    UPDATE_STATE_SCHEMA
}

/// 一次检查的记录。**成功与失败严格分开**：`ok=false` 时候选是上一次的，
/// 界面必须说"这次没查成"，而不是把旧候选描述成刚刚查出的结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckRecord {
    /// 最近一次检查的时刻（ISO8601）。`None` = 这个应用还没查过。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// 这一次成没成。失败时 `candidates` 保持不动。
    pub ok: bool,
    /// 失败原因（成功时为空）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 谁触发的：自动（拉目录顺路）/ 手动（设置页那颗按钮）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<CheckTrigger>,
    /// 这次按哪一路查的。
    #[serde(default)]
    pub channel: UpdateChannel,
}

impl Default for CheckRecord {
    fn default() -> Self {
        Self {
            at: None,
            ok: false,
            reason: None,
            trigger: None,
            channel: UpdateChannel::Stable,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum CheckTrigger {
    Auto,
    Manual,
}

/// **业务事实**的整份快照。写盘的就是它。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshot {
    #[serde(default = "schema_now")]
    pub schema: u32,
    /// 写这份快照时跑的版本。下次启动版本变了 ⇒ [`reconcile`] 会作废旧结论。
    pub current: String,
    #[serde(default)]
    pub checked: CheckRecord,
    /// 比 `current` 新的候选，正式版排第一。**只由检查事务整体替换**。
    #[serde(default)]
    pub candidates: Vec<UpdateCandidate>,
    /// 用户点过「忽略」的那一版。只压标题栏胶囊与通知，不影响候选集与手动下载。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ignored: Option<String>,
    /// 上一次安装的结果（退出后那个脚本写的账，下次启动迁进来）。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_result: Option<UpdateResult>,
}

impl Default for UpdateSnapshot {
    fn default() -> Self {
        Self {
            schema: UPDATE_STATE_SCHEMA,
            current: APP_VERSION.to_owned(),
            checked: CheckRecord::default(),
            candidates: Vec::new(),
            ignored: None,
            last_result: None,
        }
    }
}

/// 快照里的"安装这一档"—— **业务阶段，不落盘**。
///
/// 下载中 / 暂停 / 失败都是进程态（[`UpdateState`]），重启归零；这里只回答
/// "有没有一份**已经下好、等着装**的"，供界面决定要不要摆「重启并安装」。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase")]
pub enum InstallPhase {
    Idle,
    Ready { version: String },
}

/// 进程态 → 业务阶段。**只有 `Ready` 会跨进业务语义**，其余一律 `Idle`。
pub fn install_phase(state: &UpdateState) -> InstallPhase {
    match state {
        UpdateState::Ready { version, .. } => InstallPhase::Ready {
            version: version.clone(),
        },
        _ => InstallPhase::Idle,
    }
}

/// 快照的落点（`<appData>/run/update-state.json`）。
pub fn state_path(root: &Path) -> PathBuf {
    root.join(STATE_FILE)
}

/// 读整份快照。**文件不在 = 全新应用**（默认快照，不落盘）；坏档报 `CORRUPTED`
/// （静默当"没查过"会让人以为永远没有新版）。
pub fn load(root: &Path) -> Result<UpdateSnapshot, AppError> {
    let path = state_path(root);
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(UpdateSnapshot::default()),
        Err(e) => {
            return Err(AppError::io("读不了更新状态").with_detail(e.to_string()));
        }
    };
    serde_json::from_slice(&bytes)
        .map_err(|e| AppError::corrupted("更新状态不是合法 JSON").with_detail(e.to_string()))
}

/// 写整份快照（整份原子替换，没有局部更新）。
pub fn save(root: &Path, snap: &UpdateSnapshot) -> Result<(), AppError> {
    let mut snap = snap.clone();
    snap.schema = UPDATE_STATE_SCHEMA;
    fsx::atomic::atomic_write_json(&state_path(root), &snap)
}

/// 这一刻（ISO8601，UTC）。客户端没有 `workbench::clock`，这一处自己算；
/// 格式与工作台那套一致（`2026-10-05T04:01:08Z`），同一份账不写两种样子。
pub fn now_iso8601() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// 天数 → 年月日（Howard Hinnant 的 civil_from_days；算法是公历的，不用查表）
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// **唯一的失效收敛**：装上新版之后，旧结论一次性作废。
///
/// - 候选只留**比当前新**的（`is_newer_version`，与全仓同一套版本序）；
/// - `last_result` 只留"装的就是当前版本"或"比当前新"的；
/// - `ignored` 在忽略的那一版已经追上/落后时清掉（再留着只会误伤下一次）。
///
/// 返回"改过吗"：改过就该落盘。**纯函数**，判据直接喂它。
pub fn reconcile(snap: &mut UpdateSnapshot, current: &str) -> bool {
    let before = snap.clone();
    snap.current = current.to_owned();
    snap.candidates
        .retain(|c| release_info::is_newer_version(&c.version, current));
    if snap.last_result.as_ref().is_some_and(|r| {
        r.version.trim() != current.trim() && !release_info::is_newer_version(&r.version, current)
    }) {
        snap.last_result = None;
    }
    if snap
        .ignored
        .as_deref()
        .is_some_and(|v| !release_info::is_newer_version(v, current))
    {
        snap.ignored = None;
    }
    *snap != before
}

/// 现在的"待更新"：**用户选的渠道**上第一条候选，但被忽略的那一版不算。
///
/// 按渠道挑而不是取整份第一条：选了测试版的人不该收到正式版的胶囊（反过来也一样）。
/// 标题栏胶囊、通知、设置页的"发现新版本"都读这一条 —— 它们不再各自挑。
pub fn pending(snap: &UpdateSnapshot) -> Option<&UpdateCandidate> {
    snap.candidates
        .iter()
        .find(|c| c.channel == snap.checked.channel)
        .filter(|c| snap.ignored.as_deref() != Some(c.version.as_str()))
}

/// 有没有新版（候选非空）。**与 `pending` 不同**：被忽略的那一版仍算"有新版",
/// 只是不再冒胶囊与通知。
pub fn has_update(snap: &UpdateSnapshot) -> bool {
    !snap.candidates.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(version: &str) -> UpdateCandidate {
        UpdateCandidate {
            channel: UpdateChannel::Stable,
            version: version.to_owned(),
            notes: None,
            url: None,
            asset: None,
        }
    }

    fn snapshot(versions: &[&str]) -> UpdateSnapshot {
        UpdateSnapshot {
            candidates: versions.iter().map(|v| candidate(v)).collect(),
            ..Default::default()
        }
    }

    /// 装上新版之后：旧候选、同版本候选一起清掉，比当前新的留住。
    #[test]
    fn reconcile_drops_candidates_that_are_not_newer() {
        let mut s = snapshot(&["0.0.13", "0.0.14", "0.0.15"]);
        assert!(reconcile(&mut s, "0.0.14"));
        assert_eq!(
            s.candidates
                .iter()
                .map(|c| c.version.as_str())
                .collect::<Vec<_>>(),
            vec!["0.0.15"],
            "旧的与相等的都要清，只有更高的留"
        );
    }

    /// 安装结果只留"这一版"或"更新的"；旧版本的结果不许跨版本冒充。
    #[test]
    fn reconcile_drops_last_result_from_an_older_version() {
        let mut s = UpdateSnapshot {
            last_result: Some(UpdateResult {
                version: "0.0.13".to_owned(),
                ok: false,
                reason: "旧账".to_owned(),
                at: "2026-11-21T00:00:00Z".to_owned(),
            }),
            ..Default::default()
        };
        assert!(reconcile(&mut s, "0.0.14"));
        assert!(s.last_result.is_none(), "0.0.13 的失败账不许挂到 0.0.14 上");

        s.last_result = Some(UpdateResult {
            version: "0.0.14".to_owned(),
            ok: true,
            reason: String::new(),
            at: "2026-11-21T00:00:00Z".to_owned(),
        });
        assert!(!reconcile(&mut s, "0.0.14"), "同一版的结果原样留着");
        assert!(s.last_result.is_some());
    }

    /// 忽略记录跟着版本走：那一版已经追上/落后就该清，免得误伤下一次。
    #[test]
    fn reconcile_drops_an_ignored_version_that_is_no_longer_ahead() {
        let mut s = UpdateSnapshot {
            ignored: Some("0.0.13".to_owned()),
            ..Default::default()
        };
        assert!(reconcile(&mut s, "0.0.14"));
        assert!(s.ignored.is_none());

        let mut s = UpdateSnapshot {
            ignored: Some("0.0.15".to_owned()),
            ..Default::default()
        };
        assert!(!reconcile(&mut s, "0.0.14"));
        assert_eq!(s.ignored.as_deref(), Some("0.0.15"));
    }

    /// 待更新 = 候选第一条；被忽略的那一版不再冒胶囊，但候选集不动。
    #[test]
    fn pending_skips_the_ignored_version_but_keeps_the_candidates() {
        let s = snapshot(&["0.0.15"]);
        assert_eq!(pending(&s).map(|c| c.version.as_str()), Some("0.0.15"));

        let s = UpdateSnapshot {
            ignored: Some("0.0.15".to_owned()),
            ..snapshot(&["0.0.15"])
        };
        assert!(pending(&s).is_none(), "忽略的那一版不再进胶囊");
        assert_eq!(s.candidates.len(), 1, "候选集不许被忽略动作改");
        assert!(has_update(&s), "「有新版」与「要不要冒胶囊」是两件事");
    }

    /// 没有文件 = 默认快照（不落盘）；坏档必须报出来，不静默当没查过。
    #[test]
    fn load_is_explicit_about_missing_and_corrupted_files() {
        let d = tempfile::tempdir().unwrap();
        let fresh = load(d.path()).unwrap();
        assert!(fresh.candidates.is_empty());
        assert!(!state_path(d.path()).exists(), "读不该凭空造文件");

        crate::fsx::atomic::atomic_write(&state_path(d.path()), b"{ not json").unwrap();
        let err = load(d.path()).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::Corrupted);
    }

    /// 落盘再读回来，字段一个不少（含忽略记录与安装结果）。
    #[test]
    fn a_snapshot_survives_a_roundtrip() {
        let d = tempfile::tempdir().unwrap();
        let s = UpdateSnapshot {
            candidates: vec![candidate("0.0.15")],
            ignored: Some("0.0.15".to_owned()),
            last_result: Some(UpdateResult {
                version: "0.0.14".to_owned(),
                ok: true,
                reason: String::new(),
                at: "2026-11-21T00:00:00Z".to_owned(),
            }),
            ..Default::default()
        };
        save(d.path(), &s).unwrap();
        assert_eq!(load(d.path()).unwrap(), s);
    }

    /// 进程态 → 业务阶段：只有 `Ready` 跨进快照，下载中/暂停不算"可安装"。
    #[test]
    fn only_ready_crosses_into_the_snapshot() {
        assert_eq!(install_phase(&UpdateState::Idle), InstallPhase::Idle);
        assert!(matches!(
            install_phase(&UpdateState::Downloading {
                received: 1,
                total: 2
            }),
            InstallPhase::Idle
        ));
        assert_eq!(
            install_phase(&UpdateState::Ready {
                path: "/tmp/x".to_owned(),
                size: 1,
                version: "0.0.15".to_owned(),
                channel: UpdateChannel::Stable,
            }),
            InstallPhase::Ready {
                version: "0.0.15".to_owned()
            }
        );
    }
}
