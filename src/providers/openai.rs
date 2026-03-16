//! OpenAI 及兼容模型实现(DeepSeek、DashScope、GLM、Ollama).

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use eventsource_stream::Eventsource;
use futures::Stream;
use futures::StreamExt;
use serde_json::Value;
use tracing::instrument;

use super::LlmProvider;
use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::types::{ChatRequest, ChatResponse, Message, Role, StreamChunk, TokenUsage, ToolCall};

/// OpenAI 兼容 Provider.
pub struct OpenAiCompatibleProvider {
    name: String,
    base_url: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

impl OpenAiCompatibleProvider {
    /// 创建 OpenAI 兼容 Provider.
    pub fn new(name: &str, config: &ProviderConfig) -> Result<Self, LlmError> {
        let api_key = config
            .api_key_env
            .as_ref()
            .and_then(|env_var| std::env::var(env_var).ok());

        // Ollama 不需要 API key
        if name != "ollama" && api_key.is_none() {
            return Err(LlmError::ApiKeyMissing {
                provider: name.to_string(),
                env_var: config
                    .api_key_env
                    .clone()
                    .unwrap_or_else(|| "API_KEY".to_string()),
            });
        }

        let client =
            reqwest::Client::builder()
                .build()
                .map_err(|e| LlmError::ConnectionFailed {
                    url: config.base_url.clone(),
                    source: e,
                })?;

        Ok(Self {
            name: name.to_string(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        })
    }

    fn build_request_body(&self, request: &ChatRequest, model: &str) -> Result<Value, LlmError> {
        let messages: Vec<Value> = request
            .messages
            .iter()
            .map(|m| self.message_to_value(m))
            .collect::<Result<Vec<_>, _>>()?;

        let mut body = serde_json::json!({
            "model": model,
            "messages": messages,
            "temperature": request.temperature.unwrap_or(0.0),
            "max_tokens": request.max_tokens.unwrap_or(4096),
        });

        if let Some(tools) = &request.tools {
            let tools_value: Vec<Value> = tools
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "type": "function",
                        "function": {
                            "name": t.name,
                            "description": t.description,
                            "parameters": t.parameters
                        }
                    })
                })
                .collect();
            body["tools"] = serde_json::Value::Array(tools_value);
        }

        if request.json_mode {
            body["response_format"] = serde_json::json!({ "type": "json_object" });
        }

        Ok(body)
    }

    fn message_to_value(&self, m: &Message) -> Result<Value, LlmError> {
        let role = match m.role {
            Role::System => "system",
            Role::User => "user",
            Role::Assistant => "assistant",
            Role::Tool => "tool",
        };

        let mut msg = serde_json::json!({
            "role": role,
            "content": m.content
        });

        if let Some(ref id) = m.tool_call_id {
            msg["tool_call_id"] = serde_json::Value::String(id.clone());
        }

        if let Some(ref tool_calls) = m.tool_calls {
            let tc: Vec<Value> = tool_calls
                .iter()
                .map(|t| {
                    serde_json::json!({
                        "id": t.id,
                        "type": "function",
                        "function": {
                            "name": t.function_name,
                            "arguments": t.arguments.to_string()
                        }
                    })
                })
                .collect();
            msg["tool_calls"] = serde_json::Value::Array(tc);
        }

        Ok(msg)
    }

    fn parse_response(&self, body: &str, latency: Duration) -> Result<ChatResponse, LlmError> {
        let v: Value = serde_json::from_str(body).map_err(|e| LlmError::ResponseParseFailed {
            raw: body.to_string(),
            source: e,
        })?;

        let parse_err = || serde_json::from_str::<serde_json::Value>("{").unwrap_err();
        let choices = v["choices"]
            .as_array()
            .ok_or_else(|| LlmError::ResponseParseFailed {
                raw: body.to_string(),
                source: parse_err(),
            })?;

        let choice = choices
            .first()
            .ok_or_else(|| LlmError::ResponseParseFailed {
                raw: body.to_string(),
                source: parse_err(),
            })?;

        let message = &choice["message"];
        let content = message["content"].as_str().unwrap_or("").to_string();

        let tool_calls = message["tool_calls"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|tc| {
                        let id = tc["id"].as_str()?.to_string();
                        let func = &tc["function"];
                        let name = func["name"].as_str()?.to_string();
                        let args_str = func["arguments"].as_str().unwrap_or("{}");
                        let arguments =
                            serde_json::from_str(args_str).unwrap_or(serde_json::json!({}));
                        Some(ToolCall {
                            id,
                            function_name: name,
                            arguments,
                        })
                    })
                    .collect::<Vec<_>>()
            })
            .filter(|v: &Vec<ToolCall>| !v.is_empty());

        let usage = v["usage"]
            .as_object()
            .map(|u| TokenUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        let model = v["model"].as_str().unwrap_or("unknown").to_string();

        Ok(ChatResponse {
            content,
            tool_calls,
            usage,
            model,
            provider: self.name.clone(),
            latency,
        })
    }

    fn map_http_error(&self, status: u16, body: String) -> LlmError {
        match status {
            401 => LlmError::AuthenticationFailed {
                provider: self.name.clone(),
                message: body,
            },
            429 => LlmError::RateLimited {
                provider: self.name.clone(),
                retry_after: None,
            },
            404 => LlmError::ModelNotFound {
                provider: self.name.clone(),
                model: "".to_string(),
            },
            400..=499 => LlmError::InvalidRequest {
                provider: self.name.clone(),
                status,
                body,
            },
            _ => LlmError::ServerError {
                provider: self.name.clone(),
                status,
                body,
            },
        }
    }
}

