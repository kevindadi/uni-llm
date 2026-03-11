//! DashScope API 请求结构定义
//!
//! 严格映射官方文档的 JSON Schema,支持文本生成与多模态输入.

use serde::{Deserialize, Serialize};

use crate::error::DashScopeError;

/// 视频输入:支持单个 URL 或 URL 列表
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VideoInput {
    /// 单个视频 URL
    Single(String),
    /// 视频帧图像 URL 列表
    Multiple(Vec<String>),
}

/// 多模态消息中的媒体元素
///
/// 支持 image、video、audio、text 等类型,对应文档中的 content 数组元素.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaElement {
    /// 图像 URL、Base64 或本地路径
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,

    /// 视频 URL 或帧图像 URL 列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub video: Option<VideoInput>,

    /// 每秒抽帧数,与 video 配合使用
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fps: Option<f32>,

    /// 音频 URL 或 Base64
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,

    /// 文本内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl MediaElement {
    /// 创建纯图像元素
    pub fn image(url: impl Into<String>) -> Self {
        Self {
            image: Some(url.into()),
            video: None,
            fps: None,
            audio: None,
            text: None,
        }
    }

    /// 创建纯文本元素
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            image: None,
            video: None,
            fps: None,
            audio: None,
            text: Some(content.into()),
        }
    }

    /// 创建视频元素(URL 列表 + 可选 fps)
    pub fn video(urls: impl Into<VideoInput>, fps: Option<f32>) -> Self {
        Self {
            image: None,
            video: Some(urls.into()),
            fps,
            audio: None,
            text: None,
        }
    }

    /// 创建音频元素
    pub fn audio(url: impl Into<String>) -> Self {
        Self {
            image: None,
            video: None,
            fps: None,
            audio: Some(url.into()),
            text: None,
        }
    }
}

/// 消息内容:纯文本或多模态
///
/// 文档规定:纯文本模式下 content 为 String,
/// 多模态(VL/Video/Audio)模式下为 Array&lt;Object&gt;.
#[derive(Debug, Clone)]
pub enum Content {
    /// 纯文本内容
    Text(String),
    /// 多模态内容(图像、视频、音频、文本混合)
    Multimodal(Vec<MediaElement>),
}

impl Content {
    /// 创建纯文本内容
    pub fn text(s: impl Into<String>) -> Self {
        Content::Text(s.into())
    }

    /// 创建多模态内容
    pub fn multimodal(elements: Vec<MediaElement>) -> Self {
        Content::Multimodal(elements)
    }
}

impl Serialize for Content {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Content::Text(s) => s.serialize(serializer),
            Content::Multimodal(v) => v.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for Content {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        if value.is_string() {
            Ok(Content::Text(
                value.as_str().unwrap_or_default().to_string(),
            ))
        } else if value.is_array() {
            let arr: Vec<MediaElement> =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;
            Ok(Content::Multimodal(arr))
        } else {
            Err(serde::de::Error::custom(
                "content must be string or array of media elements",
            ))
        }
    }
}

/// 消息角色
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    System,
    User,
    Assistant,
    Tool,
}

/// 单条对话消息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息角色
    pub role: Role,
    /// 消息内容(纯文本或多模态)
    pub content: Content,
}

impl Message {
    /// 创建系统消息
    pub fn system(content: impl Into<Content>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
        }
    }

    /// 创建用户消息
    pub fn user(content: impl Into<Content>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
        }
    }

    /// 创建助手消息
    pub fn assistant(content: impl Into<Content>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
        }
    }
}

impl From<String> for Content {
    fn from(s: String) -> Self {
        Content::Text(s)
    }
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Content::Text(s.to_string())
    }
}

impl From<Vec<MediaElement>> for Content {
    fn from(v: Vec<MediaElement>) -> Self {
        Content::Multimodal(v)
    }
}

/// 请求输入体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Input {
    /// 对话消息列表
    pub messages: Vec<Message>,
}

/// 生成参数
///
/// 映射文档 parameters 对象中的核心参数.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Parameters {
    /// 采样温度,控制生成多样性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    /// 核采样概率阈值
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,

    /// 最大输出 Token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,

    /// 返回格式,推荐 "message"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_format: Option<String>,

    /// 停止词列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,

    /// 是否流式输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// 流式时是否增量输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incremental_output: Option<bool>,
}

