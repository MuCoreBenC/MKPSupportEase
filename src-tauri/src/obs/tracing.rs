//! 日志与 trace id。
//!
//! 一条错误从 Rust 抛到界面上，界面显示的是 `message` + `traceId`。要能按那个 id 在日志里
//! 找到完整的上下文，这条链才算通 —— 所以 trace id 由 command 包装层生成、进 span、
//! 同时盖进 `AppError`，三处是同一个值。

use std::path::Path;

use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

/// 新的 trace id。
///
/// uuid v7 而不是 v4：前 48 位是毫秒时间戳，于是"按 id 字典序排"就等于"按时间排"，
/// 翻日志时能直接看出先后，v4 做不到。
pub fn new_trace_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// 装日志。
///
/// **任何失败都不阻断启动**：日志目录建不出来（磁盘满、权限、被 MDM 管控）时退到只写 stderr
/// 并打一条 warn，而不是让应用起不来 —— 用户要的是软件能用，不是日志齐全。
///
/// dev 下同时写文件与 stderr；release 只写文件（终端里没人看）。
pub fn init_tracing(log_dir: &Path) {
    let filter = EnvFilter::try_from_env("SUPPORTEASE_LOG").unwrap_or_else(|_| {
        EnvFilter::new(if cfg!(debug_assertions) {
            "info"
        } else {
            "warn"
        })
    });

    let file_layer = match std::fs::create_dir_all(log_dir) {
        Ok(()) => {
            // 按天滚动：一天一个文件，文件名形如 supportease.log.2026-09-21
            let appender = tracing_appender::rolling::daily(log_dir, "supportease.log");
            Some(
                tracing_subscriber::fmt::layer()
                    .with_ansi(false)
                    .with_writer(appender),
            )
        }
        Err(e) => {
            eprintln!("[tracing] 日志目录建不出来，退到只写 stderr：{log_dir:?} {e}");
            None
        }
    };

    let stderr_layer = cfg!(debug_assertions)
        .then(|| tracing_subscriber::fmt::layer().with_writer(std::io::stderr));

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .with(stderr_layer);

    /* try_init 而不是 init：init 在"已经装过"时会 panic。
    测试里每个 case 都可能走到这里，panic 会把测试炸掉，而重复装本身无害。 */
    if let Err(e) = registry.try_init() {
        eprintln!("[tracing] 装日志失败，继续启动：{e}");
        return;
    }

    if file_layer_missing(log_dir) {
        tracing::warn!(dir = %log_dir.display(), "日志只写 stderr —— 目录不可用");
    }
}

/// 目录到底能不能写，装完再确认一次：`create_dir_all` 成功 ≠ 能写进文件
fn file_layer_missing(log_dir: &Path) -> bool {
    !log_dir.is_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_ids_are_unique_and_sortable() {
        let a = new_trace_id();
        std::thread::sleep(std::time::Duration::from_millis(2));
        let b = new_trace_id();
        assert_ne!(a, b);
        // v7 带时间前缀：后生成的字典序更大
        assert!(a < b, "{a} 应该排在 {b} 前面");
    }

    #[test]
    fn init_does_not_panic_on_unwritable_dir() {
        // 根下不可写的路径：装不上文件层，但不许 panic
        init_tracing(Path::new("/proc/definitely-not-writable/logs"));
    }
}
