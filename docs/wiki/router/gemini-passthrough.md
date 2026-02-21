# Gemini 双格式调用方案 (Dual Format Passthrough)

> **Created:** 2026-02-19
> **Status:** Design
> **Related Files:** `crates/router/src/lib.rs`, `crates/router/src/adaptor/factory.rs`, `crates/router/src/adaptor/gemini.rs`

---

## 概述

BurnCloud 支持 Gemini Channel 的双格式调用：
1. **OpenAI 格式** - 标准 `/v1/chat/completions` 端点，自动协议转换
2. **Gemini 原生格式** - 透传模式，直接调用 Gemini 原生 API

---

## 检测逻辑

### 双重检测机制

```
┌─────────────────────────────────────────────────────────────┐
│                     请求进入 proxy_logic                      │
└─────────────────────────────────────────────────────────────┘
                              │
                              ▼
              ┌───────────────────────────────┐
              │  检测 1: 路径匹配              │
              │  path.starts_with("/v1beta/models/")  │
              │  OR path.starts_with("/v1/models/")   │
              └───────────────────────────────┘
                     │                │
                   匹配            不匹配
                     │                │
                     ▼                ▼
              ┌──────────┐   ┌───────────────────┐
              │ 透传模式  │   │ 检测 2: 内容格式   │
              └──────────┘   │ body 有 "contents" │
                              └───────────────────┘
                                    │          │
                                  匹配       不匹配
                                    │          │
                                    ▼          ▼
                              ┌──────────┐ ┌──────────┐
                              │ 透传模式  │ │ 协议转换  │
                              └──────────┘ └──────────┘
```

### 检测函数

```rust
/// 检测是否应该透传（直接转发原始请求）
fn should_passthrough(path: &str, body: &serde_json::Value) -> bool {
    // 条件 1: Gemini 原生路径
    let is_gemini_path = path.starts_with("/v1beta/models/")
        || path.starts_with("/v1/models/");

    // 条件 2: Gemini 原生内容格式 (有 contents 字段)
    let is_gemini_content = body.get("contents").is_some();

    // 任一条件满足即透传
    is_gemini_path || is_gemini_content
}
```

---

## 场景矩阵

| 路径 | 内容格式 | 处理方式 | 说明 |
|------|----------|----------|------|
| `/v1beta/models/gemini-pro:generateContent` | Gemini (`contents`) | **透传** | 完全原生调用 |
| `/v1beta/models/gemini-pro:generateContent` | OpenAI (`messages`) | **透传** | 路径优先，用户自己负责 |
| `/v1/chat/completions` | Gemini (`contents`) | **透传** | 内容检测命中 |
| `/v1/chat/completions` | OpenAI (`messages`) | **转换** | 标准流程 |

---

## 格式对比

### OpenAI 格式 (转换为 Gemini)

**请求:**
```json
{
  "model": "gemini-pro",
  "messages": [
    {"role": "user", "content": "Hello"}
  ],
  "temperature": 0.7,
  "max_tokens": 100
}
```

**转换后发送到 Gemini:**
```json
{
  "contents": [
    {"role": "user", "parts": [{"text": "Hello"}]}
  ],
  "generationConfig": {
    "temperature": 0.7,
    "maxOutputTokens": 100
  }
}
```

### Gemini 原生格式 (透传)

**请求:**
```json
{
  "contents": [
    {"role": "user", "parts": [{"text": "Hello"}]}
  ],
  "generationConfig": {
    "temperature": 0.7,
    "maxOutputTokens": 100
  },
  "safetySettings": [
    {"category": "HARM_CATEGORY_HARASSMENT", "threshold": "BLOCK_NONE"}
  ]
}
```

**直接透传，不做任何转换。**

---

## 代码改动点

### 1. `lib.rs` - 添加检测函数

位置: `crates/router/src/lib.rs`

```rust
/// 检测是否应该透传（直接转发原始请求）
fn should_passthrough(path: &str, body: &serde_json::Value) -> bool {
    // 条件 1: Gemini 原生路径
    let is_gemini_path = path.starts_with("/v1beta/models/")
        || path.starts_with("/v1/models/");

    // 条件 2: Gemini 原生内容格式 (有 contents 字段)
    let is_gemini_content = body.get("contents").is_some();

    // 任一条件满足即透传
    is_gemini_path || is_gemini_content
}

/// 构建透传 URL
fn build_gemini_passthrough_url(upstream: &Upstream, path: &str, body: &Value) -> String {
    // 如果路径已经是 Gemini 格式，直接使用
    if path.starts_with("/v1beta/models/") || path.starts_with("/v1/models/") {
        let base = if upstream.base_url.is_empty() {
            "https://generativelanguage.googleapis.com"
        } else {
            &upstream.base_url
        };
        return format!("{}{}", base, path);
    }

    // 否则从 body 中提取 model 构建 URL
    let model = body.get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("gemini-pro");

    format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent",
        model
    )
}
```

### 2. `lib.rs` - proxy_logic 集成

位置: `crates/router/src/lib.rs` 的 `proxy_logic` 函数中，约第 635 行

