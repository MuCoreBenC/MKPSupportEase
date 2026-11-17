//! **备份与恢复**（2026-10-17）：一个 zip = 这台机器上 SupportEase 的**全部家当**。
//!
//! # 备份里有什么（作者：「备份的文件需要齐全，足够，包括设置的那些，catalog 之类，
//!   toml，json」）
//!
//! 全部路径相对**内部根**（`appDataDir`）：
//!
//! ```text
//! backup-info.json          这一份备份的说明书（代次 / 时间 / 条目数）
//! catalog.json              目录（首屏唯一数据源）
//! run/app-state.json        应用状态：主题、数据源、正在用哪一份……
//! run/preset-history/*.json **修改历史**（2026-10-17：一份文件一本，git 式）
//! content/*.toml            云端各版的原文仓库（按 SHA 一份）
//! recovery/**               回收站（删除的用户预设 + 镜像 + 来源账）
//! baseline-local/**         原文镜像（这份用户文件自己最初的样子）
//! user/presets-local/**     **用户自己的预设**（全部 toml，含导入进来的）
//! user/gcode_history/**     后处理执行归档（三件套）
//! ```
//!
//! **不进备份**：`cloud/`（云端下载区的原件 —— 重新下载即可）、`archive/`（换代归档）、
//! `logs/`（日志）、`user/exports` 与 `user/reports`（生成的产物，可重出）。
//! 备份要的是"丢了就再也回不来的东西"，不是"缓存"。
//!
//! ★ 2026-11-09：跟"预设"有关的那几棵（[`PRESET_DIRS`]：用户 toml / 修改历史 / 云端原文
//! 镜像 / 原文备份 / 回收站）可以**不**进（`include_presets = false`，界面上那颗
//! 「连同预设一起备份」）。没带预设的那份备份，恢复时也不许动现在这几棵 ——
//! 判断依据是说明书里自己写的 `presets`。
//!
//! ★ 2026-11-13：执行归档（[`HISTORY_DIR`]）也可以**不**进（`include_history = false`，
//! 界面上那颗「连同执行记录一起备份」）。它是包里最大的一块（原始 G-code + 处理后的
//! G-code），通常也正是"备份慢"的来源。同一条规矩：没带执行记录的那份备份，
//! 恢复时也不许动现在这一棵 —— 判断依据是说明书里自己写的 `history`。
//!
//! # 恢复：**全量覆盖**（作者认可的第一版口径）
//!
//! 作者问「要不要选择性恢复」。这里不做，理由与成熟方案的选择一致：
//!
//! - **选择性恢复的难点是"同名文件怎么选"** —— 同名的可能是同一份的新旧版本，
//!   也可能是不同机器上的不同文件，要可靠区分只能靠来源记录 + SHA + 时间戳做冲突解决 UI，
//!   那是另外一整个界面；
//! - **成熟方案也是这两条路**：要么整机快照（Time Machine、手机厂商备份 —— 全量恢复，
//!   靠"多份快照按时间选"拿到细粒度），要么细粒度同步（git / VS Code Settings Sync ——
//!   但那要求每个文件都有稳定 id 与合并语义）。我们走前者：
//!   **多留几份备份 = 想回到哪个时间点就选哪一份**。
//!
//! 于是恢复的语义钉死为：**备份里有的一律覆盖；被管的那几棵树里、备份里没有的删掉**
//! （= 整棵换掉，回到"那一份备份那一刻"的样子）。
//!
//! # 恢复进去的文件与云端下的一样吗？还能否支持后续更新？
//!
//! 一样 —— 备份存的是**字节原样**（含来源记录 `based_on`），恢复就是原样放回去。
//! 后续更新照常：更新判据走的是来源记录与目录对齐那条正路（`local.revision != remote.revision`、
//! 随包资产兜底），恢复回来的文件带着自己的来源记录与 SHA，进预设页照常参与对齐。
//!
//! # 进度（2026-11-09）
//!
//! 两条路都收一个 [`Progress`] 回调：备份按**字节**报（分母是这几份文件的大小和），
//! 恢复按**文件条数**报（分母是说明书里自己写的份数）。这一层不认识 IPC / Channel ——
//! 谁要进度谁自己把回调接过去（见 `ipc::backup` 那两条命令）。

use postprocess::diag::messages;

use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::error::AppError;
use crate::fsx::atomic::atomic_write_stamped;

/// 备份格式的代次。加条目不升号，改语义才升
pub const BACKUP_SCHEMA: u32 = 1;

/// 备份里那份说明书的名字（恢复时第一个读它 —— 不是我们的备份就当场拒）
const INFO_FILE: &str = "backup-info.json";

/// 被管的**整棵树**（相对内部根）：恢复时这几棵**整棵换掉**
const MANAGED_DIRS: [&str; 6] = [
    "run/preset-history",
    "content",
    "recovery",
    "baseline-local",
    "user/presets-local",
    "user/gcode_history",
];

/// 被管的**单份文件**（相对内部根）：有就覆盖
const MANAGED_FILES: [&str; 3] = [
    "catalog.json",
    "run/app-state.json",
    "run/delivery-index.json",
];

/// 跟"预设"有关的那几棵（2026-11-09：界面上可以**不**带上它们）。
///
/// 口径是"用户眼里的预设库"：自己的 toml（含导入的）、这些文件的修改历史、
/// 云端原文镜像、原文备份、回收站。**执行归档**（`user/gcode_history`）不在这里 ——
/// 那是打印过程，不是预设。于是这颗开关关掉时，包里只剩"设置 + 执行归档"。
const PRESET_DIRS: [&str; 5] = [
    "user/presets-local",
    "run/preset-history",
    "content",
    "baseline-local",
    "recovery",
];

/// 这一条 zip 条目是不是落在 `PRESET_DIRS` 里（`user/presets-local/…` 这种）
fn is_preset_entry(name: &str) -> bool {
    PRESET_DIRS
        .iter()
        .any(|d| name.len() > d.len() && name.starts_with(d) && name.as_bytes()[d.len()] == b'/')
}

