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
//! # 压缩与信息页（2026-11-03）
//!
//! 导出框左下角那颗「信息」翻到另一页时，要能看清**这些文件在归档里的真实位置**，
//! 并且不导出也能在系统文件管理器里定位它们 —— 于是：
//!
//! - 计划里每一份带上 `path`（归档里的绝对路径，只读展示，界面不拿它拼任何写路径）；
//! - [`reveal_report_source`] 只认 `id + kind`，路径由后端自己算（与「打开位置」同一条
//!   纪律：不给前端传任意路径的机会）；
//! - [`export_report_archive`] 多一格 `compress`：真时打成一个 `<基名>.zip`
//!   （三件套原样进包，不重新编码），假时还是散着复制三份。
//!   **压缩是导出框那一格的偏好**（默认开、用户关过就记住），这一层只照办。
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

use postprocess::diag::messages;

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
    /// **模型俯视几何**（`_meta.json` 顶层 `modelMap`，2026-10-27）：报告页擦料塔那一栏
    /// 的俯视图拿它画"模型在板上占哪一块" —— 与碰撞那一屏同一个画法、同一个来源。
    ///
    /// 为什么单列一格而不是塞进 `collision`：成功那趟 `collision` 恒为 `null`，
    /// 而作者要看的正是成功那趟。旧记录 / 那趟算不出来 ⇒ `null` ⇒ 页面退回"只有塔"那幅。
    pub model_map: Option<serde_json::Value>,
    /// **参数快照**（`detail.snapshot`，2026-10-21）：切片那一刻预设 TOML 的全量参数
    /// + IR 终态原样。旧记录没有这个键 ⇒ `null` ⇒ 页面整组不摆。
    pub snapshot: Option<serde_json::Value>,
    /// **挪塔记录**（`_meta.json` 顶层 `relocation`，2026-10-18）——
    /// 用户选了什么 / 从哪挪到哪 / 挪了多少 / 复检过没过。别的记录一律 `null`。
    pub relocation: Option<serde_json::Value>,
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
            return broken_summary(
                id,
                day,
                messages::text("ui.ipc.report.broken_read", &[("detail", &e.to_string())]),
            );
        }
    };
    let meta: serde_json::Value = match serde_json::from_str(&text) {
        Ok(v) => v,
        Err(e) => {
            return broken_summary(
                id,
                day,
                messages::text("ui.ipc.report.broken_json", &[("detail", &e.to_string())]),
            );
        }
    };
    let str_at = |key: &str| {
        meta.get(key)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    /*
     * 那句「为什么没成」**按当前语言重说一遍**（2026-10-27 作者：回放/报告切到中文
     * 就该是中文，不能停在写记录那一刻的语言）。
     *
     * 新记录带 `errorKey`（写的时候认出来的）；老记录只有当年那句话，就按**原文反查**
     * 一次键。反查不到（那句里已经填了值）就原样摆 —— 不猜、不模糊匹配。
     */
    let error = localized_error(&meta).or_else(|| str_at("error"));
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
        let (path, day) = find_meta(&dir, &id).ok_or_else(|| {
            AppError::not_found(messages::text(
                "ui.ipc.report.131",
                &[("id", &id.to_string())],
            ))
        })?;
        let text = std::fs::read_to_string(&path).map_err(|e| {
            AppError::io(messages::text("ui.ipc.report.132", &[])).with_detail(e.to_string())
        })?;
        let meta: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
            AppError::corrupted(messages::text("ui.ipc.report.133", &[])).with_detail(e.to_string())
        })?;
        let summary = summarize(&path, &id, &day);
        Ok(detail_of(summary, &meta))
    })
}

/**
 * 记录里那句"为什么没成"，按**当前语言**给（2026-10-27）。
 *
 * 归档的道理与管线明细同一条：**存事实不存句子**。新记录带 `errorKey`；老记录只有
 * 当年那一句，就按原文在文案表里反查一次键。两样都没有 ⇒ `None`，调用方回落原句。
 */
fn localized_error(meta: &serde_json::Value) -> Option<String> {
    localized_error_in(messages::current(), meta)
}

/// [`localized_error`] 的**指定语言**那一份（判据用；生产走上面那个"当前语言"）。
fn localized_error_in(lang: messages::Lang, meta: &serde_json::Value) -> Option<String> {
    let key = meta
        .get("errorKey")
        .and_then(|v| v.as_str())
        .map(str::to_owned)
        .or_else(|| {
            meta.get("error")
                .and_then(|v| v.as_str())
                .and_then(messages::key_of_exact_text)
                .map(str::to_owned)
        })?;
    let empty = serde_json::Map::new();
    let args = meta
        .get("errorArgs")
        .and_then(|v| v.as_object())
        .unwrap_or(&empty);
    let pairs: Vec<(String, String)> = args
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap_or_default().to_owned()))
        .collect();
    let refs: Vec<(&str, &str)> = pairs
        .iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();
    Some(error_line_in(lang, &key, &refs, meta))
}

