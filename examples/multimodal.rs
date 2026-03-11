//! 多模态(图像理解)示例
//!
//! 使用 qwen-vl 或 qwen3-vl 系列模型进行图像理解.
//! 运行前请设置环境变量 DASHSCOPE_API_KEY:
//! ```bash
//! export DASHSCOPE_API_KEY=sk-xxx
//! cargo run --example multimodal
//! ```

use qwen_sdk::{
    Client, Content, DashScopeError, GenerationRequest, MediaElement, Message, Parameters,
};

#[tokio::main]
async fn main() {
    match run().await {
        Ok(text) => println!("回复: {}", text),
        Err(e) => {
            eprintln!("错误: {}", e);
            std::process::exit(1);
        }
    }
}

async fn run() -> Result<String, DashScopeError> {
    let api_key = std::env::var("DASHSCOPE_API_KEY").map_err(|_| {
        DashScopeError::InvalidConfiguration(
            "API Key is missing. Please set DASHSCOPE_API_KEY in environment.".into(),
        )
    })?;

    let client = Client::new(api_key)?;

    // 多模态消息:图像 + 文本
    let content = Content::multimodal(vec![
        MediaElement::image(
            "https://dashscope.oss-cn-beijing.aliyuncs.com/images/dog_and_girl.jpeg",
        ),
        MediaElement::text("图中描绘的是什么景象？"),
    ]);

    let request = GenerationRequest::builder()
        .model("qwen-vl-plus")
        .messages(vec![Message::user(content)])
        .parameters(Parameters {
            result_format: Some("message".into()),
            max_tokens: Some(512),
            ..Default::default()
        })
        .build()?;

    let response = client.generate(request).await?;

    response
        .text()
        .ok_or_else(|| DashScopeError::UnexpectedResponse("response has no text content".into()))
}
