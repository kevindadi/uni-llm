//! TOML 配置加载与解析。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::LlmError;

/// 默认配置节。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct DefaultConfig {
    /// 默认 provider。
    #[serde(default = "default_provider")]
    pub provider: String,
    /// 默认模型。
    #[serde(default = "default_model")]
    pub model: String,
    /// 温度。
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    /// 最大 token 数。
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// 超时秒数。
    #[serde(default = "default_timeout_secs")]
    pub timeout_secs: u64,
    /// 最大重试次数。
    #[serde(default = "default_max_retries")]
    pub max_retries: u32,
    /// 初始重试间隔毫秒。
    #[serde(default = "default_retry_delay_ms")]
    pub retry_delay_ms: u64,
}

fn default_provider() -> String {
    "openai".to_string()
}
fn default_model() -> String {
    "gpt-4o".to_string()
}
fn default_temperature() -> f32 {
    0.0
}
fn default_max_tokens() -> u32 {
    4096
}
fn default_timeout_secs() -> u64 {
    60
}
fn default_max_retries() -> u32 {
    3
}
fn default_retry_delay_ms() -> u64 {
    1000
}

impl Default for DefaultConfig {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            model: default_model(),
            temperature: default_temperature(),
            max_tokens: default_max_tokens(),
            timeout_secs: default_timeout_secs(),
            max_retries: default_max_retries(),
            retry_delay_ms: default_retry_delay_ms(),
        }
    }
}

/// 单条 fallback 配置。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FallbackEntry {
    pub provider: String,
    pub model: String,
}

/// Fallback 配置节。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct FallbackConfig {
    pub chain: Vec<FallbackEntry>,
}

/// Provider 配置。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct ProviderConfig {
    /// 从环境变量读取 API key。
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// 百度需要 key + secret。
    #[serde(default)]
    pub api_secret_env: Option<String>,
    /// API base URL。
    pub base_url: String,
    /// 支持的模型列表。
    #[serde(default)]
    pub models: Vec<String>,
}

/// 日志配置。
#[derive(Debug, Clone, serde::Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
    #[serde(default = "default_true")]
    pub log_requests: bool,
    #[serde(default)]
    pub log_responses: bool,
    #[serde(default)]
    pub log_to_file: bool,
    #[serde(default = "default_log_file")]
    pub log_file: String,
}

fn default_log_level() -> String {
    "info".to_string()
}
fn default_true() -> bool {
    true
}
fn default_log_file() -> String {
    "uni-llm.log".to_string()
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            log_requests: default_true(),
            log_responses: false,
            log_to_file: false,
            log_file: default_log_file(),
        }
    }
}

/// 完整配置。
#[derive(Debug, Clone)]
pub struct Config {
    /// 默认配置。
    pub default: DefaultConfig,
    /// 各 provider 配置。
    pub providers: HashMap<String, ProviderConfig>,
    /// Fallback 链。
    pub fallback: Option<FallbackConfig>,
    /// 日志配置。
    pub logging: LoggingConfig,
}

/// 解析用的 TOML 结构。
#[derive(Debug, serde::Deserialize)]
struct ConfigToml {
    #[serde(default)]
    default: DefaultConfig,
    #[serde(default)]
    providers: HashMap<String, ProviderConfig>,
    #[serde(default)]
    fallback: Option<FallbackConfig>,
    #[serde(default)]
    logging: LoggingConfig,
}

impl Config {
    /// 从文件加载配置。
    pub async fn from_file(path: impl AsRef<Path>) -> Result<Self, LlmError> {
        let path: PathBuf = path.as_ref().to_path_buf();
        let content = tokio::fs::read_to_string(&path).await.map_err(|e| {
            LlmError::ConfigLoadFailed {
                path: path.clone(),
                source: Box::new(e),
            }
        })?;
        Self::parse(&content).map_err(|e| LlmError::ConfigLoadFailed {
            path,
            source: Box::new(e),
        })
    }

    /// 从字符串解析配置。
    pub fn parse(s: &str) -> Result<Self, LlmError> {
        let toml: ConfigToml = toml::from_str(s).map_err(|e| LlmError::ConfigLoadFailed {
            path: PathBuf::new(),
            source: Box::new(e),
        })?;
        Ok(Config {
            default: toml.default,
            providers: toml.providers,
            fallback: toml.fallback,
            logging: toml.logging,
        })
    }

    /// 获取超时 Duration。
    pub fn timeout(&self) -> Duration {
        Duration::from_secs(self.default.timeout_secs)
    }

    /// 获取 provider 配置。
    pub fn get_provider(&self, name: &str) -> Option<&ProviderConfig> {
        self.providers.get(name)
    }
}
