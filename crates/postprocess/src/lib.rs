//! `mkpse-pp` —— MKP 后处理内核，纯 CLI。
//!
//! **输入**一份 G-code + 一份配置文件（结构就是 [`ir::Ir`] 本身），**输出**处理后的 G-code。
//!
//! 本 crate 只做后处理：G-code 进、G-code 出，不含界面与预设体系。
//!
//! **明确不做**（不是"以后再说"）：前端、Tauri、预设文件（`.toml` 预设 / uuid / baseline /
//! 生命周期）、云端下载与同步、内容服务器、参数注册表校验、机型目录、通知总线、进度窗口。
//!
//! 分层（模块而非 crate，依赖只许指向更低）：
//!
//! ```text
//! main.rs → pipeline → postproc → ir → gcode / diag
//!             ↑ config
//! ```
//!
//! 两条硬约束（照抄源仓库，不许放松）：
//! - **消耗式步骤函数**：`second_pass(p1: Pass1Output, …)` 按值吃上一步产物，
//!   让「pass2 没有 pass1」变成**编译错误**，而不是运行时 `unwrap()`。
//! - **错误即中止**：没有「Warn 一下继续跑」。

// 模块按 Task 顺序逐个接入（每个 Task 结束时 `cargo build` 必须退 0）。
/// 校准素材的 3MF 身份（读 / 校验 / 写回 / 移除）—— **全仓唯一实现**。
/// 后处理钩子只读、检查脚本只校验、工作台可写，三张嘴都调这里（见模块头）。
pub mod calibration_asset;
pub mod diag;
pub mod gcode;
pub mod ir;
pub mod postproc;

pub mod config;
pub mod pipeline;
