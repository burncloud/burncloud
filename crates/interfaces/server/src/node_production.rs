use burncloud_node_runtime::{HardwareProbe, SystemHardwareProbe};
use burncloud_supply_model::{
    CandidateResolutionOutcome, ModelManifest, ModelResolutionOutcome, ModelResolutionRequest,
    ModelResolver, RealModelResolver, ResolvedModel,
};

use crate::node_orchestrator::ModelDemand;

/// 服务中的生产 Node 入口，负责将真实硬件数据送入模型解析器。
///
/// 上游提供已选模型名及已加载的清单。本入口不创建 HTTP 路由，
/// 也不执行下载、进程启动或路由挂载。
///
/// # Examples
///
/// ```no_run
/// use burncloud_server::node_production::ProductionNode;
/// use burncloud_supply_model::ModelManifest;
///
/// # async fn example(manifests: Vec<ModelManifest>) -> anyhow::Result<()> {
/// let node = ProductionNode::new();
/// let outcome = node.resolve_selected_model("selected-model", manifests).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct ProductionNode {
    hardware: SystemHardwareProbe,
    supported_runtimes: Vec<String>,
}

impl Default for ProductionNode {
    fn default() -> Self {
        Self {
            hardware: SystemHardwareProbe::new(),
            supported_runtimes: vec!["llama.cpp".into()],
        }
    }
}

impl ProductionNode {
    /// 组装真实硬件探测器；实际探测在模型提交后执行。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_server::node_production::ProductionNode;
    /// let node = ProductionNode::new();
    /// ```
    pub fn new() -> Self {
        Self::default()
    }

    /// 使用指定的 Runtime 能力创建生产 Node 入口。
    ///
    /// # Examples
    ///
    /// ```
    /// use burncloud_server::node_production::ProductionNode;
    ///
    /// let node = ProductionNode::with_supported_runtimes(vec!["llama.cpp".into()])?;
    /// # let _ = node;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    ///
    /// 注意事项：Runtime 名称必须是非空字符串；该列表表示后续 Runtime 准备器能够处理
    /// 的后端，而不是当前已经安装的 Runtime。
    pub fn with_supported_runtimes(supported_runtimes: Vec<String>) -> anyhow::Result<Self> {
        if supported_runtimes
            .iter()
            .any(|runtime| runtime.trim().is_empty())
        {
            anyhow::bail!("supported runtime names must not be empty");
        }
        if supported_runtimes.is_empty() {
            anyhow::bail!("supported runtime names must not be empty");
        }

        Ok(Self {
            hardware: SystemHardwareProbe::new(),
            supported_runtimes,
        })
    }

    /// 校验已选模型、探测当前主机，再用真实快照解析模型清单。
    ///
    /// 空模型名、探测失败或目录中不存在模型时返回错误。
    /// 没有兼容变体时返回 `ModelResolutionOutcome::Unsupported`。
    /// GPU 不可用时沿用探测器的空列表或部分结果，不构造虚假容量。
    /// 调用方需运行在 Tokio 上；清单由上游加载，本入口不假定其来源。
    pub async fn resolve_selected_model(
        &self,
        model: &str,
        manifests: Vec<ModelManifest>,
    ) -> anyhow::Result<ModelResolutionOutcome> {
        let demand = ModelDemand::new(model)?;
        let hardware = self.hardware.inspect().await?;
        let resolver = RealModelResolver::new(manifests)?;
        match resolver
            .resolve_candidates(
                ModelResolutionRequest {
                    model: demand.model.clone(),
                    hardware,
                },
                &self.supported_runtimes,
            )
            .await?
        {
            CandidateResolutionOutcome::Candidates(candidates) => {
                let candidate = candidates
                    .into_iter()
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("resolver returned no candidates"))?;
                Ok(ModelResolutionOutcome::Local(ResolvedModel {
                    model: candidate.model_name,
                    artifact_source: candidate.artifact_uri,
                    artifact_digest: Some(candidate.checksum),
                    runtime: candidate.runtime,
                    runtime_version: None,
                }))
            }
            CandidateResolutionOutcome::Unsupported(reason) => {
                Ok(ModelResolutionOutcome::Unsupported(reason))
            }
        }
    }
}
