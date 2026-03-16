//! Tool calling 示例。

use uni_llm::{Message, ToolDefinition, UniLlmClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UniLlmClient::from_config("uni-llm.toml").await?;

    let tools = vec![ToolDefinition::new(
        "analyze_ir",
        "分析并发 IR，检查死锁、数据竞争等问题",
        serde_json::json!({
            "type": "object",
            "properties": {
                "ir_json": {
                    "type": "string",
                    "description": "JSON 格式的并发 IR"
                }
            },
            "required": ["ir_json"]
        }),
    )];

    let response = client
        .chat_with_tools(
            vec![Message::user("请生成一个简单的并发 IR 并分析它的正确性。")],
            &tools,
        )
        .await?;

    println!("{}", response.content);
    if let Some(tool_calls) = &response.tool_calls {
        for call in tool_calls {
            println!("Tool call: {} -> {}", call.function_name, call.arguments);
        }
    }

    Ok(())
}
