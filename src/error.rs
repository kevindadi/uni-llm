//! DashScope API 错误类型定义
//!
//! 提供显式错误枚举,让调用者在编译时知晓可能发生的错误,
//! 并能打印出具体的调试信息(如 API 返回的错误码).

use std::fmt;

/// DashScope API 错误枚举
///
/// 涵盖配置错误、HTTP 错误、序列化错误和业务逻辑错误,
/// 便于开发者精准定位问题.
#[derive(Debug)]
pub enum DashScopeError {
    /// API Key 为空或未设置
    InvalidConfiguration(String),

    /// HTTP 请求构建失败(如 URL 解析错误)
    RequestBuildError(reqwest::Error),

    /// 服务器返回了非 200 状态码
    ///
    /// 包含 HTTP 状态码和响应 Body 文本,
    /// DashScope 在错误时会在 Body 中返回具体错误码(如 InvalidApiToken).
    HttpError { status_code: u16, message: String },

    /// 请求序列化失败或响应反序列化失败
    SerializationError(serde_json::Error),

    /// HTTP 请求成功(200),但业务逻辑失败
    ///
    /// 当 API 返回的 JSON 中 `status_code` 不为 200 或存在 `code` 字段时触发.
    /// 提取 `code` 和 `message` 字段,便于判断是模型名错误还是 Token 超限等.
    ApiResponseError {
        code: Option<String>,
        message: String,
    },

    /// 服务器返回了完全不符合预期的结构(例如缺少 `output` 字段)
    UnexpectedResponse(String),
}

impl fmt::Display for DashScopeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DashScopeError::InvalidConfiguration(msg) => {
                write!(f, "Invalid configuration: {}", msg)
            }
            DashScopeError::RequestBuildError(e) => {
                write!(f, "Request build failed: {}", e)
            }
            DashScopeError::HttpError {
                status_code,
                message,
            } => {
                write!(f, "HTTP error (status {}): {}", status_code, message)
            }
            DashScopeError::SerializationError(e) => {
                write!(f, "Serialization error: {}", e)
            }
            DashScopeError::ApiResponseError { code, message } => {
                if let Some(c) = code {
                    write!(f, "API error (code {}): {}", c, message)
                } else {
                    write!(f, "API error: {}", message)
                }
            }
            DashScopeError::UnexpectedResponse(msg) => {
                write!(f, "Unexpected response: {}", msg)
            }
        }
    }
}

impl std::error::Error for DashScopeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DashScopeError::RequestBuildError(e) => Some(e),
            DashScopeError::SerializationError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for DashScopeError {
    fn from(err: reqwest::Error) -> Self {
        DashScopeError::RequestBuildError(err)
    }
}

impl From<serde_json::Error> for DashScopeError {
    fn from(err: serde_json::Error) -> Self {
        DashScopeError::SerializationError(err)
    }
}
