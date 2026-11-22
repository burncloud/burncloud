# 模型目录、工件状态与 Node 硬件解析接口

本文档记录当前服务端实现及 OpenAPI（`GET /api-docs/openapi.json`）所描述的模型选择接口。模型列表、状态检查、解析候选是不同阶段：仓库页面 URL 仅用于浏览，只有验证过身份、直接文件 URL、SHA-256 和非零文件大小的 GGUF 工件才能成为远程候选。

## 流程与身份

1. `GET /api/models/presets` 返回完整的 388 项预置目录。精选的 18 项只是前端默认展示规则，服务端不裁剪目录。
2. `GET /api/models/search?q=...` 保留外部搜索，非预置结果继续以 `model + source + revision` 作为身份。
3. `GET /api/models/status` 实时检查所选身份的 GGUF 工件，并附带精确匹配的本地校验记录。
4. `POST /api/node/models/resolve` 发现并校验工件、重新检测 Node 硬件，再返回符合最低硬件要求的候选。目录预置请求须携带对应 `preset_id`；旧版非预置搜索请求可省略。

一个模型身份由 `model`、`source`、`revision` 共同构成。同一仓库可以被多个目录项引用；预置模型另外使用唯一 `preset_id` 绑定到具体目录项和硬件门槛。

## 通用值与安全边界

| 项目 | 契约 |
| --- | --- |
| `source` | `modelscope`、`hf-mirror`、`huggingface`；不接受请求方任意指定站点主机 |
| `identity_status` | `verified` 或 `pending_validation`；待校验项允许展示，但不能生成成功下载候选 |
| 目录硬件容量 | 字符串中的 `G` 按 GiB（1024³ 字节）解释；推荐值只供展示 |
| 远程 GGUF | 路径须为 `.gguf`；仓库及修订号须与选择身份一致；须有来源允许的直接文件 URL、64 位十六进制 SHA-256 和大于零的文件大小 |
| `resource_url` | ModelScope/Hugging Face 仓库页面，只供查看；不得传给下载器作为工件 URL |
| 解析硬件 | 使用本次请求中由 Node 检测的 RAM、磁盘及加速器数据；不接受前端硬件值覆盖 |
| 硬件门槛 | VRAM 为 `max(预置最低值, 工件最低值)`；RAM 和磁盘分别为 `max(预置最低值, 工件最低值, 文件实际大小)` |
| 本地回退 | 仅当上游不可用时考虑；模型、来源、修订号、Variant ID、工件 URI、期望摘要须完全匹配，且下载记录成功、路径非空、实测摘要等于期望摘要 |

## `GET /api/models/presets`

返回 JSON 数组，恰好 388 项；每个 `id` 唯一。服务器保留全目录，前端根据稳定 ID 按以下顺序默认显示精选项，其余项可通过“显示更多”查看：

`qwen3-8b`、`deepseek-r1-distill-qwen-7b`、`meta-llama-3.1-8b-instruct`、`gemma-3-4b-it`、`mistral-7b-instruct-v0.2`、`gpt-oss-20b`、`phi-4-mini-reasoning`、`granite-3.3-8b-instruct`、`c4ai-command-r7b-12-2024`、`qwen3-30b`、`deepseek-r1-distill-qwen-32b`、`meta-llama-3.1-70b-instruct`、`gemma-3-27b-it`、`gpt-oss-120b`、`c4ai-command-r-plus`、`nvidia-nemotron-3-super-120b-a12b-bf16`、`olmo-3.1-32b-instruct-row-342`、`granite-4.2-30b-gguf`。

每个目录项包含：

| 字段 | 类型 | 说明 |
| --- | --- | --- |
| `id` | string | 稳定预置 ID；解析时作为 `preset_id` |
| `name` | string | 前端展示名 |
| `preset` | boolean | 目录项为 `true` |
| `model` | string | 两段式仓库标识 `owner/name` |
| `source` | string | 固定来源枚举 |
| `revision` | string | 仓库修订号 |
| `resource_url` | URI string | 仓库浏览页面，不是文件下载地址 |
| `identity_status` | string | `verified` / `pending_validation` |
| `hardware` | object | `vram_min`、`ram_min`、`disk_min`、`vram_recommended`、`ram_recommended`、`disk_recommended` 六个容量字符串 |

