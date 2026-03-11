//! DashScope API 客户端实现

use crate::error::DashScopeError;
use crate::request::{ApiEndpoint, GenerationRequest};
use crate::response::{GenerationResponse, GenerationResponseRaw};
use crate::stream::{parse_sse_stream, StreamChunk};
use crate::task::{TaskCancelResponse, TaskListResponse, TaskResponse};
use reqwest::Client as HttpClient;
use std::time::Duration;

/// 默认 Base URL(北京地域)
pub const DEFAULT_BASE_URL: &str = "https://dashscope.aliyuncs.com/api/v1";

/// 文本生成端点
const TEXT_GENERATION_PATH: &str = "services/aigc/text-generation/generation";

/// 多模态生成端点
const MULTIMODAL_GENERATION_PATH: &str = "services/aigc/multimodal-generation/generation";

/// 判断是否为多模态模型(需使用 multimodal-generation 端点)
fn is_multimodal_model(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("vl") || m.contains("vision") || m.contains("qwen3.5")
}

/// DashScope API 客户端
#[derive(Debug, Clone)]
pub struct Client {
    api_key: String,
    base_url: String,
    http_client: HttpClient,
}

impl Client {
    /// 使用 API Key 创建客户端(北京地域)
    ///
    /// # Errors
    ///
    /// 当 API Key 为空时返回 `InvalidConfiguration`
    pub fn new(api_key: impl Into<String>) -> Result<Self, DashScopeError> {
        let api_key = api_key.into();
        if api_key.trim().is_empty() {
            return Err(DashScopeError::InvalidConfiguration(
                "API Key is missing. Please set it in the environment or client.".into(),
            ));
        }
        let http_client = HttpClient::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(DashScopeError::RequestBuildError)?;
        Ok(Self {
            api_key,
            base_url: DEFAULT_BASE_URL.to_string(),
            http_client,
        })
    }

    /// 使用自定义 Base URL 创建客户端(支持新加坡、美国等地域)
    pub fn with_base_url(
        api_key: impl Into<String>,
        base_url: impl Into<String>,
    ) -> Result<Self, DashScopeError> {
        let mut client = Self::new(api_key)?;
        client.base_url = base_url.into();
        Ok(client)
    }

    /// 创建 Builder
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    /// 调用模型生成文本
    ///
    /// 根据模型名称自动选择 text-generation 或 multimodal-generation 端点.
    ///
    /// # Errors
    ///
    /// 可能返回 `HttpError`、`ApiResponseError`、`SerializationError`、`UnexpectedResponse`
    pub async fn generate(
        &self,
        request: GenerationRequest,
    ) -> Result<GenerationResponse, DashScopeError> {
        let path = match request.endpoint {
            Some(ApiEndpoint::TextGeneration) => TEXT_GENERATION_PATH,
            Some(ApiEndpoint::MultimodalGeneration) => MULTIMODAL_GENERATION_PATH,
            None => {
                if is_multimodal_model(&request.model) {
                    MULTIMODAL_GENERATION_PATH
                } else {
                    TEXT_GENERATION_PATH
                }
            }
        };
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);

        let body = serde_json::to_string(&request).map_err(DashScopeError::SerializationError)?;

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .body(body)
            .send()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        let status = resp.status();
        let status_code = status.as_u16();
        let body_text = resp
            .text()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        if !status.is_success() {
            return Err(DashScopeError::HttpError {
                status_code,
                message: body_text,
            });
        }

        let raw: GenerationResponseRaw =
            serde_json::from_str(&body_text).map_err(DashScopeError::SerializationError)?;

        // 仅当明确返回错误时判定失败:status_code 存在且非 200,或 code 非空
        let is_error = raw.status_code.map(|c| c != 200).unwrap_or(false)
            || raw.code.as_ref().map(|c| !c.is_empty()).unwrap_or(false);

        if is_error {
            let message = raw.message.filter(|m| !m.is_empty()).unwrap_or_else(|| {
                format!(
                    "API 返回异常.原始响应: {}",
                    &body_text[..body_text.len().min(500)]
                )
            });
            return Err(DashScopeError::ApiResponseError {
                code: raw.code,
                message,
            });
        }

        let output = raw.output.ok_or_else(|| {
            DashScopeError::UnexpectedResponse("response missing 'output' field".into())
        })?;

