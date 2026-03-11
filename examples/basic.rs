//! 基础文本生成示例
//!
//! 运行前请设置环境变量 DASHSCOPE_API_KEY:
//! ```bash
//! export DASHSCOPE_API_KEY=sk-xxx
//! cargo run --example basic
//! ```

use qwen_sdk::{Client, DashScopeError, GenerationRequest, Message, Parameters};

#[tokio::main]
async fn main() {
    match run().await {
        Ok(text) => println!("回复: {}", text),
        Err(e) => {
            eprintln!("错误: {}", e);
            match &e {
                DashScopeError::InvalidConfiguration(_msg) => {
                    eprintln!("提示: 请设置环境变量 DASHSCOPE_API_KEY");
                    eprintln!("  export DASHSCOPE_API_KEY=sk-xxx");
                }
                DashScopeError::HttpError {
                    status_code,
                    message,
                } => {
                    eprintln!("HTTP {} 响应: {}", status_code, message);
                }
                DashScopeError::ApiResponseError { code, message } => {
                    eprintln!("API 错误 (code: {:?}): {}", code, message);
                }
                _ => {}
            }
            std::process::exit(1);
        }
    }
}

/// 从环境变量获取 API Key,支持 DASHSCOPE_API_KEY 和 DASHSCope_API_KEY
fn get_api_key() -> Result<String, DashScopeError> {
    std::env::var("DASHSCOPE_API_KEY")
        .or_else(|_| std::env::var("DASHSCope_API_KEY"))
        .map_err(|_| {
            DashScopeError::InvalidConfiguration(
                "API Key is missing. Please set DASHSCOPE_API_KEY in environment.".into(),
            )
        })
}

async fn run() -> Result<String, DashScopeError> {
    let api_key = get_api_key()?;

    let client = Client::new(api_key)?;

    let request = GenerationRequest::builder()
        .model("qwen-plus")
        .messages(vec![
            Message::system("You are a helpful assistant."),
            Message::user("你好,请用一句话介绍你自己"),
        ])
        .parameters(Parameters {
            result_format: Some("message".into()),
            max_tokens: Some(256),
            ..Default::default()
        })
        .build()?;

    let response = client.generate(request).await?;

    response
        .text()
        .ok_or_else(|| DashScopeError::UnexpectedResponse("response has no text content".into()))
}
