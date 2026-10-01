//! 新数据世界的读口：客户端的运行时 catalog 与下载管道的命令面。
//!
//! 读走 [`runtime::load_released_catalog`]：只读**释放进数据根的那一份**
//! （`<appDataDir>/catalog.json`）——铁律 4：运行时只认自己的运行时数据。
//!
//! 三条读命令**首屏零网络**（铁律 2）；联网的只有十问 #7 那两件事：
//! **下载**（拿文件）与**检查更新**（拿清单），两条都要用户点了才做，且都走
//! [`runtime::net`]，共用同一个 Source trait —— 管道不知道网络存在。
//!
//! # 远端在哪
//!
//! **地址不在这一层写死，也不写在 catalog 里**：`presets/` 是内容源，换 Gitee、
//! 换成自己的 CDN 是部署的事，不该为此重发一次说明书。地址来自
//! [`runtime::source`]（`run/preset-source.json`，或构建期注入的默认值），
//! 不知道就**诚实报没配**——不猜一个 URL、不假装下载成功。

use std::path::Path;

use serde::Serialize;
use sha2::Digest;
use tauri::ipc::Channel;
use tauri::AppHandle;

use crate::error::AppError;
use crate::fsx::paths::internal_root;
use crate::runtime;

use super::traced;

#[tauri::command]
pub async fn get_runtime_catalog(app: AppHandle) -> Result<runtime::Catalog, AppError> {
    traced("getRuntimeCatalog", |_| {
        let root = internal_root(&app)?;
        runtime::load_released_catalog(&root)
    })
}

/* ---------- 数据源的设置（重装 / 换 Gitee / 换自建 CDN 的入口） ---------- */

/// 给界面的当前数据源
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSourceDto {
    pub base_url: String,
    /// `true` = 用户在界面里填的（写进了设置文件）；`false` = 构建期注入的出厂默认值，还没被人动过。
    /// 界面据此把"默认值"与"你选的"分开说——不然用户不知道当前的地址是自己改的还是出厂的
    pub from_user: bool,
}

/// 当前数据源。`null` = 一个都没配（既没有设置文件，也没有出厂默认值）—
/// **装配那一步由用户完成**，程序不替他猜
#[tauri::command]
pub async fn get_preset_source(app: AppHandle) -> Result<Option<PresetSourceDto>, AppError> {
    traced("getPresetSource", |_| {
        let root = internal_root(&app)?;
        if let Some(stored) = runtime::source::load_source(&root)? {
            return Ok(Some(PresetSourceDto {
                base_url: stored.base_url,
                from_user: true,
            }));
        }
        Ok(
            runtime::source::builtin_default().map(|base_url| PresetSourceDto {
                base_url,
                from_user: false,
            }),
        )
    })
}

/// 换数据源：写完立刻生效（下一次下载就用新的），并显示 writing 出来的那份。
/// 地址不合法在这一层就被拒：来自 `normalize_base_url`
#[tauri::command]
pub async fn set_preset_source(
    app: AppHandle,
    base_url: String,
) -> Result<PresetSourceDto, AppError> {
    traced("setPresetSource", |_| {
        let root = internal_root(&app)?;
        let saved = runtime::source::save_source(&root, &base_url)?;
        Ok(PresetSourceDto {
            base_url: saved.base_url,
            from_user: true,
        })
    })
}

/// 当前数据源的地址 —— 今天所有联网动作（下载 / 检查更新 / 应用更新）的唯一入口。
///
/// **没配就报错，并且要说清去哪儿配**：这一步最容易被写成 `NOT_FOUND`（那是"目录里
/// 没有这个文件"的意思）或干脆返回一个空结果。两者都会让界面显示一句用户无从行动的话。
fn remote_base(root: &Path) -> Result<String, AppError> {
    let stored = runtime::source::current_base_url(root)?;
    stored.ok_or_else(|| {
        AppError::not_implemented(
            "还没配置数据源地址：去「同步」页填一个（官方源 / Gitee / 自己的服务器都行）",
        )
    })
}

