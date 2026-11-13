# 硬件检测 → 外部模型解析前端接口

本文档描述当前代码中“模型选择 → Node 硬件检测 → 外部仓库工件解析 → 量化版本筛选”的接口契约。

## 1. 当前实现结论

当前服务端**不再内置默认 Model Manifest 模型数据**。以下三个模型已经从 `burncloud-supply-model` 的默认清单删除：

- `qwen2.5:1.5b`
- `qwen2.5:3b`
- `llama3.2:1b`

`load_default_manifests()` 仍保留为兼容入口，但返回空数组。模型解析不再读取这三个模型，而是使用前端模型选择器中用户当前确认的完整仓库身份：

```json
{
  "model": "Qwen/Qwen3-8B",
  "source": "modelscope",
  "revision": "master"
}
```

服务端在一次解析请求内完成：

1. 校验 `model/source/revision`；
2. 从 ModelScope、Hugging Face 镜像或 Hugging Face 发现仓库文件；
3. 只保留 `.gguf` 文件；
4. 从文件名提取量化标签、文件大小和 SHA-256；
5. 检测运行 BurnCloud Server/Node 的服务端主机硬件；
6. 使用 `llama.cpp` Runtime 能力过滤兼容 Variant；
7. 返回全部兼容量化版本，交给前端选择。

## 2. 模型列表来源

浏览器中“选择模型”区域的列表来自以下公开接口：

- `GET /api/models/presets`：服务端内置的外部仓库预置目录；
- `GET /api/models/search?q=...`：按关键字搜索外部仓库模型。

这两个接口返回的是外部仓库资源，不是本地模型文件，也不是服务端 Manifest。用户点击并确认列表中的模型后，前端保存其 `model/source/revision`，再调用模型解析接口。

## 3. `GET /api/node/hardware`

### 3.1 用途

公开读取运行 BurnCloud Server/Node 的服务端主机硬件快照。不要求 JWT，响应使用 `Cache-Control: no-store`。

该接口适合独立的硬件展示或诊断。模型解析请求会在服务端再次检测硬件，以保证硬件快照和 Variant 过滤来自同一次后端流程；前端不应把本接口的结果作为解析请求参数提交。

### 3.2 成功响应

```json
{
  "success": true,
  "data": {
    "platform": "windows",
    "arch": "x86_64",
    "cpu": "AMD Ryzen 9 7950X",
    "memory_bytes": 34359738368,
    "disk_available_bytes": 512110190592,
    "accelerators": [
      {
        "kind": "nvidia",
        "name": "NVIDIA GeForce RTX 4090",
        "memory_bytes": 25769803776
      }
    ]
  }
}
```

`accelerators[].memory_bytes` 为 `null` 表示显存未知，不能解释为 `0`。没有加速器时返回空数组。

### 3.3 错误

| HTTP 状态 | `error.code` | 含义 |
|---:|---|---|
| `501` | `unsupported_platform` | 当前平台没有硬件探测实现 |
| `500` | `hardware_probe_failed` | CPU、内存、磁盘或其他硬件信息探测失败 |

### 3.4 TypeScript

```ts
type Accelerator = {
  kind: "nvidia" | "amd" | "intel" | "apple" | "other";
  name: string;
  memory_bytes: number | null;
};

type HardwareSnapshot = {
  platform: "windows" | "macos" | "linux" | "other";
  arch: string;
  cpu: string;
  memory_bytes: number;
  disk_available_bytes: number;
  accelerators: Accelerator[];
};

type HardwareResponse = {
  success: true;
  data: HardwareSnapshot;
};

async function fetchHardware(): Promise<HardwareSnapshot> {
  const response = await fetch("/api/node/hardware", {
    headers: { Accept: "application/json" },
    cache: "no-store",
  });
  const body = await response.json();
  if (!response.ok || body.success !== true) {
    throw new Error(body.error?.message ?? "硬件检测失败");
  }
  return (body as HardwareResponse).data;
}
```

## 4. `GET /api/models/presets`

返回外部模型仓库预置项。当前返回的 `model/source/revision` 是后续解析请求需要提交的完整资源身份。

```json
[
  {
    "id": "qwen3-8b",
    "name": "Qwen3-8B",
    "preset": true,
    "model": "Qwen/Qwen3-8B",
    "source": "modelscope",
    "revision": "master"
  }
]
```

字段说明：

- `id`：目录去重和 UI 标识；
- `name`：前端展示名称；
- `preset`：是否为服务端预置项；
- `model`：外部模型仓库的 `owner/name`；
- `source`：`modelscope`、`hf-mirror` 或 `huggingface`；
- `revision`：分支、标签或提交修订号。

