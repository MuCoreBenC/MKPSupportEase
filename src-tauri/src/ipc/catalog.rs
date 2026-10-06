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
//! [`runtime::source`]（AppState 的 presetSource 格，或构建期注入的默认值），
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

/// 给界面的一个**内置源**选项（作者 2026-10-05 拍"两个官方源 + 收起的自定义"）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinSourceDto {
    /// `github` / `gitee` —— **id 是契约**（盘上 / 界面 / 前端都认它）
    pub id: String,
    /// 给用户看的那一句
    pub label: String,
    /// 这个源的 Bootstrap 地址
    pub address: String,
}

/// 给界面的当前数据源
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PresetSourceDto {
    /// 用户选了哪一个：`github` / `gitee` / `custom`
    pub mode: String,
    /// 给用户看的模式名
    pub label: String,
    /// **当前生效的入口地址**（内置 = 构建期注入的那个；自定义 = 用户填的根）。
    /// 空串 = 选了自定义但没填地址（界面据此把输入框摆出来并报错，**不悄悄用别的源**）
    pub address: String,
    /// `true` = 用户自己填的（`custom`）
    pub from_user: bool,
    /// **两个内置源**（没注入的那个不出现）。界面摆两个固定单选就靠它 ——
    /// 用户没有输错地址的机会，这是防错的第一道
    pub builtin: Vec<BuiltinSourceDto>,
    /// 出厂默认是哪一个（`github`）：界面上「恢复默认」那一句要说得清它是什么
    pub default_mode: String,
    /// 出厂默认那个源的地址
    pub builtin_default: Option<String>,
}

/// 生效值的装配：按盘上选中的模式 → 界面那一格。
/// **只写这一处**（get / set / clear 共用）—— 各写一遍迟早有一份忘带新字段
fn source_dto(root: &Path) -> Result<Option<PresetSourceDto>, AppError> {
    let entry = runtime::source::current_entry(root)?;
    let mode = runtime::source::current_mode(root)?;
    let builtin: Vec<BuiltinSourceDto> = runtime::source::builtin_sources()
        .into_iter()
        .map(|(m, address)| BuiltinSourceDto {
            id: mode_wire(m),
            label: m.label().to_owned(),
            address,
        })
        .collect();
    let (address, from_user) = match entry {
        Some(runtime::source::SourceEntry::Custom { url, .. }) => (url, true),
        Some(runtime::source::SourceEntry::Bootstrap { url }) => (url, false),
        // 选了自定义却没地址：地址给空串（界面把输入框摆出来并报错），
        // **不静默回落到内置源** —— 那会让界面显示"自定义"、实际连着别的地方。
        None => (String::new(), true),
    };
    Ok(Some(PresetSourceDto {
        mode: mode_wire(mode),
        label: mode.label().to_owned(),
        address,
        from_user,
        builtin,
        default_mode: mode_wire(runtime::source::DEFAULT_MODE),
        builtin_default: runtime::source::builtin_default(),
    }))
}

/// Rust 枚举 → 线上字符串。**id 是契约**，所以只写这一处。
fn mode_wire(mode: runtime::source::SourceMode) -> String {
    match mode {
        runtime::source::SourceMode::Gitee => "gitee".to_owned(),
        runtime::source::SourceMode::Github => "github".to_owned(),
        runtime::source::SourceMode::Custom => "custom".to_owned(),
    }
}

/// 当前数据源。`null` = 一个都没配（既没有设置文件，也没有出厂默认值）—
/// **装配那一步由用户完成**，程序不替他猜
#[tauri::command]
pub async fn get_preset_source(app: AppHandle) -> Result<Option<PresetSourceDto>, AppError> {
    traced("getPresetSource", |_| {
        let root = internal_root(&app)?;
        source_dto(&root)
    })
}