/// 一次下载的水位（走 Channel 送回调用方那条 IPC —— 一次调用一路流式事件，
/// 不是全局广播：将来的"任务中心"要的是另一件事，等它真来了再说）
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTick {
    /// [`runtime::net::Stage`] 的那个词
    pub stage: String,
    pub file_name: String,
    pub received: u64,
    /// 服务端没给长度时是 `null` —— 界面就别说百分比
    pub total: Option<u64>,
    /// 只用在 `failed` 上：失败的原因。**来自后端，界面不造句**
    pub message: Option<String>,
}

/// 读一份**已经下载**的交付文件的正文（前端不碰文件系统，所以要有这一条）。
///
/// **只认 catalog 登记过的落点**：入参是文件名，落点由目录给（`resolve_in` 再过一道防穿越）。
/// 于是"能不能读"只有一个答案来源——目录说有这份、盘上也在，才读得出来。
///
/// 这一刀只服务**文本类**资源（BBS 配置就是 JSON）：字节不是 UTF-8 就如实报错，
/// 不猜编码、不做半截解码。图片 / 模型那一类将来要么走 asset 协议、要么另加一条口子。
#[tauri::command]
pub async fn read_downloaded_text(app: AppHandle, file_name: String) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("readDownloadedText", |_| {
            let root = internal_root(&app)?;
            let catalog = runtime::load_released_catalog(&root)?;
            let file = catalog
                .files
                .iter()
                .find(|f| f.file_name == file_name)
                .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name}")))?;

            let target = crate::fsx::paths::resolve_in(&root, &file.path)?;
            let bytes = std::fs::read(&target).map_err(|_| {
                AppError::not_found(format!(
                    "{file_name} 还不在本机——先下载，再读它（下载区初始是空的）"
                ))
            })?;
            String::from_utf8(bytes).map_err(|_| {
                AppError::corrupted(format!("{file_name} 不是 UTF-8 文本，这一条读不出来"))
            })
        })
    });
    task.await
        .map_err(|e| AppError::internal("读文件没跑到终局").with_detail(e.to_string()))?
}

/// 把 catalog 里登记的一份文件从数据源拉进下载区（`mkp/`）。
///
/// 文件在哪 = [地址](remote_base) + catalog 记的相对位置，地址在 catalog 之外
/// ——换源不用重发说明书。字节对不上 SHA 就整个拒绝——`mkp/` 里不会出现坏文件。
///
/// 网络与磁盘都是阻塞 IO，丢进线程池：命令体跑在异步运行时上，直接阻塞会把整个运行时堵住。
#[tauri::command]
pub async fn download_runtime_file(
    app: AppHandle,
    file_name: String,
    on_tick: Channel<DownloadTick>,
) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("downloadRuntimeFile", |_| {
            let root = internal_root(&app)?;
            let catalog = runtime::load_released_catalog(&root)?;
            let file = catalog
                .files
                .iter()
                .find(|f| f.file_name == file_name)
                .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name}")))?;

            let expected = file.size;
            let forward = |tick: &runtime::net::Tick| {
                send_tick(
                    &on_tick,
                    tick.stage,
                    &tick.file_name,
                    tick.received,
                    tick.total,
                    None,
                );
            };
            let remote = runtime::net::RemoteSource::new(remote_base(&root)?, &forward);
            let outcome = runtime::delivery::deliver(&root, file, &remote);

            match &outcome {
                Ok(_) => send_tick(
                    &on_tick,
                    runtime::net::Stage::Done,
                    &file_name,
                    expected,
                    Some(expected),
                    None,
                ),
                Err(e) => send_tick(
                    &on_tick,
                    runtime::net::Stage::Failed,
                    &file_name,
                    0,
                    Some(expected),
                    Some(e.message.clone()),
                ),
            }

            let target = outcome?;
            Ok(target
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| file_name.clone()))
        })
    });
    task.await
        .map_err(|e| AppError::internal("下载任务没能跑到终局").with_detail(e.to_string()))?
}

/// 多份下载里某一份的结局。**来自 [`runtime::delivery::FileOutcome`]**，
/// 消息原样给界面——评语不由单页造句
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadOutcomeDto {
    pub file_name: String,
    pub ok: bool,
    pub message: String,
}

