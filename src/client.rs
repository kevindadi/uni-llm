//! UniLlmClient 统一客户端实现。

use std::sync::Arc;
use std::time::Duration;

use tracing::{info, warn};

use crate::config::Config;
use crate::error::LlmError;
use crate::providers::{create_provider, LlmProvider};
use crate::types::{ChatRequest, ChatResponse, Message, StreamChunk, ToolDefinition};

/// 统一 LLM 客户端。
pub struct UniLlmClient {
    config: Arc<Config>,
    provider_override: Option<String>,
    model_override: Option<String>,
    temperature_override: Option<f32>,
    max_tokens_override: Option<u32>,
    timeout_override: Option<Duration>,
}

impl UniLlmClient {
    /// 从配置文件加载客户端。
    pub async fn from_config(path: impl AsRef<std::path::Path>) -> Result<Self, LlmError> {
        let config = Config::from_file(path).await?;
        Ok(Self {
            config: Arc::new(config),
            provider_override: None,
            model_override: None,
            temperature_override: None,
            max_tokens_override: None,
            timeout_override: None,
        })
    }

    /// 创建 builder。
    pub fn builder() -> UniLlmClientBuilder {
        UniLlmClientBuilder::default()
    }

    /// 临时切换 provider。
    pub fn with_provider(&self, provider: &str) -> Self {
        Self {
            config: Arc::clone(&self.config),
            provider_override: Some(provider.to_string()),
            model_override: self.model_override.clone(),
            temperature_override: self.temperature_override,
            max_tokens_override: self.max_tokens_override,
            timeout_override: self.timeout_override,
        }
    }

    /// 临时切换模型。
    pub fn with_model(&self, model: &str) -> Self {
        Self {
            config: Arc::clone(&self.config),
            provider_override: self.provider_override.clone(),
            model_override: Some(model.to_string()),
            temperature_override: self.temperature_override,
            max_tokens_override: self.max_tokens_override,
            timeout_override: self.timeout_override,
        }
    }

    fn provider_name(&self) -> &str {
        self.provider_override
            .as_deref()
            .unwrap_or(&self.config.default.provider)
    }

    fn model(&self) -> &str {
        self.model_override
            .as_deref()
            .unwrap_or(&self.config.default.model)
    }

    fn temperature(&self) -> f32 {
        self.temperature_override
            .unwrap_or(self.config.default.temperature)
    }

    fn max_tokens(&self) -> u32 {
        self.max_tokens_override
            .unwrap_or(self.config.default.max_tokens)
    }

    fn timeout(&self) -> Duration {
        self.timeout_override
            .unwrap_or_else(|| self.config.timeout())
    }

    fn build_request(
        &self,
        messages: Vec<Message>,
        tools: Option<Vec<ToolDefinition>>,
        json_mode: bool,
    ) -> ChatRequest {
        ChatRequest {
            messages,
            model: self.model_override.clone(),
            temperature: Some(self.temperature()),
            max_tokens: Some(self.max_tokens()),
            tools,
            json_mode,
        }
    }

