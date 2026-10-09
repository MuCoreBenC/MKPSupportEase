//! 后处理执行归档 —— 「原文件 + 输出文件 + 执行记录」三件套。
//!
//! # 账落在哪
//!
//! ```text
//! <用户根>/gcode_history/<YYYY-MM-DD>/<名>_<YYYYMMDD_HHMMSS>_original.gcode
//! <用户根>/gcode_history/<YYYY-MM-DD>/<名>_<YYYYMMDD_HHMMSS>.gcode
//! <用户根>/gcode_history/<YYYY-MM-DD>/<名>_<YYYYMMDD_HHMMSS>_meta.json
//! ```
//!
//! 目录形状照成熟版（`mkpsupporte` 的 `prepareBackupBasePath`）：日期子目录 +
//! `<原名去扩展名、去前导点>_<时刻>` 前缀。前导点必须去 —— Bambu Studio 的缓存
//! 文件名以 `.` 开头（`.63288.1.gcode`），带点开头在 macOS 上不可见。
//!
//! # 三条硬规矩
//!
//! 1. **原件先备份**（[`begin`]）：本程序是原地覆盖（`.part` + rename），这一笔
//!    落在覆盖之前 —— 没有它，成功之后原件就没了。
//! 2. **失败与取消也留记录**：都走同一个 [`write_meta`]，差别只在 `error` /
//!    `cancelled` 字段；报告页把它们照进列表（`ok = false`），不许瞒。
//! 3. **归档失败不拖垮后处理**：产物优先。调用方（`hook::run`）把这里的每个错误
//!    降级成一条警告，不改变退出码。
//!
//! # 与报告页的契约
//!
//! `_meta.json` 的字段口径与 [`crate::ipc::report`] 的读口、`PageReport.tsx` 的
//! 消费面**逐键对齐**（`startedAt` / `elapsedMs` / `pipeline[]{id,message,elapsedMs}` /
//! `detail.stats` / `detail.printTime` …）。改字段名等于撕一份已经在用的契约 ——
//! 前端那一屏显示什么，这份文件里就得有什么。

use std::path::{Path, PathBuf};

use sha2::Digest;
use time::OffsetDateTime;

use crate::error::AppError;

/// 一次归档的三件套落点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivePaths {
    /// 处理前的原文件副本（[`begin`] 写）。
    pub original: PathBuf,
    /// 处理后的输出副本（[`archive_output`] 写）。
    pub output: PathBuf,
    /// 执行记录（[`write_meta`] 写）。
    pub meta: PathBuf,
}

/// 归档根：`<用户根>/gcode_history`。`None` = 这台机器上没有数据根
/// （与 `internal_root_headless` 同一套判据、同一条降级路）。
pub fn history_root() -> Option<PathBuf> {
    Some(crate::fsx::paths::user_root_headless()?.join(crate::fsx::paths::GCODE_HISTORY_DIR))
}

/// 现在的本地时间；解析不到时区就回落 UTC —— 归档不该因为时区解析失败而停。
pub fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}

/// 日期目录名（`YYYY-MM-DD`；成熟版是 `2006-01-02`，同一个形状）。
pub fn day_dir(now: OffsetDateTime) -> String {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    now.format(fmt).unwrap_or_default()
}

/// 文件名时刻（`YYYYMMDD_HHMMSS`；成熟版是 `20060102_150405`）。
pub fn file_stamp(now: OffsetDateTime) -> String {
    let fmt = time::macros::format_description!("[year][month][day]_[hour][minute][second]");
    now.format(fmt).unwrap_or_default()
}

/// 人看的时刻（`YYYY-MM-DD HH:MM:SS`）—— `startedAt`/`finishedAt` 用它，
/// 报告页的行首时间就是它（字典序 = 时间序，报告页排序依赖这一点）。
pub fn display_stamp(now: OffsetDateTime) -> String {
    let fmt = time::macros::format_description!("[year]-[month]-[day] [hour]:[minute]:[second]");
    now.format(fmt).unwrap_or_default()
}

/// 三件套的公共前缀（**纯函数**，判据直接咬它）：
/// `<history_root>/<YYYY-MM-DD>/<剔点原名>_<YYYYMMDD_HHMMSS>`。
pub fn base_path(history_root: &Path, input: &Path, now: OffsetDateTime) -> PathBuf {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let stem = stem.trim_start_matches('.');
    history_root
        .join(day_dir(now))
        .join(format!("{stem}_{}", file_stamp(now)))
}

