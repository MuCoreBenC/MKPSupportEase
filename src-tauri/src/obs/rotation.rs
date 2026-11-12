//! 日志的**双限轮转**：14 天 + 总 20 MiB。
//!
//! # 为什么还要自己算一遍
//!
//! `tracing-appender` 的 `max_log_files` 只按**份数**删，不知道一个字节能装多少：
//! 一天写 40 MB 的会话照样只留 14 份 ⇒ 盘上 500 MB。这里是那份"按字节"的闸，
//! 两条上限同时生效，谁先超就从最旧的删（与 `crate::retention` 执行记录那双限同一套思路）。
//!
//! # 三条不删
//!
//! 1. **今天那一份**：`tracing-appender` 正开着它，Windows 上删不掉，删也会把当天的现场弄丢。
//! 2. **每个前缀最新的一份**：用户停用半个月再回来，日志里至少还有最后那一次。
//! 3. 不认的名字（`logs/` 下别的东西）一个不动 —— 只认这三条流的文件名。
//!
//! # 什么时候跑
//!
//! 启动一次（`lib.rs` setup 末尾的后台线程）+ 进程内 24 小时节流（[`maybe_prune_async`]，
//! 由 `ipc::diagnostics::log_ui_events` 顺手带一次）。只在前台节奏上做 O(几十个文件) 的
//! 目录扫描，不进热路径。

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};

use time::{Date, Duration, OffsetDateTime};

/// 保留几份（按天滚动 ⇒ 约这么多天）。
pub const KEEP_DAYS: usize = 14;

/// 三条流加起来的总字节上限。
///
/// 20 MiB 的选择：正常一周的文本量约 2–5 MB（含交互记录），给到 20 MB 就有富余；
/// 而长会话那种一天几十 MB 的极端情形，靠这条把盘占用钉死 —— 天数与体积各自管一头。
pub const MAX_BYTES: u64 = 20 * 1024 * 1024;

/// 认的三个前缀（文件名 = `<前缀>.<YYYY-MM-DD>`）。
pub const PREFIXES: [&str; 3] = ["supportease.log", "ui.log", "hook.log"];

/// 两次自动轮转之间至少隔多久（秒）。
const PRUNE_INTERVAL_SECS: i64 = 24 * 60 * 60;

/// 清完那一下的账。给日志看，**不给界面看**（日志清理是静默的）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PruneReport {
    /// 留下来的份数
    pub kept: usize,
    /// 删掉的份数（真删成功的才算）
    pub removed: usize,
    /// 删掉的字节数
    pub bytes: u64,
}

/// 盘上一个日志文件。
struct Entry {
    path: PathBuf,
    /// 属于 [`PREFIXES`] 里的第几条
    prefix: usize,
    day: Date,
    bytes: u64,
}

/// 一条流在导出清单里的名字（与 [`PREFIXES`] 同序）。
const STREAMS: [&str; 3] = ["app", "ui", "hook"];

/// 盘上一份日志（给「导出日志」那扇框的清单用）。
#[derive(Debug, Clone)]
pub struct LogFile {
    /// 文件名（`ui.log.2026-11-12`），也是 zip 里的条目名
    pub name: String,
    /// `app` / `ui` / `hook`
    pub stream: &'static str,
    /// `YYYY-MM-DD`
    pub day: String,
    pub bytes: u64,
}

/// 列出现在盘上有哪些日志（按日期升序）。目录不存在 = 空清单，不是错误。
pub fn list(dir: &Path) -> Vec<LogFile> {
    let mut out: Vec<LogFile> = collect(dir)
        .into_iter()
        .map(|e| LogFile {
            name: e
                .path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            stream: STREAMS[e.prefix],
            day: format!(
                "{:04}-{:02}-{:02}",
                e.day.year(),
                u8::from(e.day.month()),
                e.day.day()
            ),
            bytes: e.bytes,
        })
        .collect();
    out.sort_by(|a, b| (&a.day, &a.name).cmp(&(&b.day, &b.name)));
    out
}

/// 文件名 → `(前缀下标, 日期)`。不认的名字给 `None`。
fn parse_name(name: &str) -> Option<(usize, Date)> {
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    PREFIXES.iter().enumerate().find_map(|(i, prefix)| {
        let rest = name.strip_prefix(prefix)?.strip_prefix('.')?;
        Date::parse(rest, &fmt).ok().map(|day| (i, day))
    })
}

/// 扫一遍目录：只收认得出的那三种名字（目录不存在 = 一份都没有，不是错误）。
fn collect(dir: &Path) -> Vec<Entry> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let (prefix, day) = parse_name(name)?;
            let bytes = entry.metadata().map(|m| m.len()).ok()?;
            Some(Entry {
                path: entry.path(),
                prefix,
                day,
                bytes,
            })
        })
        .collect()
}

