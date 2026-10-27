//! 可观测性：日志与 trace id。
//!
//! **两条日志流**（2026-10-11 起）：应用那条住 [`tracing`]（界面进程 → `supportease.log.<日期>`），
//! 后处理那条住 [`hook_log`]（钩子进程 → `hook.log.<日期>`）。为什么分家见后者模块头。
pub mod hook_log;
pub mod tracing;