成功响应为目录数组本身，不使用 `{success, data}` 包装。目录自身无法解析或校验时返回 HTTP 500：`{"code":"preset_catalog_invalid","message":"..."}`。

## `GET /api/models/search?q=...`

`q` 去除首尾空白后必须非空，否则返回 HTTP 400 `{ "message": "..." }`。响应媒体类型为 `application/x-ndjson; charset=utf-8`，每行都是一个 JSON 快照：

```json
{"done":false,"items":[{"id":"qwen3-8b-gguf","name":"Qwen3-8B-GGUF","model":"Qwen/Qwen3-8B-GGUF","source":"modelscope","revision":"master"}]}
{"done":true,"type":"models","items":[{"id":"qwen3-8b-gguf","name":"Qwen3-8B-GGUF","model":"Qwen/Qwen3-8B-GGUF","source":"modelscope","revision":"master"}]}
```

中间帧为累计快照，`type` 省略；终帧 `done=true`，`type` 为 `models`、`unavailable` 或 `failed`。`unavailable` 表示至少一个来源成功响应但没有命中，或部分来源不可用；`failed` 表示所有来源均失败。已合并的真实命中不会因另一个来源失败而被删除。

## `GET /api/models/status`

该接口按所选身份实时发现有效远程 GGUF，并合并身份完全匹配的本地下载记录。支持且仅支持以下一种查询形式：

- 预置项：`?preset=qwen3-8b`。不得同时传入 `model`、`source` 或 `revision`。
- 外部搜索项：`?model=Qwen%2FQwen3-8B-GGUF&source=modelscope&revision=master`。`model` 与 `source` 必须同时提供；`revision` 可省略，此时 ModelScope 默认 `master`，其他来源默认 `main`。

查询形式缺失、冲突或身份无效时返回 HTTP 400 `{ "message": "..." }`。成功响应（字段均为顶层字段）：

```json
{
  "status": "downloadable",
  "model": "Qwen/Qwen3-8B-GGUF",
  "source": "modelscope",
  "revision": "master",
  "resource_url": "https://modelscope.cn/models/Qwen/Qwen3-8B-GGUF/files?Revision=master",
  "downloadable": true,
  "unavailable_reason": null,
  "identity_status": "verified",
  "artifact_status": "not_downloaded",
  "downloaded": false,
  "local_path": null,
  "actual_sha256": null,
  "variants": [{
    "id": "Qwen3-8B-Q4_K_M.gguf",
    "quantization": "Q4_K_M",
    "runtime": "llama.cpp",
    "artifact_source": "https://modelscope.cn/models/Qwen/Qwen3-8B-GGUF/resolve/master/Qwen3-8B-Q4_K_M.gguf",
    "expected_sha256": "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    "file_size_bytes": 4294967296,
    "ram_bytes": 4294967296,
    "vram_bytes": 6442450944,
    "disk_bytes": 4294967296,
    "artifact_status": "not_downloaded",
    "downloaded": false,
    "local_path": null,
    "actual_sha256": null
  }]
}
```

- `status` 是兼容字段，仅表示仓库页面探测结果：`downloadable`（页面 HTTP 200）或 `developing`（探测失败/非 200），不能据此认定存在可用文件。
- 顶层 `downloadable` 表示存在通过身份和元数据校验的远程直接文件工件；它不表示硬件兼容，也不表示已下载。
- `unavailable_reason` 为 `null` 或 `{code,message}`。当前原因码：`preset_identity_pending`、`no_gguf_artifact`、`repository_not_found`、`repository_unavailable`。
- `variants` 只包含有效远程 GGUF 或完全匹配且已验证的本地工件。`artifact_source` 是直接文件 URL；`expected_sha256` 是期望摘要；`file_size_bytes` 必须大于零。
- `artifact_status` 为 `not_downloaded`、`downloading`、`verifying`、`downloaded` 或 `failed`。本地回执有效时才设置 `downloaded=true` 并返回 `local_path` 和 `actual_sha256`。
- 对身份待校验预置，状态接口返回 `downloadable=false` 和 `preset_identity_pending`，不生成 Variant。

