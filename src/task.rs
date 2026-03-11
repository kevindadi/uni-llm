//! 异步任务 API
//!
//! 用于查询和取消 DashScope 异步任务.
//! 任务提交通过在各服务 API 请求头中添加 `X-DashScope-Async: enable` 完成,
//! 提交后返回 task_id,再通过本模块查询或取消.

use serde::{Deserialize, Serialize};

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TaskStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Canceled,
    Unknown,
}

/// 单个任务查询结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskOutput {
    pub task_id: String,
    pub task_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub submit_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheduled_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_time: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_metrics: Option<serde_json::Value>,
}

/// 任务查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    pub output: TaskOutput,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<serde_json::Value>,
}

/// 批量查询中的任务项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListItem {
    pub task_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

/// 批量查询响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskListResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(default)]
    pub data: Vec<TaskListItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_no: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page_size: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// 取消任务响应
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCancelResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}
