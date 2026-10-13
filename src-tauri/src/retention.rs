//! 后处理执行记录的**自动清理**（2026-10-13「设置里加一个自动清理开关」那一刀）。
//!
//! # 清的什么
//!
//! 只清 [`crate::archive`] 那棵三件套树（`<用户根>/gcode_history`）里的**整条记录**：
//! `_original.gcode` + `.gcode` + `_meta.json` 一起删。只删大的 gcode 而留 `_meta.json`
//! 会让报告页出现"列表里还有、导出却取不到文件"的坏状态 —— 整条删，列表才是一致的
//! （被清的记录直接从报告页消失，这是**预期**，不是故障）。
//!
//! # 按什么清：双限
//!
//! [`Retention`] 两条上限**同时**生效 —— **条数**与**总占用**，谁先超就删最旧的，
//! 删到两条都满足。新的永远先留。
//!
//! ★ **最新那条永远不删**，哪怕它自己就超了空间上限。这一条是作者 2026-10-13 点名要的：
//! **不许按日历天数清** —— 用户停用一个月再回来，他最后一次跑出来的那条记录必须还在。
//! 纯"超过 N 天就删"正是会犯这个错（那一个月里没有新记录，清一次就把最新的一条带走）。
//!
//! # 静默
//!
//! 清理**不向用户报任何东西**：不弹窗、不提示、不进后处理结果的 `warnings`、不改退出码。
//! 失败（目录读不到 / 某个文件删不掉）只写日志，下一次再来。
//! 两个触发点因此可以并存：启动一次（`lib.rs` setup 末尾，后台线程）+ 每次后处理跑完一次
//! （`hook.rs` 归档之后）。两处撞上也只是"同一个文件删两次"—— `NotFound` 一律当成功。
//!
//! # 排序键
//!
//! 用**文件名尾部的时刻**（`<YYYYMMDD_HHMMSS>`）+ 日期目录，**不读 `_meta.json`**：
//! 三件套的名字本来就带时刻（[`crate::archive::base_path`]），为了拿一个时间把几十份
//! 26MB 级的记录读一遍是纯浪费。解析不出时刻的怪名字垫底（排最后 = 最先被删）。

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::error::AppError;

/// 条数上限：保留最近这么多条。作者 2026-10-13 定的一档（"更激进，省空间优先"）。
pub const MAX_RECORDS: usize = 20;

/// 总占用上限：约 250 MiB。与 [`MAX_RECORDS`] **同时**生效，谁先超删谁。
///
/// 条数管"可回查的份数"、空间管"最坏情况下占多少盘"——只留条数的话，遇上几个
/// 特别大的模型照样能把盘吃掉；只留空间的话，条目大小一变，能回查的份数就没准了。
pub const MAX_BYTES: u64 = 250 * 1024 * 1024;

/// 双限保留策略。阈值是**内部常量**，不出现在界面（界面上只有一个开关）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Retention {
    pub max_records: usize,
    pub max_bytes: u64,
}

impl Default for Retention {
    fn default() -> Self {
        Self {
            max_records: MAX_RECORDS,
            max_bytes: MAX_BYTES,
        }
    }
}

/// 清完那一下的账。给日志与判据看，**不给界面看**（静默）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PruneReport {
    /// 留下来的记录条数
    pub kept_records: usize,
    /// 清掉的记录条数（整条三件套都删成功的才算）
    pub removed_records: usize,
    /// 清掉的字节数（按盘上那些文件的大小算）
    pub removed_bytes: u64,
}

/// 一条记录在盘上的样子：同一条三件套的所有文件。
struct Record {
    /// 排序键：`(日期目录, 文件名时刻, 基名)`，字典序 = 时间序；
    /// 最后那个基名只是"同一秒里跑了两趟"的稳定断分。
    key: (String, String, String),
    files: Vec<(PathBuf, u64)>,
    bytes: u64,
}

