//! `mkp-pp` —— MKP 后处理内核，纯 CLI。
//!
//! **输入**一份 G-code + 一份配置文件（结构就是 [`ir::Ir`] 本身），**输出**处理后的 G-code。
//!
//! 来源：`/Users/wzy/projects/mkp-rust/mkp-sr`（冻结 tag `freeze/before-postproc-extract`）。
//! 那个仓库有 9 个 crate、一个 Tauri 前端、预设文件体系、云端下载与同步共约 14.2 万行；
//! 本项目只搬**后处理语义**那部分。
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
pub mod diag;
pub mod gcode;
pub mod ir;
pub mod postproc;

pub mod config;
pub mod pipeline;