/// 换数据源：**先探一次，通了才落盘**（作者 2026-10-05：用户容易输错地址）。
///
/// - 内置两个源：地址是程序自带的，**不联网不探**（否则每次切源都要等一次网络）；
/// - 自定义：解析入口 + 取一次 catalog —— **取不到 / 解析不了就整次拒绝**，
///   刚写的那份**撤掉**（不留半份状态；错地址留在设置里比"没配"更难查）。
#[tauri::command]
pub async fn set_preset_source(
    app: AppHandle,
    mode: String,
    custom_url: Option<String>,
) -> Result<PresetSourceDto, AppError> {
    let root = internal_root(&app)?;
    let mode = runtime::source::SourceMode::parse(mode.trim()).ok_or_else(|| {
        AppError::invalid_argument(format!(
            "认不出的源：{mode}（只能是 github / gitee / custom）"
        ))
    })?;
    if mode == runtime::source::SourceMode::Custom {
        let raw = custom_url.unwrap_or_default();
        let task = tauri::async_runtime::spawn_blocking(move || {
            traced("setPresetSource", |_| {
                // ★ 先探形状（根 / source.json 两种都认），**通了才落盘** ——
                //   错地址留在设置里比"没配"更难查（0.0.2 的教训）。
                let (shape, _resolved) = runtime::source::probe_custom_shape(&raw)?;
                let source = runtime::source::make_source(mode, Some(&raw), shape)?;
                runtime::app_state::set_preset_source(&root, source)?;
                source_dto(&root)?.ok_or_else(|| AppError::internal("数据源设置写完读不回来"))
            })
        });
        let dto = task
            .await
            .map_err(|e| AppError::internal("换数据源没跑到终局").with_detail(e.to_string()))?;
        super::notify_app_state(&app);
        return dto;
    }
    traced("setPresetSource", |_| {
        let source =
            runtime::source::make_source(mode, None, runtime::source::CustomShape::Unset)?;
        runtime::app_state::set_preset_source(&root, source)?;
        super::notify_app_state(&app);
        source_dto(&root)?.ok_or_else(|| AppError::internal("数据源设置写完读不回来"))
    })
}

/// 撤掉用户覆盖：删掉设置文件（幂等），返回删除之后生效的值（有内置给内置，没有就是 `null`）。
///
/// 「回到内置默认」只有这一条路 —— 空地址不许写盘（见 `runtime::source::save_source`），
/// 原来的出口是"用户手删文件"；设置页把那件事变成一次显式动作
/// （2026-10-02「同步」页退役时，从"填地址"这一格旁边分出来的）
#[tauri::command]
pub async fn clear_preset_source(app: AppHandle) -> Result<Option<PresetSourceDto>, AppError> {
    traced("clearPresetSource", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::clear_preset_source(&root)?;
        super::notify_app_state(&app);
        source_dto(&root)
    })
}

/*
 * 「远端在哪」的解析（原来那个 `remote_base`）搬进了 `runtime::source`：
 * 第十七刀起入口有两种（手动根 / 内置 Bootstrap），解析要读 `source.json` ——
 * 那是 source 模块的知识，不是这一层的。所有联网动作（下载 / 检查更新 / 应用更新）
 * 都从 `runtime::source::resolve_source(&root)?` 出发，拿 `base_url` 或 `catalog_url`。
 */

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