fn with_suffix(base: &Path, suffix: &str) -> PathBuf {
    let mut s = base.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

/// 处理前：建齐目录并把**原件**写进归档。
///
/// 吃调用方已经读进内存的原文，不再读一次盘 —— 钩子读输入文件是管线第 1 步，
/// 原文在手上，再 `read` 一遍纯属浪费。
pub fn begin(
    history_root: &Path,
    input: &Path,
    raw: &str,
    now: OffsetDateTime,
) -> Result<ArchivePaths, AppError> {
    let base = base_path(history_root, input, now);
    let paths = ArchivePaths {
        original: with_suffix(&base, "_original.gcode"),
        output: with_suffix(&base, ".gcode"),
        meta: with_suffix(&base, "_meta.json"),
    };
    if let Some(dir) = paths.original.parent() {
        std::fs::create_dir_all(dir).map_err(|e| {
            AppError::io(format!("建不出归档目录：{}", dir.display())).with_detail(e.to_string())
        })?;
    }
    crate::fsx::atomic::atomic_write(&paths.original, raw.as_bytes())?;
    Ok(paths)
}

/// 成功后：把输出文件复制进归档。
///
/// 这里是 `fs::copy` 而不是"原子写"：源文件本身就是刚刚由内核原子落盘的那一份，
/// 复制过程再被打断，最多留下一个半截的 `archived .gcode` —— 而它旁边就是原件与
/// 记录，用户的信息一个字都没丢（关键的原子性在原件那一笔上）。
pub fn archive_output(paths: &ArchivePaths, output: &Path) -> Result<(), AppError> {
    std::fs::copy(output, &paths.output).map_err(|e| {
        AppError::io(format!("复制输出到归档失败：{}", paths.output.display()))
            .with_detail(e.to_string())
    })?;
    Ok(())
}

/// 写执行记录（成功 / 失败 / 取消都走它）。
pub fn write_meta(paths: &ArchivePaths, meta: &Meta) -> Result<(), AppError> {
    crate::fsx::atomic::atomic_write_json(&paths.meta, meta)
}

/// 一份文件的事实（`detail.inputFile` / `detail.outputFile`）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFact {
    pub path: String,
    pub size_bytes: u64,
    /// 行数：只有输入有（输出文件的行数没有现成来源，不新算一遍）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lines: Option<u64>,
    pub sha256: String,
}

/// 调用现场（exe + argv）—— 报告页「调用」那一行显示的东西，
/// 也是排查「这份是谁跑的」的唯一线索。
#[derive(Debug, Clone, serde::Serialize)]
pub struct Invocation {
    pub exe: String,
    pub args: Vec<String>,
}

/// 当前进程的调用现场。
pub fn invocation() -> Invocation {
    Invocation {
        exe: std::env::current_exe()
            .map(|p| p.display().to_string())
            .unwrap_or_default(),
        args: std::env::args().skip(1).collect(),
    }
}

/// 一条管线步骤的账。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PipelineStep {
    /// 稳定步骤 id（`input` / `pass1` / …；中文显示名只住 UI）。
    pub id: &'static str,
    pub message: String,
    pub elapsed_ms: u64,
    pub status: PipelineStatus,
}

/// 步骤状态：成功跑完 / 在这一步失败 / 在这一步被取消。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PipelineStatus {
    Complete,
    Error,
    Cancelled,
}

impl PipelineStep {
    /// 由内核的逐步耗时（[`postprocess::pipeline::StepTiming`]）构账。
    pub fn from_timing(t: &postprocess::pipeline::StepTiming, status: PipelineStatus) -> Self {
        Self {
            id: t.step.id(),
            message: t.message.clone(),
            elapsed_ms: t.elapsed_ms,
            status,
        }
    }
}

/// 文件事实 + 统计 + 打印时间（报告页详情区的那几块）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub input_file: FileFact,
    /// 失败 / 取消时没有输出，这一项缺键（前端 `??` 兜底显示 "—"）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_file: Option<FileFact>,
    /// 切片统计（`ProcessStats` 原样序列化：`towerHeight` / `glueLayerCount` /
    /// `totalLayerNumber` / `maxZHeight` / `originalPrintTime` …）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<serde_json::Value>,
    /// 打印时间估算（`printtime::Result` 原样序列化）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_time: Option<serde_json::Value>,
}

/// 一份执行记录（`_meta.json`）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    /// schema 版本：与旧世代（`mkp-ssr` 的 `history.rs`）同口径的 v2。
    pub schema_version: u32,
    pub started_at: String,
    pub finished_at: String,
    pub elapsed_ms: u64,
    pub preset_name: String,
    pub machine_type: String,
    pub preset_path: String,
    pub gcode_path: String,
    pub invocation: Invocation,
    pub warnings: Vec<String>,
    /// 成功 ⇒ `None`（落 `null`，不是缺键：报告页按"有没有 error"判 `ok`）。
    pub error: Option<String>,
    pub cancelled: bool,
    pub pipeline: Vec<PipelineStep>,
    pub detail: Detail,
}

