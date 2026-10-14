//! Static display data for the administrator settlement batches page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SettlementBatchStatus {
    PendingReview,
    SettledAndPaid,
}

impl SettlementBatchStatus {
    pub const fn label(self) -> &'static str {
        match self {
            Self::PendingReview => "Pending Review",
            Self::SettledAndPaid => "Settled & Paid",
        }
    }

    pub const fn tone(self) -> &'static str {
        match self {
            Self::PendingReview => "warning",
            Self::SettledAndPaid => "success",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminSettlementBatch {
    pub id: &'static str,
    pub billing_period: &'static str,
    pub total_suppliers: u16,
    pub total_gpus: u16,
    pub inference_tokens: &'static str,
    pub total_payout: &'static str,
    pub status: SettlementBatchStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminSettlementsModel {
    pub batches: [AdminSettlementBatch; 2],
}

pub const REFERENCE_SETTLEMENTS: AdminSettlementsModel = AdminSettlementsModel {
    batches: [
        AdminSettlementBatch {
            id: "BATCH-2026-08B",
            billing_period: "Aug 15 - Aug 31, 2026",
            total_suppliers: 24,
            total_gpus: 420,
            inference_tokens: "1,840.5M",
            total_payout: "$124,500.00",
            status: SettlementBatchStatus::PendingReview,
        },
        AdminSettlementBatch {
            id: "BATCH-2026-08A",
            billing_period: "Aug 01 - Aug 15, 2026",
            total_suppliers: 24,
            total_gpus: 412,
            inference_tokens: "1,790.0M",
            total_payout: "$121,200.00",
            status: SettlementBatchStatus::SettledAndPaid,
        },
    ],
};

impl AdminSettlementsModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_SETTLEMENTS
    }
}
