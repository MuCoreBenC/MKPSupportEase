//! **软件更新**的命令壳（2026-11-22 统一状态那一刀）。
//!
//! # 一句话规矩
//!
//! 这一层**只读快照、只跑事务**：状态全在 `runtime::update_state`（`run/update-state.json`），
//! 检查/忽略/作废都走 `runtime::update_check` 的四个事务。命令壳不联网算候选；
//! 只读命令就只读 —— 清旧账只在事务里发生。
//!
//! # 命令表
//!
//! | 命令 | 干什么 | 写盘 |
//! |---|---|---|
//! | `get_update_snapshot` | 读整份快照（标题栏 / 设置页 / 更新面板共用） | 否 |
//! | `check_software_update` | 手动检查（事务 `check`） | 是 |
//! | `ignore_update` | 忽略某一版（事务 `ignore`） | 是 |
//! | `migrate_update_ignored` | 前端把 localStorage 的旧忽略记录交过来（一次性迁移握手） | 是 |
//! | `start_update` | 从**快照的候选**里挑资产开始下载（不联网挑版本） | 否 |
//! | `pause_update` / `resume_update` / `cancel_update` | 只动开关 | 否 |
//! | `install_update` | 退出本进程，后台脚本替换并拉起 | 否（脚本写旧账） |
//! | `open_url` | 打开 http(s) 链接 | 否 |
//!
//! # 两个事件
//!
//! - `software-update-state`：整份快照（业务事实，`runtime::update_check` 发）；
//! - `software-update-progress`：下载进度（进程态，这一层发）。
//!
//! 没有第三个 `software-update-available`：通知由前端按快照里的候选派生。

use postprocess::diag::messages;

use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::ipc::traced;
use crate::runtime::release_info::{ReleaseAsset, UpdateCandidate};
use crate::runtime::update_state::{
    self, CheckRecord, InstallPhase, UpdateSnapshot, UPDATE_STATE_SCHEMA,
};
use crate::runtime::updater::{self, UpdateState};

/// 给前端的更新状态（`UpdateState` 已是 camelCase 的 `tag` 形状，直接序列化）。
pub type UpdateStateDto = UpdateState;

/// **整份更新快照 + 进程态** —— 界面一次问全，不再分两次问。
///
/// 业务字段全部来自 `UpdateSnapshot`；`state` / `install` 是本次进程的下载态，
/// 重启即 `idle`（不会被当成"还在下载"）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSnapshotDto {
    /// 下载这一档的进程态（`idle` / `downloading` / `paused` / `ready` / `failed` / `cancelled`）
    pub state: UpdateStateDto,
    /// 业务阶段：有没有一份**已经下好、等着装**的
    pub install: InstallPhase,
    /// 有没有新版（候选非空）。被忽略的那一版仍算"有新版"，只是不冒胶囊。
    pub has_update: bool,
    /// 标题栏胶囊读的那一版（被忽略 / 不是当前渠道 ⇒ `None`）
    pub pending_version: Option<String>,
    /// 并列候选（正式版排第一）。设置页按渠道切换时读它。
    pub candidates: Vec<UpdateCandidate>,
    pub current_version: String,
    /// 当前渠道上第一条候选的版本（没有则是当前版本）
    pub latest_version: String,
    pub notes: Option<String>,
    /// Release / 下载页（没有它就没什么可打开的）
    pub url: Option<String>,
    /// 这一版有没有安装包（没有 = 只能打开下载页）
    pub asset: Option<ReleaseAsset>,
    /// 上一次安装的结果（旧账已迁入快照；旧文件不再被读）
    pub last_result: Option<updater::UpdateResult>,
    /// 安装日志的绝对路径（点「重启并安装」没反应时要看的就是它）
    pub install_log: String,
    /// 最近一次检查的记录（`ok=false` 时界面要说"这次没查成"）
    pub checked: CheckRecord,
    /// 用户忽略的那一版
    pub ignored: Option<String>,
    /// 快照协议代次
    pub schema: u32,
}

/// 快照 + 进程态 → 界面那一份（派生只在这里算一次，前端不重复推导）。
fn to_dto(root: &std::path::Path, snap: &UpdateSnapshot) -> UpdateSnapshotDto {
    let state = updater::session().state();
    let chosen = snap
        .candidates
        .iter()
        .find(|c| c.channel == snap.checked.channel);
    let pending_version = update_state::pending(snap).map(|c| c.version.clone());
    let latest_version = chosen
        .map(|c| c.version.clone())
        .unwrap_or_else(|| snap.current.clone());
    UpdateSnapshotDto {
        install: update_state::install_phase(&state),
        state,
        has_update: update_state::has_update(snap),
        pending_version,
        candidates: snap.candidates.clone(),
        current_version: snap.current.clone(),
        latest_version,
        notes: chosen.and_then(|c| c.notes.clone()),
        url: chosen.and_then(|c| c.url.clone()),
        asset: chosen.and_then(|c| c.asset.clone()),
        last_result: snap.last_result.clone(),
        install_log: updater::install_log_path(root).display().to_string(),
        checked: snap.checked.clone(),
        ignored: snap.ignored.clone(),
        schema: UPDATE_STATE_SCHEMA,
    }
}

