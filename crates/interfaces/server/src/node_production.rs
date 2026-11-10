use burncloud_node_runtime::{HardwareProbe, SystemHardwareProbe};
use burncloud_supply_model::{
    ModelManifest, ModelResolutionOutcome, ModelResolutionRequest, ModelResolver, RealModelResolver,
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
#[derive(Debug, Default)]
pub struct ProductionNode {
    hardware: SystemHardwareProbe,
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
        Ok(resolver
            .resolve(ModelResolutionRequest {
                model: demand.model,
                hardware,
            })
            .await?)
    }
}
