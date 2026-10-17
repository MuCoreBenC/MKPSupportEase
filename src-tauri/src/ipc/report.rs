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
//! **账的主人是钩子那条路** —— 本模块对归档树只读，唯一的写口是「导出」
//! （见下），它只往**用户当场点名的那个目录**里写新的副本。2026-10-09 起不再读旧世代
//! （`mkp-ssr`）那棵树：换标识就是全新应用，旧目录用户自己删（旧世代的记录因此
//! 不再出现在这一页）。
//!
//! # 导出（本模块唯一的写口，2026-10-11）
//!
//! 「导出执行报告」把这一条的三件套**复制**到用户选定的目录，一字不改。
//!
//! - **落点**：默认 `<用户根>/exports/<基名>/`（`fsx::paths::EXPORTS_DIR`）——
//!   导出的东西是"用户自己要看要拷的"，所以住用户根；界面里的「更改位置」可以
//!   指到任何地方（桌面、U 盘、网盘目录都行）。
//! - **基名**：`<模型名 或 归档自己的名字>_<切片时刻>`（`帕姆_20261011142431`）。
//!   时刻取归档名尾巴上那一段，与切片写进 `_meta.json` 的时间同源。
//! - **不覆盖**：目标里已经有同名的那一份就**整批拒绝**（导出是"往你挑的文件夹里
//!   落一整批"，撞名就整批不落，免得留个半批；静默盖掉等于删用户数据）；
//!   半路失败会把已经落下的收回去。
//! - **缺件照实说**：失败的那一次不会有"处理后的文件"（`<id>.gcode` 根本没落盘），
//!   计划里照实报缺哪几份，不编、也不假装三件套都全。
//!
//! ★ 这一处是**全仓唯一一处往数据根以外写盘**（前端其它写动作全走
//! `fsx::atomic` + `resolve_in`，路径被夹在两个根里）。它越界的理由只有一条：
//! **目的是把那几份文件交到用户手里**，而"交到哪儿"是用户当场点出来的绝对路径 ——
//! 命令只从归档树**读**，往那个目录里**新建**文件，不读也不改那个目录里的任何东西。
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

use std::path::{Path, PathBuf};

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
    /// **3MF 模型名**（`_meta.json` 顶层 `modelName`，钩子在切片器临时目录里取的）。
    /// 老记录没有这个键 ⇒ `None` —— 页面那时只显示 G-code 名，**不编名字**。
    pub model_name: Option<String>,
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
    /// 估算出的打印秒数（`detail.estimatedPrintSeconds`）—— 报告页拿它与切片器
    /// 原话比「多了多久」。
    pub estimated_print_seconds: Option<f64>,
    /// 涂胶组（`detail.glue`，键与旧版 `_meta.json` 同名）。
    pub glue: Option<serde_json::Value>,
    /// 擦料塔组（`detail.tower`）。
    pub tower: Option<serde_json::Value>,
    /// 切片工艺组（`detail.slicer`）。
    pub slicer: Option<serde_json::Value>,
    /// 管线各步计时（`pipeline`：id / elapsedMs / message / status）
    pub pipeline: Vec<serde_json::Value>,
    /// **碰撞那一趟的事实**（`_meta.json` 顶层 `collision`，2026-10-18）——
    /// 只有 `E_GCODE_COLLISION_001` 会写：撞在哪、建议往哪挪多少、画俯视图的几何。
    /// 别的记录一律 `null`（原样透传，不在这一层挑字段）。
    pub collision: Option<serde_json::Value>,
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
        model_name: str_at("modelName"),
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
        model_name: None,
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
        estimated_print_seconds: detail
            .and_then(|d| d.get("estimatedPrintSeconds"))
            .and_then(|v| v.as_f64()),
        glue: detail.and_then(|d| d.get("glue")).cloned(),
        tower: detail.and_then(|d| d.get("tower")).cloned(),
        slicer: detail.and_then(|d| d.get("slicer")).cloned(),
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
        collision: meta.get("collision").cloned(),
        invocation: meta.get("invocation").cloned(),
        preset_path: str_at("presetPath"),
        gcode_path: str_at("gcodePath"),
    }
}

/* ===================== 导出（本模块唯一的写口） ===================== */