/// 一次问全。**只读**：不联网、不写盘（写盘只在事务里）。
#[tauri::command]
pub async fn get_update_snapshot(app: AppHandle) -> Result<UpdateSnapshotDto, AppError> {
    traced("getUpdateSnapshot", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let snap = update_state::load(&root)?;
        Ok(to_dto(&root, &snap))
    })
}

/// 手动检查（设置页「检查更新」）。**失败如实拒绝** —— 设置页要把原因摆出来。
///
/// 成功与失败都落在同一个事务里：失败只更新检查记录，旧候选保留。
#[tauri::command]
pub async fn check_software_update(app: AppHandle) -> Result<UpdateSnapshotDto, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("checkSoftwareUpdate", |_| {
            let root = crate::fsx::paths::internal_root(&app)?;
            /* 在线增强关着 ⇒ 不问版本（见 runtime::online） */
            crate::runtime::online::require(&root)?;
            let snap = crate::runtime::update_check::run_check(
                &root,
                crate::runtime::update_state::CheckTrigger::Manual,
            )?;
            Ok(to_dto(&root, &snap))
        })
    });
    task.await.map_err(|e| {
        AppError::internal(messages::text("ui.ipc.catalog.080", &[])).with_detail(e.to_string())
    })?
}

/// 忽略某一版（标题栏面板那颗「忽略」）。**只动 `ignored`，候选集不动。**
#[tauri::command]
pub async fn ignore_update(app: AppHandle, version: String) -> Result<UpdateSnapshotDto, AppError> {
    traced("ignoreUpdate", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let snap = crate::runtime::update_check::ignore(&root, &version)?;
        Ok(to_dto(&root, &snap))
    })
}

/// **一次性迁移握手**：前端把旧 localStorage 里的忽略版本交过来。
///
/// 幂等：已经迁过 / 那个版本已经不需要忽略 ⇒ 照常返回快照（不算失败）。
/// 前端**只有收到 `Ok` 才删旧键** —— 中断了下次再来一遍也不会丢。
#[tauri::command]
pub async fn migrate_update_ignored(
    app: AppHandle,
    value: Option<String>,
) -> Result<UpdateSnapshotDto, AppError> {
    traced("migrateUpdateIgnored", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let mut snap = update_state::load(&root)?;
        let current = crate::runtime::structure::APP_VERSION;
        let mut dirty = update_state::reconcile(&mut snap, current);
        if let Some(v) = value.map(|s| s.trim().to_owned()).filter(|s| !s.is_empty()) {
            if snap.ignored.is_none() && crate::runtime::release_info::is_newer_version(&v, current)
            {
                snap.ignored = Some(v);
                dirty = true;
            }
        }
        if dirty {
            update_state::save(&root, &snap)?;
        }
        Ok(to_dto(&root, &snap))
    })
}

/// 开始下载安装包。**秒回**（下载在后台跑）。
///
/// 版本与资产**从快照的候选里挑**，不再联网重算一遍：
/// 用户点的是哪张卡，下的就是那一版。快照里没有候选 = 先检查一次再说。
#[tauri::command]
pub async fn start_update(app: AppHandle, channel: Option<String>) -> Result<(), AppError> {
    let root = crate::fsx::paths::internal_root(&app)?;
    /* 在线增强关着 ⇒ 不下安装包（见 runtime::online） */
    crate::runtime::online::require(&root)?;
    let channel = match channel.as_deref().map(str::trim) {
        None | Some("") | Some("stable") => crate::runtime::release_info::UpdateChannel::Stable,
        Some("preview") => crate::runtime::release_info::UpdateChannel::Preview,
        Some(other) => {
            return Err(AppError::invalid_argument(format!(
                "认不出的更新通道 {other:?}（只认 stable / preview）"
            )))
        }
    };

    let snap = update_state::load(&root)?;
    let chosen = snap.candidates.iter().find(|c| c.channel == channel);
    let Some(chosen) = chosen else {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.update.157",
            &[],
        )));
    };
    let version = chosen.version.clone();
    let asset = chosen
        .asset
        .clone()
        .ok_or_else(|| AppError::invalid_argument(messages::text("ui.ipc.update.157", &[])))?;

    let emit_app = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        traced("startUpdate", |_| {
            let sink = Arc::new(move |state: UpdateState| {
                use tauri::Emitter as _;
                let _ = emit_app.emit(updater::PROGRESS_EVENT, &state);
            });
            updater::download(&asset, &version, channel, &root, &*sink)
        })
    });
    Ok(())
}

