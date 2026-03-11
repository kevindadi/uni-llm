# 模型与端点对应关系

本文档说明 DashScope 千问系列模型应使用的 API 端点(URL 路径),以及本 SDK 的自动选择逻辑.

## 端点概览

| 端点类型       | 路径                                             | 用途                     |
| -------------- | ------------------------------------------------ | ------------------------ |
| **文本生成**   | `services/aigc/text-generation/generation`       | 纯文本对话、代码生成等   |
| **多模态生成** | `services/aigc/multimodal-generation/generation` | 图像/视频/音频理解与生成 |

## 完整 URL(按地域)

Base URL 为 `https://{region}/api/v1`,路径拼接后得到完整请求地址.

### 北京(华北2)

| 端点     | 完整 URL                                                                               |
| -------- | -------------------------------------------------------------------------------------- |
| 文本生成 | `https://dashscope.aliyuncs.com/api/v1/services/aigc/text-generation/generation`       |
| 多模态   | `https://dashscope.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation` |

### 新加坡

| 端点     | 完整 URL                                                                                    |
| -------- | ------------------------------------------------------------------------------------------- |
| 文本生成 | `https://dashscope-intl.aliyuncs.com/api/v1/services/aigc/text-generation/generation`       |
| 多模态   | `https://dashscope-intl.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation` |

### 美国(弗吉尼亚)

| 端点     | 完整 URL                                                                                  |
| -------- | ----------------------------------------------------------------------------------------- |
| 文本生成 | `https://dashscope-us.aliyuncs.com/api/v1/services/aigc/text-generation/generation`       |
| 多模态   | `https://dashscope-us.aliyuncs.com/api/v1/services/aigc/multimodal-generation/generation` |

---

## 模型 → 端点对应表

### 文本生成端点(text-generation)

| 模型名称             | 说明               |
| -------------------- | ------------------ |
| qwen-plus            | 千问 Plus,能力均衡 |
| qwen-plus-latest     | 同上,指向最新      |
| qwen-plus-2025-12-01 | 快照版本           |
| qwen-turbo           | 千问 Turbo,速度快  |
| qwen-max             | 千问 Max,能力最强  |
| qwen3-max            | Qwen3 Max          |
| qwen3-max-preview    | Qwen3 Max 预览     |
| qwen-flash           | 千问 Flash         |
| qwen-long            | 长文本处理         |
| qwen-coder           | 代码模型           |
| qwen-math            | 数学模型           |
| deepseek-\*          | 第三方 DeepSeek    |
| 其他纯文本模型       | ...                |

### 多模态端点(multimodal-generation)

| 模型名称                | 说明                            |
| ----------------------- | ------------------------------- |
| qwen3.5-plus            | Qwen3.5 Plus,支持文本+图像+视频 |
| qwen3.5-plus-2026-02-15 | 快照版本                        |
| qwen3.5-flash           | Qwen3.5 Flash                   |
| qwen3-vl-plus           | Qwen3 视觉理解                  |
| qwen3-vl-flash          | Qwen3-VL 轻量版                 |
| qwen-vl-plus            | 千问 VL Plus                    |
| qwen-vl-max             | 千问 VL Max                     |
| qwen2.5-vl-\*           | Qwen2.5-VL 系列                 |
| qwen-audio              | 音频理解                        |
| qwen-omni               | 全模态                          |
| qwen-omni-realtime      | 实时多模态                      |
| QVQ 系列                | 视觉推理                        |

---

## SDK 自动选择逻辑

当未显式指定 `.endpoint()` 时,SDK 根据模型名自动选择:

- 模型名包含 **`vl`**、**`vision`**、**`qwen3.5`** → 使用 **multimodal-generation**
- 其他 → 使用 **text-generation**

```rust
// 自动选择:qwen-plus → text-generation
let req = GenerationRequest::builder()
    .model("qwen-plus")
    .messages(vec![Message::user("你好")])
    .build()?;

// 自动选择:qwen3.5-plus → multimodal-generation
let req = GenerationRequest::builder()
    .model("qwen3.5-plus")
    .messages(vec![Message::user("你好")])
    .build()?;
```

## 显式指定端点

若需覆盖自动选择,使用 `.endpoint()`:

```rust
use qwen_sdk::ApiEndpoint;

// 强制使用文本端点
let req = GenerationRequest::builder()
    .model("qwen-plus")
    .messages(vec![Message::user("你好")])
    .endpoint(ApiEndpoint::TextGeneration)
    .build()?;

// 强制使用多模态端点
let req = GenerationRequest::builder()
    .model("qwen-plus")
    .messages(vec![Message::user("你好")])
    .endpoint(ApiEndpoint::MultimodalGeneration)
    .build()?;  // 会失败:qwen-plus 不支持多模态端点
```

## 错误提示

端点与模型不匹配时,API 通常会返回:

- `InvalidParameter` - 参数错误
- `ModelNotSupported` - 模型不支持
- 或类似业务错误码

## 参考链接

- [模型总览](https://help.aliyun.com/zh/model-studio/getting-started/models)
- [DashScope API 参考](https://help.aliyun.com/zh/model-studio/qwen-api-via-dashscope)
