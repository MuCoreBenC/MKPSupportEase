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

use postprocess::pipeline::Step;
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

/// 三件套的文件名后缀 —— 字面量**只许有这一处**（[`begin`] 拼它，
/// 自动清理 [`crate::retention`] 按它把散文件归回同一条记录）。
pub const ORIGINAL_SUFFIX: &str = "_original.gcode";
/// 见 [`ORIGINAL_SUFFIX`]。判据顺序有讲究：`_original.gcode` 也以 `.gcode` 结尾，
/// 所以要**先长的后短的**（[`record_base`]）。
pub const OUTPUT_SUFFIX: &str = ".gcode";
/// 见 [`ORIGINAL_SUFFIX`]。
pub const META_SUFFIX: &str = "_meta.json";

/// 三件套里任意一个文件名 → 这条记录的公共前缀（基名）。不是三件套的名字 ⇒ `None`。
///
/// 自动清理拿它把同一条记录的几个文件**归回一组**（删就整条删）——
/// 与 [`base_path`] 拼出来的是同一份后缀，不是第二处自拼的字符串。
pub fn record_base(file_name: &str) -> Option<&str> {
    if let Some(base) = file_name.strip_suffix(ORIGINAL_SUFFIX) {
        return Some(base);
    }
    if let Some(base) = file_name.strip_suffix(META_SUFFIX) {
        return Some(base);
    }
    file_name.strip_suffix(OUTPUT_SUFFIX)
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
        original: with_suffix(&base, ORIGINAL_SUFFIX),
        output: with_suffix(&base, OUTPUT_SUFFIX),
        meta: with_suffix(&base, META_SUFFIX),
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

/// **最近一次处理用的是哪个切片器进程**（2026-10-20）—— 引导层认"我刚才切的那一份"
/// 的取数口（见 [`Meta::slicer_pid`]）。
///
/// # 怎么找"最近"
///
/// - 日期目录名是 `YYYY-MM-DD`，**字典序 = 时间序** ⇒ 倒着排即可；
/// - 同一天里按文件名倒序（名字尾巴是 `YYYYMMDD_HHMMSS`，同样是字典序）；
/// - ★ **只认 `accept` 认下的 pid**：进程号会失效（切片器重启 / 关掉），
///   旧记录里那个号可能已经不属于任何切片器了 —— 判"它还是不是切片器"是**调用方**
///   的事（那半在 `crate::ax`），这里只负责按时间倒序把它递过去。
///   ⇒ 于是"最近那条记录的号已经死了"时会自然落到**上一条还活着**的记录上。
///
/// # 代价
///
/// 只翻最近 [`SCAN_LIMIT`] 份记录（再多就没有意义了：越旧越不可能还在跑），
/// 每份只读一个 JSON 的 `slicer_pid` 一格。
pub fn latest_slicer_pid(history_root: &Path, accept: impl Fn(i32) -> bool) -> Option<i32> {
    let mut days: Vec<_> = std::fs::read_dir(history_root)
        .ok()?
        .flatten()
        .filter(|d| d.path().is_dir())
        .map(|d| d.file_name().to_string_lossy().into_owned())
        .collect();
    days.sort_unstable();
    days.reverse();

    let mut seen = 0usize;
    for day in days {
        let mut metas: Vec<_> = std::fs::read_dir(history_root.join(&day))
            .ok()?
            .flatten()
            .filter(|f| f.file_name().to_string_lossy().ends_with(META_SUFFIX))
            .map(|f| f.file_name().to_string_lossy().into_owned())
            .collect();
        metas.sort_unstable();
        metas.reverse();
        for name in metas {
            if seen >= SCAN_LIMIT {
                return None;
            }
            seen += 1;
            let text = std::fs::read_to_string(history_root.join(&day).join(&name)).ok()?;
            let pid = serde_json::from_str::<serde_json::Value>(&text)
                .ok()?
                .get("slicerPid")
                .and_then(|v| v.as_i64())
                .and_then(|v| i32::try_from(v).ok());
            if let Some(pid) = pid.filter(|p| accept(*p)) {
                return Some(pid);
            }
        }
    }
    None
}

/// 认"最近那份"时最多翻几条记录（见 [`latest_slicer_pid`]）。
const SCAN_LIMIT: usize = 30;

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
    /// 这一步自报的多行事实（照旧版 `pipeline[].detail` 的形状：`\n` 分行，
    /// 报告页在步骤名下铺小字）。没有素材的步不写这个键。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
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
    pub fn from_timing(
        t: &postprocess::pipeline::StepTiming,
        status: PipelineStatus,
        detail: Option<String>,
    ) -> Self {
        Self {
            id: t.step.id(),
            message: t.message.clone(),
            elapsed_ms: t.elapsed_ms,
            status,
            detail,
        }
    }
}

/// 打印时间估算的三种状态（写进记录的 `printTimeStatus`）。
///
/// 打印时间是**纯报告数据**（内核零读取），所以可以延后到常驻进程去算：
/// 钩子先落一条 `computing` 就退出，算完再由那一侧补成 `ready`。
pub const PRINT_TIME_COMPUTING: &str = "computing";
pub const PRINT_TIME_READY: &str = "ready";
pub const PRINT_TIME_FAILED: &str = "failed";

/// 涂胶组（`detail.glue`）—— 键名照旧版 `_meta.json` 的 `detail` 同名搬
/// （`glueTotalLength` / `penOffsetX` …），报告页按这些键取值。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GlueFacts {
    pub glue_total_length: f64,
    pub glue_pen_switches: i64,
    pub glue_pass_count: i64,
    /// mm/s（= 内核 `toolhead.max_speed` ÷ 60，全仓唯一的逆换算点）。
    pub glue_speed: f64,
    pub pen_offset_x: f64,
    pub pen_offset_y: f64,
    pub pen_offset_z: f64,
    pub glue_z_lift_height: f64,
    pub glue_z_lift_height_first_layers: f64,
    pub glue_inter_model_lift_height: f64,
    pub glue_z_offset: f64,
    pub prime_trigger_layers: i64,
    pub prime_total_length: f64,
}

/// 擦料塔组（`detail.tower`）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TowerFacts {
    pub tower_layers: i64,
    pub tower_height: f64,
    pub tower_extrude_ratio: f64,
    pub tower_z_lift: f64,
    pub tower_safe_z_offset: f64,
    pub tower_travel_speed: f64,
    pub fast_tower_mode: String,
}

