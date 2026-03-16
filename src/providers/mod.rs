//! LLM Provider 抽象与实现.

mod anthropic;
mod gemini;
mod openai;
mod wenxin;

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;

use crate::config::Config;
use crate::error::LlmError;
use crate::types::{ChatRequest, ChatResponse, StreamChunk};

/// LLM Provider 抽象 trait.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// 普通 chat 调用.
    async fn chat(
        &self,
        request: &ChatRequest,
        model: &str,
        timeout: Duration,
    ) -> Result<ChatResponse, LlmError>;

    /// 流式 chat 调用.
    async fn chat_stream(
        &self,
        request: &ChatRequest,
        model: &str,
        timeout: Duration,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>, LlmError>;

    /// Provider 名称标识.
    fn provider_name(&self) -> &str;

    /// 是否支持 tool calling.
    fn supports_tool_calling(&self) -> bool {
        true
    }

    /// 是否支持 JSON mode(response_format).
    fn supports_json_mode(&self) -> bool {
        true
    }
}

/// 创建指定 provider 实例.
pub fn create_provider(
    config: &Config,
    provider_name: &str,
) -> Result<Box<dyn LlmProvider>, LlmError> {
    let provider_config =
        config
            .get_provider(provider_name)
            .ok_or_else(|| LlmError::ModelNotFound {
                provider: provider_name.to_string(),
                model: "".to_string(),
            })?;

    match provider_name {
        "openai" | "deepseek" | "dashscope" | "glm" | "ollama" => Ok(Box::new(
            openai::OpenAiCompatibleProvider::new(provider_name, provider_config)?,
        )),
        "anthropic" => Ok(Box::new(anthropic::AnthropicProvider::new(
            provider_config,
        )?)),
        "gemini" => Ok(Box::new(gemini::GeminiProvider::new(provider_config)?)),
        "wenxin" => Ok(Box::new(wenxin::WenxinProvider::new(provider_config)?)),
        _ => Err(LlmError::ModelNotFound {
            provider: provider_name.to_string(),
            model: "".to_string(),
        }),
    }
}
