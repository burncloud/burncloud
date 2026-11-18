use async_trait::async_trait;
use burncloud_node_runtime::{
    ModelUrlRuntimeAdapter, PreparedArtifact, PreparedRuntime, ProcessPlan, ProcessSpec,
    ReadinessTarget, RuntimeAdapter, RuntimeAdapterError, RuntimePrepareError, RuntimePreparer,
    RuntimeRequest,
};
use reqwest::Url;
use std::{io::Write, path::PathBuf, sync::Mutex};
use tempfile::NamedTempFile;

#[derive(Debug, Default)]
pub(crate) struct LlamaCppRuntimePreparer;

#[async_trait]
impl RuntimePreparer for LlamaCppRuntimePreparer {
    async fn prepare(
        &self,
        request: RuntimeRequest,
    ) -> Result<PreparedRuntime, RuntimePrepareError> {
        // 先限制运行时类型，避免误把其他可执行程序按 llama.cpp 参数启动。
        if request.runtime != "llama.cpp" {
            return Err(RuntimePrepareError::PrepareFailed(format!(
                "unsupported local runtime '{}'",
                request.runtime
            )));
        }

        // 由部署环境显式指定二进制，避免从 PATH 中静默选择到不同版本。
        let executable = std::env::var_os("BURNCLOUD_LLAMA_SERVER")
            .map(PathBuf::from)
            .ok_or_else(|| {
                RuntimePrepareError::PrepareFailed(
                    "BURNCLOUD_LLAMA_SERVER must point to llama-server".to_string(),
                )
            })?;
        // 仅检查目标存在为普通文件；实际启动权限和可执行格式由进程启动阶段报告。
        if !executable.is_file() {
            return Err(RuntimePrepareError::PrepareFailed(format!(
                "llama-server executable does not exist: {}",
                executable.display()
            )));
        }

        Ok(PreparedRuntime {
            executable: executable.to_string_lossy().into_owned(),
        })
    }
}

#[derive(Debug)]
pub(crate) struct LlamaCppRuntimeAdapter {
    model_id: String,
    // preset 必须保留到子进程读取完成，因此由 adapter 持有而不是局部临时删除。
    preset_file: Mutex<Option<NamedTempFile>>,
}

impl LlamaCppRuntimeAdapter {
    pub(crate) fn new(model_id: String) -> Self {
        Self {
            model_id,
            preset_file: Mutex::new(None),
        }
    }
}

#[async_trait]
impl RuntimeAdapter for LlamaCppRuntimeAdapter {
    async fn plan(
        &self,
        runtime: &PreparedRuntime,
        artifact: &PreparedArtifact,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        // 适配器不能生成可执行的空程序计划。
        if runtime.executable.is_empty() {
            return Err(RuntimeAdapterError::PlanFailed(
                "runtime executable must not be empty".to_string(),
            ));
        }
        // 计划只接受上游已校验工件，避免把未验证路径交给真实模型进程。
        if !artifact.is_verified() {
            return Err(RuntimeAdapterError::PlanFailed(
                "prepared artifact has not been verified".to_string(),
            ));
        }
        // Verified 状态不保证路径仍存在，启动前再次确认文件系统对象可用。
        if !artifact.local_path().is_file() {
            return Err(RuntimeAdapterError::PlanFailed(format!(
                "prepared artifact is not a file: {}",
                artifact.local_path().display()
            )));
        }

        // 使用系统分配的空闲 loopback 端口，避免硬编码端口与其他服务冲突。
        let port = allocate_loopback_port().await?;
        let local_endpoint = format!("http://127.0.0.1:{port}");
        Ok(ProcessPlan {
            process: ProcessSpec {
                program: runtime.executable.clone(),
                // 直接使用已校验工件的本机路径，部署层不复制或重新定位模型文件。
                args: vec![
                    "-m".to_string(),
                    artifact.local_path().to_string_lossy().into_owned(),
                    "--host".to_string(),
                    "127.0.0.1".to_string(),
                    "--port".to_string(),
                    port.to_string(),
                ],
            },
            // readiness 与本次实际监听端口绑定，避免计划和探测访问不同实例。
            readiness: ReadinessTarget {
                endpoint: format!("{local_endpoint}/health"),
            },
            local_endpoint,
        })
    }
}

