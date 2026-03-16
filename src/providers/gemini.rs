//! Google Gemini REST API 实现。

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;

use super::LlmProvider;
use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::types::{ChatRequest, ChatResponse, Role, StreamChunk};

/// Google Gemini Provider。
pub struct GeminiProvider {
    name: String,
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

impl GeminiProvider {
    /// 创建 Gemini Provider。
    pub fn new(config: &ProviderConfig) -> Result<Self, LlmError> {
        let api_key = config
            .api_key_env
            .as_ref()
            .and_then(|env_var| std::env::var(env_var).ok())
            .ok_or_else(|| LlmError::ApiKeyMissing {
                provider: "gemini".to_string(),
                env_var: config
                    .api_key_env
                    .clone()
                    .unwrap_or_else(|| "GEMINI_API_KEY".to_string()),
            })?;

        let client = reqwest::Client::builder()
            .build()
            .map_err(|e| LlmError::ConnectionFailed {
                url: config.base_url.clone(),
                source: e,
            })?;

        Ok(Self {
            name: "gemini".to_string(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            client,
        })
    }

    fn build_contents(
        &self,
        request: &ChatRequest,
    ) -> (Vec<serde_json::Value>, Option<String>) {
        let mut contents = Vec::new();
        let mut system_instruction = None;

        for m in &request.messages {
            match m.role {
                Role::System => {
                    system_instruction = Some(m.content.clone());
                }
                Role::User => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{"text": m.content}]
                    }));
                }
                Role::Assistant => {
                    contents.push(serde_json::json!({
                        "role": "model",
                        "parts": [{"text": m.content}]
                    }));
                }
                Role::Tool => {
                    contents.push(serde_json::json!({
                        "role": "user",
                        "parts": [{"text": m.content}]
                    }));
                }
            }
        }

        (contents, system_instruction)
    }
}

#[async_trait]
impl LlmProvider for GeminiProvider {
    async fn chat(
        &self,
        request: &ChatRequest,
        model: &str,
        timeout: Duration,
    ) -> Result<ChatResponse, LlmError> {
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url, model, self.api_key
        );

        let (contents, system_instruction) = self.build_contents(request);

        let mut body = serde_json::json!({
            "contents": contents,
            "generationConfig": {
                "temperature": request.temperature.unwrap_or(0.0),
                "maxOutputTokens": request.max_tokens.unwrap_or(4096),
            }
        });

        if let Some(sys) = system_instruction {
            body["systemInstruction"] = serde_json::json!({
                "parts": [{"text": sys}]
            });
        }

        if request.json_mode {
            body["generationConfig"]["responseMimeType"] =
                serde_json::json!("application/json");
        }

        let start = std::time::Instant::now();

        let resp = self
            .client
            .post(&url)
            .json(&body)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| {
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

        let content = v["candidates"][0]["content"]["parts"][0]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();

        let usage = v["usageMetadata"].as_object().map(|u| crate::types::TokenUsage {
            prompt_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32,
            completion_tokens: u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
            total_tokens: u["promptTokenCount"].as_u64().unwrap_or(0) as u32
                + u["candidatesTokenCount"].as_u64().unwrap_or(0) as u32,
        }).unwrap_or_default();

        Ok(ChatResponse {
            content,
            tool_calls: None, // Gemini tool calling 格式不同，简化处理
            usage,
            model: model.to_string(),
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