/// 切片工艺组（`detail.slicer`）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlicerFacts {
    pub slicer_version: String,
    pub machine_preset_name: String,
    pub process_preset_name: String,
    pub filament_type: String,
    pub nozzle_diameter: f64,
    pub slicer_ironing_enabled: bool,
    pub ironing_mode: String,
    pub ironing_threshold: f64,
    pub tree_support: bool,
    pub use_disk_wipe: bool,
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
    /// 打印时间估算（`printtime::Result` 原样序列化）。延后时先为 `None`，
    /// 由 [`complete_meta`] 补全。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_time: Option<serde_json::Value>,
    /// 打印时间估算的状态。**不延后时不写这个键**（老记录形态不变）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub print_time_status: Option<String>,
    /// 估算出的打印秒数（与旧版 `_meta.json` 的 `detail.estimatedPrintSeconds` 同名）——
    /// 报告页拿它与切片器原话比「多了多久」。延后估算时先缺键，[`complete_meta`] 补。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_print_seconds: Option<f64>,
    /// 涂胶组（`detail.glue`）。失败 / 取消没有 IR 终态 ⇒ 不写这一格
    /// （页面口径：后端没给就不摆，不编占位值）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub glue: Option<GlueFacts>,
    /// 擦料塔组（`detail.tower`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tower: Option<TowerFacts>,
    /// 切片工艺组（`detail.slicer`）。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub slicer: Option<SlicerFacts>,
    /// 校准件身份证据链（`detail.calibration`，2026-10-18）。
    ///
    /// **只加不改**：旧记录读到 `null` ⇒ 前端整块不摆（与 `slicer` / `tower` / `relocation`
    /// 同一套习惯）。普通模型那一趟恒为 `null`。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub calibration: Option<CalibrationFacts>,
}

/// **校准件这一趟的证据链**（2026-10-18）。
///
/// 作者原话：「每次处理拿到了什么、从哪里拿到、怎么判断、最终用了什么，都留下记录。
/// 以后出现识别错误、模型版本不对、参数不匹配，都能回头查清楚。」
///
/// 所以这一组回答五件事，**成功与失败都写**：
///
/// | 问题 | 字段 |
/// | --- | --- |
/// | 原始模型是哪个文件、还在不在 | [`ModelOrigin`] |
/// | 3MF 说的是什么 | `identity_3mf` |
/// | 怎么判的 | `verdict` + `note` |
/// | 实测多大、命中哪一档、容差多少 | `measured` / `profile_id` / `tolerance` / `footprint` |
/// | 最终采用什么身份 | `mode` |
///
/// 形状注意：`identity_3mf` 存的是**原值**（可能是旧写法
/// `ZOffset Calibration`），不做归一 —— 排查时要看的就是"它当时到底写了什么"。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CalibrationFacts {
    /// 原始模型路径与存在性（值来自切片器临时目录的 `origin.txt`）。
    #[serde(flatten)]
    pub origin: postprocess::gcode::ModelOrigin,
    /// 3MF `MKPSE:Calibration` 的原值；读不到 = `null`。
    pub identity_3mf: Option<String>,
    /// `from_3mf`（3MF 里有身份）/ `absent`（没有、或读不到，原因见 `note`）。
    ///
    /// **只有这两种**：G-code 那一路来源已整条删除（2026-10-18），不再有
    /// `only_gcode` / `conflict` 这些格里解释得通、实际却永远出现不了的值。
    pub verdict: String,
    /// 读不到的原因（人可读、一条）。不是错误，只是解释。
    pub note: Option<String>,
    /// 实测挤出包围盒 `[xMin, xMax, yMin, yMax]`（mm）；经典分支拿不到 = `null`。
    pub measured: Option<[f64; 4]>,
    /// 命中的档 id（`calibration_models.toml` 的 `[[models]] id`）。
    pub profile_id: Option<String>,
    /// 该档的尺寸容差（mm）。
    pub tolerance: Option<f64>,
    /// 该档声明的标称外形 `[宽, 高]`。
    pub footprint: Option<[f64; 2]>,
    /// 最终采用的校准身份（`ZOffset` / `Precise` / `Rough` / `Repetition`；无 = `null`）。
    pub mode: Option<String>,
}

