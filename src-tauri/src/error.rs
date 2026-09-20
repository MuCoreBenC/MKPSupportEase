//! 错误模型 —— 前端能看懂的那一种错。
//!
//! 一个 command 出错时，前端需要三样东西：**能给人看的话**（message）、**能让代码分支的码**
//! （code）、**能去日志里查的线索**（traceId）。技术细节（io error 的原文、路径、序号）
//! 属于第四样，放 detail，界面默认不展示。
//!
//! 字段名与 `src/api/contract.ts` 的 `AppError` 逐字段对齐，靠本文件末尾那个单元测试钉住 ——
//! serde 的 `camelCase` 与 TS 的接口是两份声明，没有编译器帮忙对照，只能靠测试。

use serde::Serialize;

/// 错误分类。前端按它分支，所以值是稳定契约，不能随手改名。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ErrorCode {
    /// 要的东西不存在（文件、预设、模型）
    NotFound,
    /// 越界访问：路径穿出数据根、写只读区
    PermissionDenied,
    /// 入参不合法（绝对路径、空 id、数值越界）
    InvalidArgument,
    /// 文件在，但内容读不成合法结构
    Corrupted,
    /// 校验和不符 —— 云端原件与本地副本对不上
    ShaMismatch,
    /// 落盘/读盘失败
    Io,
    /// 这个口子还没接（骨架阶段的常态）
    NotImplemented,
    /// 兜底：没归类的内部错误
    Internal,
}

/// 跨 IPC 边界的错误结构。
///
/// `#[serde(rename_all = "camelCase")]` 让 `trace_id` 变成 `traceId`。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: ErrorCode,
    /// 可以直接显示给用户的中文。不要往这里塞 io error 的英文原文
    pub message: String,
    /// 本次调用的 trace id；`-` 表示这条错误还没被 command 包装层盖章
    pub trace_id: String,
    /// 技术细节，给开发看。界面可以折叠显示
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl AppError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            trace_id: "-".into(),
            detail: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotFound, message)
    }

    pub fn permission_denied(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::PermissionDenied, message)
    }

    pub fn invalid_argument(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidArgument, message)
    }

    pub fn corrupted(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Corrupted, message)
    }

    pub fn sha_mismatch(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::ShaMismatch, message)
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Io, message)
    }

    pub fn not_implemented(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::NotImplemented, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorCode::Internal, message)
    }

    /// 附技术细节
    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    /// 盖 trace id。由 command 包装层统一调用，业务代码不用管
    pub fn with_trace(mut self, trace_id: &str) -> Self {
        self.trace_id = trace_id.to_owned();
        self
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{:?}] {}", self.code, self.message)?;
        if let Some(d) = &self.detail {
            write!(f, " ({d})")?;
        }
        Ok(())
    }
}

impl std::error::Error for AppError {}

/// io 错误一律转成 `IO` + 中文 message，英文原文进 detail。
///
/// `NotFound` 单独分出来：它在业务上多半不是"出错"，而是"没有这一份"，前端要能分支。
impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        match e.kind() {
            std::io::ErrorKind::NotFound => {
                AppError::not_found("文件不存在").with_detail(e.to_string())
            }
            std::io::ErrorKind::PermissionDenied => {
                AppError::permission_denied("没有权限访问这个文件").with_detail(e.to_string())
            }
            _ => AppError::io("读写文件失败").with_detail(e.to_string()),
        }
    }
}

/// 原子写的最后一步（`persist`）失败
impl From<tempfile::PersistError> for AppError {
    fn from(e: tempfile::PersistError) -> Self {
        AppError::io("保存失败：临时文件没能替换目标文件").with_detail(e.to_string())
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::corrupted("数据格式不对，解析失败").with_detail(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 字段名必须与 src/api/contract.ts 的 `AppError` 一致。
    /// 那边是 `{ code, message, traceId, detail? }`，`code` 是 SCREAMING_SNAKE_CASE 的字符串联合。
    #[test]
    fn serializes_with_contract_field_names() {
        let err = AppError::not_found("没有这一份预设")
            .with_detail("presetIndex[\"x\"] 未命中")
            .with_trace("018f...");
        let v: serde_json::Value = serde_json::to_value(&err).unwrap();

        assert_eq!(v["code"], "NOT_FOUND");
        assert_eq!(v["message"], "没有这一份预设");
        assert_eq!(v["traceId"], "018f...");
        assert_eq!(v["detail"], "presetIndex[\"x\"] 未命中");
        // 只有这四个键，多一个就说明契约漂了
        assert_eq!(v.as_object().unwrap().len(), 4);
    }

    /// detail 为空时整个键不出现，前端 `'detail' in err` 才有意义
    #[test]
    fn omits_detail_when_absent() {
        let v = serde_json::to_value(AppError::internal("出错了")).unwrap();
        assert!(v.get("detail").is_none());
        assert_eq!(v["traceId"], "-");
    }

    #[test]
    fn all_codes_are_screaming_snake_case() {
        let cases = [
            (ErrorCode::NotFound, "NOT_FOUND"),
            (ErrorCode::PermissionDenied, "PERMISSION_DENIED"),
            (ErrorCode::InvalidArgument, "INVALID_ARGUMENT"),
            (ErrorCode::Corrupted, "CORRUPTED"),
            (ErrorCode::ShaMismatch, "SHA_MISMATCH"),
            (ErrorCode::Io, "IO"),
            (ErrorCode::NotImplemented, "NOT_IMPLEMENTED"),
            (ErrorCode::Internal, "INTERNAL"),
        ];
        for (code, want) in cases {
            assert_eq!(serde_json::to_value(code).unwrap(), want);
        }
    }

    #[test]
    fn io_not_found_maps_to_not_found() {
        let e: AppError = std::io::Error::new(std::io::ErrorKind::NotFound, "no such file").into();
        assert_eq!(e.code, ErrorCode::NotFound);
        assert!(e.detail.is_some(), "英文原文要进 detail");
    }
}
