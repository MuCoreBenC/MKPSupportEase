//! `CancelToken` —— 用户取消的信号线（Task 17，Phase 2）。
//!
//! 旧 Go 侧规格（实测）：`RunContext{Ctx, cancel}`（context.go:39-64）——一根
//! `context.WithCancel` 信号线，旁路（UI 按钮 / watchdog）只能 `RequestCancel`，
//! 主流程在阶段边界与长循环内协作检查。取消**不是错误码**（全仓没有
//! `E_*_CANCELLED_*`，唯一 CANCELLED 码在下载模块），Rust 侧用
//! `PostprocError::Cancelled`（`E_SYS_CANCELLED_001`，Task 2 已登记为新造）承载。
//!
//! 为什么是 `Arc<AtomicBool>` 而不是 tokio 的 `CancellationToken`：engine 及以下
//! 零 async 运行时（gatecheck 边界三）；跨线程可见性 `SeqCst` 足够（单方向置位，
//! 无 ABA、无复合读写）。

use std::sync::Arc;

use std::sync::atomic::{AtomicBool, Ordering};

/// 取消信号。`Clone` 共享同一根线（等价 Go 侧把 `rc.Ctx` 递给各协程）。
#[derive(Debug, Default, Clone)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// 新建未取消的信号线。
    pub fn new() -> Self {
        Self::default()
    }

    /// 请求取消（幂等；等价 Go `RequestCancel`，旁路只许调这个）。
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// 是否已请求取消（等价 Go `IsCancelled` = `rc.Ctx.Err() != nil`）。
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }

    /// 信号线身份判定（同一根线的两个克隆互为 true）。取消位是单方向的，
    /// 置位后无法据此区分不同 run——需要身份比较的清理逻辑用这个。
    pub fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_token_is_not_cancelled() {
        assert!(!CancelToken::new().is_cancelled());
    }

    #[test]
    fn cancel_is_idempotent_and_sticky() {
        let t = CancelToken::new();
        t.cancel();
        t.cancel();
        assert!(t.is_cancelled());
    }

    /// Clone 共享同一根线：一端置位，全端可见（跨线程）。
    #[test]
    fn clones_share_the_same_wire() {
        let a = CancelToken::new();
        let b = a.clone();
        let c = CancelToken::new();
        assert!(a.same_as(&b), "克隆与本体同线");
        assert!(!a.same_as(&c), "两根独立线不同身份");
        let h = std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(10));
            b.cancel();
        });
        h.join().unwrap();
        assert!(a.is_cancelled(), "兄弟 clone 置位后本体必须可见");
    }
}
