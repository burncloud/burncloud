# 001 下载任务模型与请求校验

## 修改的信息

- 将服务端下载请求转换为包含存储目录和安全文件名的 DownloadRequestIdentity。
- 新增 DownloadTaskState、TaskError、DownloadTaskData 与 DownloadTaskSnapshot 数据结构，统一异步下载任务状态和响应字段。
- 新增稳定任务 ID 的十进制 i64 解析，并保留现有来源、URL、路径和 SHA-256 校验规则。

## 修改的文件与文件内位置

- crates/interfaces/server/src/api/model_download.rs：DownloadSource、DownloadModelRequest、DownloadRequestIdentity、DownloadTaskState、TaskError、DownloadTaskData、DownloadTaskSnapshot；validate_request、storage_directory、parse_task_id、sha256_file。
- crates/interfaces/server/src/api/model_download_tests.rs：新增任务 ID 解析和规范化身份字段测试。

## 检查结果

- cargo fmt --all：通过。
- cargo test -p burncloud-server api::model_download::tests：通过，8 个测试通过。
- 对照验收标准：字段与状态模型已落地；请求身份校验、存储目录隔离、文件名安全化和任务 ID 边界均有实现或测试覆盖。