/// 执行归档那一棵（`user/gcode_history`）：报告页那份"历史"。
///
/// 它**不**在 [`PRESET_DIRS`] 里（那不是预设，是打印过程），但它是备份里**最大的一块**
/// —— 原始 G-code 与处理后的 G-code 都在。界面上那颗「连同执行记录一起备份」就是这一格。
const HISTORY_DIR: &str = "user/gcode_history";

/// 这一条 zip 条目是不是落在执行归档那一棵里
fn is_history_entry(name: &str) -> bool {
    name.len() > HISTORY_DIR.len()
        && name.starts_with(HISTORY_DIR)
        && name.as_bytes()[HISTORY_DIR.len()] == b'/'
}

/// 恢复这一档要不要解开这一条（三桶：预设 / 执行记录 / 设置）。
///
/// 三桶是**互斥**的：条目归一桶，就只看那一桶的开关 —— 于是"包里没有的那一块"
/// 不会被顺手清掉，也不会被当成别的桶漏进来。
fn wanted(name: &str, take_presets: bool, take_history: bool, take_settings: bool) -> bool {
    if is_preset_entry(name) {
        take_presets
    } else if is_history_entry(name) {
        take_history
    } else {
        take_settings
    }
}

/// **恢复范围**（2026-11-09）：界面恢复页那颗下拉的三档。
///
/// 只决定"动不动那一块"：选中范围内仍是**整棵换掉**（回到那一刻），范围外一个都不碰。
/// 分类只有两类 —— 预设（[`PRESET_DIRS`]）与"设置 / 记录"（其余被管的树与单文件）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    /// 设置 + 预设 + 执行记录，全都放回（原来的"全量覆盖"）
    All,
    /// 只动预设那几棵
    Presets,
    /// 只动设置与执行记录（catalog / app-state / delivery-index / 执行归档）
    Settings,
}

impl Scope {
    /// 这一档要不要动 [`PRESET_DIRS`] 那几棵
    fn presets(self) -> bool {
        matches!(self, Scope::All | Scope::Presets)
    }

    /// 这一档要不要动"设置与记录"（其余被管的树与单文件）
    fn settings(self) -> bool {
        matches!(self, Scope::All | Scope::Settings)
    }
}

/// 备份里那份说明书
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub schema: u32,
    /// 打这份备份的客户端版本（给人看；不是兼容性判据 —— 代次才是）
    pub app_version: String,
    /// 什么时候打的（UTC epoch 秒）
    pub created_unix: i64,
    /// 里面一共几个文件
    pub files: usize,
    /// 这份里有没有"用户自己的预设"（见 [`PRESET_DIRS`]）。
    ///
    /// 老备份（这一格加进来之前打的）没有它 —— 那时**一律**把预设打进去，
    /// 所以缺省是 `true`：这是那批文件的**事实**，不是拿默认值圆过去。
    #[serde(default = "included_by_default")]
    pub presets: bool,
    /// 这份里有没有"执行记录"（见 [`HISTORY_DIR`]）。缺省理由同上：这一格加进来之前
    /// 打的包一律带着执行归档，那也是那批文件的事实。
    #[serde(default = "included_by_default")]
    pub history: bool,
}

fn included_by_default() -> bool {
    true
}

/// 打完 / 恢复完的回执
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupSummary {
    /// **备份文件自己的绝对路径**（界面照原样显示给用户：他要知道东西落在哪了）
    pub path: String,
    pub files: usize,
    /// **打包前**那些文件的字节和（界面拿它说"原始多大 → 压缩后多大"）
    pub source_bytes: u64,
    /// zip 本身的字节数（恢复时 = 那份包多大）
    pub bytes: u64,
    pub created_unix: i64,
}

/// 这一趟走到哪一步了。
///
/// 只有两档：**清点 / 准备**（还没开始搬运）与**搬运**（打包是往 zip 里写、恢复是往外解）。
/// 不按"备份 / 恢复"分成两套 —— 两边要做的事一样：告诉界面"我在动，量到哪儿了"。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// 清点文件 / 认说明书 / 删旧树：总量还不知道或还在准备
    Scan,
    /// 真在搬字节 / 解条目
    Pack,
}

/// 一次进度回执（**只给界面看**，不是账本、不落盘）。
///
/// `done` / `total` 的单位**按动作定**：打包是字节数，恢复是文件条数。
/// 界面只做一件事：`total > 0` 时画百分比，否则说"正在清点"。
#[derive(Debug, Clone)]
pub struct Progress {
    pub stage: Stage,
    /// 正在处理的那一份（相对内部根；清点阶段是空串）
    pub file: String,
    pub done: u64,
    /// 总量。**清点阶段是 0**（还不知道）—— 界面据此不画百分比，不拿 0 当分母
    pub total: u64,
}

impl Progress {
    fn scanning() -> Self {
        Self {
            stage: Stage::Scan,
            file: String::new(),
            done: 0,
            total: 0,
        }
    }
}

/// 备份文件名：`SupportEase-备份-20261017-153000.zip`。
///
/// 名字里带时刻，于是**同一天打两份不会互相盖掉** —— "多留几份"是这套方案的细粒度。
fn backup_name(created: i64) -> String {
    let t = OffsetDateTime::from_unix_timestamp(created).unwrap_or(OffsetDateTime::UNIX_EPOCH);
    format!(
        "SupportEase-备份-{:04}{:02}{:02}-{:02}{:02}{:02}.zip",
        t.year(),
        u8::from(t.month()),
        t.day(),
        t.hour(),
        t.minute(),
        t.second()
    )
}

/// 一条相对路径进不进得了备份（也进不进得了恢复的目标）：**不许穿出根**。
///
/// 与用户文件那一族同一条纪律：名字是用户能改的东西，拼路径之前先过闸。
fn safe_rel(name: &str) -> Result<&str, AppError> {
    let path = Path::new(name);
    if path.is_absolute() {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.backup.352",
            &[("name", name)],
        )));
    }
    for part in path.components() {
        match part {
            Component::Normal(_) => {}
            _ => {
                return Err(AppError::invalid_argument(messages::text(
                    "ui.runtime.backup.353",
                    &[("name", name)],
                )))
            }
        }
    }
    Ok(name)
}

