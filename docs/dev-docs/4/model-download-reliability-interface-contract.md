# model-download-reliability 公有接口契约

## 1. `/api/models/status`

### 请求

保留既有查询参数：

- `preset`：预置模型 ID；或
- `model`、`source`、`revision`：自定义模型完整远程身份。

### 响应

```json
{
  "status": "downloadable",
  "model": "Qwen/Qwen3-8B-GGUF",
  "source": "modelscope",
  "revision": "master",
  "resource_url": "https://modelscope.cn/...",
  "artifact_status": "downloaded",
  "downloaded": true,
  "local_path": "C:/.../Qwen3-8B-Q4_K_M.gguf",
  "actual_sha256": "sha256:...",
  "variants": [
    {
      "id": "Qwen3-8B-Q4_K_M.gguf",
      "quantization": "Q4_K_M",
      "runtime": "llama.cpp",
      "artifact_source": "https://.../Qwen3-8B-Q4_K_M.gguf",
      "expected_sha256": "sha256:...",
      "ram_bytes": 0,
      "vram_bytes": 0,
      "disk_bytes": 0,
      "artifact_status": "downloaded",
      "downloaded": true,
      "local_path": "C:/.../Qwen3-8B-Q4_K_M.gguf",
      "actual_sha256": "sha256:..."
    }
  ]
}
```

保留 `status` 的远程资源语义，不根据它推断本地文件。新增字段：

| 字段 | 类型 | 约束 |
| --- | --- | --- |
| `artifact_status` | string | `not_downloaded`、`downloading`、`verifying`、`downloaded`、`failed` |
| `downloaded` | boolean | 仅当 `artifact_status=downloaded` 时为 `true` |
| `local_path` | string/null | 仅在已校验工件可用时返回 |
| `actual_sha256` | string/null | 仅在摘要校验完成时返回 |
| `variants[].artifact_status` | string | 对应变体的本地状态 |
| `variants[].downloaded` | boolean | 对应变体是否已校验下载 |
| `variants[].local_path` | string/null | 对应变体本地路径 |
| `variants[].actual_sha256` | string/null | 对应变体实际摘要 |

查询数据库失败时接口不能把失败伪装成已下载；记录错误并返回 `artifact_status=not_downloaded`，远程状态仍按探测结果返回。

## 2. `/api/node/models/download`

### 请求

```json
{
  "model": "Qwen/Qwen3-8B-GGUF",
  "source": "modelscope",
  "revision": "master",
  "variant_id": "Qwen3-8B-Q4_K_M.gguf",
  "artifact_uri": "https://...",
  "checksum": "sha256:..."
}
```

服务端在创建或复用任务后必须把上述完整身份写入 `sys_downloads`。

### 响应状态集合

```text
queued | downloading | verifying | succeeded | failed | cancelled
```

数据字段：

- `succeeded` 必须同时提供 `local_path` 和 `actual_sha256`；
- `verifying` 可以暂时缺少路径摘要，但 `completed_bytes` 应为最终归一化值；
- `failed` 必须提供非空结构化错误 `code/message`；
- `cancelled` 不得被客户端当作成功；
- `total_bytes=null` 表示未知大小，不使用 `0` 表示未知；
- `0 <= completed_bytes <= total_bytes`，同一任务进度只增不减。

## 3. 客户端内部类型

### `ArtifactStatus`

```rust
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
enum ArtifactStatus {
    NotDownloaded,
    Downloading,
    Verifying,
    Downloaded,
    Failed,
}
```

只在客户端模型目录域内部可见，不改变 `VersionStatus` 的远程状态职责。

### `CatalogInspection`

```rust
struct CatalogInspection {
    version: CatalogVersion,
    remote_status: VersionStatus,
    artifact_status: ArtifactStatus,
    downloaded: bool,
    local_path: Option<String>,
    actual_sha256: Option<String>,
}
```

`CatalogClient::inspect` 返回 `CatalogInspection`。选择器依据 `artifact_status` 展示“下载中”“校验中”“已下载”或“重试”；`downloaded=true` 时不显示重复下载图标。

## 4. 下载数据库与下载器接口

`SysDownload` 新增可空字段，兼容旧记录：

```text
model
source
revision
variant_id
artifact_uri
checksum
actual_sha256
```

新增或修改的接口必须满足：

| 接口 | 可见性 | 异步/线程属性 | 约束 |
| --- | --- | --- | --- |
| `DownloadDB::new_shared_db` | `pub` | async；共享既有 `Database` 连接 | 不创建第二份数据库状态 |
| `DownloadDB::set_identity` | `pub` | async；数据库连接跨任务共享 | 只更新匹配 `id + gid` 的任务 |
| `DownloadDB::mark_verified` | `pub` | async；原子更新成功状态 | 同时写入 `status=succeeded`、`actual_sha256`、完成时间 |
| `DownloadDB::mark_verification_failed` | `pub` | async；原子更新失败状态 | 写入结构化错误文本，保留路径和实际摘要 |
| `DownloadDB::find_model_artifact` | `pub` | async；只读查询 | 完整身份匹配，旧缺失身份记录不匹配 |
| `DownloadManager::set_task_identity` | `pub` | async；受下载启动生命周期保护 | 供 Node API 在创建/复用后持久化身份 |
| `DownloadManager::mark_task_verified` | `pub` | async；验证完成后调用 | 成功时完整写入校验摘要 |
| `DownloadManager::mark_task_verification_failed` | `pub` | async；验证失败时调用 | 不产生成功状态 |

所有公开接口必须返回 `Result`，不使用 `unwrap`/`expect` 处理外部输入；新增公共 API 同步补充中文 rustdoc 和测试。

## 5. 客户端阶段文案

| `PreparationStage` | 文案 |
| --- | --- |
| `ArtifactReady` | 模型文件已下载并完成校验，尚未部署。 |
| `Deploying` | 模型文件已校验，正在部署。 |
| `HealthChecking` | 模型已部署，正在检查健康状态。 |
| `Ready` | 模型已部署并就绪。 |
| `Failed` | 显示对应下载、校验或部署错误。 |

目录预览不可再对所有阶段统一显示“文件尚未部署”；页面根据当前阶段选择上表文案。