        Ok(GenerationResponse {
            request_id: raw.request_id,
            output,
            usage: raw.usage,
        })
    }

    /// 流式调用模型生成文本
    ///
    /// 自动设置 stream=true、incremental_output=true 及 X-DashScope-SSE 请求头.
    /// 返回 SSE 解析后的 StreamChunk 流.
    pub async fn generate_stream(
        &self,
        mut request: GenerationRequest,
    ) -> Result<
        impl futures_util::Stream<Item = Result<StreamChunk, DashScopeError>> + Send,
        DashScopeError,
    > {
        let path = match request.endpoint {
            Some(ApiEndpoint::TextGeneration) => TEXT_GENERATION_PATH,
            Some(ApiEndpoint::MultimodalGeneration) => MULTIMODAL_GENERATION_PATH,
            None => {
                if is_multimodal_model(&request.model) {
                    MULTIMODAL_GENERATION_PATH
                } else {
                    TEXT_GENERATION_PATH
                }
            }
        };
        let url = format!("{}/{}", self.base_url.trim_end_matches('/'), path);

        request
            .parameters
            .get_or_insert_with(Default::default)
            .stream = Some(true);
        request
            .parameters
            .get_or_insert_with(Default::default)
            .incremental_output = Some(true);

        let body = serde_json::to_string(&request).map_err(DashScopeError::SerializationError)?;

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .header("X-DashScope-SSE", "enable")
            .body(body)
            .send()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        let status = resp.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            let message = resp
                .text()
                .await
                .map_err(DashScopeError::RequestBuildError)?;
            return Err(DashScopeError::HttpError {
                status_code,
                message,
            });
        }

        Ok(parse_sse_stream(resp))
    }

    /// 查询单个异步任务
    pub async fn get_task(&self, task_id: &str) -> Result<TaskResponse, DashScopeError> {
        let url = format!("{}/tasks/{}", self.base_url.trim_end_matches('/'), task_id);
        let resp = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        let status = resp.status();
        let body_text = resp
            .text()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        if !status.is_success() {
            return Err(DashScopeError::HttpError {
                status_code: status.as_u16(),
                message: body_text,
            });
        }

        let raw: serde_json::Value =
            serde_json::from_str(&body_text).map_err(DashScopeError::SerializationError)?;

        let output = raw
            .get("output")
            .ok_or_else(|| DashScopeError::UnexpectedResponse("response missing 'output'".into()))?;
        let output: crate::task::TaskOutput =
            serde_json::from_value(output.clone()).map_err(DashScopeError::SerializationError)?;

        Ok(TaskResponse {
            request_id: raw.get("request_id").and_then(|v| v.as_str()).map(String::from),
            output,
            usage: raw.get("usage").cloned(),
        })
    }

    /// 批量查询异步任务
    #[allow(clippy::too_many_arguments)]
    pub async fn list_tasks(
        &self,
        task_id: Option<&str>,
        start_time: Option<&str>,
        end_time: Option<&str>,
        status: Option<&str>,
        model_name: Option<&str>,
        page_no: Option<u32>,
        page_size: Option<u32>,
    ) -> Result<TaskListResponse, DashScopeError> {
        let url = format!("{}/tasks/", self.base_url.trim_end_matches('/'));

        let page_no_str = page_no.map(|p| p.to_string());
        let page_size_str = page_size.map(|p| p.to_string());

        let mut query_params: Vec<(&str, &str)> = Vec::new();
        if let Some(id) = task_id {
            query_params.push(("task_id", id));
        }
        if let Some(s) = start_time {
            query_params.push(("start_time", s));
        }
        if let Some(e) = end_time {
            query_params.push(("end_time", e));
        }
        if let Some(s) = status {
            query_params.push(("status", s));
        }
        if let Some(m) = model_name {
            query_params.push(("model_name", m));
        }
        if let Some(ref p) = page_no_str {
            query_params.push(("page_no", p));
        }
        if let Some(ref p) = page_size_str {
            query_params.push(("page_size", p));
        }

        let mut req = self
            .http_client
            .get(&url)
            .header("Authorization", format!("Bearer {}", self.api_key));
        if !query_params.is_empty() {
            req = req.query(&query_params);
        }

        let resp = req
            .send()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        let status = resp.status();
        let body_text = resp
            .text()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        if !status.is_success() {
            return Err(DashScopeError::HttpError {
                status_code: status.as_u16(),
                message: body_text,
            });
        }

        let raw: TaskListResponse =
            serde_json::from_str(&body_text).map_err(DashScopeError::SerializationError)?;

        if raw.code.as_ref().map(|c| !c.is_empty()).unwrap_or(false) {
            return Err(DashScopeError::ApiResponseError {
                code: raw.code,
                message: raw.message.unwrap_or_else(|| "Unknown API error".into()),
            });
        }

        Ok(raw)
    }

    /// 取消异步任务(仅 PENDING 状态可取消)
    pub async fn cancel_task(&self, task_id: &str) -> Result<TaskCancelResponse, DashScopeError> {
        let url = format!(
            "{}/tasks/{}/cancel",
            self.base_url.trim_end_matches('/'),
            task_id
        );

        let resp = self
            .http_client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        let status = resp.status();
        let body_text = resp
            .text()
            .await
            .map_err(DashScopeError::RequestBuildError)?;

        if !status.is_success() {
            return Err(DashScopeError::HttpError {
                status_code: status.as_u16(),
                message: body_text,
            });
        }

        let raw: TaskCancelResponse =
            serde_json::from_str(&body_text).map_err(DashScopeError::SerializationError)?;

        Ok(raw)
    }
}

/// Client Builder
#[derive(Debug, Default)]
pub struct ClientBuilder {
    api_key: Option<String>,
    base_url: Option<String>,
}

impl ClientBuilder {
    /// 设置 API Key
    pub fn api_key(mut self, api_key: impl Into<String>) -> Self {
        self.api_key = Some(api_key.into());
        self
    }

    /// 设置 Base URL
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    /// 构建客户端
    pub fn build(self) -> Result<Client, DashScopeError> {
        let api_key = self
            .api_key
            .ok_or_else(|| DashScopeError::InvalidConfiguration("api_key is required".into()))?;

        match self.base_url {
            Some(base_url) => Client::with_base_url(api_key, base_url),
            None => Client::new(api_key),
        }
    }
}
