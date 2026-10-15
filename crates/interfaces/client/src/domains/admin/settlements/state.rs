use super::model::{AdminSettlementBatch, AdminSettlementsModel, SettlementBatchStatus};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminSettlementsState {
    pub batches: [AdminSettlementBatch; 2],
}

impl Default for AdminSettlementsState {
    fn default() -> Self {
        Self {
            batches: AdminSettlementsModel::reference().batches,
        }
    }
}

impl AdminSettlementsState {
    pub fn approve_batch(&mut self, id: &str) {
        if let Some(batch) = self.batches.iter_mut().find(|batch| batch.id == id) {
            if batch.status == SettlementBatchStatus::PendingReview {
                batch.status = SettlementBatchStatus::SettledAndPaid;
            }
        }
    }
}