/// 一条记录的 error 按**这个键**重说一遍。
///
/// ★ 两种情况长得不一样（2026-10-31）：
///
/// 1. `errorKey` 是 `hook.line.*`（或老记录没有 `cancelled` 那一格）⇒ 键指的就是
///    **那一整句**，直接取。
/// 2. `errorKey` 是**原因键**（`hook.cancelled` / `hook.mismatch_stopped` …）——
///    这是 2026-10-31 起钩子写的新形状。记录里那句原文是结论行
///    （`已停止：{原因}［{码}］`），只取原因会丢掉框子与码，所以按写入侧
///    [`crate::hook::conclusion_line`] 同一套判定重新拼：
///    `cancelled` × `code` 有没有 ⇒ 四个框子之一。
///
/// 判据见 `an_archived_stop_is_restated_in_the_current_language`。
fn error_line_in(
    lang: messages::Lang,
    key: &str,
    refs: &[(&str, &str)],
    meta: &serde_json::Value,
) -> String {
    let cancelled = meta.get("cancelled").and_then(serde_json::Value::as_bool);
    if key.starts_with("hook.line.") || cancelled.is_none() {
        return messages::text_in(lang, key, refs);
    }
    let reason = messages::text_in(lang, key, refs);
    let code = messages::code_of(key).unwrap_or_default();
    let line_key = match (cancelled.unwrap_or(false), code.is_empty()) {
        (false, false) => crate::hook::keys::LINE_FAILED,
        (false, true) => crate::hook::keys::LINE_FAILED_NOCODE,
        (true, false) => crate::hook::keys::LINE_STOPPED,
        (true, true) => crate::hook::keys::LINE_CANCELLED,
    };
    messages::text_in(
        lang,
        line_key,
        &[("reason", reason.as_str()), ("code", code.as_str())],
    )
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
        snapshot: detail.and_then(|d| d.get("snapshot")).cloned(),
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
        model_map: meta.get("modelMap").cloned(),
        relocation: meta.get("relocation").cloned(),
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
    /// **归档里那一份的绝对路径**（信息页要显示、复制、在访达里定位它）。
    ///
    /// 只给界面**看** —— 定位那条命令收的是 `id + kind`，不收这个路径
    /// （同 `reveal_report_export`：路径只许后端自己算）。
    ///
    /// 这一份**不在盘上**时也照给（那正是它本该在的位置）：界面上那一行要划线，
    /// 划完还得能复制到那串路径（"它本来该在这儿"）。
    pub path: String,
    /// 这一份**真在盘上**吗。
    ///
    /// 2026-11-04（作者：失败的可以加上徽章，第三个也显示出来然后灰色划线意思没有这个文件）：
    /// 计划**恒报三份**，缺的那份由这一格说明 —— 原来缺的那份整行不出现，
    /// 界面上只剩一句"归档里没有…"，看着像清单少了一行。
    pub exists: bool,
}

/// 一次导出的计划：三件套叫什么、默认落到哪、哪几份真在盘上。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportExportPlan {
    /// 基名：`<模型名 或 归档自己的名字>_<切片时刻>`
    pub base: String,
    /// 默认落点的**父目录**（`<用户根>/exports`）。界面显示时自己拼上 `<基名>/` ——
    /// 导出命令收的也是这个父目录，两处口径一致。
    pub default_dir: String,
    /// **恒为三份**（顺序见 [`EXPORT_PARTS`]）；哪一份不在盘上由 [`ReportExportFile::exists`] 说。
    ///
    /// 2026-11-04 改：原来是"只列真在盘上的那几份 + 另给一份 `missing`"，
    /// 于是失败那一次界面上少一行、还要在底下补一句话解释少的是哪一份 ——
    /// 作者要的是**三行都在**、缺的那行灰掉划线。两套说法（行 + 那句话）也就并成了一套。
    pub files: Vec<ReportExportFile>,
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
        return Err(
            AppError::invalid_argument(messages::text("ui.ipc.report.134", &[])).with_detail(
                ::postprocess::diag::messages::text(
                    "ui.ipc.report.why_not_filename",
                    &[("id", &format!("{id:?}"))],
                ),
            ),
        );
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

