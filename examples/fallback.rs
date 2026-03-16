//! Fallback 链示例.

use uni_llm::{Message, UniLlmClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UniLlmClient::from_config("uni-llm.toml").await?;

    let response = client
        .chat(vec![Message::user("你好,请用一句话介绍你自己.")])
        .await?;

    println!("Provider: {}  Model: {}", response.provider, response.model);
    println!("{}", response.content);

    Ok(())
}
