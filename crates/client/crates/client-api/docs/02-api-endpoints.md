# API 端点文档

## 概述

BurnCloud Client API 提供了标准的 OpenAI 兼容接口，支持与各种大语言模型进行交互。所有端点都遵循 REST API 设计原则，使用 JSON 格式进行数据交换。

## 基础信息

### 服务器地址
```
http://localhost:8080 (开发环境)
https://api.burncloud.com (生产环境)
```

### 认证方式
```http
Authorization: Bearer YOUR_API_KEY
Content-Type: application/json
```

## API 端点详情

### 1. 对话完成接口

#### 端点信息
- **路径**: `/v1/chat/completions`
- **方法**: `POST`
- **描述**: 发送消息到大语言模型并获取回复
- **状态**: 🟢 正常运行

#### 请求格式

```json
{
  "model": "gpt-3.5-turbo",
  "messages": [
    {
      "role": "system",
      "content": "你是一个有用的助手。"
    },
    {
      "role": "user",
      "content": "你好，请介绍一下自己。"
    }
  ],
  "temperature": 0.7,
  "max_tokens": 1000,
  "stream": false
}
```

#### 请求参数说明

| 参数 | 类型 | 必需 | 描述 |
|------|------|------|------|
| `model` | string | ✅ | 要使用的模型名称 |
| `messages` | array | ✅ | 对话消息列表 |
| `temperature` | number | ❌ | 生成随机性 (0.0-2.0) |
| `max_tokens` | integer | ❌ | 最大生成令牌数 |
| `stream` | boolean | ❌ | 是否启用流式响应 |
| `top_p` | number | ❌ | 核心采样参数 |
| `frequency_penalty` | number | ❌ | 频率惩罚 (-2.0 到 2.0) |
| `presence_penalty` | number | ❌ | 存在惩罚 (-2.0 到 2.0) |

#### 响应格式

```json
{
  "id": "chatcmpl-123",
  "object": "chat.completion",
  "created": 1677652288,
  "model": "gpt-3.5-turbo",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "你好！我是一个AI助手，很高兴为您服务。"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 20,
    "completion_tokens": 15,
    "total_tokens": 35
  }
}
```

#### 流式响应格式

当 `stream: true` 时，响应将以 Server-Sent Events 格式返回：

```
data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"content":"你"},"finish_reason":null}]}

data: {"id":"chatcmpl-123","object":"chat.completion.chunk","created":1677652288,"model":"gpt-3.5-turbo","choices":[{"index":0,"delta":{"content":"好"},"finish_reason":null}]}

data: [DONE]
```

#### 错误响应

```json
{
  "error": {
    "message": "Invalid API key provided",
    "type": "invalid_request_error",
    "code": "invalid_api_key"
  }
}
```

### 2. 模型列表接口

#### 端点信息
- **路径**: `/v1/models`
- **方法**: `GET`
- **描述**: 获取可用的模型列表
- **状态**: 🟢 正常运行

#### 请求示例

```bash
curl -H "Authorization: Bearer YOUR_API_KEY" \
     https://api.burncloud.com/v1/models
```

#### 响应格式

```json
{
  "object": "list",
  "data": [
    {
      "id": "gpt-3.5-turbo",
      "object": "model",
      "created": 1677610602,
      "owned_by": "openai",
      "permission": [
        {
          "id": "modelperm-123",
          "object": "model_permission",
          "created": 1677610602,
          "allow_create_engine": false,
          "allow_sampling": true,
          "allow_logprobs": true,
          "allow_search_indices": false,
          "allow_view": true,
          "allow_fine_tuning": false,
          "organization": "*",
          "group": null,
          "is_blocking": false
        }
      ]
    },
    {
      "id": "gpt-4",
      "object": "model",
      "created": 1687882411,
      "owned_by": "openai"
    }
  ]
}
```

## 使用示例

### JavaScript/Node.js

```javascript
// 发送聊天请求
const response = await fetch('https://api.burncloud.com/v1/chat/completions', {
  method: 'POST',
  headers: {
    'Authorization': 'Bearer YOUR_API_KEY',
    'Content-Type': 'application/json'
  },
  body: JSON.stringify({
    model: 'gpt-3.5-turbo',
    messages: [
      { role: 'user', content: '你好' }
    ]
  })
});

const data = await response.json();
console.log(data.choices[0].message.content);
```

### Python

```python
import requests

# 发送聊天请求
headers = {
    'Authorization': 'Bearer YOUR_API_KEY',
    'Content-Type': 'application/json'
}

data = {
    'model': 'gpt-3.5-turbo',
    'messages': [
        {'role': 'user', 'content': '你好'}
    ]
}

response = requests.post(
    'https://api.burncloud.com/v1/chat/completions',
    headers=headers,
    json=data
)

result = response.json()
print(result['choices'][0]['message']['content'])
```

### cURL

```bash
# 发送聊天请求
curl -X POST https://api.burncloud.com/v1/chat/completions \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [
      {"role": "user", "content": "你好"}
    ]
  }'
```

## 错误代码说明

| 状态码 | 错误类型 | 描述 |
|--------|----------|------|
| 400 | Bad Request | 请求格式错误或参数无效 |
| 401 | Unauthorized | API 密钥无效或缺失 |
| 403 | Forbidden | 访问被拒绝，可能是配额不足 |
| 404 | Not Found | 请求的资源不存在 |
| 429 | Too Many Requests | 请求频率过高，触发限流 |
| 500 | Internal Server Error | 服务器内部错误 |
| 503 | Service Unavailable | 服务暂时不可用 |

## 限制说明

### 请求频率限制
- **免费用户**: 每分钟 20 次请求
- **付费用户**: 每分钟 3000 次请求

### 令牌限制
- **单次请求**: 最多 4096 个令牌（输入+输出）
- **GPT-4**: 最多 8192 个令牌

### 超时设置
- **连接超时**: 30 秒
- **读取超时**: 300 秒

---

*本文档详细说明了 BurnCloud Client API 的所有端点用法和参数配置。*