/// 一棵树里所有**文件**（相对内部根；目录不进 zip —— 解包时按文件路径自动建）
fn files_under(internal: &Path, rel_dir: &str, out: &mut Vec<String>) -> Result<(), AppError> {
    let dir = internal.join(rel_dir);
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in walk(&dir)? {
        let at = entry?;
        if at.is_file() {
            let rel = at
                .strip_prefix(internal)
                .map_err(|_| AppError::internal(messages::text("ui.runtime.backup.354", &[])))?;
            out.push(rel.display().to_string().replace('\\', "/"));
        }
    }
    Ok(())
}

/// 递归列目录（不引新依赖：`walkdir` 那一档不值得为这一处进树）
fn walk(dir: &Path) -> Result<Vec<std::io::Result<PathBuf>>, AppError> {
    let mut out: Vec<std::io::Result<PathBuf>> = Vec::new();
    let mut stack: Vec<PathBuf> = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        let entries = std::fs::read_dir(&at).map_err(|e| {
            AppError::io(messages::text(
                "ui.runtime.backup.355",
                &[("v1", &at.display().to_string())],
            ))
            .with_detail(e.to_string())
        })?;
        for entry in entries {
            let entry = entry.map_err(|e| {
                AppError::io(messages::text("ui.runtime.backup.356", &[]))
                    .with_detail(e.to_string())
            })?;
            let at = entry.path();
            if at.is_dir() {
                stack.push(at);
            } else {
                out.push(Ok(at));
            }
        }
    }
    Ok(out)
}

/// 搬字节时每一跳读多大（越大越省事、越小越平滑；256 KB 是"几百 MB 也不至于刷爆 IPC"那一档）
const COPY_CHUNK: usize = 256 * 1024;

/// 两次进度之间至少隔这么久（文件多的时候按"一份一跳"报，文件大时按时间来 ——
/// 不然一个 400 MB 的归档会让界面好几十秒没有动静，看着就像卡死了）。
const TICK_EVERY: Duration = Duration::from_millis(120);

/// 备份里那几种**压得动**的扩展名（其余按原样存，不做 deflate）。
///
/// 为什么按扩展名分：图片 / 3MF / zip 本来就不缩，deflate 一遍只是把时间烧在 CPU 上 ——
/// 实测 17 MB 不可压数据：Deflate 3.6 s、Stored 几十毫秒；而文本（gcode / toml / json）
/// 反过来，几 MB 能压到几百 KB。2026-11-09 作者问「17 MB 怎么要十几秒」，这就是主因之一。
const COMPRESSIBLE: [&str; 6] = ["gcode", "toml", "json", "txt", "md", "csv"];

fn compression_for(name: &str) -> zip::CompressionMethod {
    let ext = name.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    if COMPRESSIBLE.contains(&ext.as_str()) {
        zip::CompressionMethod::Deflated
    } else {
        zip::CompressionMethod::Stored
    }
}

/// 本机现在的时区偏移（解析不到按 UTC —— 与 `archive::now_local` 同一条回落）。
fn local_offset() -> time::UtcOffset {
    crate::archive::now_local().offset()
}

/// 文件 mtime → zip 的时间字段。
///
/// zip 的时间字段是**本地墙钟**、不带时区（MS-DOS 那套格式），所以先按系统时区换算
/// 再拆成 y/m/d h:m:s —— macOS 与 Windows 都走这里，不各自算一套。
/// 出了 zip 能表示的年份（1980–2107）就返回 `None`，调用方落回默认值。
fn zip_time(t: SystemTime) -> Option<zip::DateTime> {
    let local = time::OffsetDateTime::from(t).to_offset(local_offset());
    zip::DateTime::from_date_and_time(
        u16::try_from(local.year()).ok()?,
        u8::from(local.month()),
        local.day(),
        local.hour(),
        local.minute(),
        local.second(),
    )
    .ok()
}

/// zip 的时间字段 → 文件 mtime（按**本地墙钟**解释）。
///
/// `1980-01-01 00:00` 是 zip 的"没记时间"哨兵（老备份全是它）—— 认出来就返回 `None`，
/// 免得把恢复出来的文件统统按到 1980 年。
fn file_time(dt: zip::DateTime) -> Option<SystemTime> {
    if dt.year() == 1980
        && dt.month() == 1
        && dt.day() == 1
        && dt.hour() == 0
        && dt.minute() == 0
        && dt.second() == 0
    {
        return None;
    }
    let month = time::Month::try_from(dt.month()).ok()?;
    let date = time::Date::from_calendar_date(i32::from(dt.year()), month, dt.day()).ok()?;
    let clock = time::Time::from_hms(dt.hour(), dt.minute(), dt.second()).ok()?;
    Some(SystemTime::from(
        time::PrimitiveDateTime::new(date, clock).assume_offset(local_offset()),
    ))
}

/// 一条 zip 条目的选项：压缩方式按扩展名，时间用**文件自己的 mtime**
/// （量不到就落回默认 —— 那是老的 1980，不是我们编的时间）。
fn entry_options(name: &str, modified: Option<SystemTime>) -> zip::write::SimpleFileOptions {
    let opts = zip::write::SimpleFileOptions::default().compression_method(compression_for(name));
    match modified.and_then(zip_time) {
        Some(when) => opts.last_modified_time(when),
        None => opts,
    }
}

