//! **应用内更新**的命令壳（2026-10-05 第五刀）。
//!
//! 六个命令，加一个**修 bug 用的** `open_url`：
//!
//! | 命令 | 干什么 | 形状 |
//! |---|---|---|
//! | `update_info` | 一次问全：状态 + 有无新版 + 资产 + 上次安装结果 | 只读 `async` |
//! | `start_update` | 开始下载（有 `asset` 才给开）；**秒回**，进度靠事件 | `async` + 后台任务 |
//! | `pause_update` / `resume_update` / `cancel_update` | 暂停 / 继续 / 取消 | 同步（只动开关） |
//! | `install_update` | **退出本进程** → 后台脚本替换 `.app` → 重新拉起 | `async` |
//! | `open_url` | 在系统默认程序里打开一个 http(s) 链接 | `async` |
//!
//! ★ **`open_url` 是修 bug 来的**：0.0.2 之前「查看更新」是个 `<a target="_blank">`，
//!   而 Tauri 的 webview **没开 opener 权限** ⇒ 点下去**什么都不发生**
//!   （用户 2026-10-05 实测："点了没反应，没弹出浏览器"）。前端不该自己开窗口，
//!   **一律过这个命令**。
//!
//! ★ 事件：下载进度用 `software-update-progress` 推（推的就是 [`UpdateState`]，
//!   **与快照同一个类型** —— 界面只有一种形状要认，不会两套渲染）。

use postprocess::diag::messages;

use std::sync::Arc;

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::error::AppError;
use crate::ipc::traced;
use crate::runtime::release_info::ReleaseAsset;
use crate::runtime::updater::{self, UpdateState};

/// 给前端的更新状态（`UpdateState` 已经是 camelCase 的 `tag` 形状，直接序列化）。
pub type UpdateStateDto = UpdateState;

/// 「上次更新成没成」+ 当前这一版的可下载信息 —— 界面一次问全，别分两次问。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfoDto {
    pub state: UpdateStateDto,
    /// 有新版吗（与 `check_software_update` 同一判定，**不重写一遍**）
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    pub notes: Option<String>,
    /// Release / 下载页（**没有它就没什么可打开的**）
    pub url: Option<String>,
    /// 这一版有没有安装包（**没有 = 只能打开下载页**，界面就不摆下载按钮）
    pub asset: Option<ReleaseAsset>,
    /// 上一次安装的结果（退出后那个脚本写的账，**下次启动**读）
    pub last_result: Option<updater::UpdateResult>,
    /// **安装日志的绝对路径**（`<appData>/run/update-install.log`）。
    ///
    /// 作者 2026-11-06 点了「重启并安装」什么都没发生，而旧脚本一个字都不留 ——
    /// 只能靠猜。把路径交给界面，就是为了让人能**一键把文件发出去**，
    /// 而不是听我说"你去 ~/Library/Application Support/... 里找找"。
    pub install_log: String,
}

/// 读当前数据源、问一次更新（**命令壳与 `update_info` 共用这一处**）。
///
/// 2026-11-05：地址解析 / 版本比较收口到 `runtime::update_check::fetch_update` ——
/// 设置页手动检查、拉目录后的自动检查、这里给标题栏更新面板供数，**三处同一份**。
/// 差别只在失败口径：这里是标题栏那一格，问不到就说"问不到"（不弹错、不打扰）。
pub(crate) fn software_update(
    root: &std::path::Path,
) -> crate::runtime::release_info::SoftwareUpdate {
    crate::runtime::update_check::fetch_update(root)
        .unwrap_or_else(|_| crate::runtime::release_info::none_available())
}

/// 一次问全。**只读**（不联网之外什么都不做；联网那部分与检查更新同一条）。
#[tauri::command]
pub async fn update_info(app: AppHandle) -> Result<UpdateInfoDto, AppError> {
    traced("updateInfo", |_| {
        let root = crate::fsx::paths::internal_root(&app)?;
        let update = software_update(&root);
        Ok(UpdateInfoDto {
            state: updater::session().state(),
            has_update: update.has_update,
            current_version: update.current_version,
            latest_version: update.latest_version,
            notes: update.notes,
            url: update.url,
            asset: update.asset,
            last_result: updater::last_result(&root),
            install_log: updater::install_log_path(&root).display().to_string(),
        })
    })
}

