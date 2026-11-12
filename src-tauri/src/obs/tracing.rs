//! 日志与 trace id。
//!
//! 一条错误从 Rust 抛到界面上，界面显示的是 `message` + `traceId`。要能按那个 id 在日志里
//! 找到完整的上下文，这条链才算通 —— 所以 trace id 由 command 包装层生成、进 span、
//! 同时盖进 `AppError`，三处是同一个值。
//!
//! # 两条文件流，同一份订阅者
//!
//! - `supportease.log.<日期>` —— 应用日志。release 缺省只放 `warn` 以上，debug 是 `info`。
//! - `ui.log.<日期>` —— 界面动作（点击 / 切页 / 开关 / 每次命令的结果），**恒 `info`**。
//!
//! 界面动作不能混进应用日志：release 那条压到 `warn` 正是为了不让它一天涨几十 MB，
//! 而点击恰好是高频低价值 —— 把整体提到 `info` 两头都输。所以用 target `ui` 单独放行，
//! 两条流各配一层过滤器：应用日志那层 `ui=off`，交互日志那层只收 `ui`。
//!
//! 保留策略见 [`super::rotation`]（14 天 + 总 20 MiB 双限，按天滚动只是其中一半）。

use std::path::Path;

use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{EnvFilter, Layer as _};

use super::rotation;

/// 交互日志的文件前缀。`tracing_appender` 会在后面拼 `.<YYYY-MM-DD>`。
pub const UI_PREFIX: &str = "ui.log";

/// 应用日志的文件前缀。
pub const APP_PREFIX: &str = "supportease.log";

/// 新的 trace id。
///
/// uuid v7 而不是 v4：前 48 位是毫秒时间戳，于是"按 id 字典序排"就等于"按时间排"，
/// 翻日志时能直接看出先后，v4 做不到。
pub fn new_trace_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// 缺省级别：release 只记警告，但 `ui` 那一支放到 `info`。
fn default_filter() -> EnvFilter {
    EnvFilter::new(if cfg!(debug_assertions) {
        "info"
    } else {
        "warn,ui=info"
    })
}

/// 一份按天滚动的文件出口。建不出来就 `None`（退到只写 stderr，绝不挡启动）。
fn appender(log_dir: &Path, prefix: &str) -> Option<RollingFileAppender> {
    match RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix(prefix)
        .max_log_files(rotation::KEEP_DAYS)
        .build(log_dir)
    {
        Ok(a) => Some(a),
        Err(e) => {
            eprintln!("[tracing] 日志文件建不出来，退到只写 stderr：{log_dir:?} {e}");
            None
        }
    }
}

/// 装日志。
///
/// **任何失败都不阻断启动**：日志目录建不出来（磁盘满、权限、被 MDM 管控）时退到只写 stderr
/// 并打一条 warn，而不是让应用起不来 —— 用户要的是软件能用，不是日志齐全。
///
/// dev 下同时写文件与 stderr；release 只写文件（终端里没人看）。
pub fn init_tracing(log_dir: &Path) {
    let filter = EnvFilter::try_from_env("SUPPORTEASE_LOG").unwrap_or_else(|_| default_filter());

    if let Err(e) = std::fs::create_dir_all(log_dir) {
        eprintln!("[tracing] 日志目录建不出来，退到只写 stderr：{log_dir:?} {e}");
    }

    /* 应用日志那一层：全局过滤器已经定好级别，这里只额外把 `ui` 那一路挡掉 ——
    否则交互记录会同时写进两条流，体积直接翻倍 */
    let app_layer = appender(log_dir, APP_PREFIX).map(|a| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(a)
            .with_filter(EnvFilter::new("trace,ui=off"))
    });

    /* 交互日志那一层：只收 `ui` 目标，其它一律不落 */
    let ui_layer = appender(log_dir, UI_PREFIX).map(|a| {
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(a)
            .with_filter(EnvFilter::new("off,ui=info"))
    });

    let stderr_layer = cfg!(debug_assertions).then(|| {
        tracing_subscriber::fmt::layer()
            .with_writer(std::io::stderr)
            .with_filter(EnvFilter::new("trace,ui=off"))
    });

    let registry = tracing_subscriber::registry()
        .with(filter)
        .with(app_layer)
        .with(ui_layer)
        .with(stderr_layer);

    /* try_init 而不是 init：init 在"已经装过"时会 panic。
    测试里每个 case 都可能走到这里，panic 会把测试炸掉，而重复装本身无害。 */
    if let Err(e) = registry.try_init() {
        eprintln!("[tracing] 装日志失败，继续启动：{e}");
        return;
    }

    if !log_dir.is_dir() {
        tracing::warn!(dir = %log_dir.display(), "日志只写 stderr —— 目录不可用");
    }
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