/// 对一棵日志目录做双限清理（`now` 由调用方给，判据才好钉时间）。
pub fn prune_at(dir: &Path, now: OffsetDateTime) -> PruneReport {
    prune_with(dir, now, MAX_BYTES)
}

/// 同上，总量上限可传（判据用它把 20 MiB 换成几十字节）。
fn prune_with(dir: &Path, now: OffsetDateTime, max_bytes: u64) -> PruneReport {
    let entries = collect(dir);
    if entries.is_empty() {
        return PruneReport::default();
    }

    let today = now.date();
    /* 每个前缀最新的一份：停用半个月回来时，最后那一次还在 */
    let mut newest: [Option<usize>; PREFIXES.len()] = [None; PREFIXES.len()];
    for i in 0..entries.len() {
        match newest[entries[i].prefix] {
            Some(j) if entries[j].day >= entries[i].day => {}
            _ => newest[entries[i].prefix] = Some(i),
        }
    }
    let protected =
        |i: usize| -> bool { entries[i].day == today || newest[entries[i].prefix] == Some(i) };

    let oldest_allowed = today - Duration::days((KEEP_DAYS - 1) as i64);
    let mut doomed = vec![false; entries.len()];

    /* ① 天数：今天往回数 KEEP_DAYS 天那一份起还算有效 */
    for i in 0..entries.len() {
        if !protected(i) && entries[i].day < oldest_allowed {
            doomed[i] = true;
        }
    }

    /* ② 总量：还留着的加起来超了，就从最旧的往前删到装得下 */
    let mut kept_bytes: u64 = entries
        .iter()
        .enumerate()
        .filter(|(i, _)| !doomed[*i])
        .map(|(_, e)| e.bytes)
        .sum();
    if kept_bytes > max_bytes {
        let mut order: Vec<usize> = (0..entries.len()).collect();
        order.sort_by(|a, b| {
            entries[*a]
                .day
                .cmp(&entries[*b].day)
                .then_with(|| entries[*a].path.cmp(&entries[*b].path))
        });
        for i in order {
            if kept_bytes <= max_bytes {
                break;
            }
            if doomed[i] || protected(i) {
                continue;
            }
            doomed[i] = true;
            kept_bytes -= entries[i].bytes;
        }
    }

    let mut report = PruneReport {
        kept: entries.len() - doomed.iter().filter(|d| **d).count(),
        ..PruneReport::default()
    };
    for (i, entry) in entries.iter().enumerate() {
        if !doomed[i] {
            continue;
        }
        match std::fs::remove_file(&entry.path) {
            Ok(()) => {
                report.removed += 1;
                report.bytes += entry.bytes;
            }
            /* 另一个进程刚删掉（工作台与客户端共用同一个数据根）：当成功 */
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                report.removed += 1;
                report.bytes += entry.bytes;
            }
            Err(e) => tracing::warn!(
                path = %entry.path.display(),
                detail = %e,
                "日志轮转：这一份没删掉"
            ),
        }
    }
    report
}

/// 界面进程的入口：清 `<内部根>/logs`。
pub fn prune_app(app: &tauri::AppHandle) {
    let Ok(root) = crate::fsx::paths::internal_root(app) else {
        tracing::debug!("日志轮转：内部根算不出来，这次不清理");
        return;
    };
    prune_at_now(&root.join("logs"));
}

/// 清一次并记一行账（没有可删的东西就不出声）。
fn prune_at_now(dir: &Path) {
    let report = prune_at(dir, OffsetDateTime::now_utc());
    if report.removed > 0 {
        tracing::info!(
            removed = report.removed,
            kept = report.kept,
            bytes = report.bytes,
            "日志轮转：旧的日志已清掉"
        );
    }
}

/// 上次跑的时刻（UTC epoch 秒）。0 = 本进程还没跑过。
static LAST_PRUNE: AtomicI64 = AtomicI64::new(0);

/// **24 小时节流**：过期就起一个后台线程清一次，立刻返回。
///
/// 长会话不靠重启也能收敛 —— 挂在 `log_ui_events` 那条命令上，用户有动作时顺手判一次
/// （判据本身是两个原子操作，不碰盘）。
pub fn maybe_prune_async(app: &tauri::AppHandle) {
    let now = OffsetDateTime::now_utc().unix_timestamp();
    let last = LAST_PRUNE.load(Ordering::Relaxed);
    if last != 0 && now.saturating_sub(last) < PRUNE_INTERVAL_SECS {
        return;
    }
    if LAST_PRUNE
        .compare_exchange(last, now, Ordering::Relaxed, Ordering::Relaxed)
        .is_err()
    {
        return;
    }
    let handle = app.clone();
    std::thread::spawn(move || prune_app(&handle));
}

