//! 流式输出示例
//!
//! 运行前请设置环境变量 DASHSCOPE_API_KEY 或 DASHSCope_API_KEY

use futures_util::{pin_mut, StreamExt};
use qwen_sdk::{Client, DashScopeError, GenerationRequest, Message, Parameters};

fn get_api_key() -> Result<String, DashScopeError> {
    std::env::var("DASHSCOPE_API_KEY")
        .or_else(|_| std::env::var("DASHSCope_API_KEY"))
        .map_err(|_| {
            DashScopeError::InvalidConfiguration(
                "API Key is missing. Please set DASHSCOPE_API_KEY.".into(),
            )
        })
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("错误: {}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<(), DashScopeError> {
    let client = Client::new(get_api_key()?)?;

    let request = GenerationRequest::builder()
        .model("qwen-plus")
        .messages(vec![
            Message::system("You are a helpful assistant."),
            Message::user("用三句话介绍 Rust 语言"),
        ])
        .parameters(Parameters {
            result_format: Some("message".into()),
            max_tokens: Some(256),
            ..Default::default()
        })
        .build()?;

    let stream = client.generate_stream(request).await?;
    pin_mut!(stream);

    print!("回复: ");
    while let Some(chunk_res) = stream.next().await {
        let chunk = chunk_res?;
        if let Some(text) = chunk.content {
            print!("{}", text);
        }
    }
    println!();

    Ok(())
}