/// **挪塔这件事的记录**（2026-10-18）。
///
/// 报告页 / 执行记录照着它回答四件事：**用户当时选了什么**、塔从哪挪到哪、
/// 挪了多少毫米、挪完复检过没过。作者原话：「我需要你把塔移动的记录进报告里面，
/// 就是怎么移动，多少距离，用户选择自动移动吗？还是选了自己去移动模型停止了」。
///
/// 只有真的走到那一问时才落；别的记录 `null`（旧记录读到 `null` ⇒ 整块不摆）。
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelocationRecord {
    /// 用户选了什么：`relocate`（自动挪塔）/ `model`（自己挪模型）/ `timeout`（没答）/ `unavailable`（板上没位）
    pub choice: String,
    /// 塔左下角，挪之前（mm，**不取整**）
    pub from: [f64; 2],
    /// 塔左下角，挪之后；没挪 = `null`
    pub to: Option<[f64; 2]>,
    /// 直线位移（mm）；没挪 = `null`
    pub moved_mm: Option<f64>,
    /// 挪之前的冲突点数
    pub before_conflict_count: i64,
    /// 挪完复检的冲突点数（没挪 = `null`）
    pub after_conflict_count: Option<i64>,
    /// 挪完复检过没过（`null` = 没挪，谈不上复检）
    pub verified: Option<bool>,
    /// 这一趟用的塔高（mm）—— 斜肋塔的底座随它变大，记下来免得下次复盘时对不上
    pub tower_height_mm: f64,
    /// **挪之前那一份碰撞事实**（2026-10-18）：报告页拿它 + `to` 画"现在 / 挪后"两幅
    /// 俯视图（作者：「成功挪塔的也可以显示，画出来碰撞位置，就是把后处理的那两个板子
    /// 也做进报告页面」）。成功那趟 `Meta.collision` 是 `null`（撞了就走不到成功出口），
    /// 所以事实必须跟着记录走，不能只靠 `collision` 那一格。
    pub before: Option<postprocess::postproc::disk::collision::CollisionFacts>,
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
    /// **3MF 模型名**（`帕姆.3mf` → `帕姆`）—— 钩子在 G-code 还在切片器临时目录里时
    /// 从旁边的 `origin.txt` / 同名 zip / `_temp_1.config` 取（三级兜底，见
    /// `postprocess::gcode::model_name`），过了那一刻就永远拿不到了。
    ///
    /// 与 `error` 同一档：**落 `null` 而不是缺键** —— 这一格是报告页的契约（列表行与
    /// 「结论 → 文件」都读它），"没有名字"与"老记录里没有这个键"是两件事。
    pub model_name: Option<String>,
    /// **这一趟是**哪个切片器进程**拉起来的**（2026-10-20）—— 钩子模式下父进程就是
    /// 那一趟切片的 Bambu Studio（见 [`crate::parent`]）。
    ///
    /// 为什么要记它：BBS 会**多开**，而"改完参数要回哪一份去重新切片"只有它答得准 ——
    /// "窗口排在最前那份"会挑错（用户可能切着 A 份、手上点着 B 份）。
    /// 引导层读最近一条记录的这一格 ⇒ 认出"最后处理用的是哪个 BBS"。
    ///
    /// ★ **只加不改**：旧记录没有这个键 ⇒ 读到 `null` ⇒ 引导退回"最前那份"，
    /// 与 `collision` / `relocation` 同一套习惯。**拿不到就落 `null`，不编数**。
    pub slicer_pid: Option<i32>,
    pub invocation: Invocation,
    pub warnings: Vec<String>,
    /// 成功 ⇒ `None`（落 `null`，不是缺键：报告页按"有没有 error"判 `ok`）。
    pub error: Option<String>,
    pub cancelled: bool,
    pub pipeline: Vec<PipelineStep>,
    /**
     * **碰撞那一趟的额外一格**（2026-10-18；只有 `E_GCODE_COLLISION_001` 会写）。
     *
     * 报告页拿它回答"撞在哪、往哪挪多少"（建议那句的码与正文也在里面），并画一张
     * 俯视图。别的失败 / 成功记录一律 `null` —— 于是"有没有这两块"是数据说了算。
     *
     * 键是**新增**的（旧记录没有 ⇒ 读到 `null` ⇒ 整组不摆），不碰任何旧键。
     */
    pub collision: Option<postprocess::postproc::disk::collision::CollisionFacts>,
    /// **挪塔记录**（2026-10-18）：走到了那一问才有，别的一律 `null`。
    pub relocation: Option<RelocationRecord>,
    pub detail: Detail,
}

impl Meta {
    pub const SCHEMA_VERSION: u32 = 2;
}

/// 组装「三组分包（glue / tower / slicer）+ 每步细节」要的全部素材 ——
/// **都是这条链上已经在手的事实**（内核搬出来的 IR 终态 / 统计 / 头部事实，
/// 加上调用方手上的行数与路径）。这里只做搬运与文案，不新算任何值。
pub struct ReportFacts<'a> {
    /// 预设文件名（`job.toml` 的最后一段；拿不到就是空串）。
    pub preset_name: &'a str,
    pub input_lines: u64,
    pub input_bytes: u64,
    pub input_sha256: &'a str,
    /// 输出文件大小（盘上 stat 一次；stat 不到就是 `None` —— 文案少一行，不编数）。
    pub output_bytes: Option<u64>,
    /// 是不是原地覆盖（输出写回输入那一份）。
    pub in_place: bool,
    /// IR 终态（内核 `DetailFacts.ir`）。
    pub ir: &'a postprocess::ir::Ir,
    /// pass1 统计（`ProcessStats`）。
    pub stats: &'a postprocess::postproc::pass1::ProcessStats,
    /// pass2 口径的塔高（内核 `Stats.tower_height`）。
    pub tower_height: f64,
    /// 输入头部刮出来的 5 个事实（内核 `HeaderFacts`）。
    pub header: &'a postprocess::pipeline::HeaderFacts,
    /// 校准模式串（非校准模式是空串）。
    pub calibration_mode: &'a str,
    /// 第 7 步认档的回执（实测尺寸 / 命中档 / 容差）。非校准件是 `None`。
    pub calibration_receipt: Option<&'a postprocess::postproc::calibration::CalibrationReceipt>,
}

/// 涂胶速度（mm/s）：内核里 `toolhead.max_speed` 是 mm/min（= SpeedLimit × 60），
/// 旧版 `glueSpeed` 是 mm/s —— 除回来，不新造数。
///
/// ★ **全仓唯一的换算点**：归档这份（[`ReportFacts::glue`]）与钩子一开场报的
/// `FactsPayload.glue_speed` 都走这里（两处各写一遍迟早会对不上）。
pub fn glue_speed_of(ir: &postprocess::ir::Ir) -> f64 {
    ir.toolhead.max_speed / 60.0
}