/// 把 catalog 里登记的一份文件从数据源拉进**它自己的落点**。
///
/// 落点 = `catalog.path`（唯一路径语义）—— 没有固定的"下载区根"，目录按需建。
/// 文件在哪 = [地址](remote_base) + catalog 记的相对位置，地址在 catalog 之外
/// ——换源不用重发说明书。字节对不上 SHA 就整个拒绝——落点上不会出现坏文件。
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

            // 目录给了期望大小就用它当水位总量；没给（随包 bootstrap 目录）就照实
            // 不报百分比 —— `total: None` 在界面上是"不知道还有多少"，不是 0
            let expected = file.expected_size();
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
            let resolved = runtime::source::resolve_source(&root)?;
            let remote = runtime::net::RemoteSource::new(resolved.resolver.clone(), &forward);
            let outcome = runtime::delivery::deliver(&root, file, &remote);

            match &outcome {
                Ok(_) => send_tick(
                    &on_tick,
                    runtime::net::Stage::Done,
                    &file_name,
                    expected.unwrap_or(0),
                    expected,
                    None,
                ),
                Err(e) => send_tick(
                    &on_tick,
                    runtime::net::Stage::Failed,
                    &file_name,
                    0,
                    expected,
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
            let resolved = runtime::source::resolve_source(&root)?;
            let remote = runtime::net::RemoteSource::new(resolved.resolver.clone(), &forward);

            let report = |outcome: &runtime::delivery::FileOutcome| {
                // 期望大小是 `Option`（随包 bootstrap 目录不登记它）：拿不到就报
                // "不知道总量"，不是报 0（`total: None` 界面上是另一句话）
                let size = wanted
                    .iter()
                    .find(|f| f.file_name == outcome.file_name)
                    .and_then(|f| f.expected_size());
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

/// 下载区 / 交付区里的一份（盘当底账），给界面的形状。
///
/// 时间**全部来自事件**（`preset_events` 账 + 版本身份反查），mtime 不再上界面：
///
/// - `downloaded_unix` / `replaced_unix` —— 这份字节是「下载」进来的还是「替换」上去的
///   （两个事件至多一个在；都没有 = 认不出出身的字节，不记账不猜，界面照实「未知」）；
/// - `published_at` —— 这份字节属于哪一代目录、那一代什么时候发布的（反查版本链；
///   链建立之前的版本查不到 = `null`）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnDiskFileDto {
    pub file_name: String,
    pub downloaded_unix: Option<u64>,
    pub replaced_unix: Option<u64>,
    pub published_at: Option<String>,
}

/// 已经下载到下载区的文件（盘就是底账：文件在且 SHA 对得上才算数），带它的事件时间
#[tauri::command]
pub async fn get_downloaded_files(app: AppHandle) -> Result<Vec<OnDiskFileDto>, AppError> {
    traced("getDownloadedFiles", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        Ok(runtime::delivery::downloaded_entries(&root, &catalog)
            .into_iter()
            .map(|e| OnDiskFileDto {
                file_name: e.file_name,
                downloaded_unix: e.downloaded_unix,
                replaced_unix: e.replaced_unix,
                published_at: e.published_at,
            })
            .collect())
    })
}

/// 有更新的文件（盘上在、字节与目录不一样），带它的事件时间。
/// "更新"就是对这些再跑一遍下载——旧份自动归档，没有单独的更新代码路径
#[tauri::command]
pub async fn get_stale_files(app: AppHandle) -> Result<Vec<OnDiskFileDto>, AppError> {
    traced("getStaleFiles", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        Ok(runtime::delivery::stale_entries(&root, &catalog)
            .into_iter()
            .map(|e| OnDiskFileDto {
                file_name: e.file_name,
                downloaded_unix: e.downloaded_unix,
                replaced_unix: e.replaced_unix,
                published_at: e.published_at,
            })
            .collect())
    })
}

/// 盘上这一份**认得出是哪一版吗**（第三圈第 6 层：SHA 报警）。
///
/// `getStaleFiles` 只回答"盘上的字节与目录不一致"，而那可能是三件不同的事 ——
/// 这一条把那三件分开，并且**只列有事的**（没下载 / 与目录一致的两种不出现）：
///
///   `old`        = 认得出它是官方的某一版旧版（归档里有它字节，或被归档的旧目录登记过）
///   `tampered`   = 目录、归档、旧目录都对不上 —— 这台机器上查不出它属于哪一版
///
/// **它与"云端有更新"是两件事**：后者是 [`check_remote_update`]（比目录指纹），
/// 与本机这一份的字节无关。别把两者混成一句「需更新」—— 用户因此既不知道自己的
/// 文件是不是被改过，也不知道该不该等更新。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeliveryTrustDto {
    pub file_name: String,
    pub verdict: String,
    /// `old` 且归档区里有它字节时给（`archive/delivery/mkp/presets/A1-fast.toml`）—— 界面据此
    /// 把那一版旧正文读出来给人对。被旧目录登记、归档里没字节的那种是 `null`
    pub archived_path: Option<String>,
}

