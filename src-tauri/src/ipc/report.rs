//! 报告页的读口 —— **后处理执行报告与历史**。
//!
//! # 账在哪
//!
//! 后处理由本程序自己跑（切片器导出时带 `--Toml/--Gcode` 调的那一次，见
//! [`crate::hook`]）。每跑一次落三件套（形状与成熟版一致，写的那一侧是
//! [`crate::archive`]）：
//!
//! ```text
//! <用户根>/gcode_history/<YYYY-MM-DD>/<名>_<时刻>_meta.json   ← 执行报告（本模块读的）
//! <用户根>/gcode_history/<YYYY-MM-DD>/<名>_<时刻>_original.gcode
//! <用户根>/gcode_history/<YYYY-MM-DD>/<名>_<时刻>.gcode
//! ```
//!
//! **本模块只读** —— 账的主人是钩子那条路。2026-10-09 起不再读旧世代（`mkp-ssr`）
//! 那棵树：换标识就是全新应用，旧目录用户自己删（旧世代的记录因此不再出现在这一页）。
//!
//! # 失败与空长得不一样
//!
//! - **没有记录**（`gcode_history` 还是空的）→ 空列表，界面说"还没有执行记录"；
//! - **读不出来**（某份 `_meta.json` 坏了）→ 这一条**照进列表**，`ok = false` +
//!   `error` 里写清"这份记录读不出来"——藏起它就等于替钩子瞒了一次事故；
//! - **跑失败的记录**（钩子写的 `error`）→ 同样进列表，`ok = false`。
//!
//! # 字段口径
//!
//! `_meta.json` 是 [`crate::archive::Meta`] 写的 schema v2（键名与旧世代同口径）：
//! 起止时间、耗时、预设 / 机型、`invocation`（exe + argv）、输入输出指纹、`warnings`、
//! `error`、`detail`（文件事实 / 统计 / 打印时间）与 `pipeline`（逐阶段计时）。
//! 本模块按**界面要显示的**截取 —— 没显示的字段不搬，将来要展示再加，不预造大 DTO。

use serde::Serialize;
use tauri::AppHandle;

use crate::error::AppError;

use super::traced;

/// 一条执行记录的摘要（列表行）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportSummary {
    /// 记录 id = `_meta.json` 的文件名去掉后缀（`25080.0_20260913_001909`），全树唯一
    pub id: String,
    /// 哪一天的账（日期目录名）
    pub day: String,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
    pub elapsed_ms: Option<u64>,
    pub preset_name: Option<String>,
    pub machine_type: Option<String>,
    /// 输入 G-code 的文件名（`gcodePath` 的最后一段）
    pub gcode_name: Option<String>,
    /// **这条记录读没读得懂、钩子那一次跑成没跑成**。`false` 有两种：跑失败（有 `error`）
    /// 与记录本身坏了（解析不动）—— 都进列表，不许藏
    pub ok: bool,
    pub error: Option<String>,
    pub warning_count: usize,
}

/// 一条执行记录的详情（展开那一屏）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportDetail {
    pub summary: ReportSummary,
    /// 输入 / 输出两份的事实（大小、行数、SHA）—— `detail.inputFile` / `outputFile`
    pub input_file: Option<serde_json::Value>,
    pub output_file: Option<serde_json::Value>,
    /// 刮出来的切片器头部事实（机型 / 工艺预设名、熨平开关、3mf 名）
    pub header: Option<serde_json::Value>,
    /// 16 项统计（`detail.stats`）
    pub stats: Option<serde_json::Value>,
    /// 打印时间估算（`detail.printTime`，含 byType 分解）
    pub print_time: Option<serde_json::Value>,
    /// 打印时间估算的状态（`computing` / `ready` / `failed`）—— 延后估算时才有，
    /// 前端据此显示"正在估算…"/"估算失败"，而不是拿 0 凑数。
    pub print_time_status: Option<String>,
    /// 管线各步计时（`pipeline`：id / elapsedMs / message / status）
    pub pipeline: Vec<serde_json::Value>,
    pub warnings: Vec<String>,
    /// 调用现场（exe + argv）—— 复制脚本 / 排查对不上时看它
    pub invocation: Option<serde_json::Value>,
    pub preset_path: Option<String>,
    pub gcode_path: Option<String>,
}