该接口不返回：

- 本地模型 Manifest；
- 量化 Variant；
- 本地下载状态；
- 当前硬件兼容结论。

因此，预置项出现不代表模型已经下载或可以在当前机器运行。

## 5. `GET /api/models/search?q=...`

搜索接口返回 `application/x-ndjson`，每一行都是累计快照。前端必须按行解析，不能等待一个完整 JSON 数组。

### 5.1 中间快照

```json
{
  "done": false,
  "items": [
    {
      "id": "qwen3-8b",
      "name": "Qwen3-8B",
      "preset": false,
      "model": "Qwen/Qwen3-8B",
      "source": "modelscope",
      "revision": "master"
    }
  ]
}
```

### 5.2 最终快照

```json
{
  "done": true,
  "type": "models",
  "items": [
    {
      "id": "qwen3-8b",
      "name": "Qwen3-8B",
      "preset": false,
      "model": "Qwen/Qwen3-8B",
      "source": "modelscope",
      "revision": "master"
    }
  ]
}
```

`type` 的含义：

- `models`：至少有一个搜索结果；
- `unavailable`：部分上游查询完成，但没有结果；
- `failed`：所有上游查询失败。

空查询的搜索请求返回 `400`。前端收到完成帧后再结束加载状态。

## 6. `GET /api/models/status`

该接口只探测外部仓库资源页面是否可访问，不进行硬件检测、Manifest 读取或 GGUF 解析。

### 6.1 预置项查询

```text
GET /api/models/status?preset=qwen3-8b
```

### 6.2 外部模型查询

```text
GET /api/models/status?model=Qwen%2FQwen3-8B&source=modelscope&revision=master
```

### 6.3 成功响应

```json
{
  "status": "downloadable",
  "model": "Qwen/Qwen3-8B",
  "source": "modelscope",
  "revision": "master",
  "resource_url": "https://modelscope.cn/models/Qwen/Qwen3-8B/files?Revision=master"
}
```

状态含义：

- `downloadable`：资源页面 HTTP GET 返回 `200`；
- `developing`：资源不可访问、返回非 `200` 或探测失败。

`downloadable` 只说明资源页面可以访问，不表示：

- 模型已经下载；
- 仓库存在 `.gguf` 文件；
- 当前硬件能够运行模型；
- 已经完成 SHA-256 校验；
- 模型已经部署或启动 Runtime。

## 7. `POST /api/node/models/resolve`

### 7.1 用途

直接解析前端当前选中的外部模型。该接口公开、不要求 JWT，响应使用 `Cache-Control: no-store`。

前端不能提交硬件信息，也不能上传 Manifest。服务端以真实硬件探测结果为准，并使用受信任的服务端逻辑创建临时 Manifest。

### 7.2 请求

```http
POST /api/node/models/resolve
Content-Type: application/json
```

```json
{
  "model": "Qwen/Qwen3-8B",
  "source": "modelscope",
  "revision": "master"
}
```

字段约束：

- `model` 必须是 `owner/name` 格式；
- `source` 必须是 `modelscope`、`hf-mirror` 或 `huggingface`；
- `revision` 必须是非空安全路径段；
- `model/source/revision` 必须来自用户在模型选择器中点击并确认的同一条目录记录；
- 前端不提交 RAM、VRAM、磁盘或 Runtime 能力；
- 当前服务端默认使用 `llama.cpp`。

### 7.3 服务端发现过程

ModelScope 使用类似以下文件列表接口：

```text
https://modelscope.cn/api/v1/models/{owner}/{name}/repo/files?Revision={revision}&Recursive=true
```

Hugging Face 和 `hf-mirror` 使用类似以下接口：

```text
https://{host}/api/models/{owner}/{name}/tree/{revision}?recursive=true
```

服务端只接受有效的 `.gguf` 文件：

- 文件路径作为 `variant_id`；
- 文件名中识别 `Q4_K_M`、`Q8_0`、`F16` 等量化标签；
- 文件大小作为 `disk_size_bytes`；
- 文件大小同时作为保守的 `min_ram_bytes`；
- 外部仓库没有声明显存门槛时使用 `min_vram_bytes: 0`；这里的 `0` 表示“显存要求未声明”，不是“需要 0 显存”；
- ModelScope 的 `Sha256` 或 Hugging Face LFS `oid` 作为 SHA-256；
- 下载地址由服务端根据仓库来源、模型、修订号和文件路径构造；
- Variant 保持上游文件列表顺序，不进行评分或重新排序。

当前服务端不会把不带有效 SHA-256 或文件大小为零的文件返回为候选。