#[tauri::command]
pub async fn get_delivery_trust(app: AppHandle) -> Result<Vec<DeliveryTrustDto>, AppError> {
    traced("getDeliveryTrust", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        Ok(runtime::delivery::trust_entries(&root, &catalog)
            .into_iter()
            .map(|e| DeliveryTrustDto {
                file_name: e.file_name,
                verdict: match e.trust {
                    runtime::delivery::FileTrust::OldVersion => "old",
                    // Absent / Current 在 `trust_entries` 里就滤掉了，到不了这里
                    _ => "tampered",
                }
                .to_owned(),
                archived_path: e.archived_path,
            })
            .collect())
    })
}

/// 归档区里的一份旧版本（给界面看的形状）。
///
/// 两个时间**各是各，永不互相顶替**（2026-10-06 预设事件时间模型）：
///
/// - `published_at` = 这一版**在云端发布**的时刻（`ReleasePublished`，反查版本出身）；
/// - `replaced_unix` = 被换下来的时刻（`DeliveryReplaced.at`，事件账）。
///
/// 以前只有一个 mtime 顶在唯一的时间位上 —— 用户看到的就是"我动它的时刻"，
/// 不是"这一版发布的时候"。现在两个都给，查不到的照实 `null`（不拿"现在"顶）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedFileDto {
    /// 相对内部根的路径（`archive/delivery/mkp/presets/A1-fast.toml`）—— 读正文时把它交回来
    pub path: String,
    pub file_name: String,
    pub size: u64,
    /// 这份旧字节的指纹 —— 事件账（`DeliveryReplaced.old_sha256`）与版本出身的对号键
    pub sha256: String,
    /// 这一版在云端发布过的时刻（RFC3339）。**链建立之前的版本查不到 = `null`**
    /// —— 界面照实「未知（早于版本记忆）」
    pub published_at: Option<String>,
    /// 被换下来的时刻（UTC epoch 秒）= `DeliveryReplaced.at`。账前档案建账时已补记；
    /// `null` = 账上没有（建账时读不动它），照实「未知」
    pub replaced_unix: Option<u64>,
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
        /* 事件账读一次（读不到/坏档 = 空表，替换时间照实「未知」） */
        let events = runtime::delivery::load_events(&root, &catalog);
        Ok(runtime::delivery::archived_files(&root)
            .into_iter()
            .map(|a| {
                /* 认人靠"同位"：`archive/delivery/mkp/presets/x.toml` ↔ 目录里的
                `delivery/mkp/presets/x.toml`（同形，新字节）。**不解析文件名**去猜机型
                版本 —— 名字规则将来会变，而"归档这份与目录里哪一份同位"
                是一个不需要额外知识的事实 */
                let rel = a.path.strip_prefix(&prefix);
                let known = rel.and_then(|rel| catalog.files.iter().find(|f| f.path == rel));
                let machine_id = known.map(|f| f.machine_id.clone());
                let version_id = known.map(|f| f.version_id.clone());
                let kind = known.map(|f| f.kind.clone());
                /* 两个时间各查各的，互不顶替：
                 * - 替换时间 = 事件账里（同位路径 + 这份旧字节指纹）那条 `DeliveryReplaced.at`；
                 * - 云端发布 = 版本身份反查（这一版登记在哪代目录、那代何时发布） */
                let replaced_unix = rel
                    .and_then(|rel| runtime::preset_events::replaced_at_of(&events, rel, &a.sha256));
                let published_at = if a.sha256.is_empty() {
                    None
                } else {
                    runtime::delivery::generation_of_sha(&root, &a.sha256)
                        .and_then(|g| g.published_at)
                };
                ArchivedFileDto {
                    path: a.path,
                    file_name: a.file_name,
                    size: a.size,
                    sha256: a.sha256,
                    published_at,
                    replaced_unix,
                    machine_id,
                    version_id,
                    kind,
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

/* ---------- 删除（作者裁决 2026-10-06：一切皆可删） ---------- */

/// **删除本机那份官方交付文件**（作者裁决 2026-10-06：一切皆可删）。
///
/// 删了它回到「未下载」，随时可以从云端重新下载（字节有目录 SHA 锚定，零数据损失）。
/// 删之前把属于这一份的**状态**一并清掉：使用中指针（官方线认文件名）撤下、
/// 没保存的草稿一并丢弃 —— 界面确认框讲清这一步。事件账与归档**不动**（历史事实）。
#[tauri::command]
pub async fn delete_delivery_file(app: AppHandle, file_name: String) -> Result<(), AppError> {
    traced("deleteDeliveryFile", |_| {
        let root = internal_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        let active = runtime::app_state::active_preset(&root)?;
        let draft = runtime::app_state::draft(&root)?;
        let mut state_cleared = false;
        if active.as_ref().is_some_and(|a| {
            a.origin == runtime::state::ActiveOrigin::Official && a.file_name == file_name
        }) {
            runtime::app_state::clear_active_preset(&root)?;
            state_cleared = true;
        }
        if draft
            .as_ref()
            .is_some_and(|d| d.subject() == runtime::state::DraftSubject::official(&file_name))
        {
            runtime::app_state::clear_draft(&root)?;
            state_cleared = true;
        }
        runtime::delivery::delete_downloaded(&root, &catalog, &file_name)?;
        if state_cleared {
            super::notify_app_state(&app);
        }
        Ok(())
    })
}

/// **删除归档区里的一份旧版本**（作者裁决 2026-10-06：允许删，代价讲清 ——
/// 云端只有最新版，这一版删了就找不回）。版本链与事件账**不动**
/// （历史事实，不是这份文件的附属）。
#[tauri::command]
pub async fn delete_archived_file(app: AppHandle, path: String) -> Result<(), AppError> {
    traced("deleteArchivedFile", |_| {
        let root = internal_root(&app)?;
        runtime::delivery::delete_archived(&root, &path)
    })
}

/* ---------- 使用中指针（第一圈 ⑤：用户状态的第一个真数据） ---------- */

/// 给界面的使用中状态：指针 + 认得出的机型/版本 + 文件是否还是当时那份
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivePresetDto {
    /// 这一份住在哪条线上（`official` / `mine`）—— 官方线与用户线**都能成为使用中**
    pub origin: runtime::state::ActiveOrigin,
    pub file_name: String,
    /// 用户线的落点（相对用户根）；官方线是 `null`（落点由目录给）
    pub path: Option<String>,
    pub sha256: String,
    /// 认不出是哪台机型的哪一版时是空串（用户自己那份没有血统、或目录里已经没有来源那份）
    pub machine_id: String,
    pub version_id: String,
    /// 盘上的文件还是不是应用时刻的那份。
    ///
    /// 两条线上它的意思不一样：官方线正常恒 true（`mkp/` 是只读区）；用户线**用户自己
    /// 又改了那份**时是 false —— 那是正常事（那份是他的），不是"这份配置坏了"。
    pub intact: bool,
}

fn active_dto(
    internal_root: &Path,
    user_root: &Path,
    catalog: &runtime::Catalog,
    state: runtime::state::ActivePreset,
) -> ActivePresetDto {
    use runtime::state::ActiveOrigin;
    let (machine_id, version_id) = match state.origin {
        ActiveOrigin::Official => {
            let listed = catalog
                .files
                .iter()
                .find(|f| f.file_name == state.file_name);
            (
                listed.map(|f| f.machine_id.clone()).unwrap_or_default(),
                listed.map(|f| f.version_id.clone()).unwrap_or_default(),
            )
        }
        ActiveOrigin::Mine => {
            /*
             * 用户那份**自己说了**它从哪台机型的哪一版派生（血统三行写在文件头），
             * 这里只把那个来源翻成机型 / 版本。认不出来就留空 —— 不猜一个。
             */
            let lineage = state
                .path
                .as_deref()
                .and_then(|rel| crate::fsx::paths::resolve_in(user_root, rel).ok())
                .and_then(|path| runtime::mine::lineage_of_file(&path));
            let source = runtime::mine::source_of(catalog, lineage.as_ref());
            (
                source.map(|f| f.machine_id.clone()).unwrap_or_default(),
                source.map(|f| f.version_id.clone()).unwrap_or_default(),
            )
        }
    };
    ActivePresetDto {
        origin: state.origin,
        path: state.path.clone(),
        intact: runtime::state::active_matches_disk(internal_root, user_root, catalog, &state),
        file_name: state.file_name,
        sha256: state.sha256,
        machine_id,
        version_id,
    }
}

/// 当前使用的是哪一份。`null` = 还没用任何一份（合法状态，不是错误）
#[tauri::command]
pub async fn get_active_preset(app: AppHandle) -> Result<Option<ActivePresetDto>, AppError> {
    traced("getActivePreset", |_| {
        let root = internal_root(&app)?;
        let user = crate::fsx::paths::user_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;
        match runtime::app_state::active_preset(&root)? {
            None => Ok(None),
            Some(state) => Ok(Some(active_dto(&root, &user, &catalog, state))),
        }
    })
}

/// 「使用这一份」。**两条线共用这一个入口**（作者 2026-10-02 定的第七层）：
/// 官方交付文件与用户自己那份都是真的 Preset，**"只读"是文件归属的属性，
/// 不是"能不能被使用"的属性** —— 所以这里只有"按来源定位 + 各自的可信度判定"，
/// 没有两套 Preset 模型、也没有第二个写指针的口。
///
/// 全局唯一：产品规则定死了同一时刻只能有一份处于已应用状态，构造上就是"一个文件"。
///
/// 两条线各自的**入口闸**（都在盘上真的读一遍，不靠界面拦）：
///
/// ```text
/// official  目录里有这一份 + 盘上字节与目录登记逐字节一致（没下载 / 被改过 / 是旧版本 —— 都不许应用）
/// mine      落点必须在 `presets-mine/` 那一格里 + 盘上真有 + 能读成一份 TOML
///           （第九层的**文件级**检查；**不看 SHA** —— 用户自己改过是正常事，
///           能不能用看"现在还能不能读"，见 `runtime::mine::read_preset_text`）
/// ```
#[tauri::command]
pub async fn apply_active_preset(
    app: AppHandle,
    file_name: String,
    origin: Option<runtime::state::ActiveOrigin>,
    path: Option<String>,
) -> Result<ActivePresetDto, AppError> {
    let dto = traced("applyActivePreset", |_| {
        use runtime::state::ActiveOrigin;
        let root = internal_root(&app)?;
        let user = crate::fsx::paths::user_root(&app)?;
        let catalog = runtime::load_released_catalog(&root)?;

        let state = match origin.unwrap_or_default() {
            ActiveOrigin::Official => {
                let file = catalog
                    .files
                    .iter()
                    .find(|f| f.file_name == file_name)
                    .ok_or_else(|| AppError::not_found(format!("目录里没有 {file_name}")))?;

                // 应用的是盘上那份：字节得真的在；**目录给了期望值**的还要与它对得上
                // （没下载/被删/漂了都不许应用）。随包 bootstrap 目录没登记 SHA ——
                // 那种情况下盘上有就允许应用（字节的权威判定归 OTA 目录生效后那次下载）
                let bytes = std::fs::read(root.join(&file.path)).map_err(|_| {
                    AppError::not_found(format!("{file_name} 还不在本机——先下载，再使用"))
                })?;
                if let Some(want) = file.expected_sha() {
                    let digest = runtime::catalog::hex(&sha2::Sha256::digest(&bytes));
                    if digest != want {
                        return Err(AppError::sha_mismatch(format!(
                            "{file_name} 盘上的内容与目录登记的当前版本对不上，拒绝应用 —— \
                             先「更新」或「重新下载」换一份干净的"
                        )));
                    }
                }
                runtime::app_state::set_active_official(&root, file)?
            }
            ActiveOrigin::Mine => {
                let rel = path.ok_or_else(|| {
                    AppError::invalid_argument(
                        "用自己那份要给出它在用户根里的路径（presets-mine/…）",
                    )
                })?;
                /* 两道闸都在读盘之前：只认用户根那一格，而且得是一份 MKP 预设（.toml） */
                runtime::mine::check_mine_prefix(&rel)?;
                if runtime::mine::kind_of(&file_name) != Some(runtime::catalog::kind::PRESET) {
                    return Err(AppError::invalid_argument(format!(
                        "{file_name} 不是一份 MKP 预设（TOML）—— 用户文件里只有预设能被使用"
                    )));
                }
                /*
                 * 第九层的文件级检查在读的那一步里（能读 + UTF-8 + TOML 语法）——
                 * 外部改过但仍是能读的 TOML 照常能用，**这里不比 SHA**（那是官方线的规矩）。
                 */
                let text = runtime::mine::read_preset_text(&user, &rel)?;
                let digest = runtime::catalog::hex(&sha2::Sha256::digest(text.as_bytes()));
                runtime::app_state::set_active_mine(&root, &rel, &digest)?
            }
        };

        Ok(active_dto(&root, &user, &catalog, state))
    })?;
    /* AppState 的写命令成功 → 广播（docs/APP-STATE.md §3.5），订阅者据此刷新 */
    super::notify_app_state(&app);
    Ok(dto)
}

/// 撤销使用。幂等：本来就没在用也不报错
#[tauri::command]
pub async fn clear_active_preset(app: AppHandle) -> Result<(), AppError> {
    traced("clearActivePreset", |_| {
        let root = internal_root(&app)?;
        runtime::app_state::clear_active_preset(&root)?;
        super::notify_app_state(&app);
        Ok(())
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
    /// 本构建读不读得懂这一代远端目录（能力优先、版本兜底，见 `runtime::structure::can_read`）。
    /// ★ 与 `upToDate` 回答的是两件不同的事（"有没有更新" vs "读不读得懂"）。
    pub readable: bool,
}

/// 对远端目录检查更新：**清单从数据源来**（`<base>/catalog.json`），与文件本体同一个
/// 地址概念——不会出现"清单在这个源、文件在另一个源"的错位。比较逻辑见 [`runtime::update`]。
#[tauri::command]
pub async fn check_remote_update(app: AppHandle) -> Result<RemoteUpdateDto, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("checkRemoteUpdate", |_| {
            let root = internal_root(&app)?;
            let resolved = runtime::source::resolve_source(&root)?;
            let bytes = runtime::net::get_catalog(&resolved.catalog_url()?)?;
            let remote = runtime::Catalog::parse(&bytes)?;
            let local = runtime::load_released_catalog(&root)?;
            let r = runtime::update::check(&local, &remote);
            Ok(RemoteUpdateDto {
                up_to_date: r.up_to_date,
                local_revision: r.local_revision,
                remote_revision: r.remote_revision,
                readable: r.readable,
            })
        })
    });
    task.await
        .map_err(|e| AppError::internal("检查更新没跑到终局").with_detail(e.to_string()))?
}

/// 应用远端目录：旧目录归档（release 管道）、新目录生效。之后 Stale 文件照常出现在
/// 「有更新」里，用既有的下载管道拉新——更新没有第三条路径。
///
/// ★ **读不懂就不采用**：远端这一代结构本构建读不了时（[`runtime::structure::can_read`] 为假），
/// 这里返回 [`AppError::not_supported`]，**在 `release_bytes` 之前就退出** —— 不落盘、不归档、
/// 本机目录零改动。"不下载不使用"的落点就在这里（作者定的产品规则 B/C）。
#[tauri::command]
pub async fn apply_remote_update(app: AppHandle) -> Result<String, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("applyRemoteUpdate", |_| {
            let root = internal_root(&app)?;
            let resolved = runtime::source::resolve_source(&root)?;
            let bytes = runtime::net::get_catalog(&resolved.catalog_url()?)?;
            let remote = runtime::Catalog::parse(&bytes)?;

            /* 先判定再落盘：读不懂就整次拒绝，盘上那份目录一个字节都不动。
            message 只写用户能懂的话 —— 结构签名 / 最低版本这些词不许出现在这里（禁区） */
            if !runtime::structure::can_read(
                &remote.structure_signature,
                remote.min_client_version.as_deref(),
            ) {
                return Err(
                    AppError::not_supported("此预设需要更新版 SupportEase").with_detail(format!(
                        "远端目录结构签名 {} 本构建读不了（minClient={:?}）",
                        remote.structure_signature, remote.min_client_version
                    )),
                );
            }

            let report = runtime::release::release_bytes(&root, &bytes)?;
            Ok(report.summary())
        })
    });
    task.await
        .map_err(|e| AppError::internal("应用远端目录没跑到终局").with_detail(e.to_string()))?
}