/// API 端点类型
///
/// 用于显式指定调用哪个端点,覆盖基于模型名的自动选择.
/// - `TextGeneration`: 纯文本模型(qwen-plus、qwen-turbo 等)
/// - `MultimodalGeneration`: 多模态模型(qwen-vl、qwen3.5-plus 等)
///
/// 若用多模态端点调用 qwen-plus,或反之,API 通常会返回 `InvalidParameter` 等错误.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiEndpoint {
    /// 文本生成:`/services/aigc/text-generation/generation`
    TextGeneration,
    /// 多模态生成:`/services/aigc/multimodal-generation/generation`
    MultimodalGeneration,
}

/// 文本生成请求体
///
/// 对应文档顶层请求结构:model、input、parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRequest {
    /// 模型名称(必选)
    pub model: String,
    /// 输入内容(必选)
    pub input: Input,
    /// 生成参数(可选)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Parameters>,
    /// 显式指定端点,不参与 JSON 序列化.为 None 时根据 model 自动选择
    #[serde(skip)]
    pub endpoint: Option<ApiEndpoint>,
}

/// GenerationRequest 的 Builder
#[derive(Debug, Default)]
pub struct GenerationRequestBuilder {
    model: Option<String>,
    messages: Option<Vec<Message>>,
    parameters: Option<Parameters>,
    endpoint: Option<ApiEndpoint>,
}

impl GenerationRequestBuilder {
    /// 创建新的 Builder
    pub fn new() -> Self {
        Self::default()
    }

    /// 设置模型名称
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// 设置消息列表
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = Some(messages);
        self
    }

    /// 设置参数
    pub fn parameters(mut self, parameters: Parameters) -> Self {
        self.parameters = Some(parameters);
        self
    }

    /// 设置 temperature
    pub fn temperature(mut self, temperature: f32) -> Self {
        self.parameters
            .get_or_insert_with(Parameters::default)
            .temperature = Some(temperature);
        self
    }

    /// 设置 top_p
    pub fn top_p(mut self, top_p: f32) -> Self {
        self.parameters
            .get_or_insert_with(Parameters::default)
            .top_p = Some(top_p);
        self
    }

    /// 设置 max_tokens
    pub fn max_tokens(mut self, max_tokens: u32) -> Self {
        self.parameters
            .get_or_insert_with(Parameters::default)
            .max_tokens = Some(max_tokens);
        self
    }

    /// 设置 result_format
    pub fn result_format(mut self, result_format: impl Into<String>) -> Self {
        self.parameters
            .get_or_insert_with(Parameters::default)
            .result_format = Some(result_format.into());
        self
    }

    /// 设置 stop 词
    pub fn stop(mut self, stop: Vec<String>) -> Self {
        self.parameters.get_or_insert_with(Parameters::default).stop = Some(stop);
        self
    }

    /// 设置 stream
    pub fn stream(mut self, stream: bool) -> Self {
        self.parameters
            .get_or_insert_with(Parameters::default)
            .stream = Some(stream);
        self
    }

    /// 显式指定 API 端点,覆盖基于模型名的自动选择
    ///
    /// 例如:用多模态端点调用 qwen-plus 通常会失败；反之亦然.
    pub fn endpoint(mut self, endpoint: ApiEndpoint) -> Self {
        self.endpoint = Some(endpoint);
        self
    }

    /// 构建请求
    pub fn build(self) -> Result<GenerationRequest, DashScopeError> {
        let model = self
            .model
            .ok_or_else(|| DashScopeError::InvalidConfiguration("model is required".into()))?;
        let messages = self
            .messages
            .ok_or_else(|| DashScopeError::InvalidConfiguration("messages is required".into()))?;

        Ok(GenerationRequest {
            model,
            input: Input { messages },
            parameters: self.parameters,
            endpoint: self.endpoint,
        })
    }
}

impl GenerationRequest {
    /// 创建 Builder
    pub fn builder() -> GenerationRequestBuilder {
        GenerationRequestBuilder::new()
    }
}