### 7.4 成功且存在兼容 Variant

```json
{
  "success": true,
  "data": {
    "status": "resolved",
    "model": "Qwen/Qwen3-8B",
    "hardware": {
      "platform": "windows",
      "arch": "x86_64",
      "cpu": "AMD Ryzen 9 7950X",
      "memory_bytes": 34359738368,
      "disk_available_bytes": 512110190592,
      "accelerators": [
        {
          "kind": "nvidia",
          "name": "NVIDIA GeForce RTX 4090",
          "memory_bytes": 25769803776
        }
      ]
    },
    "candidates": [
      {
        "model_name": "Qwen/Qwen3-8B",
        "variant_id": "Qwen3-8B-GGUF/qwen3-8b-q4_k_m.gguf",
        "quantization": "Q4_K_M",
        "min_ram_bytes": 4294967296,
        "min_vram_bytes": 0,
        "disk_size_bytes": 1929903008,
        "runtime": "llama.cpp",
        "artifact_uri": "https://modelscope.cn/models/Qwen/Qwen3-8B/resolve/master/Qwen3-8B-GGUF/qwen3-8b-q4_k_m.gguf",
        "checksum": "sha256:..."
      }
    ]
  }
}
```

### 7.5 没有兼容 Variant

没有 GGUF 文件，或者所有 GGUF 文件均未通过 RAM、显存、磁盘或 Runtime 过滤时，返回正常业务结果，不使用 HTTP 错误：

```json
{
  "success": true,
  "data": {
    "status": "unsupported",
    "model": "Qwen/Qwen3-8B",
    "hardware": {
      "platform": "windows",
      "arch": "x86_64",
      "cpu": "AMD Ryzen 9 7950X",
      "memory_bytes": 34359738368,
      "disk_available_bytes": 512110190592,
      "accelerators": []
    },
    "candidates": [],
    "reason": {
      "code": "no_compatible_variant",
      "message": "当前模型仓库没有满足当前硬件的 GGUF 版本"
    }
  }
}
```

### 7.6 候选过滤顺序

底层 `RealModelResolver` 按以下条件过滤：

1. 系统 RAM；
2. 单个加速器中的最大已知显存，不累加多张 GPU；
3. 服务端支持的 Runtime，目前为 `llama.cpp`；
4. 当前工作目录所在文件系统的可用磁盘空间；
5. 保持 Manifest/仓库发现结果的原始顺序。

### 7.7 错误映射

| HTTP 状态 | `error.code` | 场景 |
|---:|---|---|
| `400` | `invalid_model` | 请求体无效、模型名为空、模型不是 `owner/name`、来源或修订号无效 |
| `404` | `model_repository_not_found` | 外部仓库或指定修订不存在 |
| `502` | `model_repository_unavailable` | 外部仓库不可访问、返回异常状态或响应格式无效 |
| `501` | `unsupported_platform` | 当前平台没有硬件探测实现 |
| `500` | `hardware_probe_failed` | 硬件检测失败 |
| `500` | `manifest_invalid` | 服务端动态 Manifest 后端声明不一致 |
| `500` | `model_resolution_failed` | 其他解析异常 |

所有错误响应都使用：

```json
{
  "success": false,
  "error": {
    "code": "invalid_model",
    "message": "模型解析请求无效"
  }
}
```

## 8. 前端 TypeScript 契约

```ts
type ModelSource = "modelscope" | "hf-mirror" | "huggingface";

type CatalogSelection = {
  name: string;
  model: string;
  source: ModelSource;
  revision: string;
  resource_url: string;
};

type ModelCandidate = {
  model_name: string;
  variant_id: string;
  quantization: string;
  min_ram_bytes: number;
  // 0 表示服务端未声明显存门槛，UI 不应显示“显存 0 GiB”。
  min_vram_bytes: number;
  disk_size_bytes: number;
  runtime: "llama.cpp" | string;
  artifact_uri: string;
  checksum: string;
};

type ResolveModelData = {
  status: "resolved" | "unsupported";
  model: string;
  hardware: HardwareSnapshot;
  candidates: ModelCandidate[];
  reason?: {
    code: "no_compatible_variant";
    message: string;
  };
};

async function resolveSelectedModel(
  selection: CatalogSelection,
): Promise<ResolveModelData> {
  const response = await fetch("/api/node/models/resolve", {
    method: "POST",
    headers: {
      Accept: "application/json",
      "Content-Type": "application/json",
    },
    cache: "no-store",
    body: JSON.stringify({
      model: selection.model,
      source: selection.source,
      revision: selection.revision,
    }),
  });
  const body = await response.json();
  if (!response.ok || body.success !== true) {
    throw new Error(body.error?.message ?? "模型解析失败");
  }
  return body.data as ResolveModelData;
}
```