/// 三件套的**后缀 + 机器名**（顺序 = 界面上从上往下 = 归档落盘顺序）。
///
/// 后缀与 [`crate::archive::begin`] 那三个一一对应（那边是 `with_suffix(&base, …)`）：
/// 同一条记录在归档里叫 `<id>_original.gcode` / `<id>.gcode` / `<id>_meta.json`，
/// 于是"给一条记录配齐三件套"这件事只是把后缀换个人称。
const EXPORT_PARTS: [(&str, &str); 3] = [
    ("_original.gcode", "original"),
    (".gcode", "processed"),
    ("_meta.json", "meta"),
];

/// 导出计划里的一份文件。`kind` 是**机器名**（`original` / `processed` / `meta`），
/// 中文说明由界面配 —— 后端不写界面的话（同 `ErrorCode` 那条纪律）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportExportFile {
    pub kind: &'static str,
    /// 落到用户目录里叫什么（`<基名>_original.gcode` 这种，完整不省略）
    pub name: String,
}

/// 一次导出的计划：三件套叫什么、默认落到哪、哪几份归档里没有。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportExportPlan {
    /// 基名：`<模型名 或 归档自己的名字>_<切片时刻>`
    pub base: String,
    /// 默认落点的**父目录**（`<用户根>/exports`）。界面显示时自己拼上 `<基名>/` ——
    /// 导出命令收的也是这个父目录，两处口径一致。
    pub default_dir: String,
    /// 真在盘上、会跟着导出的那几份（顺序见 [`EXPORT_PARTS`]）
    pub files: Vec<ReportExportFile>,
    /// 归档里没有的那几份（失败的那次不落 `<id>.gcode`）：界面对此说一句实话
    pub missing: Vec<&'static str>,
}

/// 归档里真在盘上的一份源文件 + 它导出后叫什么。
#[derive(Debug, Clone)]
struct ExportPart {
    kind: &'static str,
    src: PathBuf,
    name: String,
}

/// 导出基名：`<模型名 或 归档名>_<切片时刻>`。
///
/// - 有模型名 ⇒ `<模型名>_<归档名尾巴上的时刻>`（`帕姆_20261011142431`）；
/// - 没有（老记录 / 拿不到 3MF 名）⇒ **就用归档自己那串**（`63288.1_20261011142431`）——
///   不编名字、也不换一套（那串本来就是"这条记录"的稳定标识）。
fn export_base(id: &str, model_name: Option<&str>) -> String {
    let Some(who) = model_name.map(sanitize_component).filter(|s| !s.is_empty()) else {
        return id.to_owned();
    };
    match id_stamp(id) {
        Some(stamp) => format!("{who}_{stamp}"),
        None => who,
    }
}

/// 归档 id 尾巴上那一段时刻（`<原名>_YYYYMMDD_HHMMSS` → `YYYYMMDDHHMMSS`）。
///
/// 认不出来就给 `None`（形状不对就少一段时刻，不编一个）—— 形态检查到"位数 + 全是数字"。
fn id_stamp(id: &str) -> Option<String> {
    let mut tail = id.rsplitn(3, '_');
    let secs = tail.next()?;
    let date = tail.next()?;
    let digits = |s: &str, n: usize| s.len() == n && s.chars().all(|c| c.is_ascii_digit());
    (digits(date, 8) && digits(secs, 6)).then(|| format!("{date}{secs}"))
}