HTTP 503 的 `catalog_unavailable` 表示读取本地工件目录失败；实时上游不可用会返回 HTTP 200、`downloadable=false` 和 `repository_unavailable`，并尝试精确匹配的已校验本地工件。HTTP 500 的 `preset_catalog_invalid` 表示目录配置无效。状态不可用与 Node 硬件不兼容是不同结果。

## `GET /api/node/hardware`

返回服务端 Node 主机硬件快照，使用 `Cache-Control: no-store`，不要求 JWT。该接口用于诊断和展示；`POST /api/node/models/resolve` 会自行重新检测硬件，不接受前端传入该快照。

成功响应包括 `platform`、`arch`、`cpu`、`memory_bytes`、`disk_available_bytes` 和 `accelerators[]`；容量单位均为字节。`accelerators[].memory_bytes=null` 表示未知显存，不等同于零；无加速器时为空数组。HTTP 501 `unsupported_platform` 表示当前平台无探测实现；HTTP 500 `hardware_probe_failed` 表示硬件探测失败。

## `POST /api/node/models/resolve`

请求体需要提交完整仓库身份。预置目录项必须连同绑定的 `preset_id` 提交；`preset_id` 与 `model`、`source`、`revision` 任一不匹配时拒绝请求。非预置外部搜索项可以省略 `preset_id`，与旧版请求兼容。

```json
{
  "model": "Qwen/Qwen3-8B-GGUF",
  "source": "modelscope",
  "revision": "master",
  "preset_id": "qwen3-8b"
}
```

不向解析接口提交硬件值。服务端校验身份、实时发现或精确回退已验证的 GGUF 工件、检测 Node 硬件并执行最低资源过滤。推荐资源不参与准入。成功 HTTP 200 响应始终带 `success=true` 和 `data`：

- `data.status=resolved`：`candidates` 仅含满足最低硬件要求的工件；候选提供工件 ID、量化、最低 RAM/VRAM/磁盘字节、runtime、直接 `artifact_uri` 和 SHA-256 `checksum`。
- `data.status=unsupported`：无有效候选时 `candidates=[]`，`reason.code` 为 `preset_identity_pending`、`no_valid_gguf` 或 `no_compatible_variant`；响应仍可带本次 Node 硬件快照。

HTTP 错误响应为 `{ "success": false, "error": { "code": "...", "message": "..." } }`：

| HTTP | 错误码 | 含义 |
| ---: | --- | --- |
| 400 | `invalid_model` | JSON 或模型/来源/修订号无效 |
| 400 | `unknown_preset` | `preset_id` 不存在 |
| 400 | `preset_identity_mismatch` | 预置 ID 与请求仓库身份不一致 |
| 500 | `preset_catalog_invalid` | 服务端预置目录无效 |
| 404 | `model_repository_not_found` | 仓库或指定修订不存在 |
| 502 | `model_repository_unavailable` | 上游不可用且无完全匹配的已验证本地工件 |
| 501 | `unsupported_platform` | 当前平台不支持 Node 硬件检测 |
| 500 | `hardware_probe_failed` | Node 硬件检测失败 |
| 500 | `manifest_invalid` | 解析清单无效 |
| 500 | `model_resolution_failed` | 其他内部解析错误 |

## 客户端准入顺序

1. 将仓库身份绑定为目录 `id`（即 `preset_id`）或外部搜索的完整 `model/source/revision`，不要用显示名代替。
2. 对预置状态按 `preset` 查询，对外部模型按 `model + source [+ revision]` 查询。
3. 若状态返回远程 `downloadable=true`，只允许从通过校验的 `variants` 中进入硬件解析；若 `downloadable=false`，仅可复用返回的完整、身份相符且 SHA-256 已验证的本地工件。
4. 调用解析接口时，预置项传 `preset_id`；普通搜索结果省略。以解析响应的 Node 实测硬件与候选列表为准。
5. 只有响应中身份匹配、具备直接文件 URL 和有效 SHA-256 的候选可交给下载器。仓库页面 `resource_url` 永远不能作为下载地址。