#[async_trait]
impl LlmProvider for OpenAiCompatibleProvider {
    #[instrument(skip(self, request))]
    async fn chat(
        &self,
        request: &ChatRequest,
        model: &str,
        timeout: Duration,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let body = self.build_request_body(request, model)?;

        let mut req = self.client.post(&url).json(&body).timeout(timeout);

        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let start = std::time::Instant::now();
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
            return Err(self.map_http_error(status.as_u16(), body_str));
        }

        self.parse_response(&body_str, start.elapsed())
    }

    async fn chat_stream(
        &self,
        request: &ChatRequest,
        model: &str,
        timeout: Duration,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, LlmError>> + Send>>, LlmError> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut body = self.build_request_body(request, model)?;
        body["stream"] = serde_json::json!(true);

        let mut req = self.client.post(&url).json(&body).timeout(timeout);

        if let Some(ref key) = self.api_key {
            req = req.bearer_auth(key);
        }

        let stream = req
            .send()
            .await
            .map_err(|e| LlmError::ConnectionFailed {
                url: url.clone(),
                source: e,
            })?
            .bytes_stream()
            .eventsource();

        let stream = stream.filter_map(|event| async move {
            let ev = match event {
                Ok(ev) => ev,
                Err(e) => {
                    return Some(Err(LlmError::StreamInterrupted {
                        chunks_received: 0,
                        source: Box::new(e),
                    }))
                }
            };
            let data = ev.data;
            if data == "[DONE]" {
                return Some(Ok(StreamChunk {
                    delta: String::new(),
                    tool_calls_delta: None,
                    is_final: true,
                    usage: None,
                }));
            }
            match serde_json::from_str::<Value>(&data) {
                Ok(v) => {
                    let choices = v["choices"].as_array();
                    let choice = choices.and_then(|a| a.first());
                    let (content, is_final) = match choice {
                        Some(c) => {
                            let delta = &c["delta"];
                            let content = delta["content"].as_str().unwrap_or("").to_string();
                            let finish = c["finish_reason"].as_str();
                            let is_final = finish.is_some() && !finish.unwrap_or("").is_empty();
                            (content, is_final)
                        }
                        None => (String::new(), false),
                    };
                    if content.is_empty() && !is_final {
                        return None;
                    }
                    Some(Ok(StreamChunk {
                        delta: content,
                        tool_calls_delta: None,
                        is_final,
                        usage: None,
                    }))
                }
                Err(e) => Some(Err(LlmError::ResponseParseFailed {
                    raw: data,
                    source: e,
                })),
            }
        });

        Ok(Box::pin(stream))
    }

    fn provider_name(&self) -> &str {
        &self.name
    }
}