    async fn do_chat_with_provider(
        &self,
        provider: &dyn LlmProvider,
        request: &ChatRequest,
    ) -> Result<ChatResponse, LlmError> {
        let model = self.model();
        let timeout = self.timeout();

        let mut last_error = None;
        let max_retries = self.config.default.max_retries;
        let retry_delay_ms = self.config.default.retry_delay_ms;

        for attempt in 0..=max_retries {
            if attempt > 0 {
                let delay_ms = retry_delay_ms * 2u64.pow(attempt - 1);
                let jitter = rand::random::<u64>() % 500;
                let total_ms = (delay_ms + jitter).min(60_000);
                tracing::info!(
                    provider = %provider.provider_name(),
                    model = %model,
                    attempt = attempt,
                    delay_ms = total_ms,
                    "retrying after error"
                );
                tokio::time::sleep(Duration::from_millis(total_ms)).await;
            }

            match provider.chat(request, model, timeout).await {
                Ok(resp) => {
                    info!(
                        provider = %resp.provider,
                        model = %resp.model,
                        latency = ?resp.latency,
                        prompt_tokens = resp.usage.prompt_tokens,
                        completion_tokens = resp.usage.completion_tokens,
                        "chat completed"
                    );
                    return Ok(resp);
                }
                Err(e) => {
                    if !e.is_retriable() {
                        return Err(e);
                    }
                    last_error = Some(e);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| LlmError::ServerError {
            provider: provider.provider_name().to_string(),
            status: 500,
            body: "unknown".to_string(),
        }))
    }

    /// 普通 chat 调用。
    pub async fn chat(&self, messages: Vec<Message>) -> Result<ChatResponse, LlmError> {
        let request = self.build_request(messages, None, false);
        self.chat_with_fallback(&request).await
    }

    /// 带 tool 的 chat 调用。
    pub async fn chat_with_tools(
        &self,
        messages: Vec<Message>,
        tools: &[ToolDefinition],
    ) -> Result<ChatResponse, LlmError> {
        let request = self.build_request(messages, Some(tools.to_vec()), false);
        self.chat_with_fallback(&request).await
    }

    /// JSON 结构化输出。
    pub async fn chat_json<T: serde::de::DeserializeOwned>(
        &self,
        messages: Vec<Message>,
    ) -> Result<T, LlmError> {
        let mut request = self.build_request(messages, None, true);

        let provider = create_provider(&self.config, self.provider_name())?;
        if !provider.supports_json_mode() {
            let json_prompt = "你必须只输出合法 JSON，不要输出任何其他文字。输出格式：\n{...}";
            if let Some(first) = request.messages.first_mut() {
                if matches!(first.role, crate::types::Role::System) {
                    first.content = format!("{}\n\n{}", json_prompt, first.content);
                } else {
                    request.messages.insert(0, Message::system(json_prompt));
                }
            } else {
                request.messages.insert(0, Message::system(json_prompt));
            }
            request.json_mode = false;
        }

        let response = self.chat_with_fallback(&request).await?;
        serde_json::from_str(&response.content).map_err(|e| LlmError::JsonOutputParseFailed {
            raw: response.content,
            target_type: std::any::type_name::<T>().to_string(),
            source: e,
        })
    }

    /// 流式 chat
    pub async fn chat_stream(
        &self,
        messages: Vec<Message>,
    ) -> Result<
        std::pin::Pin<Box<dyn futures::Stream<Item = Result<StreamChunk, LlmError>> + Send>>,
        LlmError,
    > {
        let request = self.build_request(messages, None, false);
        let provider = create_provider(&self.config, self.provider_name())?;
        provider
            .chat_stream(&request, self.model(), self.timeout())
            .await
    }

    async fn chat_with_fallback(&self, request: &ChatRequest) -> Result<ChatResponse, LlmError> {
        let provider = create_provider(&self.config, self.provider_name())?;

        match self.do_chat_with_provider(provider.as_ref(), request).await {
            Ok(resp) => Ok(resp),
            Err(e) => {
                if let Some(ref fallback) = self.config.fallback {
                    let mut errors = vec![(self.provider_name().to_string(), e)];
                    for entry in &fallback.chain {
                        let fb_client =
                            self.with_provider(&entry.provider).with_model(&entry.model);
                        let fb_provider = create_provider(&self.config, &entry.provider)?;
                        let mut fb_request = request.clone();
                        fb_request.model = Some(entry.model.clone());
                        match fb_client
                            .do_chat_with_provider(fb_provider.as_ref(), &fb_request)
                            .await
                        {
                            Ok(resp) => {
                                warn!(
                                    from = self.provider_name(),
                                    to = %entry.provider,
                                    "fallback succeeded"
                                );
                                return Ok(resp);
                            }
                            Err(err) => {
                                errors.push((entry.provider.clone(), err));
                            }
                        }
                    }
                    Err(LlmError::AllProvidersFailed(
                        errors.into_iter().map(|(p, e)| (p, Box::new(e))).collect(),
                    ))
                } else {
                    Err(e)
                }
            }
        }
    }
}

/// 客户端构建器。
#[derive(Default)]
pub struct UniLlmClientBuilder {
    config: Option<Config>,
    provider: Option<String>,
    model: Option<String>,
    temperature: Option<f32>,
    max_tokens: Option<u32>,
    timeout_secs: Option<u64>,
}

impl UniLlmClientBuilder {
    pub fn provider(mut self, provider: &str) -> Self {
        self.provider = Some(provider.to_string());
        self
    }

    pub fn model(mut self, model: &str) -> Self {
        self.model = Some(model.to_string());
        self
    }

    pub fn temperature(mut self, t: f32) -> Self {
        self.temperature = Some(t);
        self
    }

    pub fn max_tokens(mut self, n: u32) -> Self {
        self.max_tokens = Some(n);
        self
    }

    pub fn timeout(mut self, d: Duration) -> Self {
        self.timeout_secs = Some(d.as_secs());
        self
    }

    pub fn build(self) -> Result<UniLlmClient, LlmError> {
        let config = self.config.unwrap_or_else(|| {
            let default_toml = r#"
[default]
provider = "openai"
model = "gpt-4o"
temperature = 0.0
max_tokens = 4096
timeout_secs = 60
max_retries = 3
retry_delay_ms = 1000

[providers.openai]
api_key_env = "OPENAI_API_KEY"
base_url = "https://api.openai.com/v1"
models = ["gpt-4o", "gpt-4o-mini", "gpt-4-turbo"]
"#;
            let mut c = Config::parse(default_toml).unwrap_or_else(|_| Config {
                default: crate::config::DefaultConfig::default(),
                providers: std::collections::HashMap::new(),
                fallback: None,
                logging: crate::config::LoggingConfig::default(),
            });
            if let Some(p) = self.provider {
                c.default.provider = p;
            }
            if let Some(m) = self.model {
                c.default.model = m;
            }
            if let Some(t) = self.temperature {
                c.default.temperature = t;
            }
            if let Some(n) = self.max_tokens {
                c.default.max_tokens = n;
            }
            if let Some(secs) = self.timeout_secs {
                c.default.timeout_secs = secs;
            }
            c
        });

        Ok(UniLlmClient {
            config: Arc::new(config),
            provider_override: None,
            model_override: None,
            temperature_override: None,
            max_tokens_override: None,
            timeout_override: None,
        })
    }

    /// 从已有 config 构建，允许覆盖。
    pub fn from_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }
}
