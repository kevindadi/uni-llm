//! 流式输出 SSE 解析

use async_stream::stream;
use futures_util::Stream;
use serde::Deserialize;

use crate::error::DashScopeError;
use crate::response::Usage;

/// 流式输出增量片段
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// 增量文本内容
    pub content: Option<String>,
    /// 结束原因
    pub finish_reason: Option<String>,
    /// Token 使用量(通常在最后一个 chunk)
    pub usage: Option<Usage>,
}

/// 流式响应的原始 JSON 结构(用于解析每个 data 行)
#[derive(Debug, Deserialize)]
struct StreamEventRaw {
    #[serde(default)]
    output: Option<StreamOutput>,
    #[serde(default)]
    usage: Option<Usage>,
    #[serde(default)]
    code: Option<String>,
    #[serde(default)]
    message: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamOutput {
    #[serde(default)]
    choices: Option<Vec<StreamChoice>>,
}

#[derive(Debug, Deserialize)]
struct StreamChoice {
    #[serde(default)]
    message: Option<StreamMessage>,
    #[serde(default)]
    delta: Option<StreamMessage>,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StreamMessage {
    #[serde(default)]
    content: Option<String>,
}

/// 将 SSE 字节流解析为 StreamChunk 流
pub fn parse_sse_stream(
    body: reqwest::Response,
) -> impl Stream<Item = Result<StreamChunk, DashScopeError>> + Send {
    stream! {
        let mut buf = String::new();
        let mut stream = body.bytes_stream();

        while let Some(chunk_res) = stream.next().await {
            let bytes = chunk_res.map_err(DashScopeError::RequestBuildError)?;
            buf.push_str(&String::from_utf8_lossy(&bytes));

            while let Some((event, rest)) = split_sse_event(&buf) {
                let event_owned = event.to_string();
                buf = rest.to_string();
                if event_owned.is_empty() {
                    continue;
                }
                let line = event_owned.trim();
                if line.starts_with("data:") {
                    let data = line.strip_prefix("data:").map(str::trim).unwrap_or("");
                    if data == "[DONE]" || data.is_empty() {
                        continue;
                    }
                    match serde_json::from_str::<StreamEventRaw>(data) {
                        Ok(parsed) => {
                            if let (Some(code), msg) = (
                                parsed.code.as_ref(),
                                parsed.message.as_ref(),
                            ) {
                                if !code.is_empty() {
                                    yield Err(DashScopeError::ApiResponseError {
                                        code: Some(code.clone()),
                                        message: msg.cloned().unwrap_or_default(),
                                    });
                                    return;
                                }
                            }
                            let content = parsed
                                .output
                                .as_ref()
                                .and_then(|o| o.choices.as_ref())
                                .and_then(|c| c.first())
                                .and_then(|c| {
                                    c.delta
                                        .as_ref()
                                        .and_then(|m| m.content.clone())
                                        .or_else(|| {
                                            c.message.as_ref().and_then(|m| m.content.clone())
                                        })
                                });
                            let finish_reason = parsed
                                .output
                                .as_ref()
                                .and_then(|o| o.choices.as_ref())
                                .and_then(|c| c.first())
                                .and_then(|c| c.finish_reason.clone());
                            yield Ok(StreamChunk {
                                content,
                                finish_reason,
                                usage: parsed.usage,
                            });
                        }
                        Err(_) => {
                            // 忽略解析失败的行(如注释等)
                        }
                    }
                }
            }
        }
    }
}

/// 按 SSE 事件边界分割(双换行)
fn split_sse_event(s: &str) -> Option<(&str, &str)> {
    let idx = s.find("\n\n")?;
    Some((&s[..idx], &s[idx + 2..]))
}

use futures_util::StreamExt;
