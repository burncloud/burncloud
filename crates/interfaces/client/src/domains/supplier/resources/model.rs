//! Static display model for the supplier GPU resource page.

use crate::domains::supplier::overview::model::{NodeStatus, SupplierNode, SupplierOverviewModel};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupplierResource {
    pub node: SupplierNode,
    pub uptime_hours: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SupplierResourcesModel {
    pub nodes: Vec<SupplierResource>,
}

impl SupplierResourcesModel {
    pub fn mock() -> Self {
        let uptime_hours = [742, 512, 1240, 88];
        let nodes = SupplierOverviewModel::mock()
            .nodes
            .into_iter()
            .zip(uptime_hours)
            .map(|(node, uptime_hours)| SupplierResource { node, uptime_hours })
            .collect();

        Self { nodes }
    }

    pub fn online_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|resource| resource.node.status == NodeStatus::Online)
            .count()
    }
}