#[cfg(test)]
mod tests {
    // 判据要造真实文件树 —— 生产代码写盘一律走 `fsx::atomic`，测试里只用 `std::fs`。
    #![allow(clippy::disallowed_methods)]

    use super::*;

    fn day(s: &str) -> Date {
        let fmt = time::macros::format_description!("[year]-[month]-[day]");
        Date::parse(s, &fmt).unwrap()
    }

    /// 判据里的"现在"：某天的零点（UTC），不跟着机器时钟走。
    fn at(s: &str) -> OffsetDateTime {
        OffsetDateTime::new_utc(day(s), time::Time::MIDNIGHT)
    }

    fn write(dir: &Path, prefix: &str, at: &str, bytes: usize) -> PathBuf {
        let path = dir.join(format!("{prefix}.{at}"));
        std::fs::write(&path, vec![b'x'; bytes]).unwrap();
        path
    }

    /// 超天数的从最旧删起；落在窗口里的不动。
    #[test]
    fn files_older_than_the_window_are_dropped_oldest_first() {
        let d = tempfile::tempdir().unwrap();
        let old = write(d.path(), "supportease.log", "2026-10-01", 10);
        let edge = write(d.path(), "supportease.log", "2026-10-30", 10);
        let now_date = write(d.path(), "supportease.log", "2026-11-12", 10);

        let report = prune_at(d.path(), at("2026-11-12"));

        assert_eq!(report.removed, 1);
        assert!(!old.exists(), "超出 14 天的那一份该删");
        assert!(edge.exists(), "窗口里最新的那一份不许动");
        assert!(now_date.exists(), "今天那一份不许动");
    }

    /// 天数还宽着，但总量超了 ⇒ 从最旧删到装得下。
    #[test]
    fn the_size_cap_drops_oldest_until_it_fits() {
        let d = tempfile::tempdir().unwrap();
        let a = write(d.path(), "supportease.log", "2026-11-01", 8);
        let b = write(d.path(), "supportease.log", "2026-11-02", 8);
        let c = write(d.path(), "hook.log", "2026-11-03", 8);
        let now_date = write(d.path(), "ui.log", "2026-11-12", 8);

        /* 四份共 32 字节，上限 24：只够留三份 ⇒ 最旧的那一份走 */
        let report = prune_with(d.path(), at("2026-11-12"), 24);

        assert_eq!(report.removed, 1, "只该删最旧的那一份");
        assert!(!a.exists(), "最旧的那一份该删");
        assert!(b.exists() && c.exists() && now_date.exists());
    }

    /// **每个前缀最新的一份永远不删**：用户停用半个月，回来时最后一次照样在。
    #[test]
    fn the_newest_of_each_stream_survives_even_when_stale() {
        let d = tempfile::tempdir().unwrap();
        let stale_app = write(d.path(), "supportease.log", "2026-09-01", 10);
        let stale_ui = write(d.path(), "ui.log", "2026-09-02", 10);
        let today_hook = write(d.path(), "hook.log", "2026-11-12", 10);

        prune_at(d.path(), at("2026-11-12"));

        assert!(stale_app.exists(), "应用日志最新那一份不许删");
        assert!(stale_ui.exists(), "交互日志最新那一份不许删");
        assert!(today_hook.exists(), "今天那一份不许删");
    }

    /// 不认的名字一个不动。
    #[test]
    fn unknown_files_are_left_alone() {
        let d = tempfile::tempdir().unwrap();
        let loose = d.path().join("notes.txt");
        std::fs::write(&loose, b"keep").unwrap();
        let broken = d.path().join("ui.log.not-a-date");
        std::fs::write(&broken, b"keep").unwrap();

        let report = prune_at(
            d.path(),
            OffsetDateTime::from_unix_timestamp(1_763_000_000).unwrap(),
        );

        assert_eq!(report.removed, 0);
        assert!(loose.is_file() && broken.is_file());
    }

    /// 目录不存在 ⇒ 全零，不报错。
    #[test]
    fn a_missing_dir_is_not_an_error() {
        let d = tempfile::tempdir().unwrap();
        let report = prune_at(
            &d.path().join("logs"),
            OffsetDateTime::from_unix_timestamp(1_763_000_000).unwrap(),
        );
        assert_eq!(report, PruneReport::default());
    }

    /// 文件名解析只认 `<前缀>.<YYYY-MM-DD>`。
    #[test]
    fn only_the_three_prefixes_with_a_date_suffix_count() {
        assert_eq!(
            parse_name("ui.log.2026-11-12").map(|(_, d)| d),
            Some(day("2026-11-12"))
        );
        assert!(parse_name("ui.log").is_none());
        assert!(parse_name("ui.log.2026-11-12.bak").is_none());
        assert!(parse_name("other.log.2026-11-12").is_none());
    }
}