/// 暂停 / 继续 / 取消 —— **只动开关**（状态由下载循环如实推进，界面不自己编）。
#[tauri::command]
pub fn pause_update() {
    updater::pause();
}

#[tauri::command]
pub fn resume_update() {
    updater::resume();
}

#[tauri::command]
pub fn cancel_update() {
    updater::cancel();
}

/// **装上去并重启**。
///
/// ★ 必须**先退出本进程**：正在运行的 `.app` 换不掉（代码段在执行）。所以
///   spawn 一个脱离本进程的后台脚本 → `app.exit(0)`。
///   替换成没成进程看不见（已经退了）—— 脚本把结果写进 `run/update-result.json`，
///   **下次启动**由 `update_check::startup` 迁进快照（那份文件从此不再当事实源）。
#[tauri::command]
pub async fn install_update(app: AppHandle) -> Result<(), AppError> {
    let root = crate::fsx::paths::internal_root(&app)?;
    let UpdateState::Ready { path, version, .. } = updater::session().state() else {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.update.158",
            &[],
        )));
    };
    let staged = std::path::PathBuf::from(&path);
    /* ★ 记的是**装上去的那一版**。远端清单是外部输入 ⇒ 进脚本前先洗一遍。 */
    let version = updater::safe_version(&version);
    let version = version.as_str();
    let at = update_state::now_iso8601();

    /* ★ 平台分流：macOS 替换 `.app` + `open`，Windows 走 NSIS `/S` 静默安装 + 拉起 exe。
     * 两边都把结果写进同一份 `run/update-result.json`，下次启动迁进快照。 */
    #[cfg(target_os = "windows")]
    {
        let new_exe = updater::running_exe()?;
        /* 绿色版：更新只换掉自己目录里的 `SupportEase.exe`，绝不落进安装目录 ——
        否则一次更新会把绿色版变成另一份安装版（作者 2026-11-20 真机踩的）。 */
        let portable_dir = if crate::fsx::paths::is_portable_install() {
            new_exe.parent().map(std::path::Path::to_path_buf)
        } else {
            None
        };
        let script = updater::windows_install_script(
            &root,
            &staged,
            &new_exe,
            version,
            &at,
            portable_dir.as_deref(),
        );
        let mut cmd = std::process::Command::new("powershell");
        cmd.args(["-NoProfile", "-NonInteractive", "-Command", &script]);
        // 不弹控制台窗口（DETACHED_PROCESS | CREATE_NO_WINDOW 的形状由 Windows SDK 常量给）
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            cmd.creation_flags(0x0800_0000);
        }
        cmd.spawn().map_err(|e| {
            AppError::io(messages::text("ui.ipc.update.159", &[])).with_detail(e.to_string())
        })?;
    }
    #[cfg(not(target_os = "windows"))]
    {
        let target = updater::running_app()?;
        /* 把自己这一条的 PID 交给脚本：它要**等这个进程真的消失**才动手。 */
        let script =
            updater::install_script(&root, &target, &staged, version, &at, std::process::id());
        std::process::Command::new("sh")
            .arg("-c")
            .arg(&script)
            .spawn()
            .map_err(|e| {
                AppError::io(messages::text("ui.ipc.update.159", &[])).with_detail(e.to_string())
            })?;
    }
    // 给脚本一点时间把最后一句排进 shell（★ 退出后这一段才真正开始）。
    tauri::async_runtime::block_on(async {
        std::thread::sleep(std::time::Duration::from_millis(200));
    });
    /* ★ 不在这里 `cleanup_stage`：后台脚本还要用暂存里那份。清理交给下一次下载
     *   （它开头会先清）与启动时的 `startup` 收敛。 */
    app.exit(0);
    Ok(())
}

/// 在系统默认程序里打开一个 **http(s)** 链接。
///
/// ★ 只收 http(s)（`file:` 与自定义 scheme 一律拒）：这个命令的前端入口是
///   「查看更新」这类"点一下去下载"，**不是**"让网页调本地协议"。
#[tauri::command]
pub async fn open_url(app: AppHandle, url: String) -> Result<(), AppError> {
    let u = url.trim().to_owned();
    if !(u.starts_with("https://") || u.starts_with("http://")) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.update.160",
            &[],
        )));
    }
    traced("openUrl", |_| {
        app.opener().open_url(u, None::<&str>).map_err(|e| {
            AppError::io(messages::text("ui.ipc.update.161", &[])).with_detail(e.to_string())
        })
    })
}
