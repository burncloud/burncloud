//! Fixed display data for the buyer billing page.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BillingInvoice {
    pub id: &'static str,
    pub date: &'static str,
    pub description: &'static str,
    pub payment_method: &'static str,
    pub amount: f64,
    pub status: &'static str,
    pub pdf_available: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BillingModel {
    pub prepaid_balance: Option<f64>,
    pub today_spend: f64,
    pub auto_top_up_amount: f64,
    pub auto_top_up_threshold: f64,
    pub payment_method: &'static str,
    pub metering_policy: &'static str,
    pub platform_markup_note: &'static str,
    pub invoices: [BillingInvoice; 3],
}

pub const REFERENCE_BILLING: BillingModel = BillingModel {
    prepaid_balance: Some(128.50),
    today_spend: 14.28,
    auto_top_up_amount: 100.0,
    auto_top_up_threshold: 20.0,
    payment_method: "Visa •••• 4242",
    metering_policy:
        "Funds are drawn down in real time as tokens are generated. Unused balance never expires.",
    platform_markup_note: "0% Platform markup on frontier models",
    invoices: [
        BillingInvoice {
            id: "INV-2026-0882",
            date: "2026-08-20",
            description: "Prepaid Token Balance Top Up",
            payment_method: "Visa •••• 4242",
            amount: 100.0,
            status: "PAID",
            pdf_available: true,
        },
        BillingInvoice {
            id: "INV-2026-0741",
            date: "2026-08-01",
            description: "Prepaid Token Balance Top Up",
            payment_method: "Visa •••• 4242",
            amount: 200.0,
            status: "PAID",
            pdf_available: true,
        },
        BillingInvoice {
            id: "INV-2026-0610",
            date: "2026-07-15",
            description: "Prepaid Token Balance Top Up",
            payment_method: "Corporate Wire (ACH)",
            amount: 500.0,
            status: "PAID",
            pdf_available: true,
        },
    ],
};

impl BillingModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_BILLING
    }
}
