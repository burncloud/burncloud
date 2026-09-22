// ============================================================================
// 错误类型定义
// ============================================================================

#[derive(Debug)]
pub enum Aria2Error {
    DownloadError(String),
    PortError(String),
    RpcError(String),
    DaemonError(String),
    ProcessError(String),
    ConfigError(String),
}

impl std::fmt::Display for Aria2Error {
    // 输入：当前错误实例与格式化输出器。
    // 功能：将 Aria2 错误转换为面向用户的中文错误文本。
    // 错误：无，格式化结果通过 std::fmt::Result 返回。
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // 按错误类型生成对应文本
        match self {
            Aria2Error::DownloadError(msg) => write!(f, "下载错误: {}", msg),
            Aria2Error::PortError(msg) => write!(f, "端口错误: {}", msg),
            Aria2Error::RpcError(msg) => write!(f, "RPC错误: {}", msg),
            Aria2Error::DaemonError(msg) => write!(f, "守护进程错误: {}", msg),
            Aria2Error::ProcessError(msg) => write!(f, "进程错误: {}", msg),
            Aria2Error::ConfigError(msg) => write!(f, "配置错误: {}", msg),
        }
    }
}

impl std::error::Error for Aria2Error {}

pub type Aria2Result<T> = Result<T, Aria2Error>;
