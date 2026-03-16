//! 错误类型定义。

use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

/// LLM 调用相关错误。
#[derive(Error, Debug)]
pub enum LlmError {
    // —— 网络层 ——
    #[error("连接失败: url={url}, source={source}")]
    ConnectionFailed {
        url: String,
        #[source]
        source: reqwest::Error,
    },

    #[error("请求超时: elapsed={elapsed:?}, limit={limit:?}")]
    Timeout {
        elapsed: Duration,
        limit: Duration,
    },

    // —— API 层 ——
    #[error("认证失败: provider={provider}, message={message}")]
    AuthenticationFailed {
        provider: String,
        message: String,
    },

    #[error("请求频率受限: provider={provider}, retry_after={retry_after:?}")]
    RateLimited {
        provider: String,
        retry_after: Option<Duration>,
    },

    #[error("配额已用尽: provider={provider}, message={message}")]
    QuotaExceeded {
        provider: String,
        message: String,
    },

    #[error("模型不存在: provider={provider}, model={model}")]
    ModelNotFound {
        provider: String,
        model: String,
    },

    #[error("无效请求: provider={provider}, status={status}, body={body}")]
    InvalidRequest {
        provider: String,
        status: u16,
        body: String,
    },

    #[error("服务端错误: provider={provider}, status={status}, body={body}")]
    ServerError {
        provider: String,
        status: u16,
        body: String,
    },

    // —— 解析层 ——
    #[error("响应解析失败: raw={raw}, source={source}")]
    ResponseParseFailed {
        raw: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("JSON 输出解析失败: raw={raw}, target_type={target_type}, source={source}")]
    JsonOutputParseFailed {
        raw: String,
        target_type: String,
        #[source]
        source: serde_json::Error,
    },

    #[error("流式输出中断: chunks_received={chunks_received}, source={source}")]
    StreamInterrupted {
        chunks_received: usize,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    // —— 配置层 ——
    #[error("配置加载失败: path={path}, source={source}")]
    ConfigLoadFailed {
        path: PathBuf,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("API Key 缺失: provider={provider}, env_var={env_var}")]
    ApiKeyMissing {
        provider: String,
        env_var: String,
    },

    // —— Fallback ——
    #[error("所有 provider 均失败: errors={0:?}")]
    AllProvidersFailed(Vec<(String, Box<LlmError>)>),
}

impl LlmError {
    /// 是否为可重试错误。
    pub fn is_retriable(&self) -> bool {
        matches!(
            self,
            LlmError::RateLimited { .. }
                | LlmError::ServerError { .. }
                | LlmError::ConnectionFailed { .. }
                | LlmError::Timeout { .. }
        )
    }
}