/// 一次取多份：**并发在 Rust 侧**（上层 = 界面，不该自己发明并发与汇总口径）。
///
/// 返回结果按**请求顺序**，所以界面有一张稳定的列表；水位按完成顺序流式回来，
/// 所以界面能同时表现出"在动"。失败的那几份单独列出原因，成功的不受影响。
#[tauri::command]
pub async fn download_runtime_files(
    app: AppHandle,
    file_names: Vec<String>,
    on_tick: Channel<DownloadTick>,
) -> Result<Vec<DownloadOutcomeDto>, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("downloadRuntimeFiles", |_| {
            let root = internal_root(&app)?;
            let catalog = runtime::load_released_catalog(&root)?;

            // 先对一遍清单：**目录里没有的那份是调用方的错**，整次拒绝 ——
            // 否则会变成"看起来下了七份，其中两份其实没人听说过"
            let mut wanted = Vec::with_capacity(file_names.len());
            for name in &file_names {
                let file = catalog
                    .files
                    .iter()
                    .find(|f| f.file_name == *name)
                    .ok_or_else(|| AppError::not_found(format!("目录里没有 {name}")))?;
                wanted.push(file.clone());
            }

            let forward = |tick: &runtime::net::Tick| {
                send_tick(
                    &on_tick,
                    tick.stage,
                    &tick.file_name,
                    tick.received,
                    tick.total,
                    None,
                );
            };
            let remote = runtime::net::RemoteSource::new(remote_base(&root)?, &forward);

            let report = |outcome: &runtime::delivery::FileOutcome| {
                let size = wanted
                    .iter()
                    .find(|f| f.file_name == outcome.file_name)
                    .map(|f| f.size);
                let stage = if outcome.ok {
                    runtime::net::Stage::Done
                } else {
                    runtime::net::Stage::Failed
                };
                send_tick(
                    &on_tick,
                    stage,
                    &outcome.file_name,
                    size.unwrap_or(0),
                    size,
                    if outcome.ok {
                        None
                    } else {
                        Some(outcome.message.clone())
                    },
                );
            };

            let outcomes = runtime::delivery::deliver_all(&root, &wanted, &remote, &report);
            Ok(outcomes
                .into_iter()
                .map(|o| DownloadOutcomeDto {
                    file_name: o.file_name,
                    ok: o.ok,
                    message: o.message,
                })
                .collect())
        })
    });
    task.await
        .map_err(|e| AppError::internal("批量下载没跑到终局").with_detail(e.to_string()))?
}

/// 水位发货。**Channel 关了（调用方已经不再听）不是错误** —— 记一行 debug 就够，
/// 不能因为没人听而把整次下载判失败
fn send_tick(
    on_tick: &Channel<DownloadTick>,
    stage: runtime::net::Stage,
    file_name: &str,
    received: u64,
    total: Option<u64>,
    message: Option<String>,
) {
    if let Err(e) = on_tick.send(DownloadTick {
        stage: stage.as_str().to_owned(),
        file_name: file_name.to_owned(),
        received,
        total,
        message,
    }) {
        tracing::debug!("下载进度送不出去（没人听了？）：{e}");
    }
}

/// 已经下载到下载区的文件名（盘就是底账：文件在且 SHA 对得上才算数）
#[tauri::command]
pub async fn get_downloaded_files(app: AppHandle) -> Result<Vec<String>, AppError> {
    traced("getDownloadedFiles", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        Ok(runtime::delivery::downloaded_files(&root, &catalog))
    })
}

/// 有更新的文件名（盘上在、字节与目录不一样）。"更新"就是对这些再跑一遍下载——
/// 旧份自动归档，没有单独的更新代码路径
#[tauri::command]
pub async fn get_stale_files(app: AppHandle) -> Result<Vec<String>, AppError> {
    traced("getStaleFiles", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        Ok(runtime::delivery::stale_files(&root, &catalog))
    })
}

/// 归档区里的一份旧版本（给界面看的形状）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedFileDto {
    /// 相对内部根的路径（`archive/mkp/presets/A1-fast.toml`）—— 读正文时把它交回来
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 这份旧版本被换下来的时刻（UTC epoch 秒）。**界面自己转人话** ——
    /// 默认构建不引时间库（`time` 只挂在 workbench feature 下），别为一行时间戳把它拉进来
    pub modified_unix: Option<u64>,
    /// 认得出是谁的旧版本就给；**认不出照实留空**（目录里已经没有这一份了：换源 / 下线）
    pub machine_id: Option<String>,
    pub version_id: Option<String>,
    pub kind: Option<String>,
}