/// 开始下载安装包。**秒回**（下载在后台跑）。
///
/// ★ 2026-11-05 双通道：`channel` 决定下**用户点中的那一路**的资产。
///   - `None` / `"stable"`：旧的单一入口（标题栏 `UpdateIndicator` 仍无参调用）；
///   - `"preview"`：预览版那一档；
///   - 认不出的字符串**显式报错**，不悄悄兜底成正式版 —— 否则「尝试测试版」
///     会把正式版包下下来，用户还以为自己在试预览。
///
/// ★ 界面**只在候选的 `asset` 存在时**才摆这颗按钮。进度靠事件推。
#[tauri::command]
pub async fn start_update(app: AppHandle, channel: Option<String>) -> Result<(), AppError> {
    let root = crate::fsx::paths::internal_root(&app)?;
    let channel = match channel.as_deref().map(str::trim) {
        None | Some("") | Some("stable") => crate::runtime::release_info::UpdateChannel::Stable,
        Some("preview") => crate::runtime::release_info::UpdateChannel::Preview,
        Some(other) => {
            return Err(AppError::invalid_argument(format!(
                "认不出的更新通道 {other:?}（只认 stable / preview）"
            )))
        }
    };

    /* 一次取两份清单（最多两次 GET），资产按用户点中的那一路挑。
     * 取不到候选集时退回旧的「单一正式版」口子 —— 标题栏 / 老前端不传 channel
     * 时行为与改造前逐字一致，不会多打一次网络请求。 */
    let candidates =
        crate::runtime::update_check::fetch_update_candidates(&root).unwrap_or_else(|e| {
            tracing::debug!("候选集没取到，退回单一正式版资产：{}", e.message);
            Vec::new()
        });
    let chosen = candidates.iter().find(|c| c.channel == channel);
    /* 版本号跟着**选中的那一版**走：`Ready` 要带着它，界面才能判"已就绪的是不是这张卡" */
    let (asset, version) = match chosen {
        Some(c) => (c.asset.clone(), c.version.clone()),
        None if channel == crate::runtime::release_info::UpdateChannel::Stable
            && candidates.is_empty() =>
        {
            let u = software_update(&root);
            (u.asset.clone(), u.latest_version.clone())
        }
        None => (None, String::new()),
    };

    if channel == crate::runtime::release_info::UpdateChannel::Stable
        && chosen.is_some_and(|c| {
            !crate::runtime::release_info::compare(
                &c.version,
                crate::runtime::structure::APP_VERSION,
            )
        })
    {
        return Err(AppError::invalid_argument(messages::text(
            "ui.ipc.update.156",
            &[],
        )));
    }
    let asset = asset
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
///   替换成没成本进程看不见（已经退了）—— 脚本把结果写进 `run/update-result.json`，
///   **下次启动**由 [`update_info`] 读出来说清（判据 `update_result_is_read_on_next_launch`）。
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
    /* ★ 记的是**装上去的那一版**（2026-11-06 之前记的是本进程的 `APP_VERSION`：
     *   装完 0.0.10-preview.2，账上却写 `0.0.10-preview.1`，看日志只会更糊涂）。
     *   远端清单是外部输入 ⇒ 进脚本前先洗一遍。 */
    let version = updater::safe_version(&version);
    let version = version.as_str();
    let at = now_iso8601_local();

    /* ★ 平台分流（2026-11-05）：
     * macOS 沿用「替换 .app + open」，Windows 走「NSIS /S 静默安装 + 拉起 exe」。
     * 两边都把结果写进同一份 `run/update-result.json`，下次启动由 update_info 读。 */
    #[cfg(target_os = "windows")]
    {
        let new_exe = updater::running_exe()?;
        let script = updater::windows_install_script(&root, &staged, &new_exe, version, &at);
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
        /* 把自己这一条的 PID 交给脚本：它要**等这个进程真的消失**才动手。
         * 不传的话脚本只能盲等 1 秒，而旧实例还活着时 `open` 只会把它的窗口叫到前面
         * —— 作者 2026-11-06 看到的"点了没反应"就是这个。 */
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
    // 客户端**不带 tokio**：`block_on` 一个"睡 200ms"是这里最省事、也不引依赖的做法。
    tauri::async_runtime::block_on(async {
        std::thread::sleep(std::time::Duration::from_millis(200));
    });
    /* ★ 不在这里 `cleanup_stage`（2026-11-05 修掉的时序 bug）：
     * 后台脚本 `sleep 1` 之后才 `mv` 暂存里的 `.app` / 才 `/S` 装 setup.exe，
     * 而本进程只等 200ms 就退出 —— 立刻清暂存等于把脚本要用的东西删了。
     * 清理交给下一次 `download`（它开头会先清旧暂存），与「失败即停、如实说」不冲突。 */
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

/// 这一刻（ISO8601，UTC）——**客户端没有 `workbench::clock`**，这一处用系统时钟自己算。
/// 格式与工作台那套一致（`2026-10-05T04:01:08Z`），免得同一份账两种写法。
fn now_iso8601_local() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default();
    // 粗算但够用：按天折算成日期 + 时分秒（闰年细节交给系统时钟的秒数，不做历法运算）
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