/// 计划里的那三份（**恒为三份**，顺序同 [`EXPORT_PARTS`]）。
///
/// 缺的那份也照给名字与位置（名字由基名 + 后缀算，位置就是"它本该在的地方"），
/// `exists` 说明它在不在盘上 —— 界面据此把那行灰掉划线，而不是整行不出现。
fn files_of(parts: &[ExportPart], day_dir: &Path, id: &str, base: &str) -> Vec<ReportExportFile> {
    EXPORT_PARTS
        .iter()
        .map(|(suffix, kind)| ReportExportFile {
            kind,
            name: format!("{base}{suffix}"),
            path: day_dir.join(format!("{id}{suffix}")).display().to_string(),
            exists: parts.iter().any(|p| p.kind == *kind),
        })
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
        .ok_or_else(|| AppError::not_found(messages::text("ui.ipc.report.135", &[("id", id)])))?;
    let day_dir = meta_path.parent().map(Path::to_path_buf).ok_or_else(|| {
        AppError::internal(messages::text("ui.ipc.report.136", &[]))
            .with_detail(meta_path.display().to_string())
    })?;
    Ok((day_dir, meta_path))
}

/// 「位置里已经有这一份」——**带稳定码**，界面据此摆出「重新导出」。
///
/// 2026-11-04（作者问「位置已有怎么办？是显示一个打开文件夹的按钮还是干脆重新覆盖一次」）：
/// 界面要能分得清"这是撞名，可以覆盖"和"这是别的失败"，而它拿到的只有一句话。
/// 判句子是最脆的写法（翻译一改就失效），所以把码挂在这条错上，
/// 界面只认 `E_REPORT_EXPORT_EXISTS_001`。
fn export_exists(name: &str) -> AppError {
    AppError::invalid_argument(messages::text("ui.ipc.report.139", &[("v1", name)]))
        .with_message_code(EXPORT_EXISTS_CODE)
}

/// 「位置里已经有这一份」的稳定码（界面按它摆「重新导出」，见 [`export_exists`]）。
const EXPORT_EXISTS_CODE: &str = "E_REPORT_EXPORT_EXISTS_001";

/// 把这几份**复制**到 `target` 里（`target` 由调用方给出，含基名那一层）。
///
/// 三道闸，缺一道都出事：
/// 1. **目标不许就是归档自己那一天** —— `fs::copy` 源与目标同一个文件时会先把
///    目标截断，三件套当场互相吃掉（用户把"保存位置"指到归档里就中了）；
/// 2. **默认不覆盖**：目标里已经有同名的那一份就整批拒绝，并挂上
///    [`EXPORT_EXISTS_CODE`] —— 一次落三件套，撞名整批不落（免得留个"三份里成了一份"
///    的半批）；那份可能被用户改过，**没问过就不许盖**；
/// 3. **半路失败整批收回**：已经落下的删掉（"任何一项失败整批不落"），宁可什么都不
///    留，也不留一个"三份里成了一份"的残局。
///
/// `overwrite = true`（界面在那条撞名错误上问过用户、他点了「重新导出」）时改走第二条路：
/// **先把几份全写成同目录下的临时名，齐了再逐个改名换上去** —— 中途失败只删临时件，
/// 位置上原来那几份一个字节不动。不做"边查边盖"：那样八成就留下一份新两份旧的。
fn copy_parts(
    src_dir: &Path,
    parts: &[ExportPart],
    target: &Path,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    std::fs::create_dir_all(target).map_err(|e| {
        AppError::io(messages::text(
            "ui.ipc.report.137",
            &[("v1", &target.display().to_string())],
        ))
        .with_detail(e.to_string())
    })?;

    if let (Ok(dest), Ok(src)) = (target.canonicalize(), src_dir.canonicalize()) {
        if dest == src {
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.report.138",
                &[],
            )));
        }
    }

    if !overwrite {
        for part in parts {
            if target.join(&part.name).exists() {
                return Err(export_exists(&part.name));
            }
        }

        let mut written: Vec<String> = Vec::new();
        for part in parts {
            let dest = target.join(&part.name);
            if let Err(e) = std::fs::copy(&part.src, &dest) {
                for done in &written {
                    let _ = std::fs::remove_file(done);
                }
                return Err(AppError::io(messages::text(
                    "ui.ipc.report.140",
                    &[("v1", &part.name.to_string())],
                ))
                .with_detail(e.to_string()));
            }
            written.push(dest.display().to_string());
        }
        return Ok(written);
    }

    /* 覆盖这一档：先落临时件（同目录，改名才是原子的），全齐了再换上去 */
    let mut staged: Vec<(PathBuf, PathBuf)> = Vec::new();
    for part in parts {
        let tmp = target.join(format!(".{}.partial", part.name));
        if let Err(e) = std::fs::copy(&part.src, &tmp) {
            for (left, _) in &staged {
                let _ = std::fs::remove_file(left);
            }
            let _ = std::fs::remove_file(&tmp);
            return Err(AppError::io(messages::text(
                "ui.ipc.report.140",
                &[("v1", &part.name.to_string())],
            ))
            .with_detail(e.to_string()));
        }
        staged.push((tmp, target.join(&part.name)));
    }

    let mut written: Vec<String> = Vec::new();
    for (at, (tmp, dest)) in staged.iter().enumerate() {
        if let Err(e) = std::fs::rename(tmp, dest) {
            /* 还没换上去的那几份临时件清掉；已经换上去的留着（用户要的就是新的） */
            for (left, _) in &staged[at..] {
                let _ = std::fs::remove_file(left);
            }
            return Err(AppError::io(messages::text(
                "ui.ipc.report.140",
                &[("v1", &dest.display().to_string())],
            ))
            .with_detail(e.to_string()));
        }
        written.push(dest.display().to_string());
    }
    Ok(written)
}

