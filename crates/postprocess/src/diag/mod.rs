//! `diag` —— 最底层的诊断词汇：错误码、`PostprocError`、`Diagnostic`。
//!
//! 与 `gcode` 同为最底层叶子，**互不依赖**。
//! 存在理由：旧 Go 侧的错误码（`shared/errdiag/codes.go`）是对外稳定契约，
//! 需要一个所有上层都能引用的唯一定义处。
//!
//! 抽取自 mkp-sr 的 `crates/diag`（339 行）。**产品代码零改动** ——
//! 该 crate 内部零 `crate::` 引用，改成模块后连 use 行都不用动。

pub mod cancel;
pub mod error;

pub use cancel::CancelToken;
pub use error::PostprocError;

/// 诊断信息（警告级）：对应旧 Go 侧 `errdiag.Diagnostic` 的最小投影。
///
/// 与 [`PostprocError`] 的区别：警告**不中止管线**（落在 `ir.warnings`，且不得被
/// 静默吞掉，见 spec design.md §七），错误中止管线并带错误码。
///
/// 字段名与 Go JSON 标签逐字一致（code/severity/module/operation/message/cause），
/// 因为 Task 4 会把 Go 侧导出的 IR JSON 直接反序列化进 Rust 结构 ——
/// 字段名漂移会让那份机械判据静默变松。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub severity: String,
    pub module: String,
    pub operation: String,
    pub message: String,
    pub cause: Option<String>,
}

impl Diagnostic {
    /// 警告级便捷构造（module 固定为本内核）。
    pub fn warning(operation: &str, message: impl Into<String>) -> Self {
        Self {
            code: "W_POSTPROC_000".to_string(),
            severity: "warning".to_string(),
            module: "postproc".to_string(),
            operation: operation.to_string(),
            message: message.into(),
            cause: None,
        }
    }
}