```rust
// 3. Prepare Request Body
let body_json: serde_json::Value = match serde_json::from_slice(&body_bytes) {
    Ok(v) => v,
    Err(_) => {
        last_error = "Invalid JSON body".to_string();
        continue;
    }
};

// === 新增：检测是否透传 ===
let is_passthrough = should_passthrough(path, &body_json);

if is_passthrough && channel_type == ChannelType::Gemini {
    // 透传模式：直接转发原始请求
    println!("Gemini passthrough mode enabled for path: {}", path);

    let target_url = build_gemini_passthrough_url(&upstream, path, &body_json);

    let req_builder = state.client
        .request(method.clone(), &target_url)
        .header("x-goog-api-key", &upstream.api_key)
        .json(&body_json);

    // 执行透传请求
    match req_builder.send().await {
        Ok(resp) => {
            // 透传响应，不做转换
            return (
                handle_passthrough_response(resp, &token_counter),
                last_upstream_id,
                resp.status(),
            );
        }
        Err(e) => {
            last_error = format!("Network Error: {}", e);
            continue;
        }
    }
} else {
    // 现有的协议转换逻辑
    let request_body_json = adaptor.convert_request(...);
    // ...
}
```

### 3. 透传响应处理

```rust
/// 处理透传响应（不做格式转换，仅解析 token 用于计费）
fn handle_passthrough_response(
    resp: reqwest::Response,
    token_counter: &Arc<StreamingTokenCounter>,
) -> Response {
    let status = resp.status();
    let mut response_builder = Response::builder().status(status);

    if let Some(headers_mut) = response_builder.headers_mut() {
        for (k, v) in resp.headers() {
            headers_mut.insert(k, v.clone());
        }
    }

    let stream = resp.bytes_stream();
    let counter_clone = Arc::clone(token_counter);

    // 解析 Gemini 流式响应中的 token 用量
    let mapped_stream = stream.map(move |chunk_result| match chunk_result {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes);

            // 尝试解析 Gemini 的 usageMetadata
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                if let Some(usage) = json.get("usageMetadata") {
                    let prompt = usage.get("promptTokenCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    let completion = usage.get("candidatesTokenCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    counter_clone.add_tokens(prompt as u32, completion as u32);
                }
            }

            Ok(bytes)
        }
        Err(e) => Err(std::io::Error::new(std::io::ErrorKind::Other, e)),
    });

    let body = Body::from_stream(mapped_stream);
    response_builder.body(body).unwrap_or_else(|_| Response::new(Body::empty()))
}
```

---

## 完整流程图

### OpenAI 格式请求流程

```
请求: POST /v1/chat/completions
Body: {"model": "gemini-pro", "messages": [...]}

        │
        ▼
┌───────────────────────────────────┐
│ 1. Token 验证 & 额度检查           │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 2. Model Router 路由到 Gemini Channel │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 3. should_passthrough() 检测      │
│    path = "/v1/chat/completions"  │
│    body has "contents" = false    │
│    → is_passthrough = false       │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 4. 协议转换模式                    │
│    - OpenAI → Gemini 格式转换     │
│    - URL: 构建标准 Gemini URL     │
│    - Header: x-goog-api-key       │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 5. 响应转换                        │
│    Gemini → OpenAI 格式           │
└───────────────────────────────────┘
```

### Gemini 原生格式请求流程

```
请求: POST /v1/chat/completions
Body: {"contents": [...], "generationConfig": {...}}

        │
        ▼
┌───────────────────────────────────┐
│ 1. Token 验证 & 额度检查           │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 2. Model Router 路由到 Gemini Channel │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 3. should_passthrough() 检测      │
│    path = "/v1/chat/completions"  │
│    body has "contents" = true     │
│    → is_passthrough = true        │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 4. 透传模式                        │
│    - URL: build_gemini_passthrough_url() │
│    - Header: x-goog-api-key       │
│    - Body: 原始 JSON (不转换)      │
└───────────────────────────────────┘
        │
        ▼
┌───────────────────────────────────┐
│ 5. 返回原始 Gemini 响应 (不转换)   │
│    仅解析 usageMetadata 用于计费   │
└───────────────────────────────────┘
```

---

## 设计决策

| 问题 | 决策 | 理由 |
|------|------|------|
| 透传时响应是否转换？ | **否，完全透传** | 用户选择原生格式，应自行处理响应 |
| 流式响应如何处理？ | **透传 SSE** | 保持 Gemini 原生流格式 |
| 计费如何计算？ | **解析 usageMetadata** | 从 Gemini 响应中提取 token 用量 |
| 路径优先还是内容优先？ | **路径优先** | 明确的路径表示用户意图清晰 |

---

## 使用示例

### 使用 OpenAI 格式

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-pro",
    "messages": [{"role": "user", "content": "Hello"}]
  }'
```

### 使用 Gemini 原生格式

```bash
curl -X POST http://localhost:3000/v1/chat/completions \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [
      {"role": "user", "parts": [{"text": "Hello"}]}
    ]
  }'
```

### 使用 Gemini 原生路径

```bash
curl -X POST http://localhost:3000/v1beta/models/gemini-pro:generateContent \
  -H "Authorization: Bearer YOUR_TOKEN" \
  -H "Content-Type: application/json" \
  -d '{
    "contents": [
      {"role": "user", "parts": [{"text": "Hello"}]}
    ]
  }'
```

---

## 扩展性

此方案可扩展到其他供应商：

| 供应商 | 原生路径特征 | 原生内容特征 |
|--------|--------------|--------------|
| Gemini | `/v1beta/models/`, `/v1/models/` | `contents` 字段 |
| Claude | `/v1/messages` | 无需透传 (已统一) |
| OpenAI | `/v1/chat/completions` | 标准格式，无需透传 |

---

## 相关文档

- [CLAUDE.md](../CLAUDE.md) - 项目开发指南
- [api.md](./api.md) - API 文档
- [BLUEPRINT.md](./BLUEPRINT.md) - 产品蓝图
