# downloader.rs

## 公有函数

### download_aria2
- 函数定义：`pub async fn download_aria2() -> Aria2Result<PathBuf>`
- 入参：无。
- 返回值：`Aria2Result<PathBuf>`；成功时返回本地 `aria2c.exe` 路径。
- 错误信息：客户端创建、目录创建、两条下载链接均失败、解压失败或未找到可执行文件时返回 `DownloadError`；删除 ZIP 失败被忽略。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[创建带超时的 HTTP 客户端]
  B --> C{创建成功}
  C -->|否| X[返回 DownloadError]
  C -->|是| D[创建 BurnCloud 目录]
  D --> E{目录创建成功}
  E -->|否| X
  E -->|是| F{aria2c.exe 已存在}
  F -->|是| G[返回现有路径]
  F -->|否| H[从主链接下载]
  H --> I{下载成功}
  I -->|否| J[从备用链接下载]
  J --> K{下载成功}
  K -->|否| X
  I -->|是| L[解压 ZIP]
  K -->|是| L
  L --> M{解压成功}
  M -->|否| X
  M -->|是| N[尝试删除 ZIP]
  N --> O{aria2c.exe 存在}
  O -->|是| P[返回可执行文件路径]
  O -->|否| X
```

## 私有函数

### download_file
- 函数定义：`async fn download_file(client: &Client, url: &str, path: &Path) -> Aria2Result<()>`
- 入参：`client` 为 HTTP 客户端；`url` 为资源地址；`path` 为输出文件路径。
- 返回值：`Aria2Result<()>`；成功时将响应内容写入目标文件。
- 错误信息：请求、HTTP 状态、读取正文或写文件失败时返回 `DownloadError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[发送 GET 请求]
  B --> C{请求成功}
  C -->|否| X[返回 DownloadError]
  C -->|是| D{HTTP 状态成功}
  D -->|否| X
  D -->|是| E[读取响应字节]
  E --> F{读取成功}
  F -->|否| X
  F -->|是| G[写入目标文件]
  G --> H{写入成功}
  H -->|否| X
  H -->|是| I[返回 Ok]
```

### extract_aria2
- 函数定义：`fn extract_aria2(zip_path: &Path, target_dir: &Path) -> Aria2Result<()>`
- 入参：`zip_path` 为 ZIP 文件路径；`target_dir` 为解压目标目录。
- 返回值：`Aria2Result<()>`；成功时写入 `aria2c.exe`。
- 错误信息：打开或解析 ZIP、读取条目、创建文件、复制内容失败，或未找到 `aria2c.exe` 时返回 `DownloadError`。
- 设计流程图：
```mermaid
flowchart TD
  A[开始] --> B[打开 ZIP 文件]
  B --> C{打开及解析成功}
  C -->|否| X[返回 DownloadError]
  C -->|是| D[遍历归档条目]
  D --> E{条目为 aria2c.exe}
  E -->|否| F{仍有条目}
  F -->|是| D
  F -->|否| X2[返回未找到 DownloadError]
  E -->|是| G[创建目标文件并复制内容]
  G --> H{写入成功}
  H -->|否| X
  H -->|是| I[返回 Ok]
```
