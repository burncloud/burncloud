//! State for the supplier GPU resource page.

use super::model::{SupplierResource, SupplierResourcesModel};

#[derive(Clone, Debug, PartialEq)]
pub struct SupplierResourcesState {
    pub model: SupplierResourcesModel,
    pub drain_node: Option<&'static str>,
}

impl Default for SupplierResourcesState {
    fn default() -> Self {
        Self {
            model: SupplierResourcesModel::mock(),
            drain_node: None,
        }
    }
}

impl SupplierResourcesState {
    pub fn open_drain(&mut self, node_name: &'static str) {
        self.drain_node = Some(node_name);
    }

    pub fn close_drain(&mut self) {
        self.drain_node = None;
    }

    pub fn confirm_drain(&mut self) {
        self.close_drain();
    }

    pub fn selected_node(&self) -> Option<SupplierResource> {
        self.drain_node.and_then(|name| {
            self.model
                .nodes
                .iter()
                .find(|resource| resource.node.name == name)
                .copied()
        })
    }
}
