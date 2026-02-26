# Gemini AI Studio 模型测试计划

## 背景
我们需要对 Google Gemini AI Studio 的所有模型进行全面测试，覆盖价格配置、原生格式输出和路径穿透功能。

---

## 1. Gemini 模型列表

### 1.1 Gemini 3.x 系列（最新旗舰 - 2025年11月/12月）

| 模型 | API 名称 | 定位 | 特性 |
|------|----------|------|------|
| Gemini 3 Pro | `gemini-3-pro-preview-11-2025` | 性能天花板 | MoE架构，50%+ 速度提升，长上下文 |
| Gemini 3 Pro Image | `gemini-3-pro-image-preview` | 原生图像生成 | 文本+图像混合输出，对话式图像编辑 |
| Gemini 3 Flash | `gemini-3-flash` | 超快速轻量 | 3x 速度，1/4 价格，强多模态 |
| Gemini 3 Flash Thinking | `gemini-3-flash-thinking` | 轻量推理 | 激活深度推理，默认关闭 thinking |
| Gemini 3 Deep Think | `gemini-3-deep-think` | 深度推理 | 增强长链思考，高难度数学 |
| Gemini 3.1 Pro | `gemini-3.1-pro-preview` | 最新推理增强 | 2026.2.20 发布，ARC-AGI-2 77.1% |

### 1.2 Gemini 2.5 系列（成熟稳定版）

| 模型 | API 名称 | 定位 | 价格（每百万 token）|
|------|----------|------|-------------------|
| Gemini 2.5 Pro | `gemini-2.5-pro` | 高级推理 | ≤200K: $1.25 入 / $10 出, >200K: $2.50 入 / $15 出 |
| Gemini 2.5 Flash | `gemini-2.5-flash` | 平衡效率 | 高性价比，低延迟 |
| Gemini 2.5 Flash Image | `gemini-2.5-flash-image` | 原生图像生成 | 文本+图像混合输出 |

### 1.3 Gemini 2.0 系列

| 模型 | API 名称 | 特性 |
|------|----------|------|
| Gemini 2.0 Flash | `gemini-2.0-flash` | 快速多模态，$0.10 入 / $0.40 出 |
| Gemini 2.0 Flash Thinking | `gemini-2.0-flash-thinking` | 思考链推理 |
| Gemini 2.0 Pro | `gemini-2.0-pro` | 高级功能 |
| Gemini 2.0 Flash Lite | `gemini-2.0-flash-lite` | 超轻量版 |

### 1.4 其他模型

| 模型 | API 名称 | 用途 |
|------|----------|------|
| Imagen 3 | `imagen-3.0-generate-002` | 高质量图像生成 |
| Lyria | `lyria-002` | AI 音乐/音频生成 |

---

## 2. 测试维度

### 2.1 价格测试（Price Testing）

测试目标：验证所有 Gemini 模型的价格配置正确性

**测试项目**：
1. 标准输入/输出价格
2. 长上下文价格（>200K tokens）
3. 优先级价格（priority_input/output_price）
4. 音频输入价格（audio_input_price）
5. 批量处理价格（batch_input/output_price）
6. 区域差异化价格（cn vs international）

**价格数据参考**：

| 模型 | 区域 | Input ($/1M) | Output ($/1M) | Priority In | Priority Out |
|------|------|--------------|---------------|-------------|--------------|
| gemini-2.5-pro | US | 1.25 | 10.00 | 2.50 | 15.00 |
| gemini-2.5-pro (>200K) | US | 2.50 | 15.00 | - | - |
| gemini-2.0-flash | US | 0.10 | 0.40 | - | - |
| gemini-3-pro | US | TBD | TBD | - | - |
| gemini-3-flash | US | TBD | TBD | - | - |

### 2.2 原生格式测试（Native Format Testing）

测试目标：验证模型的特殊输出格式支持

**测试项目**：

#### 2.2.1 原生图像生成（Native Image Generation）
- **模型**: `gemini-3-pro-image-preview`, `gemini-2.5-flash-image`, `gemini-2.0-flash-exp`
- **配置**: `responseModalities: ["TEXT", "IMAGE"]`
- **测试用例**：
  - 纯文本生成图像
  - 图像编辑（对话式修改）
  - 图像融合（多图合成）
  - 角色一致性生成

#### 2.2.2 思考链输出（Thinking Chain）
- **模型**: `gemini-3-flash-thinking`, `gemini-2.0-flash-thinking`
- **测试用例**：
  - 开启 thinking 模式
  - 关闭 thinking 模式（默认）
  - 复杂推理任务

#### 2.2.3 音频输出（Audio Output）
- **模型**: `lyria-002`
- **测试用例**：
  - 文本转音乐生成
  - 不同风格/乐器组合

### 2.3 路径穿透测试（Path Passthrough Testing）

测试目标：验证 API 请求正确转发到 Google AI Studio

**测试项目**：

#### 2.3.1 基础路径穿透
- `/v1/chat/completions` → `https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent`
- 验证 Authorization header 正确传递
- 验证请求体格式转换（OpenAI → Gemini 格式）

