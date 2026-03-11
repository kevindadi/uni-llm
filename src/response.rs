//! DashScope API 响应结构定义
//!
//! 映射文档 chat 响应对象格式.

use serde::{Deserialize, Deserializer, Serialize};

use crate::request::MediaElement;

/// 解析 status_code:API 可能返回数字或字符串
fn deserialize_status_code<'de, D>(deserializer: D) -> Result<Option<u16>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StatusCodeValue {
        Num(u16),
        Str(String),
    }
    let opt = Option::<StatusCodeValue>::deserialize(deserializer)?;
    match opt {
        None => Ok(None),
        Some(StatusCodeValue::Num(n)) => Ok(Some(n)),
        Some(StatusCodeValue::Str(s)) => Ok(Some(s.parse().map_err(serde::de::Error::custom)?)),
    }
}

/// 响应中的消息内容
///
/// 纯文本模型返回 String,VL/Audio 模型返回 Array.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResponseContent {
    /// 纯文本
    Text(String),
    /// 多模态(VL/Audio 模型)
    Multimodal(Vec<MediaElement>),
}

/// 响应中的消息对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseMessage {
    /// 角色,固定为 assistant
    pub role: String,
    /// 内容,纯文本或数组
    #[serde(default)]
    pub content: Option<ResponseContent>,
}

/// 单个生成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Choice {
    /// 结束原因:stop、length、tool_calls 等
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// 模型输出的消息
    pub message: ResponseMessage,
}

/// 输出对象
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Output {
    /// 当 result_format 为 text 时的回复内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// 当 result_format 为 text 时的结束原因
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
    /// 当 result_format 为 message 时的选项列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub choices: Option<Vec<Choice>>,
}

/// Token 使用量
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// 输入 Token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    /// 输出 Token 数
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u32>,
    /// 总 Token 数(纯文本时)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_tokens: Option<u32>,
}

/// 生成响应(原始 JSON 结构)
///
/// 用于解析 API 返回的完整 JSON,包含 status_code、code、message 等.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationResponseRaw {
    /// 状态码,200 表示成功(API 可能返回数字或字符串)
    #[serde(default, deserialize_with = "deserialize_status_code")]
    pub status_code: Option<u16>,
    /// 请求 ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    /// 错误码,成功时为空
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    /// 错误或提示信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// 输出结果
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Output>,
    /// Token 使用量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// 解析后的成功响应
///
/// 仅包含业务成功时的有效数据.
#[derive(Debug, Clone)]
pub struct GenerationResponse {
    /// 请求 ID
    pub request_id: Option<String>,
    /// 输出结果
    pub output: Output,
    /// Token 使用量
    pub usage: Option<Usage>,
}

impl GenerationResponse {
    /// 获取第一个 choice 的文本内容
    pub fn text(&self) -> Option<String> {
        if let Some(ref text) = self.output.text {
            return Some(text.clone());
        }
        self.output.choices.as_ref().and_then(|choices| {
            choices.first().and_then(|c| {
                c.message.content.as_ref().map(|content| match content {
                    ResponseContent::Text(s) => s.clone(),
                    ResponseContent::Multimodal(elements) => elements
                        .iter()
                        .find_map(|e| e.text.clone())
                        .unwrap_or_default(),
                })
            })
        })
    }
}
