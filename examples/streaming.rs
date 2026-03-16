//! 流式输出示例.

use futures::StreamExt;
use uni_llm::{Message, UniLlmClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UniLlmClient::from_config("uni-llm.toml").await?;

    let mut stream = client
        .chat_stream(vec![Message::user("请逐步分析并发设计中的常见问题.")])
        .await?;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        print!("{}", chunk.delta);
    }
    println!();

    Ok(())
}
