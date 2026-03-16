//! 公共类型定义：消息、请求、响应、工具调用等。

use serde::{Deserialize, Serialize};

/// 消息角色。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    /// 系统提示，用于设定模型行为。
    System,
    /// 用户消息。
    User,
    /// 助手回复。
    Assistant,
    /// 工具调用结果回传。
    Tool,
}

/// 工具调用请求（由 assistant 发起）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// 工具调用唯一 ID。
    pub id: String,
    /// 函数名称。
    pub function_name: String,
    /// JSON 格式的参数。
    pub arguments: serde_json::Value,
}

/// 流式 tool call 增量。
#[derive(Debug, Clone, Default)]
pub struct ToolCallDelta {
    /// 工具调用 ID 增量。
    pub id: Option<String>,
    /// 函数名增量。
    pub function_name: Option<String>,
    /// 参数 JSON 增量。
    pub arguments: Option<String>,
}

/// 工具定义（JSON Schema）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// 工具名称。
    pub name: String,
    /// 工具描述。
    pub description: String,
    /// JSON Schema 参数定义。
    pub parameters: serde_json::Value,
}

impl ToolDefinition {
    /// 创建工具定义。
    pub fn new(name: impl Into<String>, description: impl Into<String>, parameters: serde_json::Value) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters,
        }
    }
}

/// 单条对话消息。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// 消息角色。
    pub role: Role,
    /// 消息内容。
    pub content: String,
    /// 用于 tool 结果回传时关联的 tool_call_id。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// assistant 发起的 tool 调用列表。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

impl Message {
    /// 创建系统消息。
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: Role::System,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    /// 创建用户消息。
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: Role::User,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    /// 创建助手消息（纯文本）。
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: None,
        }
    }

    /// 创建带 tool calls 的助手消息。
    pub fn assistant_with_tool_calls(content: impl Into<String>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: Role::Assistant,
            content: content.into(),
            tool_call_id: None,
            tool_calls: Some(tool_calls),
        }
    }

    /// 创建 tool 结果回传消息。
    pub fn tool(tool_call_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            role: Role::Tool,
            content: content.into(),
            tool_call_id: Some(tool_call_id.into()),
            tool_calls: None,
        }
    }
}

/// Token 用量统计。
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    /// 输入 token 数。
    pub prompt_tokens: u32,
    /// 输出 token 数。
    pub completion_tokens: u32,
    /// 总 token 数。
    pub total_tokens: u32,
}

/// Chat 请求。
#[derive(Debug, Clone)]
pub struct ChatRequest {
    /// 消息历史。
    pub messages: Vec<Message>,
    /// 覆盖默认模型。
    pub model: Option<String>,
    /// 温度，0.0 表示确定性输出。
    pub temperature: Option<f32>,
    /// 最大生成 token 数。
    pub max_tokens: Option<u32>,
    /// 工具定义，不传则走普通 chat。
    pub tools: Option<Vec<ToolDefinition>>,
    /// 是否启用 JSON 结构化输出。
    pub json_mode: bool,
}

/// Chat 响应。
#[derive(Debug, Clone)]
pub struct ChatResponse {
    /// 回复内容。
    pub content: String,
    /// 工具调用请求（若有）。
    pub tool_calls: Option<Vec<ToolCall>>,
    /// Token 用量。
    pub usage: TokenUsage,
    /// 实际使用的模型名。
    pub model: String,
    /// 实际使用的 provider 名。
    pub provider: String,
    /// 请求耗时。
    pub latency: std::time::Duration,
}

/// 流式 chunk。
#[derive(Debug, Clone)]
pub struct StreamChunk {
    /// 增量文本。
    pub delta: String,
    /// 流式 tool call 增量。
    pub tool_calls_delta: Option<Vec<ToolCallDelta>>,
    /// 是否为最后一个 chunk。
    pub is_final: bool,
    /// 最后一个 chunk 可能带 usage。
    pub usage: Option<TokenUsage>,
}