/// 归档区里有什么（官方文件换版本时，被换下来的那一份）。
///
/// 归档**不是用户修改历史**：它是官方版本生命周期的一部分 —— 换版本时旧份进
/// `archive/`（保留最早一份，不覆盖、不删）。用户改出来的东西是**另一条线**
/// （另存成另一份文件），永远不回写官方原件。
///
/// 这条读只回答"盘上躺着哪些旧版本、认得出是谁的"：**不提供删除、不提供恢复**
/// （那是归档管理的活，这一层不做）。
#[tauri::command]
pub async fn get_archived_files(app: AppHandle) -> Result<Vec<ArchivedFileDto>, AppError> {
    traced("getArchivedFiles", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let prefix = format!("{}/", runtime::paths::ARCHIVE_DIR);
        Ok(runtime::delivery::archived_files(&root)
            .into_iter()
            .map(|a| {
                /* 认人靠"同位"：`archive/mkp/presets/x.toml` ↔ 目录里的 `mkp/presets/x.toml`
                （同名，新字节）。**不解析文件名**去猜机型版本 —— 名字规则将来会变，
                而"归档这份与目录里哪一份同位"是一个不需要额外知识的事实 */
                let known = a
                    .path
                    .strip_prefix(&prefix)
                    .and_then(|rel| catalog.files.iter().find(|f| f.path == rel));
                ArchivedFileDto {
                    path: a.path,
                    file_name: a.file_name,
                    size: a.size,
                    modified_unix: a.modified_unix,
                    machine_id: known.map(|f| f.machine_id.clone()),
                    version_id: known.map(|f| f.version_id.clone()),
                    kind: known.map(|f| f.kind.clone()),
                }
            })
            .collect())
    })
}

/// 读归档区里某一份旧版本的正文（旧版 TOML）。
///
/// **只认归档区**：入参是 [`get_archived_files`] 给的那个相对路径，这里再核一次前缀
/// 并过防穿越 —— 归档区之外的东西这条读一概不碰（下载区有它自己那条读）。
/// 只服务文本类资源：不是 UTF-8 就如实报错，不猜编码。
#[tauri::command]
pub async fn read_archived_text(app: AppHandle, path: String) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("readArchivedText", |_| {
            let root = internal_root(&app)?;
            let rel = path.trim_start_matches('/').to_owned();
            let prefix = format!("{}/", runtime::paths::ARCHIVE_DIR);
            if !rel.starts_with(&prefix) {
                return Err(AppError::invalid_argument(format!(
                    "只读归档区里的文件（要 {prefix}… 开头，给的是 {rel}）"
                )));
            }
            let target = crate::fsx::paths::resolve_in(&root, &rel)?;
            let bytes = std::fs::read(&target).map_err(|_| {
                AppError::not_found(format!("归档里没有 {rel} —— 它可能已经被清掉了"))
            })?;
            String::from_utf8(bytes)
                .map_err(|_| AppError::corrupted(format!("{rel} 不是 UTF-8 文本，这一条读不出来")))
        })
    });
    task.await
        .map_err(|e| AppError::internal("读归档没跑到终局").with_detail(e.to_string()))?
}

/* ---------- 使用中指针（第一圈 ⑤：用户状态的第一个真数据） ---------- */

/// 给界面的使用中状态：指针 + 从目录反查出来的机型/版本 + 文件是否还是当时那份
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePresetDto {
    pub file_name: String,
    pub sha256: String,
    /// 目录里已经没有这份时是空串（目录更新了、状态还在——过渡期的诚实表达）
    pub machine_id: String,
    pub version_id: String,
    /// 盘上的文件还是不是应用时刻的那份（`mkp/` 是只读区，正常恒 true）
    pub intact: bool,
}