/// **打一份备份**：把上面那些树与文件收进 zip，落在 `dest_dir`。
///
/// `dest_dir` 是用户挑的文件夹（界面那侧用系统文件选择框要的），文件名由这里定 ——
/// 用户不需要想它叫什么。
///
/// `include_presets = false`：跟预设有关的那几棵（[`PRESET_DIRS`]）**不进包**。
/// 这份包恢复时也不许动它们 —— 判断依据写在说明书的 `presets` 那一格。
///
/// `include_history = false`：执行归档（[`HISTORY_DIR`]）**不进包**，恢复时同样不动
/// 现在那一棵 —— 判断依据写在说明书的 `history` 那一格。
///
/// `on_progress` 每一跳报一次（见 [`Progress`]）：打包按**字节**报，于是界面上那根条
/// 与盘上真写出去的量是同一个数，不是按文件个数估出来的。
pub fn create(
    internal: &Path,
    dest_dir: &Path,
    include_presets: bool,
    include_history: bool,
    on_progress: &mut dyn FnMut(&Progress),
) -> Result<BackupSummary, AppError> {
    let created_unix = OffsetDateTime::now_utc().unix_timestamp();
    /* 清点要一个个 stat（几千个文件时不是一瞬间）—— 先报一句，界面才不是"点了没反应" */
    on_progress(&Progress::scanning());
    let scan_start = Instant::now();

    let mut names: Vec<String> = Vec::new();
    for f in MANAGED_FILES {
        if internal.join(f).is_file() {
            names.push(f.to_owned());
        }
    }
    for d in MANAGED_DIRS {
        /* 「不连同预设 / 不连同执行记录」：那几棵整个不进（界面上那两颗开关就是这两格） */
        if !include_presets && PRESET_DIRS.contains(&d) {
            continue;
        }
        if !include_history && d == HISTORY_DIR {
            continue;
        }
        files_under(internal, d, &mut names)?;
    }
    names.sort();

    /*
     * 每份文件的 stat 只做一次：字节数（进度分母）与 mtime（zip 里那一格时间）都要它。
     * 量不到的单份按 0 / 没有时间算 —— 它照样会被打开（真读不出来会在下面报）。
     */
    let metas: Vec<Option<std::fs::Metadata>> = names
        .iter()
        .map(|n| std::fs::metadata(internal.join(n)).ok())
        .collect();
    let total_bytes: u64 = metas
        .iter()
        .map(|m| m.as_ref().map(|m| m.len()).unwrap_or(0))
        .sum();
    let scan_ms = scan_start.elapsed().as_millis();

    let dest = dest_dir.join(backup_name(created_unix));
    let info = BackupInfo {
        schema: BACKUP_SCHEMA,
        app_version: env!("CARGO_PKG_VERSION").to_owned(),
        created_unix,
        files: names.len(),
        presets: include_presets,
        history: include_history,
    };
    let info_bytes = serde_json::to_vec_pretty(&info)?;

    /*
     * 写盘仍走全仓唯一出口（`fsx::atomic`）—— 只是走**流式**那一档：
     * 备份可能几百 MB（执行归档三件套），不该先整个进内存再落。
     */
    let mut pack_ms = 0u128;
    crate::fsx::atomic::atomic_write_stream(&dest, |file| {
        let pack_start = Instant::now();
        let mut zip = zip::ZipWriter::new(file);

        zip.start_file(INFO_FILE, entry_options(INFO_FILE, Some(SystemTime::now())))
            .map_err(zip_err)?;
        zip.write_all(&info_bytes).map_err(io_err)?;
        let mut written = 0u64;
        for (i, name) in names.iter().enumerate() {
            safe_rel(name)?;
            let modified = metas[i].as_ref().and_then(|m| m.modified().ok());
            zip.start_file(name.as_str(), entry_options(name, modified))
                .map_err(zip_err)?;
            let mut reader = std::fs::File::open(internal.join(name)).map_err(|e| {
                AppError::io(messages::text(
                    "ui.runtime.backup.357",
                    &[("name", &name.to_string())],
                ))
                .with_detail(e.to_string())
            })?;
            /* `on_progress` 在这里是重借用（`&mut dyn` 不会因为传参被移走） */
            copy_into_zip(
                &mut reader,
                &mut zip,
                name,
                &mut written,
                total_bytes,
                on_progress,
            )?;
        }
        zip.finish().map_err(zip_err)?;
        pack_ms = pack_start.elapsed().as_millis();
        Ok(())
    })?;

    let bytes = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(0);
    /*
     * 三段分开记（作者 2026-11-09：「17 MB 怎么要十几秒」）：
     *   scan  = 清点 + stat；
     *   pack  = 压缩 + 写 zip；
     *   其余  = sync_all / rename（写盘落稳那一段，落在慢盘或同步盘上会明显）。
     */
    tracing::info!(
        files = names.len(),
        source_bytes = total_bytes,
        zip_bytes = bytes,
        scan_ms,
        pack_ms,
        total_ms = scan_start.elapsed().as_millis(),
        "备份耗时"
    );
    Ok(BackupSummary {
        path: dest.display().to_string(),
        files: names.len(),
        source_bytes: total_bytes,
        bytes,
        created_unix,
    })
}

/// 把一份文件搬进 zip，顺手报进度。
///
/// 不用 `std::io::copy`：它一次搬完、中间没有插话的地方，几百 MB 的那一份会让界面
/// 从头僵到尾。这里按块搬、按时间节流报；**一份搬完总是补报一次** —— 小文件靠这一下
/// 走完进度（只按时间节流会把它们全漏掉，进度条就停在原地）。
fn copy_into_zip(
    from: &mut std::fs::File,
    to: &mut zip::ZipWriter<&mut std::fs::File>,
    name: &str,
    written: &mut u64,
    total: u64,
    on_progress: &mut dyn FnMut(&Progress),
) -> Result<(), AppError> {
    let mut buf = vec![0u8; COPY_CHUNK];
    let mut last = Instant::now();
    loop {
        let n = from.read(&mut buf).map_err(|e| {
            AppError::io(messages::text("ui.runtime.backup.358", &[("name", name)]))
                .with_detail(e.to_string())
        })?;
        if n == 0 {
            break;
        }
        to.write_all(&buf[..n]).map_err(|e| {
            AppError::io(messages::text("ui.runtime.backup.358", &[("name", name)]))
                .with_detail(e.to_string())
        })?;
        *written += n as u64;
        if last.elapsed() >= TICK_EVERY {
            last = Instant::now();
            on_progress(&Progress {
                stage: Stage::Pack,
                file: name.to_owned(),
                done: *written,
                total,
            });
        }
    }
    on_progress(&Progress {
        stage: Stage::Pack,
        file: name.to_owned(),
        done: *written,
        total,
    });
    Ok(())
}