#[async_trait]
impl ModelUrlRuntimeAdapter for LlamaCppRuntimeAdapter {
    async fn plan_model_url(
        &self,
        runtime: &PreparedRuntime,
        model_url: &str,
    ) -> Result<ProcessPlan, RuntimeAdapterError> {
        // 使用 URL 解析器校验 scheme/host 并安全编码后续的查询参数。
        let model_url = Url::parse(model_url)
            .map_err(|error| RuntimeAdapterError::PlanFailed(error.to_string()))?;
        if !matches!(model_url.scheme(), "http" | "https") || model_url.host_str().is_none() {
            return Err(RuntimeAdapterError::PlanFailed(
                "model URL must use HTTP or HTTPS".to_string(),
            ));
        }
        // preset 以 model_id 作为 section 标识；拒绝控制字符和括号以免改变其结构。
        if self.model_id.is_empty()
            || self
                .model_id
                .chars()
                .any(|character| matches!(character, '[' | ']' | '\r' | '\n'))
        {
            return Err(RuntimeAdapterError::PlanFailed(
                "model_id cannot be represented in a llama.cpp preset".to_string(),
            ));
        }

        // llama.cpp router 通过 preset 注册模型别名，并在服务启动时加载对应模型。
        let mut preset_file = NamedTempFile::new()
            .map_err(|error| RuntimeAdapterError::PlanFailed(error.to_string()))?;
        writeln!(preset_file, "[{}]", self.model_id)
            .and_then(|()| writeln!(preset_file, "model-url = {}", model_url))
            .and_then(|()| writeln!(preset_file, "load-on-startup = true"))
            .map_err(|error| RuntimeAdapterError::PlanFailed(error.to_string()))?;
        let preset_path = preset_file.path().to_string_lossy().into_owned();
        // trait 对象要求 adapter 可跨线程共享，互斥锁保护临时文件所有权与生命周期。
        *self
            .preset_file
            .lock()
            .map_err(|error| RuntimeAdapterError::PlanFailed(error.to_string()))? =
            Some(preset_file);

        // 每个部署使用独立 loopback 端口，使 readiness 和 endpoint 都指向本次进程。
        let port = allocate_loopback_port().await?;

        let endpoint = format!("http://127.0.0.1:{port}");
        // readiness 查询精确 model_id；HTTP 200 但模型未加载时不能算部署就绪。
        let mut readiness_url = Url::parse(&format!("{endpoint}/v1/models"))
            .map_err(|error| RuntimeAdapterError::PlanFailed(error.to_string()))?;
        readiness_url
            .query_pairs_mut()
            .append_pair("model_id", &self.model_id);
        Ok(ProcessPlan {
            process: ProcessSpec {
                program: runtime.executable.clone(),
                // 将 preset 和绑定地址传给 llama-server，adapter 生成的参数是启动唯一来源。
                args: vec![
                    "--models-preset".to_string(),
                    preset_path,
                    "--host".to_string(),
                    "127.0.0.1".to_string(),
                    "--port".to_string(),
                    port.to_string(),
                ],
            },
            readiness: ReadinessTarget {
                endpoint: readiness_url.to_string(),
            },
            local_endpoint: endpoint,
        })
    }
}

async fn allocate_loopback_port() -> Result<u16, RuntimeAdapterError> {
    // 让操作系统挑选端口，避免固定端口冲突并限定服务只绑定本机回环接口。
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|error| RuntimeAdapterError::PlanFailed(error.to_string()))?;
    // 只把已分配端口写入计划；listener 随函数返回释放，子进程随后绑定该端口。
    listener
        .local_addr()
        .map(|address| address.port())
        .map_err(|error| RuntimeAdapterError::PlanFailed(error.to_string()))
}

#[cfg(test)]
#[path = "node_runtime/tests.rs"]
mod tests;