impl ReportFacts<'_> {
    /// 涂胶组。
    pub fn glue(&self) -> GlueFacts {
        let ir = self.ir;
        GlueFacts {
            glue_total_length: self.stats.glue_total_length,
            // 旧版 `gluePenSwitches` ← `GlueCutterChanges`（换笔 = 切刀次数）。
            glue_pen_switches: self.stats.glue_cutter_changes,
            glue_pass_count: ir.glue.pass_count,
            glue_speed: glue_speed_of(ir),
            pen_offset_x: ir.toolhead.x_offset,
            pen_offset_y: ir.toolhead.y_offset,
            pen_offset_z: ir.toolhead.z_offset,
            glue_z_lift_height: ir.glue.z_lift_height,
            glue_z_lift_height_first_layers: ir.glue.z_lift_height_first_layers,
            glue_inter_model_lift_height: ir.glue.inter_model_lift_height,
            glue_z_offset: ir.glue.z_offset,
            prime_trigger_layers: ir.prime.trigger_layers,
            prime_total_length: self.stats.prime_total_length,
        }
    }

    /// 擦料塔组。
    pub fn tower(&self) -> TowerFacts {
        let ir = self.ir;
        TowerFacts {
            // 塔层数只认 IR 终态：pass2 每写一层塔 +1（`ir.tower.layer_count`），
            // 与旧版 `detail.towerLayers ← irData.Tower.LayerCount` 同一个来源。
            tower_layers: ir.tower.layer_count,
            tower_height: self.tower_height,
            tower_extrude_ratio: ir.tower.extrude_ratio,
            tower_z_lift: ir.tower.safe_z_offset,
            tower_safe_z_offset: ir.tower.safe_z_offset,
            tower_travel_speed: ir.tower.travel_speed,
            fast_tower_mode: ir.tower.fast_mode.clone(),
        }
    }

    /// 切片工艺组。
    pub fn slicer(&self) -> SlicerFacts {
        let ir = self.ir;
        SlicerFacts {
            slicer_version: self.stats.slicer_version.clone(),
            machine_preset_name: self.header.machine_preset_name.clone(),
            process_preset_name: self.header.process_preset_name.clone(),
            filament_type: ir.filament.filament_type.clone(),
            nozzle_diameter: ir.machine.nozzle_diameter,
            slicer_ironing_enabled: self.header.slicer_ironing_enabled,
            ironing_mode: ir.ironing.mode.clone(),
            ironing_threshold: ir.ironing.e_sum_threshold,
            tree_support: ir.machine.tree_support,
            use_disk_wipe: self.use_disk_wipe(),
        }
    }

    /// 这一盘是不是圆盘擦拭（旧版 `detail.useDiskWipe` 的判据）。
    fn use_disk_wipe(&self) -> bool {
        self.ir.wiping.mode == postprocess::ir::WIPING_MODE_DISK
    }

    /// 擦料塔类型（旧版三档：无 / 普通擦料塔 / 圆盘擦拭）。
    fn tower_type(&self) -> &'static str {
        if self.use_disk_wipe() {
            "圆盘擦拭"
        } else if self.ir.tower.use_towers {
            "普通擦料塔"
        } else {
            "无"
        }
    }

    /// 一条步骤的多行事实（`pipeline[].detail`，照旧版 `\n` 分行的形状）。
    ///
    /// 没有素材的步返回 `None`（键直接不写）：
    /// - `calib-check`：包围盒 / 质心是管线局部量、没出过门，不编；
    /// - `printtime`：只有一段耗时，报告页的「打印」组已经摆了它的结果。
    pub fn step_detail(&self, step: Step) -> Option<String> {
        match step {
            Step::Input => Some(format!(
                "读取 {} 行\n文件大小: {:.2} MB\nSHA256: {}",
                self.input_lines,
                mib(self.input_bytes),
                self.input_sha256.chars().take(8).collect::<String>(),
            )),
            Step::Config => {
                (!self.preset_name.is_empty()).then(|| format!("预设: {}", self.preset_name))
            }
            Step::MachineDetect => Some(format!(
                "G-code 机型: {}\n预设机型: {}\n机型维度: {:.0}×{:.0}mm\n禁区数量: {}",
                non_empty(&self.ir.machine.gcode_machine_type),
                self.ir.machine.machine_type,
                self.ir.machine.max_x - self.ir.machine.min_x,
                self.ir.machine.max_y - self.ir.machine.min_y,
                self.ir.machine.forbidden_zones.len(),
            )),
            Step::Parse => Some(format!(
                "机型: {}\n耗材类型: {}\n切片软件: {}",
                non_empty(&self.ir.machine.gcode_machine_type),
                self.ir.filament.filament_type,
                self.ir.filament.slicer,
            )),
            Step::Support => Some(format!(
                "支撑类型: {}\n熨烫模式: {}\n熨烫覆盖率阈值: {:.1}%",
                if self.ir.machine.tree_support {
                    "树状支撑"
                } else {
                    "普通支撑"
                },
                ironing_mode_cn(&self.ir.ironing.mode),
                self.ir.ironing.coverage_threshold,
            )),
            Step::Collision => {
                let mut lines: Vec<String> = Vec::new();
                // 只有非校准模式这一步真跑碰撞检测（`need_collision_check`）；
                // 过到这儿就是没碰撞 —— 校准模式没跑过，不许写这句结论。
                if self.calibration_mode.is_empty() {
                    lines.push("无碰撞，擦料塔位置安全".to_string());
                    lines.push("模型坐标均在打印范围内".to_string());
                }
                lines.push(format!("擦料塔类型: {}", self.tower_type()));
                lines.push(format!(
                    "擦料塔位置: ({:.1}, {:.1})",
                    self.ir.wiping.wiper_x, self.ir.wiping.wiper_y
                ));
                Some(lines.join("\n"))
            }
            Step::CalibCheck => {
                /* 这一步原来恒为 `None`（只跑 dry-run、什么事实都不留）。现在把认档结果
                摆出来：**成功那趟也写** —— 否则"为什么这次用了加长版的基准"没法解释。
                只写事实，不写结论（结论在 `detail.calibration` 那一组里）。 */
                let r = self.calibration_receipt?;
                let mut lines = Vec::new();
                if let Some(b) = r.measured {
                    lines.push(format!(
                        "实测包围盒: {:.4} × {:.4} mm\nX[{:.4}, {:.4}]  Y[{:.4}, {:.4}]",
                        b.x_max - b.x_min,
                        b.y_max - b.y_min,
                        b.x_min,
                        b.x_max,
                        b.y_min,
                        b.y_max
                    ));
                }
                match (&r.profile_id, r.footprint, r.tolerance) {
                    (Some(id), Some((w, h)), Some(tol)) => {
                        lines.push(format!("命中档位: {id}（标称 {w} × {h}，容差 {tol}）"))
                    }
                    (Some(id), _, _) => lines.push(format!("命中档位: {id}")),
                    _ => lines.push("档位: 按机型固定坐标（经典分支，未认档）".to_string()),
                }
                Some(lines.join("\n"))
            }
            Step::Pass1 => Some(format!(
                "收集 {} 层高索引\n涂胶遍数: {}\n润笔: {}",
                self.stats.total_layer_number,
                self.ir.glue.pass_count,
                if self.ir.toolhead.first_pen_revitalization_flag {
                    format!(
                        "是（每模型: {}, 每层: {}）",
                        yes_no(self.ir.prime.per_model),
                        yes_no(self.ir.prime.every_layer),
                    )
                } else {
                    "否".to_string()
                },
            )),
            Step::Pass2 => Some(format!(
                "擦料塔层数: {}\n擦料塔高度: {:.2}mm\n擦料塔类型: {}\n\
                 从擦料塔到模型的抬笔高度: {:.1}mm\n斜肋外墙: {}\n圆盘擦拭: {}\n写入完成",
                self.ir.tower.layer_count,
                self.tower_height,
                self.tower_type(),
                self.ir.tower.safe_z_offset,
                yes_no(self.ir.wiping.outer_structure == "rib"),
                yes_no(self.use_disk_wipe()),
            )),
            Step::Calibration => {
                let mut text = format!("校准模式: {}", self.calibration_mode);
                match self.ir.safety.calibration_execution_mode.as_str() {
                    "direct_calibrate" => text.push_str("\n执行方式: 直接校准（跳过模型打印）"),
                    "print_then_calibrate" => text.push_str("\n执行方式: 打印后校准"),
                    _ => {}
                }
                text.push_str("\n校准 G-code 已插入");
                Some(text)
            }
            Step::Write => Some(match self.output_bytes {
                Some(bytes) => format!("{}\n输出 {:.2} MB", self.write_result_text(), mib(bytes)),
                // 大小还没量到（stat 不到）就不写那行 —— 不编 0.00 MB。
                None => self.write_result_text().to_string(),
            }),
            Step::PrintTime => None,
        }
    }

    /// 写入那一步的结论句（旧版两个说法）。
    fn write_result_text(&self) -> &'static str {
        if self.in_place {
            "原地覆盖完成"
        } else {
            "写入独立文件完成"
        }
    }
}

