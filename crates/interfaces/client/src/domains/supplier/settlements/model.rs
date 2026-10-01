//! Fixed display data for the supplier settlements page.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SettlementBatch {
    pub id: &'static str,
    pub date: &'static str,
    pub payment_method: &'static str,
    pub tokens: &'static str,
    pub amount: f64,
    pub status: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupplierSettlementsModel {
    pub next_payout: f64,
    pub next_payout_date: &'static str,
    pub payout_destination: &'static str,
    pub payout_account: &'static str,
    pub payout_verified: bool,
    pub available_balance: f64,
    pub completed_batches: u32,
    pub disputes: u32,
    pub batches: [SettlementBatch; 3],
}

pub const REFERENCE_SETTLEMENTS: SupplierSettlementsModel = SupplierSettlementsModel {
    next_payout: 6_420.00,
    next_payout_date: "Sept 1",
    payout_destination: "Silicon Valley Bank (ACH)",
    payout_account: "•••• 9102",
    payout_verified: true,
    available_balance: 38_910.40,
    completed_batches: 12,
    disputes: 0,
    batches: [
        SettlementBatch {
            id: "SET-2026-0815",
            date: "2026-08-15",
            payment_method: "ACH •••• 9102",
            tokens: "512.4M",
            amount: 7_840.00,
            status: "COMPLETED",
        },
        SettlementBatch {
            id: "SET-2026-0801",
            date: "2026-08-01",
            payment_method: "ACH •••• 9102",
            tokens: "490.1M",
            amount: 7_120.50,
            status: "COMPLETED",
        },
        SettlementBatch {
            id: "SET-2026-0715",
            date: "2026-07-15",
            payment_method: "USDC (0x9a8f...3d11)",
            tokens: "620.0M",
            amount: 8_940.00,
            status: "COMPLETED",
        },
    ],
};

impl SupplierSettlementsModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_SETTLEMENTS
    }
}