/// 基名尾部的时刻（[`crate::archive::file_stamp`] 的形状：`YYYYMMDD_HHMMSS`）。
///
/// 从后面切三段，所以 `<名>_20261009_220851` 这种带下划线的名字也能认；
/// 认不出 ⇒ `None`（怪名字没有时间，垫底 = 最先被删）。
fn stamp_of(base: &str) -> Option<String> {
    let mut parts = base.rsplitn(3, '_');
    let seconds = parts.next()?;
    let date = parts.next()?;
    let digits = |s: &str, n: usize| s.len() == n && s.bytes().all(|b| b.is_ascii_digit());
    (digits(date, 8) && digits(seconds, 6)).then(|| format!("{date}_{seconds}"))
}

/// 扫一棵 `gcode_history` 树，把散文件**按记录归组**。
///
/// 只认日期子目录里、名字属于三件套的那些（与报告页读口同一层）——
/// 根下直接躺的杂物、日期目录里的 `notes.txt` 一概不认。
fn collect(history_root: &Path) -> Vec<Record> {
    let mut grouped: BTreeMap<(String, String, String), Vec<(PathBuf, u64)>> = BTreeMap::new();
    /* 树还没有 = 一条记录都没有（不是错误，与 `ipc::report::collect` 同一口径） */
    let Ok(days) = std::fs::read_dir(history_root) else {
        return Vec::new();
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
            let file_name = entry.file_name();
            let Some(name) = file_name.to_str() else {
                continue;
            };
            let Some(base) = crate::archive::record_base(name) else {
                continue;
            };
            /* 只取大小，不读内容 —— 26MB 一份的记录读一遍就为了数个数，不值 */
            let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
            grouped
                .entry((
                    day_name.clone(),
                    stamp_of(base).unwrap_or_default(),
                    base.to_owned(),
                ))
                .or_default()
                .push((entry.path(), size));
        }
    }
    grouped
        .into_iter()
        .map(|(key, files)| Record {
            key,
            bytes: files.iter().map(|(_, size)| *size).sum(),
            files,
        })
        .collect()
}

/// 对一棵 `gcode_history` 树做双限清理：**新的先留**，条数或总占用任一超标就把更旧的
/// 整条三件套删掉，最后回收被清空的日期目录。
///
/// 树不存在 ⇒ `Ok` 全零（还没有记录不是错误）。
pub fn prune(history_root: &Path, policy: Retention) -> Result<PruneReport, AppError> {
    let mut records = collect(history_root);
    /* 新的在前：从头往下留，一旦超限，后面剩下的全是更旧的 */
    records.sort_by(|a, b| b.key.cmp(&a.key));

    let mut report = PruneReport::default();
    let mut kept_bytes: u64 = 0;
    /* 删过文件的日期目录：最后统一试一次回收（还留着记录的会失败，属正常） */
    let mut touched: BTreeMap<PathBuf, ()> = BTreeMap::new();

    for record in &records {
        /*
         * ★ `kept_records > 0` 就是"最新那条永远不删"那一条规矩 ——
         * 连它自己都超空间上限时，也留着它（等将来有更新的记录，它自然会被清掉）。
         */
        let over = report.kept_records > 0
            && (report.kept_records >= policy.max_records
                || kept_bytes.saturating_add(record.bytes) > policy.max_bytes);
        if !over {
            report.kept_records += 1;
            kept_bytes = kept_bytes.saturating_add(record.bytes);
            continue;
        }

        let mut all_gone = true;
        let mut gone_bytes: u64 = 0;
        for (path, size) in &record.files {
            match std::fs::remove_file(path) {
                Ok(()) => gone_bytes += size,
                /* 另一个进程刚删掉（启动那次与跑完那次撞上）：**当成功**，不报错 */
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => gone_bytes += size,
                Err(e) => {
                    all_gone = false;
                    tracing::warn!(
                        path = %path.display(),
                        detail = %e,
                        "自动清理：这个文件没删掉"
                    );
                }
            }
            if let Some(dir) = path.parent() {
                touched.insert(dir.to_path_buf(), ());
            }
        }
        /* 整条都真删掉了才记这一笔账（删一半的记录不冒充"已清"） */
        if all_gone {
            report.removed_records += 1;
            report.removed_bytes += gone_bytes;
        }
    }

    /* 清空的日期目录回收。非空（还留着记录）/ 已不在都当没事 —— 只是"回收拾" */
    for dir in touched.keys() {
        if let Err(e) = std::fs::remove_dir(dir) {
            tracing::debug!(dir = %dir.display(), detail = %e, "日期目录没回收（还留着记录或已不在）");
        }
    }

    Ok(report)
}

