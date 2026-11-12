//! **交互记录 + 导出日志**（2026-11-12）。
//!
//! # 为什么值得单独一模块
//!
//! 用户遇到问题时能给的就两样：截图里那句错误（带 `traceId`）与一包日志。这一模块把
//! 第二样做出来：把 `logs/` 下三条流（应用 / 交互 / 后处理）脱敏后打成一个 zip，
//! 再附一份 `summary.txt`（版本、构建指纹、系统、数据根、语言与主题）。
//!
//! # 三条口径
//!
//! 1. **只打包日志**：预设文件、3MF、G-code、`app-state.json` 原文一概不进包。
//!    排查要的是"发生了什么"，不是用户的数据。
//! 2. **先脱敏再落包**：家目录 → `~`、IP → `x.x.x.x`（[`crate::obs::redact`]）。
//!    预设名与机型名**保留** —— 抹掉它们等于把要查的东西抹掉。
//! 3. **没有日志就如实报错**：`E_LOG_EXPORT_002`，不产一个空包糊弄人。
//!
//! # 交互记录走 `ui` 目标
//!
//! `tracing::info!(target: "ui", …)` 只被 `ui.log` 那一层收（见 `obs::tracing`），
//! 不会污染按 `warn` 级别的应用日志。一条事件 = 一行，字段固定，翻日志时可以直接 grep。

use std::io::Write;
use std::path::{Path, PathBuf};

use postprocess::diag::messages;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::error::AppError;
use crate::obs::{redact, rotation};

/// 打包失败 / 目标写不进 / 没有可导出日志的稳定码。
const CODE_PACK: &str = "E_LOG_EXPORT_001";
const CODE_EMPTY: &str = "E_LOG_EXPORT_002";
const CODE_DEST: &str = "E_LOG_EXPORT_003";

/// 一条界面动作（前端 `track()` 攒够一批发过来）。
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiEvent {
    pub event: String,
    #[serde(default)]
    pub page: Option<String>,
    #[serde(default)]
    pub detail: Option<String>,
}

/// 导出清单里的一份。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogFileInfo {
    pub name: String,
    /// `app` / `ui` / `hook`
    pub stream: String,
    /// `YYYY-MM-DD`
    pub day: String,
    pub bytes: u64,
}

/// 打开导出框时读一次：包里会有什么。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogPlan {
    pub files: Vec<LogFileInfo>,
    pub bytes: u64,
    /// 有日志覆盖的日期范围（`from` / `to`），没有日志时两个都是 `null`
    pub from: Option<String>,
    pub to: Option<String>,
}

/// 打完之后回给界面的那笔账。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogExport {
    /// zip 的绝对路径（界面上那颗「打开位置」拿它）
    pub path: String,
    pub files: usize,
    pub bytes: u64,
}

/// 截断（前端已经截过一次，这里是二道闸：命令是外部输入，不能信）。
fn cut(text: &str, max: usize) -> String {
    let mut out: String = text.chars().take(max).collect();
    if text.chars().count() > max {
        out.push('…');
    }
    out
}

/// **记一批界面动作**。
///
/// 单次调用上限 256 条：前端一批最多 64，给到四倍是防"界面卡了一会儿攒了很多"。
/// 再多就截掉 —— 日志不是无底洞，宁可少记也不让一条命令吃掉几 MB 内存。
#[tauri::command]
pub fn log_ui_events(app: AppHandle, events: Vec<UiEvent>) -> Result<(), AppError> {
    for ev in events.iter().take(256) {
        tracing::info!(
            target: "ui",
            page = ev.page.as_deref().unwrap_or("-"),
            detail = ev.detail.as_deref().unwrap_or(""),
            "{}",
            cut(&ev.event, 48)
        );
    }
    /* 长会话不靠重启收敛：24 小时节流的那一趟顺路带上（见 `obs::rotation`） */
    rotation::maybe_prune_async(&app);
    Ok(())
}