/// **压成一包**：`<父目录>/<基名>.zip`，里面就是那几份（包里的名字与散着导出去的一样）。
///
/// 三道闸与 [`copy_parts`] 同一条，只是"一份一份地查"换成"整包查"：
/// 1. **父目录不许就是归档自己那一天**（不然等于把包写进原件旁边，之后还认不出谁是谁）；
/// 2. **默认不覆盖**：`<基名>.zip` 已经在就整批拒绝（那份可能是用户自己压的），
///    并挂 [`EXPORT_EXISTS_CODE`]；`overwrite = true` 时才换掉它；
/// 3. **半路失败不留半个包** —— 走 `atomic_write_stream`：临时文件写坏了它自己删，
///    改名成功才算数，盘上永远不会出现"能打开但少一半"的 zip（覆盖那一档也一样：
///    `persist` 是原子换名）。
///
/// 压缩只做 deflate，**不重新编码任何内容**（G-code 与 JSON 都是原字节进包）。
fn export_zip(
    src_dir: &Path,
    parts: &[ExportPart],
    parent: &Path,
    base: &str,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    let zip_name = format!("{base}.zip");
    let target = parent.join(&zip_name);

    if let (Ok(dest), Ok(src)) = (parent.canonicalize(), src_dir.canonicalize()) {
        if dest == src {
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.report.138",
                &[],
            )));
        }
    }

    if target.exists() && !overwrite {
        return Err(export_exists(&zip_name));
    }

    crate::fsx::atomic::atomic_write_stream(&target, |file| {
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for part in parts {
            zip.start_file(part.name.as_str(), opts)
                .map_err(|e| zip_write_err(&zip_name, e))?;
            let mut reader = std::fs::File::open(&part.src).map_err(|e| {
                AppError::io(messages::text(
                    "ui.ipc.report.140",
                    &[("v1", &part.name.to_string())],
                ))
                .with_detail(e.to_string())
            })?;
            std::io::copy(&mut reader, &mut zip).map_err(|e| {
                AppError::io(messages::text(
                    "ui.ipc.report.140",
                    &[("v1", &part.name.to_string())],
                ))
                .with_detail(e.to_string())
            })?;
        }
        zip.finish().map_err(|e| zip_write_err(&zip_name, e))?;
        Ok(())
    })?;

    Ok(vec![target.display().to_string()])
}

/// 写 zip 时那几处出错：一律说"写不出这个包"（带上包名），细节原样留着排查。
fn zip_write_err(name: &str, e: zip::result::ZipError) -> AppError {
    AppError::io(messages::text("ui.ipc.report.140", &[("v1", name)])).with_detail(e.to_string())
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
            files: files_of(&parts, &day_dir, &id, &base),
            base,
            default_dir: default_export_dir(&app)?.display().to_string(),
        })
    })
}