/// **从一份备份恢复**：按 [`Scope`] 只动选中的那一块。
///
/// 三步：① 认说明书（不是我们的备份 / 代次读不懂 ⇒ 拒，一个字都不动）；
/// ② 把被管的几棵树整棵删掉；③ 逐条解开覆盖回去。
///
/// ★ 先认说明书再动手 —— 中途报错会留下"恢复了一半"的现场，越早拒越好。
/// ★ 说明书说这份**没带预设**（`presets = false`）时，`PRESET_DIRS` 那几棵既不删也不碰；
///   这时还硬要「仅预设」⇒ 当场报错，不静默什么都不做。
/// ★ 说明书说这份**没带执行记录**（`history = false`）时，`user/gcode_history` 同样不删不碰
///   （没有"仅执行记录"这一档，所以这里不报错；设置那几份照常换回去）。
///
/// `on_progress` 按**文件条数**报；分母是**这一档**真正会解开的条目数（不是全包，
/// 否则按范围恢复时那条永远走不满）。
pub fn restore(
    internal: &Path,
    zip_path: &Path,
    scope: Scope,
    on_progress: &mut dyn FnMut(&Progress),
) -> Result<BackupSummary, AppError> {
    on_progress(&Progress::scanning());
    let file = std::fs::File::open(zip_path).map_err(|e| {
        AppError::not_found(messages::text(
            "ui.runtime.backup.359",
            &[("v1", &zip_path.display().to_string())],
        ))
        .with_detail(e.to_string())
    })?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_err)?;

    /* ① 认说明书 */
    let info_text = {
        let mut f = archive
            .by_name(INFO_FILE)
            .map_err(|_| AppError::corrupted(messages::text("ui.runtime.backup.360", &[])))?;
        let mut s = String::new();
        f.read_to_string(&mut s).map_err(io_err)?;
        s
    };
    let info: BackupInfo = serde_json::from_str(&info_text).map_err(|e| {
        AppError::corrupted(messages::text("ui.runtime.backup.361", &[])).with_detail(e.to_string())
    })?;
    if info.schema != BACKUP_SCHEMA {
        return Err(AppError::corrupted(messages::text(
            "ui.runtime.backup.362",
            &[("v1", &info.schema.to_string())],
        )));
    }

    /*
     * **更新的版本打的包，老客户端不许恢复**（2026-11-09 作者问「旧版备份 + 新版软件怎么办」）。
     *
     * 反方向是安全的：新客户端读旧数据有兜底 —— `app-state` 每一格都是 `Option` +
     * `serde(default)`（缺的那一段 = 没设过，走默认），预设文件是字节原样。
     * 但老客户端读新数据没有兜底（新字段、新的 catalog 代次它不认识），
     * 所以只拦这一头，并让用户先升级。
     */
    let current = env!("CARGO_PKG_VERSION");
    if crate::runtime::release_info::is_newer_version(&info.app_version, current) {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.backup.369",
            &[("v1", &info.app_version), ("v2", current)],
        )));
    }

    /*
     * 要预设、可这份当初就没带 ⇒ **明说**（不静默什么都不做 —— 那让人以为恢复了）。
     */
    if scope == Scope::Presets && !info.presets {
        return Err(AppError::invalid_argument(messages::text(
            "ui.runtime.backup.368",
            &[],
        )));
    }

    /*
     * 这一趟动不动那三块。`take_presets` / `take_history` 还要看说明书：
     * **没带预设 / 执行记录**的包，就算选「全部」也不许动那几棵 —— 用户当初就是不想
     * 把它们卷进来，恢复时更不该顺手清掉（那会把"没备份的"变成"删掉的"）。
     */
    let take_presets = scope.presets() && info.presets;
    let take_settings = scope.settings();
    let take_history = take_settings && info.history;

    /* ② 整棵换掉（"回到那一刻"的语义）：只换这一档管的那几棵 */
    for d in MANAGED_DIRS {
        let want = if PRESET_DIRS.contains(&d) {
            take_presets
        } else if d == HISTORY_DIR {
            take_history
        } else {
            take_settings
        };
        if !want {
            continue;
        }
        let at = internal.join(d);
        if at.is_dir() {
            std::fs::remove_dir_all(&at).map_err(|e| {
                AppError::io(messages::text("ui.runtime.backup.363", &[("d", d)]))
                    .with_detail(e.to_string())
            })?;
        }
    }

    /*
     * ③ 先数分母：**这一档**真正会解开的条目数。
     *
     * 不用说明书里的 `files` —— 那是全包的份数，按范围恢复时那条会永远走不满。
     * 这一趟只读 zip 头（名字 / 是不是目录），不解压、不进内存。
     */
    let mut total_files = 0u64;
    for i in 0..archive.len() {
        let entry = archive.by_index(i).map_err(zip_err)?;
        let name = entry.name();
        if name == INFO_FILE || entry.is_dir() {
            continue;
        }
        if wanted(name, take_presets, take_history, take_settings) {
            total_files += 1;
        }
    }

    /* ④ 逐条解开（只解这一档管的那些；范围外一个都不碰） */
    let mut count = 0usize;
    let mut source_bytes = 0u64;
    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(zip_err)?;
        let name = entry.name().to_owned();
        if name == INFO_FILE || entry.is_dir() {
            continue;
        }
        if !wanted(&name, take_presets, take_history, take_settings) {
            continue;
        }
        let rel = safe_rel(&name)?;
        let target = internal.join(rel);
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                AppError::io(messages::text(
                    "ui.runtime.backup.364",
                    &[("v1", &parent.display().to_string())],
                ))
                .with_detail(e.to_string())
            })?;
        }
        /* 备份里记着这一份的时间（老备份是 1980 哨兵 ⇒ `None`，那就按写入时刻） */
        let modified = entry.last_modified().and_then(file_time);
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).map_err(io_err)?;
        /* 写盘仍走唯一出口；时间在 persist 之前钉上（见 `atomic_write_stamped`） */
        atomic_write_stamped(&target, &bytes, modified)?;
        source_bytes += bytes.len() as u64;
        count += 1;
        on_progress(&Progress {
            stage: Stage::Pack,
            file: rel.to_owned(),
            done: count as u64,
            total: total_files,
        });
    }

    Ok(BackupSummary {
        path: zip_path.display().to_string(),
        files: count,
        source_bytes,
        bytes: std::fs::metadata(zip_path).map(|m| m.len()).unwrap_or(0),
        created_unix: info.created_unix,
    })
}

