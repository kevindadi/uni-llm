//! # Qwen SDK for Rust
//!
//! DashScope API 的 Rust 客户端,用于调用千问(Qwen)系列模型.
//! 支持文本生成与多模态(图像、视频、音频)输入.
//!
//! ## 特性
//!
//! - 类型安全的请求/响应结构
//! - 显式错误枚举,便于精准定位问题
//! - Builder 模式构建请求
//! - 异步 API(基于 tokio)
//!
//! ## 示例
//!
//! ```rust,no_run
//! use qwen_sdk::{Client, GenerationRequest, Message, Parameters};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), qwen_sdk::DashScopeError> {
//!     let api_key = std::env::var("DASHSCOPE_API_KEY")
//!         .map_err(|_| qwen_sdk::DashScopeError::InvalidConfiguration(
//!             "API Key is missing. Please set DASHSCOPE_API_KEY.".into()
//!         ))?;
//!
//!     let client = Client::new(api_key)?;
//!
//!     let request = GenerationRequest::builder()
//!         .model("qwen-plus")
//!         .messages(vec![
//!             Message::system("You are a helpful assistant."),
//!             Message::user("你好,请介绍一下你自己"),
//!         ])
//!         .parameters(Parameters {
//!             result_format: Some("message".into()),
//!             ..Default::default()
//!         })
//!         .build()?;
//!
//!     let response = client.generate(request).await?;
//!
//!     if let Some(text) = response.text() {
//!         println!("{}", text);
//!     }
//!
//!     Ok(())
//! }
//! ```
//!
//! ## 错误处理
//!
//! 所有公共 API 返回 `Result<T, DashScopeError>`,调用者应显式处理:
//!
//! ```rust,ignore
//! match client.generate(request).await {
//!     Ok(resp) => println!("{:?}", resp.text()),
//!     Err(qwen_sdk::DashScopeError::HttpError { status_code, message }) => {
//!         eprintln!("HTTP {}: {}", status_code, message);
//!     }
//!     Err(qwen_sdk::DashScopeError::ApiResponseError { code, message }) => {
//!         eprintln!("API error ({}): {}", code.as_deref().unwrap_or(""), message);
//!     }
//!     Err(e) => eprintln!("{}", e),
//! }
//! ```

pub mod client;
pub mod error;
pub mod request;
pub mod response;

pub use client::{Client, ClientBuilder, DEFAULT_BASE_URL};
pub use error::DashScopeError;
pub use request::{
    ApiEndpoint, Content, GenerationRequest, GenerationRequestBuilder, Input, MediaElement,
    Message, Parameters, Role, VideoInput,
};
pub use response::{GenerationResponse, Output, Usage};
