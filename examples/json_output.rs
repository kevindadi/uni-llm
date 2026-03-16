//! JSON 结构化输出示例.

use serde::Deserialize;
use uni_llm::{Message, UniLlmClient};

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct Task {
    id: String,
    description: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct MyIR {
    tasks: Vec<Task>,
    dependencies: Vec<(String, String)>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UniLlmClient::from_config("uni-llm.toml").await?;

    let ir: MyIR = client
        .chat_json(vec![
            Message::system("你是一个并发系统设计专家.请输出合法 JSON."),
            Message::user("请为「读取文件并解析」设计一个包含 2 个任务的并发 IR,输出 JSON."),
        ])
        .await?;

    println!("{:#?}", ir);

    Ok(())
}