/// 界面进程（有 `AppHandle`）的入口：读开关 → 开着才清。
///
/// ★ 调用方放**后台线程**里跑（别挡首屏；启动那一趟见 `lib.rs` setup 末尾）。
pub fn prune_if_enabled_app(app: &tauri::AppHandle) {
    let settings_root = match crate::fsx::paths::internal_root(app) {
        Ok(root) => root,
        Err(e) => {
            tracing::warn!(detail = %e, "自动清理：内部根没算出来，这次不清理");
            return;
        }
    };
    let history_root = match crate::fsx::paths::user_root(app) {
        Ok(user) => user.join(crate::fsx::paths::GCODE_HISTORY_DIR),
        Err(e) => {
            tracing::warn!(detail = %e, "自动清理：用户根没算出来，这次不清理");
            return;
        }
    };
    run(&settings_root, &history_root);
}

/// 钩子进程（无 `AppHandle`）的入口：同一套判据走 headless 根。
///
/// 在 [`crate::hook::run`] 归档之后调 —— 刚写下的那条是最新的，必被保留。
pub fn prune_if_enabled_headless() {
    let Some(settings_root) = crate::fsx::paths::internal_root_headless() else {
        tracing::debug!("自动清理：这台机器上算不出数据根，这次不清理");
        return;
    };
    let Some(history_root) = crate::archive::history_root() else {
        tracing::debug!("自动清理：这台机器上算不出归档根，这次不清理");
        return;
    };
    run(&settings_root, &history_root);
}

/// 读开关（**没写过 = 默认开**）→ 开着就清一次。全程静默，只写日志。
fn run(settings_root: &Path, history_root: &Path) {
    match crate::runtime::app_state::auto_clean_history(settings_root) {
        Ok(true) => {}
        Ok(false) => return,
        Err(e) => {
            tracing::warn!(detail = %e, "自动清理：开关读不出来，这次不清理");
            return;
        }
    }
    match prune(history_root, Retention::default()) {
        Ok(report) if report.removed_records > 0 => tracing::info!(
            removed = report.removed_records,
            kept = report.kept_records,
            bytes = report.removed_bytes,
            "自动清理：旧执行记录已清掉"
        ),
        Ok(_) => {}
        Err(e) => tracing::warn!(detail = %e, "自动清理：这次没跑成（不影响使用）"),
    }
}

#[cfg(test)]
mod tests {
    // 判据要造真实文件树 —— 生产代码的写盘一律走 `fsx::atomic` 那一个洞，
    // 这里的 `std::fs::write` 只活在测试里。
    #![allow(clippy::disallowed_methods)]

    use super::*;

    /// 一条记录的基名：`<名>_<YYYYMMDD>_<HHMMSS>`（形状与 `archive::base_path` 一致）。
    fn base(name: &str, day: &str, hhmmss: &str) -> String {
        format!("{name}_{}_{hhmmss}", day.replace('-', ""))
    }