/// 扫一棵 `gcode_history` 树，收集全部记录（纯函数，判据与命令共用）。
fn collect(dir: &std::path::Path) -> Vec<ReportSummary> {
    let mut out = Vec::new();
    let Ok(days) = std::fs::read_dir(dir) else {
        return out; // 树还没有 = 一条记录都没有（不是错误）
    };
    for day in days.flatten() {
        if !day.path().is_dir() {
            continue;
        }
        let day_name = day.file_name().to_string_lossy().into_owned();
        let Ok(entries) = std::fs::read_dir(day.path()) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let Some(id) = name.strip_suffix("_meta.json") else {
                continue;
            };
            out.push(summarize(&path, id, &day_name));
        }
    }
    // 时间倒序（新的在前）。`startedAt` 是 `YYYY-MM-DD HH:MM:SS`，字典序即时间序；
    // 读不出来的那些按 id（自带时刻）垫底对齐，顺序仍然稳定
    out.sort_by(|a, b| {
        b.started_at
            .cmp(&a.started_at)
            .then_with(|| b.id.cmp(&a.id))
    });
    out
}

/// 一份 `_meta.json` → 摘要。**解析失败也要出一条**（`ok=false` + 原因），不许消失。
fn summarize(path: &std::path::Path, id: &str, day: &str) -> ReportSummary {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => {
            return broken_summary(id, day, format!("这份记录读不出来：{e}"));
        }
    };
    let meta: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return broken_summary(id, day, format!("这份记录不是合法的 JSON：{e}"));
        }
    };
    let str_at = |key: &str| {
        meta.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let error = str_at("error");
    let warnings = meta
        .get("warnings")
        .and_then(|v| v.as_array())
        .map(Vec::len)
        .unwrap_or(0);
    let gcode_name = str_at("gcodePath").map(|p| {
        std::path::Path::new(&p)
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or(p)
    });
    ReportSummary {
        id: id.to_owned(),
        day: day.to_owned(),
        started_at: str_at("startedAt"),
        finished_at: str_at("finishedAt"),
        elapsed_ms: meta.get("elapsedMs").and_then(|v| v.as_u64()),
        preset_name: str_at("presetName"),
        machine_type: str_at("machineType"),
        gcode_name,
        ok: error.is_none(),
        error,
        warning_count: warnings,
    }
}

/// 记录文件本身坏了的那一条：**照进列表**，把原因写在 `error` 里
fn broken_summary(id: &str, day: &str, why: String) -> ReportSummary {
    ReportSummary {
        id: id.to_owned(),
        day: day.to_owned(),
        started_at: None,
        finished_at: None,
        elapsed_ms: None,
        preset_name: None,
        machine_type: None,
        gcode_name: None,
        ok: false,
        error: Some(why),
        warning_count: 0,
    }
}

/// 归档树的根（`<用户根>/gcode_history`）—— 与钩子写的那棵树是同一处：
/// 两边都从 [`crate::fsx::paths`] 认同一个子目录名，字面量只住那里一处。
///
/// `user_root` 会把缺失的子目录建齐（首启动就建），所以"还没有记录"是
/// **空列表**而不是错误 —— 三态里的第一种（见模块头）。
fn history_root(app: &AppHandle) -> Result<std::path::PathBuf, AppError> {
    Ok(crate::fsx::paths::user_root(app)?.join(crate::fsx::paths::GCODE_HISTORY_DIR))
}

/// 全部执行记录（新在前）。
#[tauri::command]
pub fn get_report_list(app: AppHandle) -> Result<Vec<ReportSummary>, AppError> {
    traced("getReportList", |_| Ok(collect(&history_root(&app)?)))
}

/// 一条记录的详情。`id` 是 [`ReportSummary::id`] 那个值。
#[tauri::command]
pub fn get_report_detail(app: AppHandle, id: String) -> Result<ReportDetail, AppError> {
    traced("getReportDetail", |_| {
        let dir = history_root(&app)?;
        let (path, day) = find_meta(&dir, &id)
            .ok_or_else(|| AppError::not_found(format!("没有这条执行记录：{id}")))?;
        let text = std::fs::read_to_string(&path)
            .map_err(|e| AppError::io("这份记录读不出来").with_detail(e.to_string()))?;
        let meta: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            AppError::corrupted("这份记录不是合法的 JSON").with_detail(e.to_string())
        })?;
        let summary = summarize(&path, &id, &day);
        Ok(detail_of(summary, &meta))
    })
}

/// 在整棵树里找 `<id>_meta.json`（id 全树唯一，文件名即 id）
fn find_meta(dir: &std::path::Path, id: &str) -> Option<(std::path::PathBuf, String)> {
    let days = std::fs::read_dir(dir).ok()?;
    for day in days.flatten() {
        if !day.path().is_dir() {
            continue;
        }
        let day_name = day.file_name().to_string_lossy().into_owned();
        let candidate = day.path().join(format!("{id}_meta.json"));
        if candidate.is_file() {
            return Some((candidate, day_name));
        }
    }
    None
}

