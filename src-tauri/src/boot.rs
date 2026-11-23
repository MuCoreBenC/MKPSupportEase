//! 进程启动的运行时数据初始化 —— **界面与钩子共用这一份**。
//!
//! 两个角色各有各的生命周期（界面常驻开窗、钩子跑完就退），但运行时数据是同一套：
//! 随包的机型表 / 别名 / 禁区 / 校准档表。这里只做「装数据」这一件事；数据根、日志、
//! 存储迁移那些按角色不同的初始化留在各自入口。
//!
//! # 为什么是一个必须拿到的凭据
//!
//! [`Runtime`] 只能由 [`Runtime::initialize`] 造出来，而两个入口
//! （[`crate::hook::run_with_channel`] / [`crate::run`]）的签名都要它 ——
//! 以后在主程序里加第三种角色，忘了初始化是**编译错误**，而不是装到用户机器上
//! 才发现机型表是空的（那次的现场是切片器只有一个 `Error code: 101`）。
//!
//! 初始化失败就返回 `Err`：调用方不许"先跑起来再说"。

use crate::runtime;

/// 已完成运行时初始化的凭据（见模块头）。
pub struct Runtime(());

impl Runtime {
    /// 装随包数据。失败就是失败 —— 入口不往下跑。
    pub fn initialize() -> Result<Self, String> {
        runtime::install_machine_tables_from_catalog()?;
        Ok(Self(()))
    }
}

/// 判据要跑管线时先过这里（进程内一次）。
///
/// `install` 第二次返回 `Err`，而判据之间共享一个进程、并发跑 —— 用 `Once` 收口，
/// 免得"谁先跑"决定谁红。
#[cfg(test)]
pub(crate) fn install_machine_tables_for_tests() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        runtime::install_machine_tables_from_catalog().expect("随包机型表必须装得上");
    });
}