    /// 造一条记录：两个 gcode 各 `payload` 字节 + 一份 `_meta.json`。
    fn write_record(root: &Path, day: &str, base: &str, payload: usize) {
        let dir = root.join(day);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join(format!("{base}_original.gcode")),
            vec![b'a'; payload],
        )
        .unwrap();
        std::fs::write(dir.join(format!("{base}.gcode")), vec![b'b'; payload]).unwrap();
        std::fs::write(dir.join(format!("{base}_meta.json")), b"{}").unwrap();
    }

    /// 排序键只认尾部那一刻；认不出的名字没有时间（垫底）。
    #[test]
    fn only_the_trailing_timestamp_counts_as_the_sort_key() {
        assert_eq!(
            stamp_of("63288.1_20261009_220851").as_deref(),
            Some("20261009_220851")
        );
        assert_eq!(stamp_of("a_b"), None);
        assert_eq!(stamp_of("x_2026-10-09_220851"), None);
    }

    /// 条数限：25 条 → 只留最新 20 条，最旧 5 条**整条**消失（`_meta.json` 也不留）。
    #[test]
    fn the_count_limit_drops_only_the_oldest() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..25u32 {
            let stamp = format!("{:02}0000", i);
            write_record(d.path(), "2026-10-13", &base("m", "2026-10-13", &stamp), 16);
        }

        let report = prune(
            d.path(),
            Retention {
                max_records: 20,
                max_bytes: u64::MAX,
            },
        )
        .unwrap();

        assert_eq!(report.kept_records, 20);
        assert_eq!(report.removed_records, 5);
        let day = d.path().join("2026-10-13");
        for i in 0..5u32 {
            let base = base("m", "2026-10-13", &format!("{:02}0000", i));
            assert!(!day.join(format!("{base}_original.gcode")).exists());
            assert!(!day.join(format!("{base}.gcode")).exists());
            assert!(
                !day.join(format!("{base}_meta.json")).exists(),
                "记录要整条删，`_meta.json` 也不留"
            );
        }
        /* 最新的那条一定还在 */
        assert!(day
            .join(format!("{}.gcode", base("m", "2026-10-13", "240000")))
            .is_file());
    }

    /// 空间限：条数还宽着，但总量超标 → 从最旧删到装得下。
    #[test]
    fn the_size_limit_drops_oldest_until_it_fits() {
        let d = tempfile::tempdir().unwrap();
        for i in 0..5u32 {
            write_record(
                d.path(),
                "2026-10-13",
                &base("m", "2026-10-13", &format!("{:02}0000", i)),
                100,
            );
        }

        /* 一条 ≈ 202 字节：装得下 2 条（404），第 3 条就越过 500 */
        let report = prune(
            d.path(),
            Retention {
                max_records: 20,
                max_bytes: 500,
            },
        )
        .unwrap();

        assert_eq!(report.kept_records, 2);
        assert_eq!(report.removed_records, 3);
        assert_eq!(report.removed_bytes, 3 * 202);
    }

    /// ★ 最新那条**永远不删**：哪怕它自己就超了空间上限。
    ///
    /// 这就是"停用一个月再回来，最后一次的记录还在"那一条规矩。
    #[test]
    fn the_newest_record_survives_even_when_it_alone_breaks_the_cap() {
        let d = tempfile::tempdir().unwrap();
        write_record(
            d.path(),
            "2026-09-01",
            &base("old", "2026-09-01", "010000"),
            10,
        );
        write_record(
            d.path(),
            "2026-10-13",
            &base("new", "2026-10-13", "020000"),
            10_000,
        );

        let report = prune(
            d.path(),
            Retention {
                max_records: 20,
                max_bytes: 100,
            },
        )
        .unwrap();

        assert_eq!(report.kept_records, 1);
        assert_eq!(report.removed_records, 1);
        assert!(
            d.path()
                .join("2026-10-13")
                .join(format!("{}.gcode", base("new", "2026-10-13", "020000")))
                .is_file(),
            "最新那条哪怕超大也不许删"
        );
    }

    /// 跨日期排序：日期目录说了算（保留的是最近那天的那条）。
    #[test]
    fn the_newest_day_wins_across_day_dirs() {
        let d = tempfile::tempdir().unwrap();
        for day in ["2026-10-11", "2026-10-12", "2026-10-13"] {
            write_record(d.path(), day, &base("m", day, "010000"), 4);
        }

        let report = prune(
            d.path(),
            Retention {
                max_records: 1,
                max_bytes: u64::MAX,
            },
        )
        .unwrap();

        assert_eq!(report.kept_records, 1);
        assert_eq!(report.removed_records, 2);
        assert!(
            !d.path().join("2026-10-11").exists(),
            "清空的日期目录该回收"
        );
        assert!(
            !d.path().join("2026-10-12").exists(),
            "清空的日期目录该回收"
        );
        assert!(
            d.path().join("2026-10-13").is_dir(),
            "还留着记录的目录不许动"
        );
    }

    /// 不认的名字一个不动：根下直接躺的杂物、日期目录里的别的文件。
    /// 顺带钉住"目录里还剩别的东西就**不硬删**目录"。
    #[test]
    fn files_that_are_not_part_of_the_trio_are_left_alone() {
        let d = tempfile::tempdir().unwrap();
        std::fs::write(d.path().join("loose.gcode"), b"x").unwrap();
        write_record(
            d.path(),
            "2026-10-11",
            &base("a", "2026-10-11", "010000"),
            4,
        );
        write_record(
            d.path(),
            "2026-10-12",
            &base("b", "2026-10-12", "010000"),
            4,
        );
        let old_day = d.path().join("2026-10-11");
        std::fs::write(old_day.join("notes.txt"), b"keep me").unwrap();

        let report = prune(
            d.path(),
            Retention {
                max_records: 1,
                max_bytes: u64::MAX,
            },
        )
        .unwrap();

        assert_eq!(report.removed_records, 1);
        assert!(d.path().join("loose.gcode").is_file(), "根下的杂物不许动");
        assert!(
            old_day.join("notes.txt").is_file(),
            "不是三件套的文件不许动"
        );
        assert!(old_day.is_dir(), "目录里还剩东西，不许硬删");
    }

    /// 树不存在 ⇒ 空账，不是错误；再清一次也不动（幂等）。
    #[test]
    fn pruning_is_idempotent_and_a_missing_tree_is_not_an_error() {
        let d = tempfile::tempdir().unwrap();
        let missing = d.path().join("gcode_history");
        assert_eq!(
            prune(&missing, Retention::default()).unwrap(),
            PruneReport::default()
        );

        for i in 0..3u32 {
            write_record(
                d.path(),
                "2026-10-13",
                &base("m", "2026-10-13", &format!("{:02}0000", i)),
                4,
            );
        }
        let policy = Retention {
            max_records: 1,
            max_bytes: u64::MAX,
        };
        let first = prune(d.path(), policy).unwrap();
        let second = prune(d.path(), policy).unwrap();
        assert_eq!(first.removed_records, 2);
        assert_eq!(
            second,
            PruneReport {
                kept_records: 1,
                removed_records: 0,
                removed_bytes: 0
            }
        );
    }

    /// ★ **开关真的管着清理**：两条入口（启动 / 跑完）共用的正是这一段 glue
    ///（读设置 → 开着才清）。出厂默认（状态文件还没写过）该清；**明确关掉之后
    /// 一个字节都不许动** —— 用户关掉的意思是"别碰我的记录"。
    #[test]
    fn the_switch_gates_the_prune() {
        let d = tempfile::tempdir().unwrap();
        let history = d.path().join("gcode_history");
        for i in 0..25u32 {
            write_record(
                &history,
                "2026-10-13",
                &base("m", "2026-10-13", &format!("{i:02}0000")),
                4,
            );
        }

        /* 还没选过 ⇒ 出厂默认开着：按默认策略清到只剩最近 20 条（一条 2 个 gcode） */
        run(d.path(), &history);
        assert_eq!(gcode_files(&history), 20 * 2, "默认开着就该只留最近 20 条");

        /* 明确关掉 ⇒ 再攒一条也不清 */
        crate::runtime::app_state::set_auto_clean_history(d.path(), false).unwrap();
        write_record(
            &history,
            "2026-10-13",
            &base("m", "2026-10-13", "990000"),
            4,
        );
        run(d.path(), &history);
        assert_eq!(gcode_files(&history), 21 * 2, "关掉了就不许再清");
    }

    /// 树里所有文件的路径（判据只用来数数，不看内容）
    fn walk(dir: &Path) -> Vec<PathBuf> {
        let mut out = Vec::new();
        let Ok(entries) = std::fs::read_dir(dir) else {
            return out;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                out.extend(walk(&path));
            } else {
                out.push(path);
            }
        }
        out
    }

    /// 树里还剩几个 `.gcode`（原件与输出都算，一条记录两个）
    fn gcode_files(root: &Path) -> usize {
        walk(root)
            .iter()
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("gcode"))
            .count()
    }
}