/// 字节 → MB（÷ 1024²，与前端 `mb()` 同一个数）。
fn mib(bytes: u64) -> f64 {
    bytes as f64 / 1024.0 / 1024.0
}

/// 是 / 否（旧版 `BoolToCN`）。
fn yes_no(v: bool) -> &'static str {
    if v {
        "是"
    } else {
        "否"
    }
}

/// 空串兜底：机型认不出时内核写 `UNKNOWN`，但空串也要有个说法 —— 文案不留空。
fn non_empty(s: &str) -> &str {
    if s.is_empty() {
        "UNKNOWN"
    } else {
        s
    }
}

/// 熨烫模式的中文（照旧版 `_meta.json` 的说法：off=关 / auto=自动 / on=开）。
fn ironing_mode_cn(mode: &str) -> &str {
    match mode {
        "off" => "关",
        "auto" => "自动",
        "on" => "开",
        other => other,
    }
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

/// 按记录 id 找 `<日期>/<id>_meta.json`（id 全树唯一，文件名即 id）。
///
/// 目录约定与报告页读口（`ipc::report`）同一套 —— 这一份也给"补全"那一侧用。
pub fn find_meta(history_root: &Path, id: &str) -> Option<PathBuf> {
    let days = std::fs::read_dir(history_root).ok()?;
    for day in days.flatten() {
        if !day.path().is_dir() {
            continue;
        }
        let candidate = day.path().join(format!("{id}_meta.json"));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// 补全一条记录：**读-改-写，原子落盘**（走唯一的写盘出口）。
///
/// 补全的两样东西都是"报告用途"、且都要读/复制一份 26MB 级的输出 ——
/// 钩子把它们交给常驻进程，自己写完记录就退。
///
/// 只动 `detail.outputFile` / `detail.printTime` 与顶层 `printTimeStatus`
/// 这三个键 —— 其余字段原样保留（先读后写，不重建整份记录）。
pub fn complete_meta(
    meta_path: &Path,
    output_file: Option<FileFact>,
    print_time: Option<serde_json::Value>,
) -> Result<(), AppError> {
    let text = std::fs::read_to_string(meta_path)
        .map_err(|e| AppError::io("这条记录读不出来（补全前）").with_detail(e.to_string()))?;
    let mut meta: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        AppError::corrupted("这条记录不是合法的 JSON（补全前）").with_detail(e.to_string())
    })?;
    {
        let Some(obj) = meta.as_object_mut() else {
            return Err(AppError::corrupted("这条记录不是 JSON 对象"));
        };
        let detail = obj.entry("detail").or_insert_with(|| serde_json::json!({}));
        if let Some(of) = output_file {
            detail["outputFile"] = serde_json::to_value(of)
                .map_err(|e| AppError::io("输出事实序列化失败").with_detail(e.to_string()))?;
        }
        if let Some(pt) = print_time {
            // 估算秒数与 printTime 同批落：`detail.estimatedPrintSeconds` 是旧版同名的
            // 单列数字（报告页拿它算「比切片器预估多了多久」）。
            if let Some(total) = pt.get("totalSeconds").and_then(|v| v.as_f64()) {
                detail["estimatedPrintSeconds"] = serde_json::json!(total);
            }
            detail["printTime"] = pt;
        }
        obj.insert(
            "printTimeStatus".to_string(),
            serde_json::json!(PRINT_TIME_READY),
        );
    }
    crate::fsx::atomic::atomic_write_json(meta_path, &meta)
}

/// 估算失败（或文件已经不在）时把状态标成 `failed` —— **不许留着 `computing`**，
/// 否则报告页会一直显示"正在估算"。
pub fn mark_print_time_failed(meta_path: &Path) -> Result<(), AppError> {
    let text = std::fs::read_to_string(meta_path)
        .map_err(|e| AppError::io("这条记录读不出来").with_detail(e.to_string()))?;
    let mut meta: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| AppError::corrupted("这条记录不是合法的 JSON").with_detail(e.to_string()))?;
    if let Some(obj) = meta.as_object_mut() {
        obj.insert(
            "printTimeStatus".to_string(),
            serde_json::json!(PRINT_TIME_FAILED),
        );
    }
    crate::fsx::atomic::atomic_write_json(meta_path, &meta)
}

#[cfg(test)]
mod tests {
    // 判据要造真实文件（原件、输出体、`_meta.json`）—— 生产代码的写盘一律走
    // `fsx::atomic` 那一个洞，这里的 `std::fs::write` 只活在测试里。
    #![allow(clippy::disallowed_methods)]

    use super::*;
    use time::macros::datetime;

