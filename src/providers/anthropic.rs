//! Anthropic Claude Messages API 实现。

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;

use super::LlmProvider;
use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::types::{ChatRequest, ChatResponse, StreamChunk};

/// Anthropic Claude Provider。
pub struct AnthropicProvider {
    name: String,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl AnthropicProvider {
    /// 创建 Anthropic Provider。
    pub fn new(config: &ProviderConfig) -> Result<Self, LlmError> {
        let api_key = config
            .api_key_env
            .as_ref()
            .and_then(|env_var| std::env::var(env_var).ok())
            .ok_or_else(|| LlmError::ApiKeyMissing {
                provider: "anthropic".to_string(),
                env_var: config
                    .api_key_env
                    .clone()
                    .unwrap_or_else(|| "ANTHROPIC_API_KEY".to_string()),
            })?;

        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| LlmError::ConnectionFailed {
                url: config.base_url.clone(),
                source: e,
            })?;

        Ok(Self {
            name: "anthropic".to_string(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        })
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn chat(
        &self,
        request: &ChatRequest,
        model: &str,
        timeout: Duration,
    ) -> Result<ChatResponse, LlmError> {
        // 提取 system 消息到顶级，转换其余消息
        let (system, messages): (Option<String>, Vec<_>) = request
            .messages
            .iter()
            .fold((None, vec![]), |(mut sys, mut msgs), m| {
                if matches!(m.role, crate::types::Role::System) {
                    sys = Some(m.content.clone());
                } else {
                    msgs.push(m.clone());
                }
                (sys, msgs)
            });

        // 合并连续的 Tool 消息为 user content 中的 tool_result 数组
        let mut anthropic_messages: Vec<serde_json::Value> = Vec::new();
        let mut i = 0;
        while i < messages.len() {
            let m = &messages[i];
            match m.role {
                crate::types::Role::User => {
                    anthropic_messages.push(serde_json::json!({
                        "role": "user",
                        "content": m.content
                    }));
                    i += 1;
                }
                crate::types::Role::Assistant => {
                    let content: serde_json::Value = if let Some(ref tc) = m.tool_calls {
                        let blocks: Vec<_> = tc
                            .iter()
                            .map(|t| {
                                serde_json::json!({
                                    "type": "tool_use",
                                    "id": t.id,
                                    "name": t.function_name,
                                    "input": t.arguments
                                })
                            })
                            .collect();
                        serde_json::json!(blocks)
                    } else {
                        serde_json::json!(m.content)
                    };
                    anthropic_messages.push(serde_json::json!({
                        "role": "assistant",
                        "content": content
                    }));
                    i += 1;
                }
                crate::types::Role::Tool => {
                    let mut tool_results = vec![serde_json::json!({
                        "type": "tool_result",
                        "tool_use_id": m.tool_call_id.as_ref().unwrap_or(&String::new()),
                        "content": m.content
                    })];
                    i += 1;
                    while i < messages.len() && matches!(messages[i].role, crate::types::Role::Tool) {
                        let tm = &messages[i];
                        tool_results.push(serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tm.tool_call_id.as_ref().unwrap_or(&String::new()),
                            "content": tm.content
                        }));
                        i += 1;
                    }
                    anthropic_messages.push(serde_json::json!({
                        "role": "user",
                        "content": tool_results
                    }));
                }
                _ => i += 1,
            }
        }

        let mut body = serde_json::json!({
            "model": model,
            "max_tokens": request.max_tokens.unwrap_or(4096),
            "messages": anthropic_messages,
        });

        if let Some(s) = system {
            body["system"] = serde_json::json!(s);
        }

        if let Some(tools) = &request.tools {
            let tool_defs: Vec<serde_json::Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "name": t.name,
                        "description": t.description,
                        "input_schema": t.parameters
                    })
                })
                .collect();
            body["tools"] = serde_json::json!(tool_defs);
        }

        let url = format!("{}/messages", self.base_url);
        let start = std::time::Instant::now();

        let req = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .timeout(timeout);

        let resp = req.send().await.map_err(|e| {
            if e.is_timeout() {
                LlmError::Timeout {
                    elapsed: start.elapsed(),
                    limit: timeout,
                }
            } else {
                LlmError::ConnectionFailed {
                    url: url.clone(),
                    source: e,
                }
            }
        })?;

        let status = resp.status();
        let body_str = resp.text().await.unwrap_or_default();

        if !status.is_success() {
            return Err(match status.as_u16() {
                401 => LlmError::AuthenticationFailed {
                    provider: self.name.clone(),
                    message: body_str,
                },
                429 => LlmError::RateLimited {
                    provider: self.name.clone(),
                    retry_after: None,
                },
                404 => LlmError::ModelNotFound {
                    provider: self.name.clone(),
                    model: model.to_string(),
                },
                s if (400..500).contains(&s) => LlmError::InvalidRequest {
                    provider: self.name.clone(),
                    status: s,
                    body: body_str,
                },
                s => LlmError::ServerError {
                    provider: self.name.clone(),
                    status: s,
                    body: body_str,
                },
            });
        }

        let v: serde_json::Value = serde_json::from_str(&body_str).map_err(|e| {
            LlmError::ResponseParseFailed {
                raw: body_str.clone(),
                source: e,
            }
        })?;

        let content = v["content"]
            .as_array()
            .and_then(|arr| {
                arr.iter()
                    .find(|b| b["type"] == "text")
                    .and_then(|b| b["text"].as_str())
            })
            .unwrap_or("")
            .to_string();

        let tool_calls: Option<Vec<crate::types::ToolCall>> = v["content"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter(|b| b["type"] == "tool_use")
                    .filter_map(|b| {
                        Some(crate::types::ToolCall {
                            id: b["id"].as_str()?.to_string(),
                            function_name: b["name"].as_str()?.to_string(),
                            arguments: b["input"].clone(),
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .and_then(|v| if v.is_empty() { None } else { Some(v) });

        let usage = v["usage"].as_object().map(|u| crate::types::TokenUsage {
            prompt_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["output_tokens"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["input_tokens"].as_u64().unwrap_or(0) as u32
                + u["output_tokens"].as_u64().unwrap_or(0) as u32,
        }).unwrap_or_default();

        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
            model: v["model"].as_str().unwrap_or(model).to_string(),
            provider: self.name.clone(),
            latency: start.elapsed(),
        })
    }

    async fn chat_stream(
        &self,
        _request: &ChatRequest,
        _model: &str,
        _timeout: Duration,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
        LlmError,
    > {
        // Anthropic SSE 格式不同，简化实现：返回空流
        let stream = futures::stream::iter(vec![Ok(StreamChunk {
            delta: String::new(),
            tool_calls_delta: None,
            is_final: true,
            usage: None,
        })]);
        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &str {
        &self.name
    }
}