/// **把这一条的三件套导出去**（本模块唯一的写口）。
///
/// `dir` 是**父目录**（默认值由 [`get_report_export_plan`] 给，用户可以在界面上改）；
/// 必须是绝对路径 —— 相对路径会落到进程自己的当前目录，那不是用户点过的任何一个地方。
///
/// `compress` 由导出框那一格决定（默认开，用户关过就记住）：
///
/// - `true` ⇒ 打成一个 `<基名>.zip` 落在 `dir` 下；
/// - `false` ⇒ 散着复制到 `dir/<基名>/` 里（原来那一条）。
///
/// 返回真正落盘的路径（散着时按 [`EXPORT_PARTS`] 的顺序；压缩时就是那个 zip）。
/// 缺的那几份不在这里报错：计划里已经说过缺哪个了（失败的那一次本来就没有"处理后的文件"），
/// 一条记录一份都不在盘上才是错误（那时什么也导不出来）。
///
/// `overwrite`（2026-11-04 作者问「位置已有怎么办」）：
///
/// - `false` ⇒ 位置上已经有同名的那一份就**整批拒绝**，错里带 [`EXPORT_EXISTS_CODE`]；
/// - `true` ⇒ 盖掉它。界面从不默认传 `true`：它只在那条撞名错误上问过用户、
///   他点了「重新导出」之后才带 —— 那份可能是他自己动过的唯一一份。
#[tauri::command]
pub fn export_report_archive(
    app: AppHandle,
    id: String,
    dir: String,
    compress: bool,
    overwrite: bool,
) -> Result<Vec<String>, AppError> {
    traced("exportReportArchive", |_| {
        check_id(&id)?;
        let parent = PathBuf::from(&dir);
        if dir.trim().is_empty() || !parent.is_absolute() {
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.report.141",
                &[],
            )));
        }
        let root = history_root(&app)?;
        let (day_dir, meta_path) = record_paths(&root, &id)?;
        let base = export_base(&id, model_name_of(&meta_path).as_deref());
        let parts = parts_of(&day_dir, &id, &base);
        if parts.is_empty() {
            return Err(AppError::not_found(messages::text(
                "ui.ipc.report.142",
                &[("id", &id.to_string())],
            )));
        }
        if compress {
            return export_zip(&day_dir, &parts, &parent, &base, overwrite);
        }
        copy_parts(&day_dir, &parts, &parent.join(&base), overwrite)
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
            return Err(AppError::invalid_argument(messages::text(
                "ui.ipc.report.143",
                &[],
            )));
        }
        let root = history_root(&app)?;
        let (_day_dir, meta_path) = record_paths(&root, &id)?;
        let base = export_base(&id, model_name_of(&meta_path).as_deref());
        let target = export_target(&parent, &base)?;
        let opener = tauri_plugin_opener::OpenerExt::opener(&app);
        /*
         * 两种落点两种开法（2026-11-03，加了压缩那一条）：
         *   · 散着导出去 ⇒ 落点是一个**目录**，"去那儿看看"就是把文件夹打开；
         *   · 压成一包 ⇒ 落点是一个 **zip**，没有"那个文件夹"可开 —— 在父目录里
         *     把它**选中**才是用户点「打开位置」时想看的东西（`.zip` 交给系统会被
         *     解压成一个新目录，那是另一个意思）。
         */
        if target.is_dir() {
            opener
                .open_path(target.to_string_lossy().into_owned(), None::<&str>)
                .map_err(|e| {
                    AppError::io(messages::text("ui.ipc.report.144", &[]))
                        .with_detail(e.to_string())
                })
        } else {
            opener.reveal_item_in_dir(&target).map_err(|e| {
                AppError::io(messages::text("ui.ipc.report.144", &[])).with_detail(e.to_string())
            })
        }
    })
}

/// 这次导出的落点：`<父目录>/<基名>`（目录）或 `<父目录>/<基名>.zip`（压缩），
/// 而且**必须真的在**才给。
///
/// 不在就如实拒绝并附上期望路径（与 `wb_reveal_asset` 的 `reveal_target` 同一条：
/// 不去猜、也不打开一个不存在的东西）。两种都不在时口径仍然是"这个位置里还没有它"——
/// 名字里那个 `<基名>.zip` 由调用方按需拼（这里只判在不在）。
fn export_target(parent: &Path, base: &str) -> Result<PathBuf, AppError> {
    let dir = parent.join(base);
    if dir.is_dir() {
        return Ok(dir);
    }
    let zipped = parent.join(format!("{base}.zip"));
    if zipped.is_file() {
        return Ok(zipped);
    }
    Err(
        AppError::not_found(messages::text("ui.ipc.report.145", &[("base", base)])).with_detail(
            ::postprocess::diag::messages::text(
                "ui.ipc.report.why_expected_path",
                &[("path", &dir.display().to_string())],
            ),
        ),
    )
}