/* ---------- 软件更新（`release.json`）—— 与预设数据**两条链** ---------- */

/// 当前客户端版本号（构建期 `CARGO_PKG_VERSION`，唯一真值）。
///
/// 设置页「软件更新」块要显示"当前版本 → 最新版本"，这是它拿"当前版本"的唯口子 ——
/// 版本号**不许在前端再写一遍**（那是第二处真值）。
#[tauri::command]
pub async fn get_app_version() -> Result<String, AppError> {
    Ok(runtime::structure::APP_VERSION.to_owned())
}

/// 检查**软件**更新（`release.json`）—— 与预设数据链完全分开的一条。
///
/// - 只读、`async`、**不在启动 / 首屏路径**（设置页打开时才调；铁律 2：云端不参与首屏）；
/// - 信息源地址 = 数据源解析出的两个地址的**上一级**（`release.json` 住发布根
///   `presets/` 之外，见 `runtime::source::RELEASE_FILE`）；
/// - 远端没有这一份 / 没联网时**如实拒绝**（`NOT_FOUND` / `IO`）—— 由调用方决定说还是略过，
///   这一层不编一份"已是最新"糊过去。
#[tauri::command]
pub async fn check_software_update(app: AppHandle) -> Result<SoftwareUpdateDto, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        traced("checkSoftwareUpdate", |_| {
            let root = internal_root(&app)?;
            let resolved = runtime::source::resolve_source(&root)?;

            /* release.json 住发布根（presets/）之外：从 base_url（文件下载根 = presets/delivery）
            上去两级就是仓库根发布位置。手写死三级最容易在换部署时错位，
            所以用 source 的目录回退规矩，从 base_url 推它自己的"上一级" */
            /* release.json 的地址由 Source Manifest 声明（不再从 base"上跳一级"去猜） */
            let release_url = resolved.release_url()?;
            let bytes = runtime::net::get_release(&release_url)?;
            let info = runtime::release_info::parse(&bytes)?;
            Ok(SoftwareUpdateDto::from(runtime::release_info::to_update(
                &info,
            )))
        })
    });
    task.await
        .map_err(|e| AppError::internal("检查软件更新没跑到终局").with_detail(e.to_string()))?
}

/// 给界面的软件更新状态（与 `runtime::release_info::SoftwareUpdate` 同形，camelCase）。
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoftwareUpdateDto {
    pub has_update: bool,
    pub current_version: String,
    pub latest_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

impl From<runtime::release_info::SoftwareUpdate> for SoftwareUpdateDto {
    fn from(u: runtime::release_info::SoftwareUpdate) -> Self {
        Self {
            has_update: u.has_update,
            current_version: u.current_version,
            latest_version: u.latest_version,
            notes: u.notes,
            url: u.url,
        }
    }
}
