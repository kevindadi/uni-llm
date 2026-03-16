//! uni-llm: 统一调用多家商用大模型 API 的 Rust 库。
//!
//! 支持 OpenAI、Anthropic、Google Gemini、DeepSeek、通义千问、文心一言、GLM、Ollama 等厂商。

pub mod client;
pub mod config;
pub mod error;
pub mod types;
pub mod providers;

pub mod logging;

pub use client::{UniLlmClient, UniLlmClientBuilder};
pub use config::{Config, DefaultConfig, FallbackConfig, FallbackEntry, LoggingConfig, ProviderConfig};
pub use error::LlmError;
pub use logging::init_logging;
pub use types::{
    ChatRequest, ChatResponse, Message, Role, StreamChunk, TokenUsage, ToolCall, ToolCallDelta,
    ToolDefinition,
};