/// **在系统文件管理器里定位归档里的那一份**（导出框信息页那颗「定位」，2026-11-03）。
///
/// 作者的原话是「不导出直接定位这个文件」—— 那一页要回答的是"这些文件此刻在盘上
/// 哪儿"，所以开的是**归档里那一份**，不是导出去的副本（那时可能还没导出过）。
///
/// 闸门与 [`reveal_report_export`] 同一条：
///
/// - 前端只给 `id + kind`，路径由后端自己算（不许拿一个路径来"打开它"）；
/// - 那一份必须**真的在**（失败的那一次没有 `<id>.gcode`）—— 不在就如实拒绝并附期望路径；
/// - 只读、只开系统文件管理器、一个应用状态都不碰。
#[tauri::command]
pub async fn reveal_report_source(
    app: AppHandle,
    id: String,
    kind: String,
) -> Result<(), AppError> {
    traced("revealReportSource", |_| {
        check_id(&id)?;
        let suffix = export_suffix(&kind).ok_or_else(|| {
            AppError::invalid_argument(messages::text("ui.ipc.report.146", &[("kind", &kind)]))
        })?;
        let root = history_root(&app)?;
        let (day_dir, _meta_path) = record_paths(&root, &id)?;
        let target = day_dir.join(format!("{id}{suffix}"));
        if !target.is_file() {
            return Err(AppError::not_found(messages::text(
                "ui.ipc.report.147",
                &[("name", &target.display().to_string())],
            ))
            .with_detail(::postprocess::diag::messages::text(
                "ui.ipc.report.why_expected_path",
                &[("path", &target.display().to_string())],
            )));
        }
        tauri_plugin_opener::OpenerExt::opener(&app)
            .reveal_item_in_dir(&target)
            .map_err(|e| {
                AppError::io(messages::text("ui.ipc.report.144", &[])).with_detail(e.to_string())
            })
    })
}