#### 2.3.2 流式响应穿透
- SSE 流式输出正确转发
- 多模态响应流式处理

#### 2.3.3 特殊参数穿透
- `responseModalities` 参数
- `thinkingBudget` 参数
- `safetySettings` 参数
- `generationConfig` 参数

---

## 3. 详细测试计划

### 3.1 渠道配置测试

```
# 添加 Gemini AI Studio 渠道
./burncloud channel add \
  --name "gemini-aistudio" \
  --type 1 \
  --key "AIza..." \
  --base-url "https://generativelanguage.googleapis.com/v1beta" \
  --models "gemini-2.0-flash,gemini-2.5-pro,gemini-2.5-flash,gemini-3-pro" \
  --pricing-region "international"
```

### 3.2 价格配置测试

```bash
# Gemini 2.5 Pro 价格
./burncloud price set gemini-2.5-pro \
  --input 1.25 --output 10.0 \
  --region international --currency USD

# Gemini 2.0 Flash 价格
./burncloud price set gemini-2.0-flash \
  --input 0.10 --output 0.40 \
  --region international --currency USD

# 查询验证
./burncloud price get gemini-2.5-pro --region international
./burncloud price list --region international | grep gemini
```

### 3.3 API 调用测试

#### 3.3.1 文本生成测试
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-2.0-flash",
    "messages": [{"role": "user", "content": "Hello, how are you?"}]
  }'
```

#### 3.3.2 多模态输入测试
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-2.5-pro",
    "messages": [{
      "role": "user",
      "content": [
        {"type": "text", "text": "What is in this image?"},
        {"type": "image_url", "image_url": {"url": "https://example.com/image.jpg"}}
      ]
    }]
  }'
```

#### 3.3.3 原生图像生成测试
```bash
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Authorization: Bearer sk-xxx" \
  -H "Content-Type: application/json" \
  -d '{
    "model": "gemini-2.5-flash-image",
    "messages": [{"role": "user", "content": "Generate an image of a sunset over mountains"}],
    "response_modalities": ["TEXT", "IMAGE"]
  }'
```

---

## 4. 测试任务清单

### Phase 1: 基础配置（P0）

- [ ] 创建 Gemini AI Studio 渠道
- [ ] 配置所有模型的价格（cn + international）
- [ ] 验证渠道连通性（简单文本请求）

### Phase 2: 文本模型测试（P0）

- [ ] gemini-2.0-flash 基础测试
- [ ] gemini-2.5-pro 基础测试
- [ ] gemini-2.5-flash 基础测试
- [ ] gemini-3-pro 基础测试
- [ ] gemini-3-flash 基础测试
- [ ] 流式响应测试
- [ ] Token 计费验证

### Phase 3: 多模态测试（P1）

- [ ] 图像输入理解测试
- [ ] PDF/文档输入测试
- [ ] 音频输入测试

### Phase 4: 原生格式测试（P1）

- [ ] gemini-3-pro-image-preview 原生图像生成
- [ ] gemini-2.5-flash-image 原生图像生成
- [ ] gemini-3-flash-thinking 思考链输出
- [ ] 验证 responseModalities 参数穿透

### Phase 5: 路径穿透测试（P1）

- [ ] 验证请求头正确转发
- [ ] 验证请求体格式转换
- [ ] 验证错误响应处理
- [ ] 验证流式响应处理

### Phase 6: 计费验证（P0）

- [ ] 验证 token 计数准确性
- [ ] 验证价格计算正确性
- [ ] 验证区域价格差异化
- [ ] 验证优先级/批量价格

---

## 5. 测试模型清单

| 序号 | 模型名称 | 价格测试 | 原生格式 | 路径穿透 | 优先级 |
|------|----------|----------|----------|----------|--------|
| 1 | gemini-2.0-flash | ✓ | - | ✓ | P0 |
| 2 | gemini-2.0-flash-thinking | ✓ | thinking | ✓ | P1 |
| 3 | gemini-2.5-pro | ✓ | - | ✓ | P0 |
| 4 | gemini-2.5-flash | ✓ | - | ✓ | P0 |
| 5 | gemini-2.5-flash-image | ✓ | image | ✓ | P1 |
| 6 | gemini-3-pro | ✓ | - | ✓ | P1 |
| 7 | gemini-3-pro-image-preview | ✓ | image | ✓ | P1 |
| 8 | gemini-3-flash | ✓ | - | ✓ | P1 |
| 9 | gemini-3-flash-thinking | ✓ | thinking | ✓ | P2 |
| 10 | gemini-3-deep-think | ✓ | thinking | ✓ | P2 |
| 11 | imagen-3.0-generate-002 | - | image | ✓ | P2 |
| 12 | lyria-002 | - | audio | ✓ | P2 |

---

## 6. 参考资源

- [Google AI Studio](https://aistudio.google.com/)
- [Gemini API Documentation](https://ai.google.dev/)
- [Vertex AI Documentation](https://cloud.google.com/vertex-ai)
