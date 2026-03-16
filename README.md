# uni-llm

一个 Rust 库，用来统一调用各家大模型 API.

## 支持的厂商

| 厂商      | 说明                 |
| --------- | -------------------- |
| OpenAI    | GPT 系列             |
| Anthropic | Claude               |
| Google    | Gemini               |
| DeepSeek  | 兼容 OpenAI 格式     |
| 通义千问  | 阿里，兼容 OpenAI    |
| 文心一言  | 百度千帆             |
| GLM       | 智谱，兼容 OpenAI    |
| Ollama    | 本地跑模型，不用 key |

## 快速开始

先加依赖:

```toml
[dependencies]
uni-llm = "0.1"
tokio = { version = "1", features = ["full"] }
```

然后搞个配置文件 `uni-llm.toml`(项目根目录有示例)，把 API key 填进环境变量，就可以用了:

```rust
use uni_llm::{Message, UniLlmClient};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = UniLlmClient::from_config("uni-llm.toml").await?;

    let response = client.chat(vec![
        Message::system("你是一个并发系统设计专家."),
        Message::user("请简要介绍什么是并发执行."),
    ]).await?;

    println!("{}", response.content);
    Ok(())
}
```

## 不用配置文件也行

```rust
let client = UniLlmClient::builder()
    .provider("openai")
    .model("gpt-4o")
    .temperature(0.0)
    .build()?;
```

记得设 `OPENAI_API_KEY` 环境变量.

跑示例:`cargo run --example basic_chat` .

## 配置说明

主要就是 `[default]` 和 `[providers.xxx]`.每个 provider 的 `api_key_env` 写环境变量名，库会自己去读.Ollama 不用 key，文心要 key + secret 两个.

`[fallback]` 里配失败时依次尝试的 provider.

## License

MIT OR Apache-2.0
