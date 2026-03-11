# Qwen-SDK-Rust

DashScope API 的 Rust 客户端,用于调用千问(Qwen)系列模型.支持文本生成与多模态(图像、视频、音频)输入.

## 特性

- 类型安全的请求/响应结构
- 显式错误枚举(`DashScopeError`),便于精准定位问题
- Builder 模式构建请求
- 异步 API(基于 tokio + reqwest)

## 安装

在 `Cargo.toml` 中添加:

```toml
[dependencies]
qwen-sdk = { path = "." }
# 或从 crates.io(发布后)
# qwen-sdk = "0.1"
```

## 快速开始

### 文本生成

```rust
use qwen_sdk::{Client, GenerationRequest, Message, Parameters};

#[tokio::main]
async fn main() -> Result<(), qwen_sdk::DashScopeError> {
    let client = Client::new(std::env::var("DASHSCOPE_API_KEY")?)?;

    let request = GenerationRequest::builder()
        .model("qwen-plus")
        .messages(vec![
            Message::system("You are a helpful assistant."),
            Message::user("你好"),
        ])
        .parameters(Parameters {
            result_format: Some("message".into()),
            ..Default::default()
        })
        .build()?;

    let response = client.generate(request).await?;
    println!("{}", response.text().unwrap_or_default());
    Ok(())
}
```

### 多模态(图像理解)

```rust
use qwen_sdk::{Client, Content, GenerationRequest, MediaElement, Message, Parameters};

let content = Content::multimodal(vec![
    MediaElement::image("https://example.com/image.jpg"),
    MediaElement::text("图中是什么？"),
]);

let request = GenerationRequest::builder()
    .model("qwen-vl-plus")
    .messages(vec![Message::user(content)])
    .build()?;
```

## 运行示例

```bash
export DASHSCOPE_API_KEY=sk-xxx
cargo run --example basic
cargo run --example multimodal
cargo run --example stream   # 流式输出
```

## 显式指定端点

默认根据模型名自动选择 text-generation 或 multimodal-generation.若需强制使用某端点:

```rust
use qwen_sdk::{ApiEndpoint, Client, GenerationRequest, Message};

let request = GenerationRequest::builder()
    .model("qwen-plus")
    .messages(vec![Message::user("你好")])
    .endpoint(ApiEndpoint::TextGeneration)  // 强制用文本端点
    .build()?;
```

注意:用多模态端点调用 qwen-plus(或反之)通常会导致 API 返回 `InvalidParameter` 等错误.

## 地域配置

默认使用北京地域.新加坡、美国等需指定 base_url:

```rust
let client = Client::with_base_url(
    api_key,
    "https://dashscope-intl.aliyuncs.com/api/v1",  // 新加坡
)?;
```

## 错误处理

所有 API 返回 `Result<T, DashScopeError>`.错误变体包括:

- `InvalidConfiguration` - API Key 未设置
- `HttpError` - HTTP 非 200(含 status_code 和 body)
- `ApiResponseError` - 业务错误(含 code、message)
- `SerializationError` - JSON 解析失败
- `UnexpectedResponse` - 响应结构异常

## 文档

- [功能支持情况](docs/SUPPORT.md) - 文本/流式/图像/视频/音频/联网搜索/工具调用/异步/文档理解 的适配状态
- [模型与端点对应关系](docs/MODELS.md) - 各模型应使用的 URL/端点说明
- [DashScope API 参考](https://help.aliyun.com/zh/model-studio/qwen-api-via-dashscope)
- [错误码说明](https://www.alibabacloud.com/help/zh/model-studio/error-code)

## License

MIT OR Apache-2.0
