//! 基础 chat 调用示例。

use uni_llm::{Message, UniLlmClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UniLlmClient::from_config("uni-llm.toml").await?;

    let response = client
        .chat(vec![
            Message::system("你是一个并发系统设计专家。"),
            Message::user("请简要介绍什么是并发执行。"),
        ])
        .await?;

    println!("模型: {}  耗时: {:?}", response.model, response.latency);
    println!("{}", response.content);

    Ok(())
}