/// 一个文件名分量：把**外面来的字符串**清成能当文件名用的一段。
///
/// 模型名是从 3MF 文件名里取的，正常情况干净得很；但它是外面来的，拼进路径之前
/// 必须过一道：Windows 禁 `\ / : * ? " < > |` 与控制符，另外不许以点或空格收尾
/// （`帕姆.` 那种在 Windows 上建不出来）。全是非法字符 ⇒ 空串（调用方回落到归档名）。
fn sanitize_component(raw: &str) -> String {
    let cleaned: String = raw
        .trim()
        .chars()
        .map(|c| {
            if c.is_control() || r#"\/:*?"<>|"#.contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    /* 首尾的点与空格：`..` / `.` / 尾空格在 Windows 上都不是合法的文件名结尾 */
    cleaned.trim_matches([' ', '.']).to_owned()
}

/// 记录 id 只许是**一段普通文件名**。
///
/// 它是从列表里来的，正常情况永远只有一段；但下面每一处 `join(format!("{id}…"))`
/// 都拿它拼路径 —— 一旦带了分隔符，就能拼出归档树外面去。在门口拦掉（写在门口是
/// 因为这是**写盘**那一侧，越界在这里的代价最大）。
///
/// 「全是一串点」（`.` / `..`）也拦：基名最坏情况下就是 id 本身，而它要当**一段目录名**
/// 用（`exports/<基名>/`）—— `..` 会把落点抬到上一层去（盘上一个叫 `.._meta.json` 的
/// 文件就能造出这个 id）。
fn check_id(id: &str) -> Result<(), AppError> {
    if id.is_empty() || id.contains(['/', '\\']) || id.chars().all(|c| c == '.') {
        return Err(AppError::invalid_argument("这条记录的 id 不合法")
            .with_detail(format!("{id:?} 不是一段文件名")));
    }
    Ok(())
}

/// 这一条记录的三件套里**真在盘上**的那些（顺序见 [`EXPORT_PARTS`]）。
fn parts_of(day_dir: &Path, id: &str, base: &str) -> Vec<ExportPart> {
    EXPORT_PARTS
        .iter()
        .filter_map(|(suffix, kind)| {
            let src = day_dir.join(format!("{id}{suffix}"));
            src.is_file().then(|| ExportPart {
                kind,
                src,
                name: format!("{base}{suffix}"),
            })
        })
        .collect()
}

/// 计划里那三份都该有 —— 缺的是哪几个（`missing` 的取值顺序同 [`EXPORT_PARTS`]）。
fn missing_parts(parts: &[ExportPart]) -> Vec<&'static str> {
    EXPORT_PARTS
        .iter()
        .map(|(_, kind)| *kind)
        .filter(|kind| !parts.iter().any(|p| p.kind == *kind))
        .collect()
}

/// 这一条记录里的模型名（`_meta.json` 顶层 `modelName`）。
///
/// 读不出来 / 没有这一格 ⇒ `None`：记录本身坏了也照样能导出（那三份文件与
/// `_meta.json` 读不读得懂无关），基名回落到归档自己的名字。
fn model_name_of(meta_path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(meta_path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("modelName")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// 一条记录在归档里的两处落点：**三件套住的那一天** + 它那份 `_meta.json`。
fn record_paths(root: &Path, id: &str) -> Result<(PathBuf, PathBuf), AppError> {
    let (meta_path, _day) = find_meta(root, id)
        .ok_or_else(|| AppError::not_found(format!("没有这条执行记录：{id}")))?;
    let day_dir = meta_path.parent().map(Path::to_path_buf).ok_or_else(|| {
        AppError::internal("归档路径没有父目录").with_detail(meta_path.display().to_string())
    })?;
    Ok((day_dir, meta_path))
}

/// 把这几份**复制**到 `target` 里（`target` 由调用方给出，含基名那一层）。
///
/// 三道闸，缺一道都出事：
/// 1. **目标不许就是归档自己那一天** —— `fs::copy` 源与目标同一个文件时会先把
///    目标截断，三件套当场互相吃掉（用户把"保存位置"指到归档里就中了）；
/// 2. **不覆盖**：目标里已经有同名的那一份就整批拒绝 —— 一次落三件套，撞名整批不落
///    （免得留个"三份里成了一份"的半批）；那份可能被用户改过，静默盖掉等于删用户数据；
/// 3. **半路失败整批收回**：已经落下的删掉（"任何一项失败整批不落"），宁可什么都不
///    留，也不留一个"三份里成了一份"的残局。
fn copy_parts(
    src_dir: &Path,
    parts: &[ExportPart],
    target: &Path,
) -> Result<Vec<String>, AppError> {
    std::fs::create_dir_all(target).map_err(|e| {
        AppError::io(format!("建不出保存位置：{}", target.display())).with_detail(e.to_string())
    })?;

    if let (Ok(dest), Ok(src)) = (target.canonicalize(), src_dir.canonicalize()) {
        if dest == src {
            return Err(AppError::invalid_argument(
                "保存位置就是归档自己的目录 —— 换一个地方（在这里会把原件覆盖掉）",
            ));
        }
    }

    for part in parts {
        let dest = target.join(&part.name);
        if dest.exists() {
            return Err(AppError::invalid_argument(format!(
                "保存位置里已经有 {} 了 —— 换个位置，或先把那一份挪走",
                part.name
            )));
        }
    }

    let mut written: Vec<String> = Vec::new();
    for part in parts {
        let dest = target.join(&part.name);
        if let Err(e) = std::fs::copy(&part.src, &dest) {
            for done in &written {
                let _ = std::fs::remove_file(done);
            }
            return Err(AppError::io(format!("写不出 {}", part.name)).with_detail(e.to_string()));
        }
        written.push(dest.display().to_string());
    }
    Ok(written)
}

/// 导出的默认父目录（`<用户根>/exports`，`fsx::paths::EXPORTS_DIR`）。
fn default_export_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    Ok(crate::fsx::paths::user_root(app)?.join(crate::fsx::paths::EXPORTS_DIR))
}

/// **导出计划**：三件套叫什么、默认落到哪、缺哪几份。
///
/// 界面的导出框先要它（文件名要照实显示、路径要给出默认值），随后
/// [`export_report_archive`] 才真写盘。两侧的基名由同一个纯函数算，不会漂。
#[tauri::command]
pub fn get_report_export_plan(app: AppHandle, id: String) -> Result<ReportExportPlan, AppError> {
    traced("getReportExportPlan", |_| {
        check_id(&id)?;
        let root = history_root(&app)?;
        let (day_dir, meta_path) = record_paths(&root, &id)?;
        let base = export_base(&id, model_name_of(&meta_path).as_deref());
        let parts = parts_of(&day_dir, &id, &base);
        Ok(ReportExportPlan {
            files: parts
                .iter()
                .map(|p| ReportExportFile {
                    kind: p.kind,
                    name: p.name.clone(),
                })
                .collect(),
            missing: missing_parts(&parts),
            base,
            default_dir: default_export_dir(&app)?.display().to_string(),
        })
    })
}

/// **把这一条的三件套导出去**（本模块唯一的写口）：复制到 `dir/<基名>/` 下。
///
/// `dir` 是**父目录**（默认值由 [`get_report_export_plan`] 给，用户可以在界面上改）；
/// 必须是绝对路径 —— 相对路径会落到进程自己的当前目录，那不是用户点过的任何一个地方。
///
/// 返回真正落盘的路径（按 [`EXPORT_PARTS`] 的顺序）。缺的那几份不在这里报错：
/// 计划里已经说过缺哪个了（失败的那一次本来就没有"处理后的文件"），
/// 一条记录一份都不在盘上才是错误（那时什么也导不出来）。
#[tauri::command]
pub fn export_report_archive(
    app: AppHandle,
    id: String,
    dir: String,
) -> Result<Vec<String>, AppError> {
    traced("exportReportArchive", |_| {
        check_id(&id)?;
        let parent = PathBuf::from(&dir);
        if dir.trim().is_empty() || !parent.is_absolute() {
            return Err(AppError::invalid_argument("保存位置要给一个绝对路径"));
        }
        let root = history_root(&app)?;
        let (day_dir, meta_path) = record_paths(&root, &id)?;
        let base = export_base(&id, model_name_of(&meta_path).as_deref());
        let parts = parts_of(&day_dir, &id, &base);
        if parts.is_empty() {
            return Err(AppError::not_found(format!(
                "这条记录的文件一份都不在盘上：{id}"
            )));
        }
        copy_parts(&day_dir, &parts, &parent.join(&base))
    })
}

/// **打开这一次导出的那个文件夹**（导出框上那颗「打开位置」，2026-10-11 作者点的）。
///
/// 导出成功之后用户要做的事只有一件：去那个文件夹里看一眼、或把这几份交给别人 ——
/// 所以那一刻「更改位置」换成「打开位置」（位置已经定了，没什么可改的）。
///
/// 闸门与 `local::reveal_in_folder` / `wb_reveal_asset` 同一条（**路径由后端自己算**，
/// 不给前端传任意路径的机会）：
///
/// - 落点是**后端算出来的基名**接在 `dir` 后面（`<dir>/<基名>`）—— 前端传的只当父目录；
/// - 它必须**真的是一个目录**（见 [`export_target`]）：既挡住"这条记录根本没导出过"，
///   也挡住把一个**文件**交给系统 —— `open_path` 拿到可执行文件是会**启动**它的，
///   那就是另一回事了（`wb_open_external` 那条"白名单要窄"的纪律）；
/// - **只读、只开系统文件管理器、一个应用状态都不碰**。
#[tauri::command]
pub async fn reveal_report_export(app: AppHandle, id: String, dir: String) -> Result<(), AppError> {
    traced("revealReportExport", |_| {
        check_id(&id)?;
        let parent = PathBuf::from(&dir);
        if dir.trim().is_empty() || !parent.is_absolute() {
            return Err(AppError::invalid_argument("保存位置要给一个绝对路径"));
        }
        let root = history_root(&app)?;
        let (_day_dir, meta_path) = record_paths(&root, &id)?;
        let base = export_base(&id, model_name_of(&meta_path).as_deref());
        let target = export_target(&parent, &base)?;
        tauri_plugin_opener::OpenerExt::opener(&app)
            .open_path(target.to_string_lossy().into_owned(), None::<&str>)
            .map_err(|e| AppError::io("打不开系统文件管理器").with_detail(e.to_string()))
    })
}

/// 这次导出的落点：`<父目录>/<基名>`，而且**必须真是一个目录**才给。
///
/// 不在就如实拒绝并附上期望路径（与 `wb_reveal_asset` 的 `reveal_target` 同一条：
/// 不去猜、也不打开一个不存在的目录）。同名的是一个**文件**时同样拒绝 —— 交给系统的
/// 路径是"打开它"，而一个文件会被**启动**（`.exe` 尤其），那不是用户点「打开位置」的意思。
fn export_target(parent: &Path, base: &str) -> Result<PathBuf, AppError> {
    let target = parent.join(base);
    if !target.is_dir() {
        return Err(AppError::not_found(format!("这个位置里还没有「{base}」"))
            .with_detail(format!("期望路径：{}", target.display())));
    }
    Ok(target)
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
            "modelName": "帕姆",
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
        assert_eq!(r.model_name.as_deref(), Some("帕姆"));
        assert_eq!(r.elapsed_ms, Some(2719));
        assert!(r.ok);
        assert_eq!(r.warning_count, 1);
    }

    /// 老记录（`modelName` 缺键或落 null）⇒ `None`，不 panic、不编名字。
    #[test]
    fn an_old_record_has_no_model_name() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("2026-08-19");
        std::fs::create_dir_all(&day).unwrap();
        for (id, body) in [
            ("old_20260819_000000", r#"{"schemaVersion":2,"error":null}"#),
            (
                "null_20260819_000001",
                r#"{"schemaVersion":2,"modelName":null,"error":null}"#,
            ),
        ] {
            std::fs::write(day.join(format!("{id}_meta.json")), body).unwrap();
        }

        let rows = collect(dir.path());
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|r| r.model_name.is_none()));
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

    /// 三组分包与新数字透传：`detail.glue/tower/slicer` 原样带出、
    /// `estimatedPrintSeconds` 取成数字 —— 报告页拿的就是这些键。
    #[test]
    fn detail_carries_the_new_report_groups() {
        let summary = ReportSummary {
            id: "x_20261011_100000".into(),
            day: "2026-10-11".into(),
            started_at: Some("2026-10-11 10:00:00".into()),
            finished_at: None,
            elapsed_ms: None,
            preset_name: None,
            machine_type: None,
            gcode_name: None,
            model_name: None,
            ok: true,
            error: None,
            warning_count: 0,
        };
        let meta = serde_json::json!({
            "detail": {
                "estimatedPrintSeconds": 2952.9109519850203,
                "glue": {"glueTotalLength": 9350.845588078173},
                "tower": {"towerLayers": 536},
                "slicer": {"machinePresetName": "Bambu Lab A1 mini 0.4 nozzle"}
            }
        });
        let d = detail_of(summary.clone(), &meta);
        assert_eq!(d.estimated_print_seconds, Some(2952.9109519850203));
        assert_eq!(d.glue.unwrap()["glueTotalLength"], 9350.845588078173);
        assert_eq!(d.tower.unwrap()["towerLayers"], 536);
        assert_eq!(
            d.slicer.unwrap()["machinePresetName"],
            "Bambu Lab A1 mini 0.4 nozzle"
        );

        // 旧记录（没有这些键）：全是 None，不 panic —— 报告页整组不摆
        let old = serde_json::json!({"detail": {"stats": {}}});
        let d = detail_of(summary, &old);
        assert!(d.glue.is_none() && d.tower.is_none() && d.slicer.is_none());
        assert!(d.estimated_print_seconds.is_none());
    }

    /* ---------- 导出（2026-10-11） ---------- */

    /// 在 `<dir>/2026-10-11/` 下造一条记录（形状照 [`crate::archive`] 那三件套）。
    /// `with_output = false` = 跑失败的那一次（`archive_output` 没跑，没有 `<id>.gcode`）。
    fn record(dir: &std::path::Path, id: &str, model_name: Option<&str>, with_output: bool) {
        let day = dir.join("2026-10-11");
        std::fs::create_dir_all(&day).unwrap();
        let meta = match model_name {
            Some(m) => format!(r#"{{"modelName":"{m}","error":null}}"#),
            None => r#"{"error":null}"#.to_owned(),
        };
        std::fs::write(day.join(format!("{id}_meta.json")), meta).unwrap();
        std::fs::write(day.join(format!("{id}_original.gcode")), "; 原件").unwrap();
        if with_output {
            std::fs::write(day.join(format!("{id}.gcode")), "; 处理后").unwrap();
        }
    }

    /// 基名：有模型名 ⇒ `<模型名>_<归档名尾巴上的时刻>`；没有 ⇒ 归档自己那串。
    #[test]
    fn export_base_prefers_the_model_name_and_keeps_the_archive_stamp() {
        assert_eq!(
            export_base("63288.1_20261011_142431", Some("帕姆")),
            "帕姆_20261011142431"
        );
        /* 老记录 / 拿不到 3MF 名 ⇒ 原样用归档那串（不编名字） */
        for none in [None, Some(""), Some("   ")] {
            assert_eq!(
                export_base("63288.1_20261011_142431", none),
                "63288.1_20261011_142431"
            );
        }
        /* 归档名尾巴不是时刻（手工丢进去的目录）⇒ 只留模型名，不编时刻 */
        assert_eq!(export_base("手写的名字", Some("帕姆")), "帕姆");
    }

    /// 模型名是外面来的字符串：拼进路径之前必须清干净（它要当一段目录名用）。
    #[test]
    fn export_base_sanitizes_what_comes_from_outside() {
        let got = export_base("x_20261011_142431", Some("a/b:c*?"));
        assert!(got.starts_with("a_b_c"), "{got}");
        assert!(!got.contains(['/', ':', '*', '?']), "{got}");
        assert!(got.ends_with("_20261011142431"), "{got}");
        /* 清完什么都不剩 ⇒ 回落到归档名 */
        assert_eq!(
            export_base("x_20261011_142431", Some("..")),
            "x_20261011_142431"
        );
        /* 尾点 / 尾空格在 Windows 上建不出来 ⇒ 去掉 */
        assert_eq!(
            export_base("x_20261011_142431", Some("帕姆. ")),
            "帕姆_20261011142431"
        );
    }

    /// 跑失败的那一次：归档里**没有** `<id>.gcode` —— 计划照实报缺哪一份，不假装齐。
    #[test]
    fn a_failed_run_misses_only_the_processed_file() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("2026-10-11");

        record(dir.path(), "ok_20261011_142431", None, true);
        let parts = parts_of(&day, "ok_20261011_142431", "ok_20261011_142431");
        assert_eq!(parts.len(), 3);
        assert!(missing_parts(&parts).is_empty());
        assert_eq!(parts[1].name, "ok_20261011_142431.gcode");

        let failed = "63288.1_20261011_142431";
        record(dir.path(), failed, Some("帕姆"), false);
        let parts = parts_of(&day, failed, "帕姆_20261011142431");
        assert_eq!(parts.len(), 2);
        assert_eq!(missing_parts(&parts), vec!["processed"]);
        assert_eq!(parts[0].name, "帕姆_20261011142431_original.gcode");
        assert_eq!(parts[1].name, "帕姆_20261011142431_meta.json");
    }

    /// 三件套按基名落到目标目录里，**归档那一份一个字节不动**（复制不是移动）。
    #[test]
    fn export_copies_the_three_files_under_the_base_name() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let day = src.path().join("2026-10-11");
        let id = "63288.1_20261011_142431";
        record(src.path(), id, Some("帕姆"), true);

        let parts = parts_of(&day, id, "帕姆_20261011142431");
        let target = out.path().join("帕姆_20261011142431");
        let written = copy_parts(&day, &parts, &target).unwrap();

        assert_eq!(written.len(), 3);
        assert_eq!(
            std::fs::read_to_string(target.join("帕姆_20261011142431.gcode")).unwrap(),
            "; 处理后"
        );
        assert_eq!(
            std::fs::read_to_string(target.join("帕姆_20261011142431_meta.json")).unwrap(),
            r#"{"modelName":"帕姆","error":null}"#
        );
        assert_eq!(
            std::fs::read_to_string(day.join(format!("{id}.gcode"))).unwrap(),
            "; 处理后"
        );
    }

    /// 目标里已经有同名的那一份 ⇒ **整批拒绝**（静默盖掉等于删用户数据；导出落的是一整批，
    /// 撞名整批不落 —— 这一条与「复制进切片器」无关，那条 2026-10-13 已改判成覆盖）
    #[test]
    fn export_refuses_to_overwrite_what_is_already_there() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let day = src.path().join("2026-10-11");
        let id = "63288.1_20261011_142431";
        record(src.path(), id, Some("帕姆"), true);

        let parts = parts_of(&day, id, "帕姆_20261011142431");
        let target = out.path().join("帕姆_20261011142431");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("帕姆_20261011142431.gcode"), "用户自己改过的").unwrap();

        let err = copy_parts(&day, &parts, &target).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert!(err.message.contains("已经有"), "{}", err.message);
        /* 那一份没被动过，别的也一份都没落（先过一遍地雷再写） */
        assert_eq!(
            std::fs::read_to_string(target.join("帕姆_20261011142431.gcode")).unwrap(),
            "用户自己改过的"
        );
        assert!(!target.join("帕姆_20261011142431_meta.json").exists());
    }

    /// 保存位置指到归档自己那一天 ⇒ 拒绝（`fs::copy` 源与目标同一个文件会先把目标截断，
    /// 三件套当场互相吃掉），且原件毫发无损。
    #[test]
    fn export_refuses_the_archive_directory_itself() {
        let src = tempfile::tempdir().unwrap();
        let day = src.path().join("2026-10-11");
        let id = "63288.1_20261011_142431";
        record(src.path(), id, Some("帕姆"), true);

        let parts = parts_of(&day, id, "帕姆_20261011142431");
        let err = copy_parts(&day, &parts, &day).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert_eq!(
            std::fs::read_to_string(day.join(format!("{id}.gcode"))).unwrap(),
            "; 处理后"
        );
    }

    /// 「打开位置」只认**真在的那个导出目录**：还没导出过就如实拒绝（附期望路径），
    /// 同名的是一个**文件**也拒绝 —— 交给系统的那条路径是"打开它"，
    /// 而一个文件会被**启动**（`.exe` 尤其），那不是用户点「打开位置」的意思。
    #[test]
    fn reveal_only_opens_a_real_export_directory() {
        let out = tempfile::tempdir().unwrap();
        let base = "帕姆_20261011142431";

        let err = export_target(out.path(), base).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::NotFound);
        assert!(err.message.contains(base), "{}", err.message);

        let target = out.path().join(base);
        std::fs::create_dir_all(&target).unwrap();
        assert_eq!(export_target(out.path(), base).unwrap(), target);

        std::fs::write(out.path().join("帕姆.exe"), "x").unwrap();
        assert!(export_target(out.path(), "帕姆.exe").is_err(), "文件不给");
    }

    /// id 是拼路径的那一段：带分隔符 / 全是点的一律在门口拦掉。
    #[test]
    fn an_id_that_is_not_a_plain_file_name_is_refused() {
        assert!(check_id("63288.1_20261011_142431").is_ok());
        for bad in ["", ".", "..", "../x", r"..\x", "a/b", "a\\b"] {
            assert!(check_id(bad).is_err(), "{bad:?} 该被拦下");
        }
    }
}
