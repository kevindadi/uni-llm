//! 百度文心一言千帆 API 实现.

use std::sync::Mutex;
use std::time::Duration;

use async_trait::async_trait;

use super::LlmProvider;
use crate::config::ProviderConfig;
use crate::error::LlmError;
use crate::types::{ChatRequest, ChatResponse, Role, StreamChunk};

/// 缓存的 access_token.
struct TokenCache {
    token: Option<String>,
    expires_at: Option<std::time::Instant>,
}

/// 百度文心 Provider.
pub struct WenxinProvider {
    name: String,
    base_url: String,
    api_key: String,
    api_secret: String,
    client: reqwest::Client,
    token_cache: Mutex<TokenCache>,
}

impl WenxinProvider {
    /// 创建文心 Provider.
    pub fn new(config: &ProviderConfig) -> Result<Self, LlmError> {
        let api_key = config
            .api_key_env
            .as_ref()
            .and_then(|env_var| std::env::var(env_var).ok())
            .ok_or_else(|| LlmError::ApiKeyMissing {
                provider: "wenxin".to_string(),
                env_var: config
                    .api_key_env
                    .clone()
                    .unwrap_or_else(|| "WENXIN_API_KEY".to_string()),
            })?;

        let api_secret = config
            .api_secret_env
            .as_ref()
            .and_then(|env_var| std::env::var(env_var).ok())
            .ok_or_else(|| LlmError::ApiKeyMissing {
                provider: "wenxin".to_string(),
                env_var: config
                    .api_secret_env
                    .clone()
                    .unwrap_or_else(|| "WENXIN_API_SECRET".to_string()),
            })?;

        let client =
            reqwest::Client::builder()
                .build()
                .map_err(|e| LlmError::ConnectionFailed {
                    url: config.base_url.clone(),
                    source: e,
                })?;

        Ok(Self {
            name: "wenxin".to_string(),
            base_url: config.base_url.trim_end_matches('/').to_string(),
            api_key,
            api_secret,
            client,
            token_cache: Mutex::new(TokenCache {
                token: None,
                expires_at: None,
            }),
        })
    }

    async fn get_access_token(&self) -> Result<String, LlmError> {
        {
            let cache = self.token_cache.lock().unwrap();
            #[allow(clippy::collapsible_if)]
            if let (Some(t), Some(exp)) = (&cache.token, cache.expires_at) {
                if exp > std::time::Instant::now() {
                    return Ok(t.clone());
                }
            }
        }

        let url = format!(
            "https://aip.baidubce.com/oauth/2.0/token?grant_type=client_credentials&client_id={}&client_secret={}",
            self.api_key, self.api_secret
        );

        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionFailed {
                url: url.clone(),
                source: e,
            })?;

        let body_str = resp.text().await.map_err(|e| LlmError::ConnectionFailed {
            url: url.clone(),
            source: e,
        })?;

        let body: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| LlmError::ResponseParseFailed {
                raw: body_str,
                source: e,
            })?;

        let token =
            body["access_token"]
                .as_str()
                .ok_or_else(|| LlmError::AuthenticationFailed {
                    provider: self.name.clone(),
                    message: body.to_string(),
                })?;

        let expires_in = body["expires_in"].as_u64().unwrap_or(2592000);
        let expires_at =
            std::time::Instant::now() + Duration::from_secs(expires_in.saturating_sub(300));

        {
            let mut cache = self.token_cache.lock().unwrap();
            cache.token = Some(token.to_string());
            cache.expires_at = Some(expires_at);
        }

        Ok(token.to_string())
    }

    fn build_messages(&self, request: &ChatRequest) -> Vec<serde_json::Value> {
        request
            .messages
            .iter()
            .filter(|m| !matches!(m.role, Role::System))
            .map(|m| {
                let role = match m.role {
                    Role::User => "user",
                    Role::Assistant => "assistant",
                    Role::Tool => "user",
                    _ => "user",
                };
                serde_json::json!({
                    "role": role,
                    "content": m.content
                })
            })
            .collect()
    }
}

#[async_trait]
impl LlmProvider for WenxinProvider {
    async fn chat(
        &self,
        request: &ChatRequest,
        model: &str,
        timeout: Duration,
    ) -> Result<ChatResponse, LlmError> {
        let token = self.get_access_token().await?;

        let url = format!("{}/chat/{}?access_token={}", self.base_url, model, token);

        let messages = self.build_messages(request);
        let system = request
            .messages
            .iter()
            .find(|m| matches!(m.role, Role::System))
            .map(|m| m.content.as_str())
            .unwrap_or("");

        let body = if system.is_empty() {
            serde_json::json!({
                "messages": messages,
                "stream": false,
                "temperature": request.temperature.unwrap_or(0.0),
                "max_output_tokens": request.max_tokens.unwrap_or(4096),
            })
        } else {
            serde_json::json!({
                "messages": messages,
                "stream": false,
                "temperature": request.temperature.unwrap_or(0.0),
                "max_output_tokens": request.max_tokens.unwrap_or(4096),
                "system": system,
            })
        };

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

        let v: serde_json::Value =
            serde_json::from_str(&body_str).map_err(|e| LlmError::ResponseParseFailed {
                raw: body_str.clone(),
                source: e,
            })?;

        let content = v["result"]
            .as_str()
            .or_else(|| {
                v["result"]
                    .as_array()
                    .and_then(|a| a.first())
                    .and_then(|c| c.as_str())
            })
            .unwrap_or("")
            .to_string();

        let usage = v["usage"]
            .as_object()
            .map(|u| crate::types::TokenUsage {
                prompt_tokens: u["prompt_tokens"].as_u64().unwrap_or(0) as u32,
                completion_tokens: u["completion_tokens"].as_u64().unwrap_or(0) as u32,
                total_tokens: u["total_tokens"].as_u64().unwrap_or(0) as u32,
            })
            .unwrap_or_default();

        Ok(ChatResponse {
            content,
            tool_calls: None,
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
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
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

    fn supports_tool_calling(&self) -> bool {
        false
    }

    fn supports_json_mode(&self) -> bool {
        false
    }
}
