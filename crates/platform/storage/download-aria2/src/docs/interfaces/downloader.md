# downloader.rs 函数文档

## 公有函数

### download_aria2

- 函数定义：
  ```rust
  pub async fn download_aria2() -> Aria2Result<PathBuf>
  ```
- 入参：无。
- 返回值：`Aria2Result<PathBuf>`。成功时返回 `aria2c.exe` 的本地路径；目标文件已存在时直接返回该路径。
- 错误信息：
  - HTTP 客户端创建、目录创建、主链接与备用链接下载均失败时返回 `Aria2Error::DownloadError`。
  - ZIP 解压失败或解压后未找到 `aria2c.exe` 时返回 `Aria2Error::DownloadError`。
  - 删除临时 ZIP 文件失败会被忽略。

## 私有函数

### download_file

- 函数定义：
  ```rust
  async fn download_file(client: &Client, url: &str, path: &Path) -> Aria2Result<()>
  ```
- 入参：
  - `client: &Client`：用于发送 HTTP 请求的客户端。
  - `url: &str`：待下载资源的 URL。
  - `path: &Path`：响应内容的写入目标路径。
- 返回值：`Aria2Result<()>`。成功时将响应字节写入目标文件。
- 错误信息：请求发送、非成功 HTTP 状态、响应正文读取或文件写入失败时返回 `Aria2Error::DownloadError`。

### extract_aria2

- 函数定义：
  ```rust
  fn extract_aria2(zip_path: &Path, target_dir: &Path) -> Aria2Result<()>
  ```
- 入参：
  - `zip_path: &Path`：待读取的 ZIP 文件路径。
  - `target_dir: &Path`：`aria2c.exe` 的解压目录。
- 返回值：`Aria2Result<()>`。成功时在目标目录写入 `aria2c.exe`。
- 错误信息：打开或解析 ZIP、读取条目、创建输出文件、复制内容失败，或归档中未找到 `aria2c.exe` 时返回 `Aria2Error::DownloadError`。
