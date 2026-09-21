# types.rs 函数文档

## 公有函数

### is_running

- 函数定义：
  ```rust
  pub fn is_running(&mut self) -> bool
  ```
- 入参：
  - `&mut self`：包含 aria2 子进程的运行实例；可变借用用于调用 `Child::try_wait`。
- 返回值：`bool`。子进程仍在运行时返回 `true`；已经退出或状态查询失败时返回 `false`。
- 错误信息：不返回 `Result`；`try_wait` 失败会转换为 `false`。

### kill

- 函数定义：
  ```rust
  pub fn kill(&mut self) -> Aria2Result<()>
  ```
- 入参：
  - `&mut self`：包含 aria2 子进程的运行实例。
- 返回值：`Aria2Result<()>`。成功时终止子进程并等待其退出。
- 错误信息：终止子进程或等待子进程退出失败时返回 `Aria2Error::ProcessError`。

## 私有函数

### default

- 函数定义：
  ```rust
  fn default() -> Self
  ```
- 入参：无。
- 返回值：`Self`，默认 `Aria2Config`：端口为 `DEFAULT_PORT`，密钥为空，下载目录为当前目录下的 `downloads`，连接数为 `16`，分片大小为 `"1M"`，aria2 路径为 BurnCloud 目录下的 `aria2c.exe`。
- 错误信息：不返回 `Result`；获取当前工作目录失败时使用空路径，再拼接 `downloads`。