impl Meta {
    pub const SCHEMA_VERSION: u32 = 2;
}

/// 读一份文件的 sha256（输入 / 输出的事实）。
pub fn sha256_of(path: &Path) -> Result<String, AppError> {
    let mut file = std::fs::File::open(path).map_err(|e| {
        AppError::io(format!("读不到文件：{}", path.display())).with_detail(e.to_string())
    })?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hasher).map_err(|e| {
        AppError::io(format!("读文件失败：{}", path.display())).with_detail(e.to_string())
    })?;
    Ok(format!("{:x}", hasher.finalize()))
}

/// 组装一份文件事实（大小从盘上取，行数与 sha 由调用方给 —— 它们已经在手上）。
///
/// ★ **只适用于"盘上那份就是要写的那份"**（即输出文件）。输入文件**不能**用它 ——
/// 见 [`input_file_fact`]。
pub fn file_fact(path: &Path, lines: Option<u64>, sha256: String) -> Result<FileFact, AppError> {
    let size_bytes = std::fs::metadata(path)
        .map_err(|e| {
            AppError::io(format!("读不到文件事实：{}", path.display())).with_detail(e.to_string())
        })?
        .len();
    Ok(FileFact {
        path: path.display().to_string(),
        size_bytes,
        lines,
        sha256,
    })
}