/// 日志目录（`<内部根>/logs`）。
fn log_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(crate::fsx::paths::internal_root(app)?.join("logs"))
}

/// 清单：包里会有什么。**不打包、不写盘** —— 打开框时读它。
#[tauri::command]
pub fn log_export_plan(app: AppHandle) -> Result<LogPlan, AppError> {
    crate::ipc::traced("logExportPlan", |_| {
        let files = rotation::list(&log_dir(&app)?);
        Ok(LogPlan {
            bytes: files.iter().map(|f| f.bytes).sum(),
            from: files.first().map(|f| f.day.clone()),
            to: files.last().map(|f| f.day.clone()),
            files: files
                .into_iter()
                .map(|f| LogFileInfo {
                    name: f.name,
                    stream: f.stream.to_owned(),
                    day: f.day,
                    bytes: f.bytes,
                })
                .collect(),
        })
    })
}

/// 默认落点：桌面（与备份同一个算法）。
#[tauri::command]
pub fn default_log_dir(app: AppHandle) -> Result<String, AppError> {
    crate::ipc::traced("defaultLogDir", |_| {
        Ok(super::backup::default_export_dir(&app)?
            .display()
            .to_string())
    })
}

/// 导出时刻，按中国时间印（与 `build_fingerprint` 同一个口径，人读得懂）。
fn summary_time() -> String {
    let off = time::UtcOffset::from_hms(8, 0, 0).unwrap_or(time::UtcOffset::UTC);
    let fmt =
        time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second] +08:00");
    time::OffsetDateTime::now_utc()
        .to_offset(off)
        .format(&fmt)
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// 文件名里的时刻（`20261112-201103`）。用 UTC，与日志文件名同一个日期源。
fn file_stamp() -> String {
    let fmt = time::macros::format_description!("[year][month][day]-[hour][minute][second]");
    time::OffsetDateTime::now_utc()
        .format(&fmt)
        .unwrap_or_else(|_| "unknown".to_owned())
}

/// `summary.txt` 的正文。**只写事实**：读不到的格子写"未知"，不猜。
fn summary_text(app: &AppHandle, files: usize, from: Option<&str>, to: Option<&str>) -> String {
    let home = app.path().home_dir().ok().map(|p| p.display().to_string());
    let scrub = |s: String| redact::scrub(&s, home.as_deref());

    let root = crate::fsx::paths::internal_root(app).ok();
    let user = crate::fsx::paths::user_root(app).ok();
    let theme = root
        .as_ref()
        .and_then(|r| crate::runtime::app_state::theme(r).ok().flatten())
        .map(|t| format!("{t:?}"))
        .unwrap_or_else(|| "未知".to_owned());
    let locale = root
        .as_ref()
        .and_then(|r| crate::runtime::app_state::locale(r).ok().flatten())
        .map(|l| format!("{l:?}"))
        .unwrap_or_else(|| "未知".to_owned());

    let mut out = String::new();
    out.push_str("SupportEase 日志导出\n");
    out.push_str(&format!("导出时刻：{}\n", summary_time()));
    out.push_str(&format!(
        "版本：{}\n",
        crate::runtime::structure::APP_VERSION
    ));
    out.push_str(&format!("构建指纹：{}\n", crate::build_fingerprint()));
    out.push_str(&format!(
        "系统：{} {} {}\n",
        std::env::consts::OS,
        std::env::consts::ARCH,
        std::env::consts::FAMILY
    ));
    out.push_str(&format!(
        "数据根：{}\n",
        root.map(|p| scrub(p.display().to_string()))
            .unwrap_or_else(|| "未知".to_owned())
    ));
    out.push_str(&format!(
        "用户根：{}\n",
        user.map(|p| scrub(p.display().to_string()))
            .unwrap_or_else(|| "未知".to_owned())
    ));
    out.push_str(&format!("界面语言：{locale}\n主题：{theme}\n"));
    out.push_str(&format!(
        "日志：{files} 份，{} .. {}\n",
        from.unwrap_or("未知"),
        to.unwrap_or("未知")
    ));
    out.push_str("脱敏：家目录替换为 ~，IP 地址替换为 x.x.x.x\n");
    out.push_str("不含：预设文件、模型与 G-code、设置文件原文\n");
    out
}

