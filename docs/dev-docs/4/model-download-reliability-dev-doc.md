# model-download-reliability 开发文档

## 1. 目标与范围

本开发在现有异步模型下载、`DownloadManager`、`sys_downloads` 和 `PreparationStage` 基础上修复状态契约、状态持久化和进度刷新。实现必须只覆盖 Issue #954，不能重写初始化状态机、模型解析、模型源或推理路由。

## 2. 数据模型

### 2.1 下载身份

下载记录的完整身份为：

```text
model + source + revision + variant_id + artifact_uri + checksum
```

字段全部保存到 `sys_downloads`，新增字段允许为 `NULL` 以兼容 0019 以前的历史任务。任何缺少身份字段的历史记录都不能判定当前模型已下载。

### 2.2 状态映射

数据库状态到目录工件状态：

| `sys_downloads.status` | 目录状态 | 条件 |
| --- | --- | --- |
| `pending`、`active`、`waiting`、`paused` | `downloading` | 身份完整匹配 |
| `completed`、`verifying` | `verifying` | 下载器已完成但没有成功校验凭据 |
| `succeeded` | `downloaded` | 路径存在、`actual_sha256` 非空且等于 `checksum` |
| `failed`、`cancelled` | `failed` | 有匹配失败记录 |
| 无匹配记录 | `not_downloaded` | 没有可识别本地工件 |

### 2.3 迁移

新增 `0020_download_model_identity`：

- SQLite/PostgreSQL 均使用可空文本列；
- 不重建已存在记录的身份；
- 旧记录保留进度和路径，但因缺少完整身份不会被误判；
- 迁移重复执行必须安全；
- 为状态和完整身份查询建立必要索引，但不新增独立模型工件表。

## 3. 服务端实现

### 3.1 `DownloadDB`

将 `DownloadDB` 内部数据库句柄改为 `Arc<Database>`，保留已有构造函数并新增 `new_shared_db` 供 `model_selection` 复用主服务连接。所有 SQL 查询列同步包含新增字段。

新增：

1. `set_identity(id, gid, model, source, revision, variant_id, artifact_uri, checksum)`：按稳定 ID 和当前 GID 更新身份，防止旧 GID 回调覆盖新任务。
2. `mark_verified(id, gid, actual_sha256)`：在摘要匹配后原子写入 `succeeded`、实际摘要和完成时间。
3. `mark_verification_failed(id, gid, actual_sha256, error)`：写入 `failed`、摘要（若可计算）和错误。
4. `find_model_artifact(identity)`：完整字段相等查询，按最新记录优先。

### 3.2 `DownloadManager`

增加三个私有/公开边界方法：

- 创建/复用下载任务后持久化身份；
- 校验成功或失败时更新数据库；
- 读取记录时保留 `total_bytes=null` 的未知语义。

监控循环：

1. 轮询周期从 2 秒缩短到 250 毫秒；
2. 读取 aria2 进度后通过 `max(previous_completed, incoming_completed)` 保证单调递增；
3. 总大小保持历史最大值；未知总大小继续返回 `None`；
4. aria2 完成时将总大小和已完成字节统一为同一最终值，满足 `completed_bytes == total_bytes`；
5. 数据库写入失败必须记录并转为任务失败，不能静默忽略。

### 3.3 `/api/node/models/download`

创建或复用任务后，先写完整身份，再注册当前 Node 任务状态并启动摘要校验。`verify_task` 的顺序固定为：

```text
wait_for_task
→ 状态切换 verifying
→ 计算文件 SHA-256
→ 成功则 DownloadDB::mark_verified
→ 失败则 DownloadDB::mark_verification_failed
→ 更新当前任务注册表
```

任何取消或终态任务均不能被后台校验回调重新写成成功。成功响应只在路径和实际摘要都已写入后返回。

## 4. 模型目录接口

### 4.1 服务端

`model_selection::routes` 接收主服务的 `Arc<Database>`，状态处理器使用 `DownloadDB::new_shared_db` 查询本地记录。远程资源探测逻辑保持不变；仅增加本地工件聚合字段。

变体构造后逐一查询完整身份，检查最终路径是否为文件且实际摘要与期望摘要一致。返回模型级 `downloaded=true` 的条件是至少一个变体为 `downloaded`。

### 4.2 客户端

`CatalogVersion` / `CatalogInspection` 反序列化新增字段并校验：

- `artifact_status` 必须属于固定集合；
- `downloaded` 必须与 `artifact_status` 一致；
- `downloaded=true` 必须带路径和实际摘要；
- 目录状态与远程状态分离，不再通过远程 `status` 猜测本地文件。

选择器：

- `not_downloaded`：显示下载按钮；
- `downloading` / `verifying`：显示进行中状态，禁用重复下载；
- `downloaded`：显示“已下载”和“使用”入口，不显示下载图标；
- `failed`：显示“下载失败”和重试入口。

## 5. 客户端初始化状态与文案

1. `PreparationEvent::Downloaded` 只进入 `Verifying`，不依赖一个可能滞后的进度快照；
2. `DownloadProgress` 只接受单调不下降的字节数；
3. `run_id` 检查保持在事件入口；
4. `page.rs` 根据 `PreparationStage` 选择阶段文案；
5. 页面进度继续使用 HTML 轮询，但增加字节数展示和小数百分比，避免大型文件早期长期显示“0%”。

建议页面格式：

```text
下载中：12.5%（1.25 GiB / 10 GiB，速度 25 MiB/s）
```

总大小未知时仅显示已接收字节和速度。

## 6. 错误与并发

- 外部 HTTP、数据库、文件和 aria2 错误均返回 `Result` 或结构化 API 错误；
- 旧数据库记录保留但不能参与已下载判定；
- 旧 `run_id`、旧 GID 和取消后的回调必须丢弃；
- 任务复用按完整模型身份和已校验文件判定，不能只按 URL 或文件名；
- 不为本需求新增 SSE/WebSocket 或独立状态仓库。

## 7. 验收与回归测试

### 客户端

- 正常 `queued → downloading → verifying → succeeded`；
- verifying 快照落后不触发非法状态；
- succeeded 缺路径/摘要拒绝；
- 进度不倒退；
- 旧 run_id 不覆盖新运行；
- 各阶段文案和目录“已下载”展示正确。

### 服务端与下载器

- 完整身份和摘要匹配返回 downloaded；
- 远程可下载但本地不存在返回 not_downloaded；
- active/completed/succeeded/failed 映射正确；
- 迁移后旧记录可读；
- 监控更新 `0 → 5 → 10` 单调，完成为 `10/10`；
- 失败、取消、重试和旧 GID 清理正确。

### 集成

使用固定 HTTP 回放 `queued 0/10 → downloading 5/10 → verifying 10/10 → succeeded 10/10 + path + sha`，并覆盖关闭服务后重新查询模型目录仍返回 `downloaded=true`。