fn detail_of(summary: ReportSummary, meta: &serde_json::Value) -> ReportDetail {
    let detail = meta.get("detail");
    let str_at = |key: &str| {
        meta.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    ReportDetail {
        summary,
        input_file: detail.and_then(|d| d.get("inputFile")).cloned(),
        output_file: detail.and_then(|d| d.get("outputFile")).cloned(),
        header: detail.and_then(|d| d.get("header")).cloned(),
        stats: detail.and_then(|d| d.get("stats")).cloned(),
        print_time: detail.and_then(|d| d.get("printTime")).cloned(),
        print_time_status: meta
            .get("printTimeStatus")
            .and_then(|v| v.as_str())
            .map(|s| s.to_owned()),
        pipeline: meta
            .get("pipeline")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default(),
        warnings: meta
            .get("warnings")
            .and_then(|v| v.as_array())
            .map(|a| {
                a.iter()
                    .filter_map(|w| w.as_str().map(|s| s.to_owned()))
                    .collect()
            })
            .unwrap_or_default(),
        invocation: meta.get("invocation").cloned(),
        preset_path: str_at("presetPath"),
        gcode_path: str_at("gcodePath"),
    }
}

#[cfg(test)]
mod tests {
    // 判据要造真实归档文件 —— 生产代码的写盘走 `fsx::atomic` 那一个洞。
    #![allow(clippy::disallowed_methods)]

    use super::*;

    /// 一条真形状的 `_meta.json` → 摘要按真字段取值（不是演示数据）
    #[test]
    fn a_real_meta_summarizes_from_its_own_fields() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("2026-09-13");
        std::fs::create_dir_all(&day).unwrap();
        let meta = r#"{
            "schemaVersion": 2,
            "startedAt": "2026-09-13 00:19:09",
            "finishedAt": "2026-09-13 00:19:12",
            "elapsedMs": 2719,
            "presetName": "A1_MINI-fastv3.3.toml",
            "machineType": "A1_MINI",
            "gcodePath": "C:/tmp/.25080.0.gcode",
            "warnings": ["一条警告"],
            "error": null
        }"#;
        std::fs::write(day.join("25080.0_20260913_001909_meta.json"), meta).unwrap();

        let rows = collect(dir.path());
        assert_eq!(rows.len(), 1);
        let r = &rows[0];
        assert_eq!(r.id, "25080.0_20260913_001909");
        assert_eq!(r.day, "2026-09-13");
        assert_eq!(r.preset_name.as_deref(), Some("A1_MINI-fastv3.3.toml"));
        assert_eq!(r.machine_type.as_deref(), Some("A1_MINI"));
        assert_eq!(r.gcode_name.as_deref(), Some(".25080.0.gcode"));
        assert_eq!(r.elapsed_ms, Some(2719));
        assert!(r.ok);
        assert_eq!(r.warning_count, 1);
    }

    /// 钩子跑失败的记录（`error` 有值）：进列表、`ok=false`，原因原样带出来
    #[test]
    fn a_failed_run_still_appears_with_its_error() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("2026-09-14");
        std::fs::create_dir_all(&day).unwrap();
        let meta = r#"{"schemaVersion":2,"startedAt":"2026-09-14 10:00:00","error":"处理失败 [E_GCODE_BOUNDARY_001]：模型超出打印边界"}"#;
        std::fs::write(day.join("bad_20260914_100000_meta.json"), meta).unwrap();

        let rows = collect(dir.path());
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].ok);
        assert!(rows[0]
            .error
            .as_deref()
            .unwrap()
            .contains("E_GCODE_BOUNDARY_001"));
    }

    /// 记录文件坏了：**照进列表**，写明读不出来 —— 不许悄悄消失
    #[test]
    fn a_broken_meta_still_appears() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("2026-09-15");
        std::fs::create_dir_all(&day).unwrap();
        std::fs::write(day.join("broken_20260915_000000_meta.json"), "不是 JSON").unwrap();

        let rows = collect(dir.path());
        assert_eq!(rows.len(), 1);
        assert!(!rows[0].ok);
        assert!(rows[0]
            .error
            .as_deref()
            .unwrap()
            .contains("不是合法的 JSON"));
    }

    /// 没有树 = 一条都没有（空列表，不是错误）
    #[test]
    fn no_tree_means_no_rows() {
        let dir = tempfile::tempdir().unwrap();
        assert!(collect(&dir.path().join("nothing-here")).is_empty());
    }
}