fn io_err(e: std::io::Error) -> AppError {
    AppError::io(messages::text("ui.ipc.log.001", &[]))
        .with_detail(e.to_string())
        .with_message_code(CODE_PACK)
}

fn zip_err(e: zip::result::ZipError) -> AppError {
    AppError::io(messages::text("ui.ipc.log.001", &[]))
        .with_detail(e.to_string())
        .with_message_code(CODE_PACK)
}

/// 真正打包的那一段（纯函数，判据直接调它 —— 不必造一个 `AppHandle`）。
///
/// 写盘走全仓唯一出口 `fsx::atomic` 的流式那一档；每条日志**逐行脱敏**，
/// 摘要原样写入（它自己就是按脱敏后的路径拼的）。
fn pack_zip(
    zip_path: &Path,
    logs: &Path,
    files: &[rotation::LogFile],
    summary: &str,
    home: Option<&str>,
) -> Result<(), AppError> {
    crate::fsx::atomic::atomic_write_stream(zip_path, |file| {
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        zip.start_file("summary.txt", opts).map_err(zip_err)?;
        zip.write_all(summary.as_bytes()).map_err(io_err)?;

        for entry in files {
            let raw = std::fs::read(logs.join(&entry.name)).map_err(io_err)?;
            let text = String::from_utf8_lossy(&raw);
            let mut body = String::with_capacity(raw.len());
            for line in text.lines() {
                body.push_str(&redact::scrub(line, home));
                body.push('\n');
            }
            zip.start_file(format!("logs/{}", entry.name), opts)
                .map_err(zip_err)?;
            zip.write_all(body.as_bytes()).map_err(io_err)?;
        }
        zip.finish().map_err(zip_err)?;
        Ok(())
    })
}

/// **打一份日志包**，落在 `dest_dir` 里。
///
/// 逐行脱敏后写进 zip：日志是纯文本，20 MiB 级的东西流式处理，不整包进内存。
#[tauri::command]
pub async fn export_logs(app: AppHandle, dest_dir: String) -> Result<LogExport, AppError> {
    let task = tauri::async_runtime::spawn_blocking(move || {
        crate::ipc::traced("exportLogs", |_| {
            let dir = PathBuf::from(dest_dir.trim());
            let shown = dir.display().to_string();
            if dest_dir.trim().is_empty() {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.ipc.log.003",
                    &[("v1", &shown)],
                ))
                .with_message_code(CODE_DEST));
            }
            if !dir.is_dir() {
                return Err(AppError::not_found(messages::text(
                    "ui.ipc.log.003",
                    &[("v1", &shown)],
                ))
                .with_message_code(CODE_DEST));
            }

            let logs = log_dir(&app)?;
            let files = rotation::list(&logs);
            if files.is_empty() {
                return Err(AppError::not_found(messages::text("ui.ipc.log.002", &[]))
                    .with_message_code(CODE_EMPTY));
            }

            let home = app.path().home_dir().ok().map(|p| p.display().to_string());
            let summary = summary_text(
                &app,
                files.len(),
                files.first().map(|f| f.day.as_str()),
                files.last().map(|f| f.day.as_str()),
            );
            let zip_path = dir.join(format!("MKP-Logs-{}.zip", file_stamp()));

            pack_zip(&zip_path, &logs, &files, &summary, home.as_deref())?;

            let bytes = std::fs::metadata(&zip_path).map(|m| m.len()).unwrap_or(0);
            tracing::info!(path = %zip_path.display(), files = files.len(), bytes, "日志包打好了");
            Ok(LogExport {
                path: zip_path.display().to_string(),
                files: files.len(),
                bytes,
            })
        })
    });
    task.await.map_err(|e| {
        AppError::internal(messages::text("ui.ipc.log.001", &[]))
            .with_detail(e.to_string())
            .with_message_code(CODE_PACK)
    })?
}