/// 组装**输入侧**的文件事实：大小**必须由调用方给**（原文字节数），不从盘上读。
///
/// 为什么不能 [`file_fact`] 那样从盘上取：本程序是**原地覆盖**，`_meta.json` 在写盘
/// **之后**才组装 —— 那一刻 `job.gcode` 里已经是**输出**了，从盘上量出来的会是
/// 输出的大小（实测踩过：输入被记成 26,877,295 = 输出大小，而真实输入是 23.76 MB）。
pub fn input_file_fact(
    path: &Path,
    size_bytes: u64,
    lines: Option<u64>,
    sha256: String,
) -> FileFact {
    FileFact {
        path: path.display().to_string(),
        size_bytes,
        lines,
        sha256,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    /// 输入侧的大小必须来自调用方（原文），**不是**盘上的现文件。
    /// 回归：曾经因为原地覆盖，把输入大小记成了输出大小。
    #[test]
    fn input_file_size_comes_from_the_caller_not_the_disk() {
        let d = tempfile::tempdir().unwrap();
        let input = d.path().join("in.gcode");
        std::fs::write(&input, "G1 X1\n").unwrap();

        // 模拟"原地覆盖"：同一个路径上的内容被换成了更长的一份
        std::fs::write(&input, "G1 X1\nG1 X2\nG1 X3\n").unwrap();
        let fact = input_file_fact(&input, 6, Some(1), "sha".into());

        assert_eq!(fact.size_bytes, 6, "大小必须是调用方给的原文字节数");
        assert_eq!(fact.lines, Some(1));
        assert_eq!(fact.path, input.display().to_string());
        // 对照：从盘上取会拿到"覆盖后"的 18 字节 —— 这正是 bug 的形态
        assert_eq!(
            file_fact(&input, None, String::new()).unwrap().size_bytes,
            18,
            "file_fact 读的是盘上的现文件（输出侧才该这么做）"
        );
    }

    /// 命名规则：**前导点必须去掉**（Bambu Studio 的缓存文件名以 `.` 开头，
    /// 带点在 macOS 上不可见），目录用日期、前缀带时刻。
    #[test]
    fn base_path_strips_leading_dot_and_uses_day_dir() {
        let root = Path::new("/tmp/hist");
        let now = datetime!(2026-10-09 22:08:51 +8);
        let base = base_path(root, Path::new("C:/tmp/Metadata/.63288.1.gcode"), now);
        assert_eq!(
            base,
            root.join("2026-10-09").join("63288.1_20261009_220851"),
            "目录 = 日期，文件名 = 剔点原名 + 下划线 + 时刻"
        );
    }

    /// 三件套的落点：原文件 / 输出 / 记录，都挂在同一个前缀上。
    #[test]
    fn begin_writes_original_and_returns_three_paths() {
        let d = tempfile::tempdir().unwrap();
        let now = datetime!(2026-10-09 22:08:51 +8);
        let paths = begin(
            d.path(),
            Path::new("/tmp/input/.63288.1.gcode"),
            "G1 X1\n",
            now,
        )
        .expect("归档原件");

        assert!(paths.original.ends_with("63288.1_20261009_220851_original.gcode"));
        assert!(paths.output.ends_with("63288.1_20261009_220851.gcode"));
        assert!(paths.meta.ends_with("63288.1_20261009_220851_meta.json"));
        assert_eq!(
            std::fs::read_to_string(&paths.original).unwrap(),
            "G1 X1\n",
            "原件内容 = 交给它的原文，一个字节不差"
        );
    }

    /// `_meta.json` 的键**咬住**报告页读口与前端消费面（camelCase）。
    /// 改键名等于撕契约 —— 这条判据就是拦这个的。
    #[test]
    fn meta_json_keeps_the_keys_the_report_page_reads() {
        let meta = Meta {
            schema_version: Meta::SCHEMA_VERSION,
            started_at: "2026-10-09 22:08:51".into(),
            finished_at: "2026-10-09 22:08:53".into(),
            elapsed_ms: 2530,
            preset_name: "A1-fastv3.3.toml".into(),
            machine_type: "A1".into(),
            preset_path: "/p/A1-fastv3.3.toml".into(),
            gcode_path: "/tmp/.63288.1.gcode".into(),
            invocation: Invocation {
                exe: "SupportEase.exe".into(),
                args: vec!["--Gcode".into()],
            },
            warnings: vec!["这一盘没有支撑".into()],
            error: None,
            cancelled: false,
            pipeline: vec![PipelineStep {
                id: "input",
                message: "正在读取文件...".into(),
                elapsed_ms: 96,
                status: PipelineStatus::Complete,
            }],
            detail: Detail {
                input_file: FileFact {
                    path: "/tmp/.63288.1.gcode".into(),
                    size_bytes: 23_758_782,
                    lines: Some(909_465),
                    sha256: "abc".into(),
                },
                output_file: None,
                stats: Some(serde_json::json!({ "glueLayerCount": 124 })),
                print_time: None,
            },
        };
        let v = serde_json::to_value(&meta).unwrap();
        for key in [
            "schemaVersion",
            "startedAt",
            "finishedAt",
            "elapsedMs",
            "presetName",
            "machineType",
            "presetPath",
            "gcodePath",
            "invocation",
            "warnings",
            "error",
            "cancelled",
            "pipeline",
            "detail",
        ] {
            assert!(v.get(key).is_some(), "缺键：{key}");
        }
        assert_eq!(v["pipeline"][0]["id"], "input");
        assert_eq!(v["pipeline"][0]["elapsedMs"], 96);
        assert_eq!(v["pipeline"][0]["status"], "complete");
        assert!(v["error"].is_null(), "成功记录落 null，不是缺键");
        assert_eq!(v["detail"]["inputFile"]["lines"], 909_465);
        assert_eq!(v["detail"]["inputFile"]["sizeBytes"], 23_758_782);
    }

    /// 时刻格式化：三个口径（日期目录 / 文件名 / 人看）互不串味。
    #[test]
    fn stamps_are_stable() {
        let now = datetime!(2026-10-09 22:08:51 +8);
        assert_eq!(day_dir(now), "2026-10-09");
        assert_eq!(file_stamp(now), "20261009_220851");
        assert_eq!(display_stamp(now), "2026-10-09 22:08:51");
    }

    /// 记录写盘后能被读回来（原子写的闭环）。
    #[test]
    fn write_meta_round_trips() {
        let d = tempfile::tempdir().unwrap();
        let now = datetime!(2026-10-09 22:08:51 +8);
        let paths = begin(d.path(), Path::new("/tmp/x.gcode"), "G1\n", now).unwrap();
        let meta = Meta {
            schema_version: Meta::SCHEMA_VERSION,
            started_at: display_stamp(now),
            finished_at: display_stamp(now),
            elapsed_ms: 1,
            preset_name: String::new(),
            machine_type: String::new(),
            preset_path: String::new(),
            gcode_path: "/tmp/x.gcode".into(),
            invocation: invocation(),
            warnings: Vec::new(),
            error: Some("E_GCODE_BOUNDARY_001: 越界".into()),
            cancelled: false,
            pipeline: Vec::new(),
            detail: Detail {
                input_file: FileFact {
                    path: "/tmp/x.gcode".into(),
                    size_bytes: 3,
                    lines: Some(1),
                    sha256: "z".into(),
                },
                output_file: None,
                stats: None,
                print_time: None,
            },
        };
        write_meta(&paths, &meta).expect("写记录");
        let back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.meta).unwrap()).unwrap();
        assert_eq!(back["elapsedMs"], 1);
        assert_eq!(back["error"], "E_GCODE_BOUNDARY_001: 越界");
        assert!(back["detail"].get("outputFile").is_none(), "失败记录没有输出件");
    }
}
