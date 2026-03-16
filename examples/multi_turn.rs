//! 多轮对话示例。

use uni_llm::{Message, UniLlmClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UniLlmClient::from_config("uni-llm.toml").await?;

    let mut messages = vec![
        Message::system("你是一个并发系统设计专家。"),
        Message::user("什么是死锁？"),
    ];

    let r1 = client.chat(messages.clone()).await?;
    println!("Assistant: {}", r1.content);

    messages.push(Message::assistant(&r1.content));
    messages.push(Message::user("如何避免死锁？"));

    let r2 = client.chat(messages).await?;
    println!("Assistant: {}", r2.content);

    Ok(())
}