/// 读写出错（zip 里的条目也是普通 IO）—— 与下面的"格式读不动"分开：
/// 前者是"这份盘 / 这次写入有问题"，后者是"这根本不是一份能读的 zip"
fn io_err(e: std::io::Error) -> AppError {
    AppError::io(messages::text("ui.runtime.backup.365", &[])).with_detail(e.to_string())
}

fn zip_err(e: zip::result::ZipError) -> AppError {
    match e {
        zip::result::ZipError::Io(e) => {
            AppError::io(messages::text("ui.runtime.backup.366", &[])).with_detail(e.to_string())
        }
        other => AppError::corrupted(messages::text("ui.runtime.backup.367", &[]))
            .with_detail(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /* 只有测试会直接铺原始文件（产品代码一律走 create / restore 那两条） */
    use crate::fsx::atomic::atomic_write;

    fn seed(internal: &Path) {
        atomic_write(&internal.join("catalog.json"), b"{\"catalogSchema\":1}").unwrap();
        atomic_write(
            &internal.join("run/app-state.json"),
            b"{\"theme\":\"dark\"}",
        )
        .unwrap();
        atomic_write(
            &internal.join("run/preset-history/abc.json"),
            b"{\"schema\":1,\"batches\":2,\"entries\":[]}",
        )
        .unwrap();
        atomic_write(
            &internal.join("user/presets-local/A1 涂胶.toml"),
            b"# machine: A1\n[toolhead]\noffset_x = -0.7\n",
        )
        .unwrap();
        /* 执行归档：报告页那一条（三件套里最小的一份够用了） */
        atomic_write(
            &internal.join("user/gcode_history/2026-11-10/23295.4_20261110_171911_meta.json"),
            b"{\"ok\":true}",
        )
        .unwrap();
        /* 不该进备份的那些 */
        atomic_write(&internal.join("cloud/A1-standard.toml"), b"# cloud\n").unwrap();
        atomic_write(&internal.join("logs/app.log"), b"noise\n").unwrap();
    }

    /// 备份里**有**该有的（目录 / 状态 / 历史 / 用户 toml / 执行记录），**没有**可重建的那些
    #[test]
    fn a_backup_carries_everything_that_cannot_be_rebuilt() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);

        let out = d.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let sum = create(&internal, &out, true, true, &mut |_| {}).unwrap();

        let mut names = Vec::new();
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&sum.path).unwrap()).unwrap();
        for i in 0..archive.len() {
            names.push(archive.by_index(i).unwrap().name().to_owned());
        }
        for want in [
            "backup-info.json",
            "catalog.json",
            "run/app-state.json",
            "run/preset-history/abc.json",
            "user/presets-local/A1 涂胶.toml",
            "user/gcode_history/2026-11-10/23295.4_20261110_171911_meta.json",
        ] {
            assert!(
                names.contains(&want.to_owned()),
                "备份里该有 {want}：{names:?}"
            );
        }
        for bad in ["cloud/A1-standard.toml", "logs/app.log"] {
            assert!(!names.iter().any(|n| n == bad), "不该进备份：{bad}");
        }
        assert!(sum.path.ends_with(".zip"));
    }

    /// 同一天打两份不会互相盖掉（名字里带时刻）—— 这是"多留几份"能成立的前提
    #[test]
    fn two_backups_in_the_same_minute_do_not_overwrite_each_other() {
        assert_ne!(backup_name(1_700_000_000), backup_name(1_700_000_060));
    }

    /// **恢复 = 全量覆盖**：坏掉的现状被换回备份那一刻的样子，
    /// 且**备份里没有的那一份会被删掉**（整棵换掉才是"回到那一刻"）
    #[test]
    fn restoring_puts_back_what_the_backup_had_and_drops_the_rest() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let zip_at = {
            let out = d.path().join("out");
            std::fs::create_dir_all(&out).unwrap();
            create(&internal, &out, true, true, &mut |_| {})
                .unwrap()
                .path
        };

        /* 现状被搞坏 / 多了东西 */
        atomic_write(&internal.join("catalog.json"), b"{\"catalogSchema\":9}").unwrap();
        atomic_write(
            &internal.join("user/presets-local/多出来的.toml"),
            "# 后来加的\n".as_bytes(),
        )
        .unwrap();
        atomic_write(
            &internal.join("user/gcode_history/2026-11-11/多出来的一条_meta.json"),
            b"{\"ok\":true}",
        )
        .unwrap();

        let sum = restore(
            &internal,
            std::path::Path::new(&zip_at),
            Scope::All,
            &mut |_| {},
        )
        .unwrap();
        assert!(sum.files >= 4);

        let catalog = std::fs::read_to_string(internal.join("catalog.json")).unwrap();
        assert!(
            catalog.contains("\"catalogSchema\":1"),
            "覆盖回备份那一刻：{catalog}"
        );
        assert!(
            !internal.join("user/presets-local/多出来的.toml").exists(),
            "备份里没有的那一份要跟着删掉（整棵换掉）"
        );
        assert!(
            internal.join("user/presets-local/A1 涂胶.toml").is_file(),
            "备份里有的那份得回来"
        );
        assert!(
            internal.join("run/preset-history/abc.json").is_file(),
            "修改历史跟着备份一起回来"
        );
        assert!(
            !internal
                .join("user/gcode_history/2026-11-11/多出来的一条_meta.json")
                .exists(),
            "执行记录那一棵也是整棵换掉"
        );
        assert!(
            internal
                .join("user/gcode_history/2026-11-10/23295.4_20261110_171911_meta.json")
                .is_file(),
            "备份里的执行记录得回来"
        );
    }

    /// 「不连同预设」：那两棵不进包；拿这份恢复时，现在的预设也**原样不动**
    #[test]
    fn a_backup_can_leave_presets_out_and_restore_keeps_them() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let out = d.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let sum = create(&internal, &out, false, true, &mut |_| {}).unwrap();

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&sum.path).unwrap()).unwrap();
        let entries = archive.len();
        let names: Vec<String> = (0..entries)
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect();
        assert!(names.contains(&"catalog.json".to_owned()), "该有的还在");
        assert!(
            !names.iter().any(|n| is_preset_entry(n)),
            "开关关了就不该有预设：{names:?}"
        );

        /* 打完这份之后用户又写了新预设 —— 恢复不许碰它们 */
        atomic_write(
            &internal.join("user/presets-local/后来写的.toml"),
            b"# later\n",
        )
        .unwrap();
        restore(
            &internal,
            std::path::Path::new(&sum.path),
            Scope::All,
            &mut |_| {},
        )
        .unwrap();
        assert!(
            internal.join("user/presets-local/后来写的.toml").is_file(),
            "没带预设的备份，恢复时不许动预设"
        );
        assert!(
            internal.join("user/presets-local/A1 涂胶.toml").is_file(),
            "原有的那份也还在"
        );
    }

    /// 「不连同执行记录」：那一棵不进包；拿这份恢复时，现在的执行记录也**原样不动**
    #[test]
    fn a_backup_can_leave_runs_out_and_restore_keeps_them() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let out = d.path().join("out");
        std::fs::create_dir_all(&out).unwrap();
        let sum = create(&internal, &out, true, false, &mut |_| {}).unwrap();

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&sum.path).unwrap()).unwrap();
        let names: Vec<String> = (0..archive.len())
            .map(|i| archive.by_index(i).unwrap().name().to_owned())
            .collect();
        assert!(names.contains(&"catalog.json".to_owned()), "该有的还在");
        assert!(
            !names.iter().any(|n| is_history_entry(n)),
            "开关关了就不该有执行记录：{names:?}"
        );

        /* 打完这份之后钩子又写了一条 —— 恢复不许碰执行记录那一棵 */
        atomic_write(
            &internal.join("user/gcode_history/2026-11-11/后来跑的一条_meta.json"),
            b"{\"ok\":true}",
        )
        .unwrap();
        restore(
            &internal,
            std::path::Path::new(&sum.path),
            Scope::All,
            &mut |_| {},
        )
        .unwrap();
        assert!(
            internal
                .join("user/gcode_history/2026-11-11/后来跑的一条_meta.json")
                .is_file(),
            "没带执行记录的备份，恢复时不许动它"
        );
        assert!(
            internal
                .join("user/gcode_history/2026-11-10/23295.4_20261110_171911_meta.json")
                .is_file(),
            "原有的那条也还在"
        );
    }

    /// 「仅预设」：只换预设那几棵，设置 / 目录一个都不碰
    #[test]
    fn restoring_only_presets_leaves_settings_alone() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let zip_at = {
            let out = d.path().join("out");
            std::fs::create_dir_all(&out).unwrap();
            create(&internal, &out, true, true, &mut |_| {})
                .unwrap()
                .path
        };

        /* 备份之后：设置被改坏、又多写了一份预设 */
        atomic_write(&internal.join("catalog.json"), b"{\"catalogSchema\":9}").unwrap();
        atomic_write(
            &internal.join("user/presets-local/后来写的.toml"),
            b"# later\n",
        )
        .unwrap();

        let sum = restore(
            &internal,
            std::path::Path::new(&zip_at),
            Scope::Presets,
            &mut |_| {},
        )
        .unwrap();
        assert!(sum.files >= 2, "至少那两份预设条目");

        /* 预设整棵换回那一刻：后来写的没了、原来那份回来 */
        assert!(
            !internal.join("user/presets-local/后来写的.toml").exists(),
            "选中范围内仍是整棵换掉"
        );
        assert!(internal.join("user/presets-local/A1 涂胶.toml").is_file());
        /* 设置没被碰：坏掉的目录还是坏的 */
        let catalog = std::fs::read_to_string(internal.join("catalog.json")).unwrap();
        assert!(
            catalog.contains("\"catalogSchema\":9"),
            "选「仅预设」时不许动设置：{catalog}"
        );
    }

    /// 「仅设置」：只换设置与执行记录，预设原样不动
    #[test]
    fn restoring_only_settings_leaves_presets_alone() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let zip_at = {
            let out = d.path().join("out");
            std::fs::create_dir_all(&out).unwrap();
            create(&internal, &out, true, true, &mut |_| {})
                .unwrap()
                .path
        };

        /* 备份之后：设置被改坏、又多写了一份预设 */
        atomic_write(&internal.join("catalog.json"), b"{\"catalogSchema\":9}").unwrap();
        atomic_write(
            &internal.join("user/presets-local/后来写的.toml"),
            b"# later\n",
        )
        .unwrap();

        restore(
            &internal,
            std::path::Path::new(&zip_at),
            Scope::Settings,
            &mut |_| {},
        )
        .unwrap();

        /* 设置回到备份那一刻 */
        let catalog = std::fs::read_to_string(internal.join("catalog.json")).unwrap();
        assert!(
            catalog.contains("\"catalogSchema\":1"),
            "「仅设置」要把目录换回来：{catalog}"
        );
        /* 预设一个都没动：后来写的与原来那份都在 */
        assert!(internal.join("user/presets-local/后来写的.toml").is_file());
        assert!(internal.join("user/presets-local/A1 涂胶.toml").is_file());
        assert!(internal.join("run/preset-history/abc.json").is_file());
    }

    /// 没带预设的包还要「仅预设」⇒ 当场报错，一个字都不动（不静默当成功）
    #[test]
    fn asking_for_presets_from_a_backup_without_them_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let zip_at = {
            let out = d.path().join("out");
            std::fs::create_dir_all(&out).unwrap();
            create(&internal, &out, false, true, &mut |_| {})
                .unwrap()
                .path
        };

        let err = restore(
            &internal,
            std::path::Path::new(&zip_at),
            Scope::Presets,
            &mut |_| {},
        )
        .expect_err("没带预设还要仅预设，该拒");
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert!(
            internal.join("user/presets-local/A1 涂胶.toml").is_file(),
            "拒了就不许动预设"
        );
    }

    /// 每份文件的时间进 zip、恢复时再放回去（作者 2026-11-09：文件时间全是 1980）
    #[test]
    fn each_entry_keeps_its_own_time_and_restore_puts_it_back() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);

        /* 给一份文件钉一个"本地墙钟"的时间（zip 只有 2 秒精度，秒取偶数） */
        let want = {
            let date = time::Date::from_calendar_date(2026, time::Month::January, 2).unwrap();
            let clock = time::Time::from_hms(3, 4, 6).unwrap();
            SystemTime::from(
                time::PrimitiveDateTime::new(date, clock).assume_offset(local_offset()),
            )
        };
        /* 铺一份带时间的：与恢复走同一条出口（`atomic_write_stamped`） */
        atomic_write_stamped(
            &internal.join("catalog.json"),
            b"{\"catalogSchema\":1}",
            Some(want),
        )
        .unwrap();

        let zip_at = {
            let out = d.path().join("out");
            std::fs::create_dir_all(&out).unwrap();
            create(&internal, &out, true, true, &mut |_| {})
                .unwrap()
                .path
        };

        {
            let mut archive = zip::ZipArchive::new(std::fs::File::open(&zip_at).unwrap()).unwrap();
            let entry = archive.by_name("catalog.json").unwrap();
            let dt = entry.last_modified().expect("这一条该带时间");
            assert_eq!(
                (dt.year(), dt.month(), dt.day(), dt.hour(), dt.minute()),
                (2026, 1, 2, 3, 4),
                "zip 里记的应是**本地墙钟**"
            );
        }

        restore(
            &internal,
            std::path::Path::new(&zip_at),
            Scope::All,
            &mut |_| {},
        )
        .unwrap();
        let got = std::fs::metadata(internal.join("catalog.json"))
            .unwrap()
            .modified()
            .unwrap();
        let delta = got.duration_since(want).unwrap_or_else(|e| e.duration());
        assert!(
            delta < Duration::from_secs(2),
            "文件时间没还原：差 {delta:?}"
        );
    }

    /// 更新的版本打的包 ⇒ 老客户端拒绝恢复（先升级再说）
    #[test]
    fn a_backup_from_a_newer_version_is_refused() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);

        let zip_at = d.path().join("未来的.zip");
        {
            #[allow(clippy::disallowed_methods)]
            let f = std::fs::File::create(&zip_at).unwrap();
            let mut zip = zip::ZipWriter::new(f);
            let opts = zip::write::SimpleFileOptions::default();
            zip.start_file(INFO_FILE, opts).unwrap();
            zip.write_all(
                br#"{"schema":1,"appVersion":"99.0.0","createdUnix":0,"files":0,"presets":true}"#,
            )
            .unwrap();
            zip.finish().unwrap();
        }

        let err =
            restore(&internal, &zip_at, Scope::All, &mut |_| {}).expect_err("更新的版本，该拒");
        assert_eq!(err.code, crate::error::ErrorCode::InvalidArgument);
        assert!(
            std::fs::read_to_string(internal.join("catalog.json"))
                .unwrap()
                .contains("\"catalogSchema\":1"),
            "拒了就一个字都不许动"
        );
    }

    /// 不是我们的备份 ⇒ 一个字都不动（先认说明书再动手）
    #[test]
    fn a_zip_that_is_not_ours_is_refused_before_anything_is_touched() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let foreign = d.path().join("别的.zip");
        {
            /* 测试造坏档 / 造外来 zip，不走 atomic（不是产品数据落点） */
            #[allow(clippy::disallowed_methods)]
            let f = std::fs::File::create(&foreign).unwrap();
            let mut zip = zip::ZipWriter::new(f);
            zip.start_file("whatever.txt", zip::write::SimpleFileOptions::default())
                .unwrap();
            zip.write_all(b"hi").unwrap();
            zip.finish().unwrap();
        }
        let err = restore(&internal, &foreign, Scope::All, &mut |_| {}).expect_err("该拒");
        assert_eq!(err.code, crate::error::ErrorCode::Corrupted);
        assert!(
            std::fs::read_to_string(internal.join("catalog.json"))
                .unwrap()
                .contains("\"catalogSchema\":1"),
            "拒了就一个字都不许动"
        );
    }

    /// 备份会一跳一跳报进度，**最后那一跳走满总量**（界面那根条就是这么来的）
    #[test]
    fn a_backup_reports_progress_that_ends_at_the_total() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let out = d.path().join("out");
        std::fs::create_dir_all(&out).unwrap();

        let mut ticks: Vec<Progress> = Vec::new();
        create(&internal, &out, true, true, &mut |p| ticks.push(p.clone())).unwrap();

        assert!(
            ticks.iter().any(|p| p.stage == Stage::Scan),
            "清点要有一跳（界面据此说\"正在清点\"，不是 0%）"
        );
        let packed: Vec<&Progress> = ticks.iter().filter(|p| p.stage == Stage::Pack).collect();
        assert!(!packed.is_empty(), "打包阶段一条都没有");
        let last = packed.last().expect("上面判过非空");
        assert!(last.total > 0, "分母是这几份文件的大小和");
        assert_eq!(
            last.done, last.total,
            "最后一跳要走满（盘上那个包确实写完了）"
        );
    }

    /// 恢复也报进度：一条文件一跳，最后一条走满说明书里的份数
    #[test]
    fn a_restore_reports_one_tick_per_file() {
        let d = tempfile::tempdir().unwrap();
        let internal = d.path().join("appdata");
        seed(&internal);
        let zip_at = {
            let out = d.path().join("out");
            std::fs::create_dir_all(&out).unwrap();
            create(&internal, &out, true, true, &mut |_| {})
                .unwrap()
                .path
        };

        let mut ticks: Vec<Progress> = Vec::new();
        let sum = restore(
            &internal,
            std::path::Path::new(&zip_at),
            Scope::All,
            &mut |p| ticks.push(p.clone()),
        )
        .unwrap();

        let packed: Vec<&Progress> = ticks.iter().filter(|p| p.stage == Stage::Pack).collect();
        assert_eq!(packed.len(), sum.files, "一份一跳");
        let last = packed.last().expect("至少一份");
        assert_eq!(last.done, sum.files as u64);
        assert_eq!(last.total, sum.files as u64);
    }

    /// 穿出根的那种路径不许进来（名字是用户能改的东西）
    #[test]
    fn a_path_that_escapes_the_root_is_refused() {
        assert!(safe_rel("../../etc/passwd").is_err());
        assert!(safe_rel("/etc/passwd").is_err());
        assert!(safe_rel("user/presets-local/A1.toml").is_ok());
    }
}