/// 在文件管理器里选中那份 zip（导出框那颗「打开位置」）。
///
/// 闸门与备份那条一致：**绝对路径 + `.zip` + 盘上真有**，别的一律拒。
#[tauri::command]
pub async fn reveal_log_export(app: AppHandle, path: String) -> Result<(), AppError> {
    crate::ipc::traced("revealLogExport", |_| {
        let at = PathBuf::from(path.trim());
        let zipped = at
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("zip"));
        if !at.is_absolute() || !zipped {
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.log.004",
                &[],
            )));
        }
        if !at.is_file() {
            return Err(AppError::not_found(messages::text(
                "ui.ipc.log.005",
                &[("v1", &at.display().to_string())],
            )));
        }
        let opener = tauri_plugin_opener::OpenerExt::opener(&app);
        opener.reveal_item_in_dir(&at).map_err(|e| {
            AppError::io(messages::text("ui.ipc.log.006", &[])).with_detail(e.to_string())
        })
    })
}

/// 判据用：清单只列认得出的三种名字，且按日期升序。
#[cfg(test)]
mod tests {
    #![allow(clippy::disallowed_methods)]

    use super::*;

    fn touch(dir: &Path, name: &str, body: &str) {
        std::fs::write(dir.join(name), body).unwrap();
    }

    #[test]
    fn the_plan_lists_only_known_streams_in_date_order() {
        let d = tempfile::tempdir().unwrap();
        touch(d.path(), "ui.log.2026-11-02", "b\n");
        touch(d.path(), "supportease.log.2026-11-01", "a\n");
        touch(d.path(), "notes.txt", "x");
        touch(d.path(), "hook.log.not-a-date", "x");

        let files = rotation::list(d.path());
        let names: Vec<&str> = files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(
            names,
            vec!["supportease.log.2026-11-01", "ui.log.2026-11-02"]
        );
        assert_eq!(files[0].stream, "app");
        assert_eq!(files[1].stream, "ui");
    }

    #[test]
    fn long_fields_are_cut_not_rejected() {
        assert_eq!(cut("abcdef", 3), "abc…");
        assert_eq!(cut("abc", 3), "abc");
    }

    /// 包里就两份：`summary.txt` + `logs/<原名>`；日志逐行脱敏，摘要原样带过去。
    #[test]
    fn the_zip_holds_the_summary_and_redacted_logs() {
        let d = tempfile::tempdir().unwrap();
        let logs = d.path().join("logs");
        std::fs::create_dir_all(&logs).unwrap();
        touch(
            &logs,
            "ui.log.2026-11-12",
            "click /Users/wzy/a.toml\npeer 127.0.0.1:8787\n",
        );
        let files = rotation::list(&logs);
        assert_eq!(files.len(), 1);

        let zip_path = d.path().join("MKP-Logs.zip");
        pack_zip(
            &zip_path,
            &logs,
            &files,
            "版本：0.0.11\n",
            Some("/Users/wzy"),
        )
        .unwrap();

        let mut zip = zip::ZipArchive::new(std::fs::File::open(&zip_path).unwrap()).unwrap();
        let names: Vec<String> = (0..zip.len())
            .map(|i| zip.by_index(i).unwrap().name().to_owned())
            .collect();
        assert_eq!(names, vec!["summary.txt", "logs/ui.log.2026-11-12"]);

        let mut body = String::new();
        std::io::Read::read_to_string(
            &mut zip.by_name("logs/ui.log.2026-11-12").unwrap(),
            &mut body,
        )
        .unwrap();
        assert!(body.contains("click ~/a.toml"), "家目录该变成 ~：{body}");
        assert!(body.contains("peer x.x.x.x:8787"), "IP 该打码：{body}");
        assert!(!body.contains("/Users/wzy"));
    }
}