/// `kind`（机器名）→ 归档里的后缀。只认 [`EXPORT_PARTS`] 那三个。
fn export_suffix(kind: &str) -> Option<&'static str> {
    EXPORT_PARTS
        .iter()
        .find(|(_, k)| *k == kind)
        .map(|(suffix, _)| *suffix)
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

    /// 新记录（`errorKey` = 原因键）：那句结论行按**读的那一刻的语言**重说，码不许丢。
    ///
    /// 2026-10-31 那条真机记录：钩子以简体写账，日语界面上那句一直是中文。
    /// 修法在写侧（`hook::archive_failure` 存原因键 + 参数）与本函数（重建结论行）。
    #[test]
    fn an_archived_stop_is_restated_in_the_readers_language() {
        let meta = serde_json::json!({
            "error": "已停止：这一盘没有做后处理，原文件没有被改动［W_SYS_CANCELLED_001］",
            "errorKey": "hook.cancelled",
            "cancelled": true
        });
        let zh = localized_error_in(messages::Lang::Zh, &meta).unwrap();
        assert!(zh.contains("已停止："), "{zh}");
        assert!(zh.contains("W_SYS_CANCELLED_001"), "{zh}");

        let ja = localized_error_in(messages::Lang::Ja, &meta).unwrap();
        assert!(ja.contains("停止しました"), "日语界面要说日语：{ja}");
        assert!(ja.contains("W_SYS_CANCELLED_001"), "码是契约，不许丢：{ja}");
        assert!(!ja.contains("这一盘"), "日语界面不许再冒出中文：{ja}");

        let en = localized_error_in(messages::Lang::En, &meta).unwrap();
        assert!(en.contains("Stopped:"), "{en}");
    }

    /// 带占位符的那几条：值原样进新语言，句子按那门语言重新拼。
    #[test]
    fn a_keyed_failure_keeps_its_placeholders_in_the_new_language() {
        let meta = serde_json::json!({
            "error": "已停下（机型不匹配：G-code 是 A1_MINI，预设是 A1）—— 原文件没有被改动［W_CFG_MISMATCH_001］",
            "errorKey": "hook.mismatch_stopped",
            "errorArgs": { "gcode": "A1_MINI", "preset": "A1" },
            "cancelled": true
        });
        let ja = localized_error_in(messages::Lang::Ja, &meta).unwrap();
        assert!(ja.contains("A1_MINI") && ja.contains("A1"), "{ja}");
        assert!(ja.contains("W_CFG_MISMATCH_001"), "{ja}");
        assert!(!ja.contains("机型不匹配"), "句子该整句换语言：{ja}");
    }

    /// 老记录没有 `errorKey`：**不猜**，回落原句（调用方 `.or_else(|| str_at("error"))`）。
    #[test]
    fn an_old_record_without_a_key_is_not_guessed() {
        let meta = serde_json::json!({
            "error": "处理失败 [E_GCODE_BOUNDARY_001]：模型超出打印边界"
        });
        assert!(localized_error_in(messages::Lang::Ja, &meta).is_none());
    }

    /// `errorKey` 本身就是一整句的键（`hook.line.*`）⇒ 直接取，不许再包一层框子。
    #[test]
    fn a_line_key_is_not_wrapped_twice() {
        let meta = serde_json::json!({
            "errorKey": "hook.line.stopped",
            "errorArgs": { "reason": "x", "code": "W_SYS_CANCELLED_001" },
            "cancelled": true
        });
        let ja = localized_error_in(messages::Lang::Ja, &meta).unwrap();
        assert_eq!(ja.matches("停止しました").count(), 1, "{ja}");
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

    /// 跑失败的那一次：归档里**没有** `<id>.gcode` —— 计划里三份照列，那一份 `exists=false`。
    ///
    /// 2026-11-04 改口径：原来缺的那份整行不出现（`parts.len() == 2`），界面上就得靠
    /// 底下一句话解释"少的是哪一份"。现在恒报三份、缺的由 `exists` 说明。
    #[test]
    fn a_failed_run_marks_only_the_processed_file_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let day = dir.path().join("2026-10-11");

        record(dir.path(), "ok_20261011_142431", None, true);
        let parts = parts_of(&day, "ok_20261011_142431", "ok_20261011_142431");
        assert_eq!(parts.len(), 3);
        let files = files_of(&parts, &day, "ok_20261011_142431", "ok_20261011_142431");
        assert_eq!(files.len(), 3);
        assert!(files.iter().all(|f| f.exists), "三份都在盘上");
        assert_eq!(files[1].name, "ok_20261011_142431.gcode");

        let failed = "63288.1_20261011_142431";
        record(dir.path(), failed, Some("帕姆"), false);
        let parts = parts_of(&day, failed, "帕姆_20261011142431");
        assert_eq!(parts.len(), 2);
        let files = files_of(&parts, &day, failed, "帕姆_20261011142431");
        assert_eq!(files.len(), 3, "计划恒报三份");
        assert_eq!(
            files.iter().map(|f| (f.kind, f.exists)).collect::<Vec<_>>(),
            vec![("original", true), ("processed", false), ("meta", true)],
        );
        assert_eq!(files[0].name, "帕姆_20261011142431_original.gcode");
        assert_eq!(files[1].name, "帕姆_20261011142431.gcode");
        assert_eq!(files[2].name, "帕姆_20261011142431_meta.json");
        /* 缺的那份也带着"它本该在哪儿"—— 信息页那一行要能复制这串路径 */
        assert!(files[1].path.ends_with("63288.1_20261011_142431.gcode"));
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
        let written = copy_parts(&day, &parts, &target, false).unwrap();

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

        let err = copy_parts(&day, &parts, &target, false).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert!(err.message.contains("已经有"), "{}", err.message);
        /* 界面按这个码摆「重新导出」（不认句子：翻译一改就失效） */
        assert_eq!(err.message_code.as_deref(), Some(EXPORT_EXISTS_CODE));
        /* 那一份没被动过，别的也一份都没落（先过一遍地雷再写） */
        assert_eq!(
            std::fs::read_to_string(target.join("帕姆_20261011142431.gcode")).unwrap(),
            "用户自己改过的"
        );
        assert!(!target.join("帕姆_20261011142431_meta.json").exists());
    }

    /// 点了「重新导出」（`overwrite = true`）⇒ 盖掉，且**只有那三份**被动过：
    /// 目录里别的东西（用户自己放的）一个不动，半成品临时件也不留。
    #[test]
    fn export_overwrites_only_the_three_names_when_asked() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let day = src.path().join("2026-10-11");
        let id = "63288.1_20261011_142431";
        record(src.path(), id, Some("帕姆"), true);

        let parts = parts_of(&day, id, "帕姆_20261011142431");
        let target = out.path().join("帕姆_20261011142431");
        std::fs::create_dir_all(&target).unwrap();
        std::fs::write(target.join("帕姆_20261011142431.gcode"), "旧的那一份").unwrap();
        std::fs::write(target.join("用户自己的笔记.txt"), "别动我").unwrap();

        let written = copy_parts(&day, &parts, &target, true).unwrap();
        assert_eq!(written.len(), 3);
        assert_eq!(
            std::fs::read_to_string(target.join("帕姆_20261011142431.gcode")).unwrap(),
            "; 处理后"
        );
        /* 目录里别的东西没被卷进来 */
        assert_eq!(
            std::fs::read_to_string(target.join("用户自己的笔记.txt")).unwrap(),
            "别动我"
        );
        /* 临时件一个不剩 */
        let leftovers = std::fs::read_dir(&target)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|e| e.file_name().to_string_lossy().ends_with(".partial"))
            .count();
        assert_eq!(leftovers, 0);
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
        let err = copy_parts(&day, &parts, &day, false).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert_eq!(
            std::fs::read_to_string(day.join(format!("{id}.gcode"))).unwrap(),
            "; 处理后"
        );
    }

    /// 「打开位置」只认**真在的那个导出落点**：还没导出过就如实拒绝（附期望路径），
    /// 同名的是一个**文件**也拒绝 —— 交给系统的那条路径是"打开它"，
    /// 而一个文件会被**启动**（`.exe` 尤其），那不是用户点「打开位置」的意思。
    ///
    /// ★ 2026-11-03：压缩导出之后落点是一个 `<基名>.zip`（不是目录），它照样认。
    #[test]
    fn reveal_only_opens_a_real_export_target() {
        let out = tempfile::tempdir().unwrap();
        let base = "帕姆_20261011142431";

        let err = export_target(out.path(), base).unwrap_err();
        assert_eq!(err.code, crate::error::ErrorCode::NotFound);
        assert!(err.message.contains(base), "{}", err.message);

        let target = out.path().join(base);
        std::fs::create_dir_all(&target).unwrap();
        assert_eq!(export_target(out.path(), base).unwrap(), target);

        /* 压成一包：落点就是那个 zip 文件（选中它，而不是把它解压成一个新目录） */
        let zip_base = "帕姆_zip_20261011142431";
        let zipped = out.path().join(format!("{zip_base}.zip"));
        std::fs::write(&zipped, "PK").unwrap();
        assert_eq!(export_target(out.path(), zip_base).unwrap(), zipped);

        std::fs::write(out.path().join("帕姆.exe"), "x").unwrap();
        assert!(export_target(out.path(), "帕姆.exe").is_err(), "文件不给");
    }

    /// 压缩导出：**一个** `<基名>.zip`，里面就是那三份（名字与散着导出去的一样），
    /// 归档里那几份一个字节不动。
    #[test]
    fn export_zipped_writes_one_package_with_the_three_files() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let day = src.path().join("2026-10-11");
        let id = "63288.1_20261011_142431";
        let base = "帕姆_20261011142431";
        record(src.path(), id, Some("帕姆"), true);

        let parts = parts_of(&day, id, base);
        let written = export_zip(&day, &parts, out.path(), base, false).unwrap();

        assert_eq!(written.len(), 1, "压缩就是一个包：{written:?}");
        assert_eq!(
            written[0],
            out.path().join(format!("{base}.zip")).display().to_string()
        );

        let file = std::fs::File::open(&written[0]).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        assert_eq!(archive.len(), 3);
        let mut names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                format!("{base}.gcode"),
                format!("{base}_meta.json"),
                format!("{base}_original.gcode"),
            ]
        );

        let mut packed = String::new();
        {
            use std::io::Read as _;
            archive
                .by_name(&format!("{base}.gcode"))
                .unwrap()
                .read_to_string(&mut packed)
                .unwrap();
        }
        assert_eq!(packed, "; 处理后", "包里的字节必须与归档一致");

        /* 原件毫发无损（压缩只是读） */
        assert_eq!(
            std::fs::read_to_string(day.join(format!("{id}.gcode"))).unwrap(),
            "; 处理后"
        );
    }

    /// 压缩导出**不许覆盖**已有的那个包（与散着导同一道闸：静默盖掉等于删用户数据）。
    #[test]
    fn a_zipped_export_refuses_to_overwrite() {
        let src = tempfile::tempdir().unwrap();
        let out = tempfile::tempdir().unwrap();
        let day = src.path().join("2026-10-11");
        let id = "63288.1_20261011_142431";
        let base = "帕姆_20261011142431";
        record(src.path(), id, Some("帕姆"), true);

        let already = out.path().join(format!("{base}.zip"));
        std::fs::write(&already, "用户自己的包").unwrap();

        let parts = parts_of(&day, id, base);
        let err = export_zip(&day, &parts, out.path(), base, false).unwrap_err();
        /* 撞名这一档也带码（界面据此摆「重新导出」） */
        assert_eq!(err.message_code.as_deref(), Some(EXPORT_EXISTS_CODE));
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert!(err.message.contains("已经"), "{}", err.message);
        assert_eq!(
            std::fs::read_to_string(&already).unwrap(),
            "用户自己的包",
            "那一份不许被动过"
        );
    }

    /// 定位只认三件套的机器名（`original` / `processed` / `meta`）——
    /// 别的串一律在门口拦掉，不拿它去拼路径。
    #[test]
    fn export_suffix_only_knows_the_three_parts() {
        assert_eq!(export_suffix("original"), Some("_original.gcode"));
        assert_eq!(export_suffix("processed"), Some(".gcode"));
        assert_eq!(export_suffix("meta"), Some("_meta.json"));
        for bad in ["", "..", "processed/../../x", "ORIGINAL"] {
            assert_eq!(export_suffix(bad), None, "{bad:?} 该被拦下");
        }
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