fn active_dto(
    root: &Path,
    catalog: &runtime::Catalog,
    state: runtime::state::ActivePreset,
) -> ActivePresetDto {
    let listed = catalog
        .files
        .iter()
        .find(|f| f.file_name == state.file_name);
    ActivePresetDto {
        machine_id: listed.map(|f| f.machine_id.clone()).unwrap_or_default(),
        version_id: listed.map(|f| f.version_id.clone()).unwrap_or_default(),
        intact: runtime::state::active_matches_disk(root, &state),
        file_name: state.file_name,
        sha256: state.sha256,
    }
}

/// 当前使用的是哪一份。`null` = 还没用任何一份（合法状态，不是错误）
#[tauri::command]
pub async fn get_active_preset(app: AppHandle) -> Result<Option<ActivePresetDto>, AppError> {
    traced("getActivePreset", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        match runtime::state::load_active(&root)? {
            None => Ok(None),
            Some(state) => Ok(Some(active_dto(&root, &catalog, state))),
        }
    })
}

/// 「使用这一份」：把目录里登记的某份下载文件记成使用中。全局唯一——
/// 产品规则定死了同一时刻只能有一份处于已应用状态，构造上就是"一个文件"。
#[tauri::command]
pub async fn apply_active_preset(
    app: AppHandle,
    file_name: String,
) -> Result<ActivePresetDto, AppError> {
    traced("applyActivePreset", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let file = catalog
            .files
            .iter()
            .find(|f| f.file_name == file_name)
            .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name}")))?;

        // 应用的是盘上那份：字节得真的在、且与目录对得上（没下载/被删/漂了都不许应用）
        let on_disk = std::fs::read(root.join(&file.path)).ok();
        let bytes = on_disk.ok_or_else(|| {
            AppError::not_found(format!("{file_name} 还不在本机——先下载，再使用"))
        })?;
        let digest = runtime::catalog::hex(&sha2::Sha256::digest(&bytes));
        if digest != file.sha256 {
            return Err(AppError::sha_mismatch(format!(
                "{file_name} 盘上的内容与目录对不上，拒绝应用"
            )));
        }

        let state = runtime::state::save_active(&root, file)?;
        Ok(active_dto(&root, &catalog, state))
    })
}

/// 撤销使用。幂等：本来就没在用也不报错
#[tauri::command]
pub async fn clear_active_preset(app: AppHandle) -> Result<(), AppError> {
    traced("clearActivePreset", |_| {
        let root = internal_root(&app)?;
        runtime::state::clear_active(&root)
    })
}

/* ---------- 检查 / 应用更新（两端共用契约的消费者侧） ---------- */

/// 给界面的更新检查结果
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteUpdateDto {
    pub up_to_date: bool,
    pub local_revision: String,
    pub remote_revision: String,
}

/// 对远端目录检查更新：**清单从数据源来**（`<base>/catalog.json`），与文件本体同一个
/// 地址概念——不会出现"清单在这个源、文件在另一个源"的错位。比较逻辑见 [`runtime::update`]。
#[tauri::command]
pub async fn check_remote_update(app: AppHandle) -> Result<RemoteUpdateDto, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("checkRemoteUpdate", |_| {
            let root = internal_root(&app)?;
            let bytes = runtime::net::get_manifest(&remote_base(&root)?)?;
            let remote = runtime::Catalog::parse(&bytes)?;
            let local = runtime::load_released_catalog(&root)?;
            let r = runtime::update::check(&local, &remote);
            Ok(RemoteUpdateDto {
                up_to_date: r.up_to_date,
                local_revision: r.local_revision,
                remote_revision: r.remote_revision,
            })
        })
    });
    task.await
        .map_err(|e| AppError::internal("检查更新没跑到终局").with_detail(e.to_string()))?
}

/// 应用远端目录：旧目录归档（release 管道）、新目录生效。之后 Stale 文件照常出现在
/// 「有更新」里，用既有的下载管道拉新——更新没有第三条路径。
#[tauri::command]
pub async fn apply_remote_update(app: AppHandle) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("applyRemoteUpdate", |_| {
            let root = internal_root(&app)?;
            let bytes = runtime::net::get_manifest(&remote_base(&root)?)?;
            let report = runtime::release::release_bytes(&root, &bytes)?;
            Ok(report.summary())
        })
    });
    task.await
        .map_err(|e| AppError::internal("应用远端目录没跑到终局").with_detail(e.to_string()))?
}