    /// 延后的打印时间：`computing` → 补全成 `ready`，且**其余字段一字不动**。
    #[test]
    fn deferred_print_time_is_completed_in_place() {
        let d = tempfile::tempdir().unwrap();
        let input = d.path().join("in.gcode");
        std::fs::write(&input, "G1 X1\n").unwrap();
        let paths = begin(
            d.path(),
            &input,
            "G1 X1\n",
            datetime!(2026-10-10 1:00:00 +8),
        )
        .unwrap();

        let meta = serde_json::json!({
            "startedAt": "2026-10-10 01:00:00",
            "elapsedMs": 1234,
            "detail": {
                "inputFile": {"path": "x", "sizeBytes": 6, "sha256": "s"},
                "stats": {"glueLayerCount": 7},
            },
            "printTimeStatus": "computing",
        });
        std::fs::write(&paths.meta, serde_json::to_string(&meta).unwrap()).unwrap();

        let pt = serde_json::json!({"totalSeconds": 17046.8, "segments": 708937});
        complete_meta(&paths.meta, None, Some(pt)).expect("补全打印时间");

        let back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.meta).unwrap()).unwrap();
        assert_eq!(back["printTimeStatus"], "ready");
        assert_eq!(back["detail"]["printTime"]["segments"], 708937);
        // 估算秒数与 printTime 同批补上（旧版同名的单列数字，报告页拿它比对）
        assert_eq!(back["detail"]["estimatedPrintSeconds"], 17046.8);
        // 其余字段必须原样保留（读-改-写，不重建整份记录）
        assert_eq!(back["elapsedMs"], 1234);
        assert_eq!(back["detail"]["stats"]["glueLayerCount"], 7);
        assert_eq!(back["detail"]["inputFile"]["sizeBytes"], 6);
    }

    /// 估算失败**不许**留着 `computing` —— 否则报告页永远显示"正在估算"。
    #[test]
    fn failed_estimation_is_marked_not_left_computing() {
        let d = tempfile::tempdir().unwrap();
        let input = d.path().join("in.gcode");
        std::fs::write(&input, "G1 X1\n").unwrap();
        let paths = begin(
            d.path(),
            &input,
            "G1 X1\n",
            datetime!(2026-10-10 1:00:00 +8),
        )
        .unwrap();
        std::fs::write(
            &paths.meta,
            r#"{"elapsedMs":1,"printTimeStatus":"computing"}"#,
        )
        .unwrap();

        mark_print_time_failed(&paths.meta).expect("标失败");
        let back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.meta).unwrap()).unwrap();
        assert_eq!(back["printTimeStatus"], "failed");
        assert_eq!(back["elapsedMs"], 1);
    }

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

        assert!(paths
            .original
            .ends_with("63288.1_20261009_220851_original.gcode"));
        assert!(paths.output.ends_with("63288.1_20261009_220851.gcode"));
        assert!(paths.meta.ends_with("63288.1_20261009_220851_meta.json"));
        assert_eq!(
            std::fs::read_to_string(&paths.original).unwrap(),
            "G1 X1\n",
            "原件内容 = 交给它的原文，一个字节不差"
        );
    }

    /// **归组口径**：三件套里任意一个文件名都能还原到同一条记录的基名
    /// （自动清理按它把散文件归回一组，删就整条删）；不是三件套的东西不认。
    #[test]
    fn record_base_groups_the_trio_and_rejects_outsiders() {
        let base = "63288.1_20261009_220851";
        for name in [
            format!("{base}_original.gcode"),
            format!("{base}.gcode"),
            format!("{base}_meta.json"),
        ] {
            assert_eq!(record_base(&name), Some(base), "{name} 归不回同一条记录");
        }
        /* 名前缀自己带 `_original` 的原件：剥的必须是**最后那个后缀**，前缀原样留着 */
        assert_eq!(
            record_base("x_original_20261009_220851_original.gcode"),
            Some("x_original_20261009_220851")
        );
        /* 不是三件套的东西（临时文件 / 用户塞进来的）不认 —— 清理一个都不碰 */
        assert_eq!(record_base("notes.txt"), None);
        assert_eq!(record_base("readme"), None);
        assert_eq!(record_base(""), None);
    }

    /// 造一份「成功记录」的报告素材（判据共用）：IR 只填被读到的那些字段。
    struct Sample {
        ir: postprocess::ir::Ir,
        stats: postprocess::postproc::pass1::ProcessStats,
        header: postprocess::pipeline::HeaderFacts,
        preset_name: String,
        sha: String,
    }

    impl Sample {
        fn facts(&self) -> ReportFacts<'_> {
            ReportFacts {
                preset_name: &self.preset_name,
                input_lines: 909_465,
                input_bytes: 23_758_782,
                input_sha256: &self.sha,
                output_bytes: Some(24_000_000),
                in_place: true,
                ir: &self.ir,
                stats: &self.stats,
                tower_height: 6.46,
                header: &self.header,
                calibration_mode: "",
                calibration_receipt: None,
            }
        }
    }

    fn sample() -> Sample {
        let mut ir = postprocess::ir::Ir::default();
        ir.toolhead.max_speed = 5400.0; // = 90 mm/s × 60
        ir.glue.pass_count = 1;
        ir.glue.z_lift_height = 5.0;
        ir.glue.z_lift_height_first_layers = 6.0;
        ir.glue.inter_model_lift_height = 2.0;
        ir.glue.z_offset = -0.05;
        ir.prime.trigger_layers = 30;
        ir.tower.layer_count = 536;
        ir.tower.extrude_ratio = 1.0;
        ir.tower.safe_z_offset = 0.6;
        ir.tower.travel_speed = 200.0;
        ir.tower.fast_mode = "fast".into();
        ir.wiping.mode = postprocess::ir::WIPING_MODE_DISK.into();
        ir.wiping.wiper_x = 20.0;
        ir.wiping.wiper_y = 20.0;
        ir.ironing.mode = "auto".into();
        ir.ironing.e_sum_threshold = 7.9;
        ir.ironing.coverage_threshold = 20.0;
        ir.machine.nozzle_diameter = 0.4;
        ir.machine.gcode_machine_type = "A1Mini".into();
        ir.machine.machine_type = "A1_MINI".into();
        ir.machine.max_x = 180.0;
        ir.machine.max_y = 180.0;
        ir.filament.filament_type = "PETG".into();
        ir.filament.slicer = "BambuStudio".into();

        let stats = postprocess::postproc::pass1::ProcessStats {
            glue_total_length: 344.836_999_999_999_76,
            glue_cutter_changes: 1,
            prime_total_length: 35.449,
            slicer_version: "BambuStudio 02.06.01.55".into(),
            total_layer_number: 539,
            ..Default::default()
        };

        Sample {
            ir,
            stats,
            header: postprocess::pipeline::HeaderFacts {
                machine_preset_name: "Bambu Lab A1 mini 0.4 nozzle".into(),
                process_preset_name: "0.20mm Standard @BBL A1M".into(),
                slicer_ironing_enabled: false,
                three_mf_file_name: ".65112.2_.gcode.3mf".into(),
                model_name: Some("帕姆".into()),
            },
            preset_name: "A1MF.toml".into(),
            sha: "dea417626c060af54c11e3613468153148046c81a57f358ea86d63805aacd771".into(),
        }
    }

    /// 三组分包（`detail.glue/tower/slicer`）的键名照旧版 `_meta.json` **逐字搬** ——
    /// 报告页按这些键取值，改一个键等于撕一处契约。
    #[test]
    fn glue_tower_slicer_keys_match_the_old_meta() {
        let s = sample();
        let facts = s.facts();

        let glue = serde_json::to_value(facts.glue()).unwrap();
        for key in [
            "glueTotalLength",
            "gluePenSwitches",
            "gluePassCount",
            "glueSpeed",
            "penOffsetX",
            "penOffsetY",
            "penOffsetZ",
            "glueZLiftHeight",
            "glueZLiftHeightFirstLayers",
            "glueInterModelLiftHeight",
            "glueZOffset",
            "primeTriggerLayers",
            "primeTotalLength",
        ] {
            assert!(glue.get(key).is_some(), "涂胶组缺键：{key}");
        }
        assert_eq!(glue["glueSpeed"], 90.0, "mm/min ÷ 60 回旧版的 mm/s");
        assert_eq!(glue["glueZOffset"], -0.05);
        assert_eq!(glue["gluePenSwitches"], 1, "换笔 ← GlueCutterChanges");

        let tower = serde_json::to_value(facts.tower()).unwrap();
        for key in [
            "towerLayers",
            "towerHeight",
            "towerExtrudeRatio",
            "towerZLift",
            "towerSafeZOffset",
            "towerTravelSpeed",
            "fastTowerMode",
        ] {
            assert!(tower.get(key).is_some(), "擦料塔组缺键：{key}");
        }
        assert_eq!(tower["towerLayers"], 536, "塔层数认 IR 终态");
        assert_eq!(tower["towerHeight"], 6.46);

        let slicer = serde_json::to_value(facts.slicer()).unwrap();
        for key in [
            "slicerVersion",
            "machinePresetName",
            "processPresetName",
            "filamentType",
            "nozzleDiameter",
            "slicerIroningEnabled",
            "ironingMode",
            "ironingThreshold",
            "treeSupport",
            "useDiskWipe",
        ] {
            assert!(slicer.get(key).is_some(), "切片工艺组缺键：{key}");
        }
        assert_eq!(slicer["machinePresetName"], "Bambu Lab A1 mini 0.4 nozzle");
        assert_eq!(slicer["useDiskWipe"], true);
    }

    /// 每步细节：有素材的步写多行事实；没有素材的（calib-check / printtime）不写键。
    #[test]
    fn step_details_carry_the_facts_the_report_shows() {
        let s = sample();
        let facts = s.facts();

        let input = facts.step_detail(Step::Input).unwrap();
        assert!(input.starts_with("读取 909465 行\n"), "实际：{input}");
        assert!(input.contains("SHA256: dea41762"), "sha 取前 8 位");

        let collision = facts.step_detail(Step::Collision).unwrap();
        assert!(collision.contains("无碰撞，擦料塔位置安全"));
        assert!(collision.contains("擦料塔位置: (20.0, 20.0)"));

        let pass2 = facts.step_detail(Step::Pass2).unwrap();
        assert!(pass2.contains("擦料塔层数: 536"));
        assert!(pass2.contains("斜肋外墙: 否"));

        let write = facts.step_detail(Step::Write).unwrap();
        assert!(write.contains("原地覆盖完成"));
        assert!(write.contains("输出 22.89 MB"), "实际：{write}");

        // 没有素材的步：键直接不写（页面口径：没给就不摆）。
        // `CalibCheck` 在**非校准件**那一趟没有回执 ⇒ 整格不写（校准件那趟见
        // `calib_check_step_reports_the_profile_it_used`）。
        assert!(facts.step_detail(Step::CalibCheck).is_none());
        assert!(facts.step_detail(Step::PrintTime).is_none());
    }

    /// 第 7 步认档**成功那趟也要留事实**（2026-10-18 补的证据链）：
    /// 原来这一步恒为 `None`，于是"为什么这次用了加长版的基准"没法解释。
    #[test]
    fn calib_check_step_reports_the_profile_it_used() {
        use postprocess::postproc::calibration::CalibrationReceipt;
        use postprocess::postproc::disk::BBox;

        let s = sample();
        let mut facts = s.facts();
        let receipt = CalibrationReceipt {
            measured: Some(BBox {
                x_min: 67.921,
                x_max: 187.4321,
                y_min: 121.408,
                y_max: 137.471,
            }),
            profile_id: Some("zoffset-ext".to_string()),
            tolerance: Some(0.5),
            footprint: Some((119.58, 16.063)),
        };
        facts.calibration_receipt = Some(&receipt);

        let detail = facts.step_detail(Step::CalibCheck).unwrap();
        assert!(
            detail.contains("119.5111 × 16.0630"),
            "实测尺寸要在，实际：{detail}"
        );
        assert!(detail.contains("命中档位: zoffset-ext"), "实际：{detail}");
        assert!(detail.contains("容差 0.5"), "实际：{detail}");
    }

    /// ★★ **最近一次处理用的是哪个切片器**（2026-10-20）：按时间倒序挑，且只认
    /// `accept` 认下的号 —— 切片器重启后旧号就废了，那时该落到**上一条还活着**的记录。
    #[test]
    fn latest_slicer_pid_picks_the_newest_live_one() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("2026-10-19")).unwrap();
        std::fs::create_dir_all(d.path().join("2026-10-20")).unwrap();
        let write = |day: &str, name: &str, pid: Option<i32>| {
            let pid = pid.map(|p| p.to_string()).unwrap_or("null".to_owned());
            std::fs::write(
                d.path().join(day).join(name),
                format!(r#"{{"schemaVersion":2,"startedAt":"","slicerPid":{pid}}}"#),
            )
            .unwrap();
        };
        write("2026-10-19", "a_20261019_100000_meta.json", Some(111));
        write("2026-10-20", "b_20261020_090000_meta.json", Some(222));
        /* 最新的一条（同一天里名字最大） */
        write("2026-10-20", "c_20261020_093000_meta.json", Some(333));

        /* 全都认 ⇒ 拿最新的 333 */
        assert_eq!(latest_slicer_pid(d.path(), |_| true), Some(333));
        /* 最新那条号已经"不是切片器了" ⇒ 落到上一条还活着的 222 */
        assert_eq!(latest_slicer_pid(d.path(), |p| p != 333), Some(222));
        /* 一条都不认 ⇒ None（不是随便挑一个） */
        assert_eq!(latest_slicer_pid(d.path(), |_| false), None);
    }

    /// **旧记录没有 `slicerPid` 这个键**（只加不改那一档）⇒ 读到 `null`，
    /// 跳过它继续往前找，而不是报错、也不是编一个号。
    #[test]
    fn records_without_the_key_are_skipped_not_faked() {
        let d = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(d.path().join("2026-10-20")).unwrap();
        std::fs::write(
            d.path()
                .join("2026-10-20")
                .join("new_20261020_093000_meta.json"),
            r#"{"schemaVersion":2,"startedAt":""}"#,
        )
        .unwrap();
        std::fs::write(
            d.path()
                .join("2026-10-20")
                .join("old_20261020_090000_meta.json"),
            r#"{"schemaVersion":2,"startedAt":"","slicerPid":777}"#,
        )
        .unwrap();
        assert_eq!(latest_slicer_pid(d.path(), |_| true), Some(777));
    }

    /// 校准模式没跑过碰撞检测（`need_collision_check`）—— 细节**不许**写"无碰撞"，
    /// 那是编的结论；类型与位置两行照写。
    #[test]
    fn calibration_mode_collision_detail_does_not_claim_a_check() {
        let s = sample();
        let mut facts = s.facts();
        facts.calibration_mode = "ZOffset";
        let collision = facts.step_detail(Step::Collision).unwrap();
        assert!(!collision.contains("无碰撞"), "实际：{collision}");
        assert!(collision.contains("擦料塔类型: 圆盘擦拭"));
    }

    /// `_meta.json` 的键**咬住**报告页读口与前端消费面（camelCase）。
    /// 改键名等于撕契约 —— 这条判据就是拦这个的。
    #[test]
    fn meta_json_keeps_the_keys_the_report_page_reads() {
        let s = sample();
        let facts = s.facts();
        let meta = Meta {
            schema_version: Meta::SCHEMA_VERSION,
            started_at: "2026-10-09 22:08:51".into(),
            finished_at: "2026-10-09 22:08:53".into(),
            elapsed_ms: 2530,
            preset_name: "A1-fastv3.3.toml".into(),
            machine_type: "A1".into(),
            preset_path: "/p/A1-fastv3.3.toml".into(),
            gcode_path: "/tmp/.63288.1.gcode".into(),
            model_name: Some("帕姆".into()),
            slicer_pid: Some(4242),
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
                detail: Some("读取 909465 行".into()),
            }],
            /* 成功记录没有碰撞事实 */
            collision: None,
            relocation: None,
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
                print_time_status: None,
                estimated_print_seconds: Some(2530.5),
                glue: Some(facts.glue()),
                tower: Some(facts.tower()),
                slicer: Some(facts.slicer()),
                calibration: None,
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
            "modelName",
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
        assert_eq!(v["pipeline"][0]["detail"], "读取 909465 行");
        assert!(v["error"].is_null(), "成功记录落 null，不是缺键");
        assert_eq!(v["modelName"], "帕姆", "3MF 模型名要进顶层（报告页读它）");
        assert_eq!(v["detail"]["inputFile"]["lines"], 909_465);
        assert_eq!(v["detail"]["inputFile"]["sizeBytes"], 23_758_782);
        assert_eq!(v["detail"]["estimatedPrintSeconds"], 2530.5);
        assert!(
            v["detail"]["glue"]["glueTotalLength"].is_number(),
            "三组分包落进 detail"
        );
        assert!(v["detail"]["tower"]["towerLayers"].is_number());
        assert!(v["detail"]["slicer"]["nozzleDiameter"].is_number());
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
            // 三级兜底都没拿到 ⇒ 落 null（**不是缺键**：报告页读口按同一把尺子取它）
            model_name: None,
            slicer_pid: Some(99),
            invocation: invocation(),
            warnings: Vec::new(),
            error: Some("E_GCODE_BOUNDARY_001: 越界".into()),
            cancelled: false,
            pipeline: Vec::new(),
            /* 越界失败不是碰撞 —— 那一格必须是 null（判据里钉住键名即可） */
            collision: None,
            relocation: None,
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
                print_time_status: None,
                estimated_print_seconds: None,
                glue: None,
                tower: None,
                slicer: None,
                calibration: None,
            },
        };
        write_meta(&paths, &meta).expect("写记录");
        let back: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&paths.meta).unwrap()).unwrap();
        assert_eq!(back["elapsedMs"], 1);
        assert_eq!(back["error"], "E_GCODE_BOUNDARY_001: 越界");
        assert!(
            back["modelName"].is_null(),
            "没拿到模型名 ⇒ 落 null（不是缺键），报告页那一格退回只显示 gcode 名"
        );
        assert!(
            back["detail"].get("outputFile").is_none(),
            "失败记录没有输出件"
        );
        // 失败记录连三组分包一起缺键（没有 IR 终态 ⇒ 整组不摆，不是摆一排空）
        assert!(back["detail"].get("glue").is_none());
        assert!(back["detail"].get("tower").is_none());
        assert!(back["detail"].get("slicer").is_none());
    }
}