前端展示量化版本时：

```ts
function vramLabel(bytes: number): string {
  return bytes === 0 ? "显存要求未声明" : `显存 ${(bytes / 2 ** 30).toFixed(1)} GiB`;
}
```

## 9. 推荐前端调用流程

```text
GET /api/models/presets
或
GET /api/models/search?q=...
        ↓
用户点击并确认某一条目录模型
        ↓
保存该条记录的 model/source/revision
        ↓
GET /api/models/status（可选，仅显示仓库页面状态）
        ↓
POST /api/node/models/resolve
        ↓
服务端实时发现 GGUF + 检测硬件 + 过滤候选
        ↓
前端展示 candidates
        ↓
用户选择 variant_id
        ↓
进入后续下载、校验、部署流程
```

当前 Chat 前端已经删除了旧的“服务端 Manifest 模型列表”请求，不再调用：

```text
GET /api/node/models/manifests
```

用户确认模型后，`CatalogSelection` 会直接传入 `POST /api/node/models/resolve`。如果仓库没有可用 GGUF 或当前硬件不满足要求，前端展示 `unsupported` 的业务原因。

当前下载、SHA-256 校验、Runtime 启动和部署仍未全部接入生产流程；解析接口返回候选并不代表模型已经部署完成。

## 10. 关键不一致与边界

### 10.1 外部模型 ID 与旧本地 Manifest 名称

旧流程使用：

```text
qwen2.5:1.5b
qwen2.5:3b
llama3.2:1b
```

新流程使用用户选择的外部仓库身份：

```text
Qwen/Qwen3-8B + modelscope + master
```

两套标识不再映射。前端不能把旧的 Ollama 风格名称或显示名称传给新接口，必须提交目录记录中的 `model/source/revision`。

### 10.2 `/api/models/status` 不能替代模型解析

资源状态接口不读取：

- GGUF 文件列表；
- 量化信息；
- RAM/VRAM/磁盘门槛；
- Runtime 要求；
- SHA-256；
- 当前硬件兼容结果。

### 10.3 不要让前端提交硬件参数

如果前端先调用 `/api/node/hardware`，再把硬件值发送给解析接口，可能造成快照过期或被伪造。解析接口必须由服务端重新检测硬件。

## 11. 对应实现位置

| 功能 | 文件 |
|---|---|
| 硬件 HTTP Handler | `crates/interfaces/server/src/api/hardware.rs` |
| 模型解析 HTTP Handler | `crates/interfaces/server/src/api/model_resolution.rs` |
| 路由注册 | `crates/interfaces/server/src/api/mod.rs` |
| OpenAPI | `crates/interfaces/server/src/api/openapi.rs` |
| 生产 Node 解析入口 | `crates/interfaces/server/src/node_production.rs` |
| Manifest 数据结构和空默认入口 | `crates/supply/model/src/manifest.rs` |
| Runtime/资源过滤 | `crates/supply/model/src/real_model_resolver.rs` |
| 前端目录 HTTP 客户端 | `crates/interfaces/client/src/domains/chat/catalog.rs` |
| 前端选择器 | `crates/interfaces/client/src/domains/chat/selector.rs` |
| 前端准备状态机 | `crates/interfaces/client/src/domains/chat/state.rs` |
| 前端准备卡片 | `crates/interfaces/client/src/domains/chat/page.rs` |

## 12. 测试覆盖

当前应覆盖以下行为：

- 硬件成功、无加速器、未知显存、平台不支持和探测失败；
- 预置目录字段和顺序；
- 搜索 NDJSON 累计快照与完成帧；
- `downloadable/developing` 状态语义；
- 空模型、非法仓库标识和未知来源；
- 外部文件列表中的 `.gguf` 筛选；
- 量化名称、文件大小、下载地址和 SHA-256 解析；
- 空默认 Manifest；
- RAM、单卡最大 VRAM、磁盘和 Runtime 过滤；
- 多个候选保持仓库发现顺序；
- 前端将当前 `CatalogSelection` 的 `model/source/revision` 发送到解析接口；
- `min_vram_bytes: 0` 在前端显示为“显存要求未声明”。

如果后续把下载和部署完整接入，还需要增加：

- 解析 Handler 的真实上游 mock 集成测试；
- OpenAPI 路由和 schema 校验；
- 前端选择器到解析请求的端到端测试；
- 下载、校验、Runtime 启动和部署状态机测试。
