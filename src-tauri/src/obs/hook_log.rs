//! 后处理那一趟的**专用日志**：`<内部根>/logs/hook.log.<日期>`。
//!
//! # 为什么单开一条流（作者 2026-10-11 点名要的）
//!
//! 应用日志（[`crate::obs::tracing`] → `supportease.log.<日期>`）记的是**界面那个常驻进程**：
//! 命令、IPC、数据根、发布链。后处理是**另一个进程**（切片器拉起的那一次），读者也不一样 ——
//! 排查一次失败想看的是"这一趟：哪份输入、哪个预设、走到哪一步、什么码、退出码是多少"。
//! 混在界面日志里，这几条会被启动与 IPC 的噪音埋掉，而它们**一天可能只出现一次**。
//!
//! 另一个更硬的理由：钩子进程以前**根本没装日志** —— 内核那几句
//! `tracing::warn!`（机型维度缺失、收笔 G-code 未定义、熨烫走的是回退路径…）
//! 在钩子这一路**哪儿都没去**（没有 subscriber，事件直接丢掉）。装上它，
//! 那些"处理过程中的问题"才有落点；而它们的量级只配写在**后处理自己的**日志里。
//!
//! # 只有文件，**刻意不叠 stderr**
//!
//! 钩子的 stderr 是**对外契约**（切片器把它整段摆进自己的错误框，见
//! `hook.rs` 模块头）：只写结论。哪怕在 debug 构建里也不给 stderr 叠日志层 ——
//! 多一行 INFO 就会把真正的原因往下推（旧世代实测过 30 行 INFO 埋掉真因）。
//! 开发者要看实时过程：结论行里会打印**这份日志的路径**（表里 `hook.log_path`）。
//!
//! # 装不上就退化成"没有日志"
//!
//! 数据根建不出来 / 目录不可写 ⇒ 返回 `None`，调用方照常干活（结论行少一行日志路径）。
//! **后处理能不能跑，不该由日志决定** —— 与应用侧 [`crate::obs::tracing::init_tracing`]
//! 同一个立场。

use std::path::{Path, PathBuf};

use tracing_appender::rolling;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// 日志文件前缀。按天滚动 ⇒ 实际落点 `hook.log.2026-10-11`
/// （日期那一段由 `tracing_appender` 自己拼，与 [`today_suffix`] 是同一个格式）。
const PREFIX: &str = "hook.log";

/// 装日志，交回**今天那一份**的落点（结论行要把它给用户）。
///
/// 级别取 `SUPPORTEASE_LOG`（与界面那条同一把旋钮），缺省 `info` ——
/// 后处理这条流的信噪比本来就高（一趟十几条），没必要像 release 界面那样压到 warn。
pub fn init(log_dir: &Path) -> Option<PathBuf> {
    if std::fs::create_dir_all(log_dir).is_err() {
        return None;
    }

    /* 同步写、不引 WorkerGuard：这一趟进程活得很短，丢日志的风险（进程被杀时缓冲没落盘）
    比阻塞的代价更值钱。`rolling::daily` 自己就是 `MakeWriter`，不需要包 non_blocking。 */
    let appender = rolling::daily(log_dir, PREFIX);
    let layer = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(appender);
    let filter =
        EnvFilter::try_from_env("SUPPORTEASE_LOG").unwrap_or_else(|_| EnvFilter::new("info"));

    /* `try_init` 而不是 `init`：判据里钩子逻辑是在**测试进程**里跑的，那边可能已经装过
    （界面那条）。重复装无害，装不上就退化成"没有日志"。 */
    if tracing_subscriber::registry()
        .with(filter)
        .with(layer)
        .try_init()
        .is_err()
    {
        return None;
    }

    Some(log_dir.join(format!("{PREFIX}.{}", today_suffix()?)))
}

/// 今天（本地时区）的 `YYYY-MM-DD` —— 与 `tracing_appender` 的按天后缀同一个格式，
/// 拼出来的路径就是它此刻正在写的那一份。
fn today_suffix() -> Option<String> {
    let now = time::OffsetDateTime::now_local().ok()?;
    let fmt = time::macros::format_description!("[year]-[month]-[day]");
    now.format(&fmt).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 装得上就交回一个**能写出内容**的路径；装不上（同进程已装过）返回 `None` 也不算失败。
    #[test]
    fn init_reports_the_daily_file() {
        let d = tempfile::tempdir().unwrap();
        let Some(path) = init(d.path()) else {
            /* 测试进程里可能已经被别的用例装过 —— 这是允许的降级，不是失败 */
            return;
        };
        assert!(path.starts_with(d.path()), "{path:?} 该落在给的目录里");
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            name.starts_with("hook.log."),
            "文件名该是 hook.log.<日期>：{name}"
        );

        tracing::warn!(probe = 1, "判据写一条");
        let text = std::fs::read_to_string(&path).expect("日志该写出来了");
        assert!(text.contains("判据写一条"), "实际: {text}");
    }

    /// 目录建不出来 ⇒ `None`，不 panic（后处理不该因为日志而跑不起来）。
    #[test]
    fn unusable_dir_degrades_quietly() {
        #[cfg(unix)]
        {
            let d = tempfile::tempdir().unwrap();
            let file = d.path().join("not-a-dir");
            crate::fsx::atomic::atomic_write(&file, b"x").unwrap();
            assert!(init(&file.join("logs")).is_none());
        }
        /* Windows 上"路径被文件占着"给的是另一种错误码，行为一样（`create_dir_all` 失败）；
        这里只跑 unix 那一支：判据要的是"失败不 panic"，平台差异不值得各写一遍 */
    }
}
