//! pass 长循环内的协作取消（Task 17.2）。
//!
//! 旧 Go 侧规格（`processor/cancel.go`）：
//! - `cancelCheckInterval = 100ms`，注释明写「取时间而非行数，使取消延迟与
//!   机器性能解耦」——每行都查原子量太贵，按行数节流则快慢机上延迟不可预测；
//! - `Check`：未到间隔时只做一次时间比较即返回；到间隔才真正查取消位。
//!
//! 本模块逐字保持这个形态：`check` 在未到间隔时是一次 `duration_since` 比较。
//!
//! 间隔是**字段而非常量**（two-repo-ci-repair）：产品路径取
//! `DEFAULT_CANCEL_CHECK_INTERVAL`（100ms，语义与 Go 逐字一致），判据路径可注入
//! 极小值。根因是 100ms 窗把「循环内是否真查了取消位」这条判据变成了**墙钟相关**的
//! ——在快机器上整条 pass 的墙钟远小于 100ms（CI run 32925530475 实测：同一测试
//! 二进制 4 条用例合计 0.25s），第一次真检查根本没到点，于是取消只被下一个步骤
//! 边界截住、事件比例 ≈ 1.0，判据**确定性假红**。把间隔显式化之后，判据阈值
//! 一个字不改就恢复成机器无关的确定性事实。
//!
//! **只有一个构造器**：`with_interval`。刻意不留 `start()` 这类"隐式取默认"的
//! 便捷入口——那会让「这条 pass 用的是哪个间隔」重新变成看不见的事实，而这正是
//! 本次要消掉的东西。默认值由调用方显式引用具名常量。

use std::time::{Duration, Instant};

use crate::diag::{CancelToken, PostprocError};

/// 协作检查间隔的**产品默认值**（对照 `cancel.go:14`）。产品路径语义零变化。
/// **全仓唯一定义处**：engine 再导出它，pass1/pass2 的调用方一律引用，
/// 不允许第二处 `from_millis(100)` 字面量。
pub const DEFAULT_CANCEL_CHECK_INTERVAL: Duration = Duration::from_millis(100);

/// 时间基节流器。每条 pass 各持一个（first_pass / second_pass 互不共享时刻）。
pub(crate) struct CancelChecker {
    last: Instant,
    interval: Duration,
}

impl CancelChecker {
    /// 唯一构造器：以起始时刻 + 显式节流间隔构造
    /// （用传入的 `now` 而非自行取时，便于测试注入）。
    /// 产品路径传 `DEFAULT_CANCEL_CHECK_INTERVAL`；判据路径传 `Duration::ZERO`
    /// ⇒ 每轮都真查取消位，把「取消停在循环内」从墙钟相关变成确定性。
    pub(crate) fn with_interval(now: Instant, interval: Duration) -> Self {
        Self {
            last: now,
            interval,
        }
    }

    /// 循环体每轮调用。未到间隔 = 一次时间比较即 `Ok(())`；到间隔且已取消 =
    /// `Err(Cancelled)`，主循环立即向上传播。
    pub(crate) fn check(
        &mut self,
        cancel: &CancelToken,
        now: Instant,
    ) -> Result<(), PostprocError> {
        if now.duration_since(self.last) < self.interval {
            return Ok(());
        }
        self.last = now;
        if cancel.is_cancelled() {
            // 文案 = Go CancelledResultError（context.go:105）逐字，用户可见面一致。
            return Err(PostprocError::Cancelled {
                message: "用户已取消后处理".to_string(),
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_check_before_interval() {
        let t0 = Instant::now();
        let token = CancelToken::new();
        let mut c = CancelChecker::with_interval(t0, DEFAULT_CANCEL_CHECK_INTERVAL);
        // 未到 100ms：即使已取消也不触发（Go 同形——间隔内连原子读都不做）
        token.cancel();
        assert!(c.check(&token, t0 + Duration::from_millis(50)).is_ok());
    }

    #[test]
    fn cancelled_at_interval_boundary() {
        let t0 = Instant::now();
        let token = CancelToken::new();
        let mut c = CancelChecker::with_interval(t0, DEFAULT_CANCEL_CHECK_INTERVAL);
        token.cancel();
        let err = c
            .check(&token, t0 + Duration::from_millis(150))
            .expect_err("到间隔且已取消必须报 Cancelled");
        assert!(matches!(err, PostprocError::Cancelled { .. }));
        assert_eq!(err.code(), "E_SYS_CANCELLED_001");
    }

    #[test]
    fn interval_resets_after_each_real_check() {
        let t0 = Instant::now();
        let token = CancelToken::new();
        let mut c = CancelChecker::with_interval(t0, DEFAULT_CANCEL_CHECK_INTERVAL);
        // t0+120ms 真查一次（未取消，过）；t0+180ms 距上次真查仅 60ms，必须跳过
        assert!(c.check(&token, t0 + Duration::from_millis(120)).is_ok());
        token.cancel();
        assert!(
            c.check(&token, t0 + Duration::from_millis(180)).is_ok(),
            "间隔重置自上次真查起算，不是自构造时刻起算"
        );
        assert!(c.check(&token, t0 + Duration::from_millis(240)).is_err());
    }

    /// 注入 `Duration::ZERO`：每轮都真查取消位（判据路径的确定性来源）。
    /// 与 `no_check_before_interval` 成对——同一时刻 t0 下，默认间隔放过、
    /// 注入 ZERO 必须拦住。
    #[test]
    fn zero_interval_checks_every_round() {
        let t0 = Instant::now();
        let token = CancelToken::new();
        token.cancel();
        let mut default = CancelChecker::with_interval(t0, DEFAULT_CANCEL_CHECK_INTERVAL);
        assert!(
            default.check(&token, t0).is_ok(),
            "默认 100ms 间隔在 t0 必须放过"
        );
        let mut injected = CancelChecker::with_interval(t0, Duration::ZERO);
        assert!(
            injected.check(&token, t0).is_err(),
            "注入 ZERO 后同一时刻必须真查并拦住"
        );
    }
}
