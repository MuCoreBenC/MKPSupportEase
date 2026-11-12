//! 可观测性：日志、trace id、日志保留与导出前的脱敏。
//!
//! **三条日志流**（2026-11-12 起）：
//! - 应用日志 → [`tracing`] 的 `supportease.log.<日期>`（界面那个常驻进程）
//! - 交互记录 → 同一订阅者的 `ui.log.<日期>`（点击 / 切页 / 开关 / 命令结果，恒 info）
//! - 后处理日志 → [`hook_log`] 的 `hook.log.<日期>`（钩子进程，为什么分家见该模块头）
//!
//! 三者的保留由 [`rotation`] 统一执行（14 天 + 总 20 MiB）；导出前的路径 / IP 打码在
//! [`redact`]，打包命令在 `ipc::diagnostics`。
pub mod hook_log;
pub mod redact;
pub mod rotation;
pub mod tracing;
