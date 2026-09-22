//! Immutable API key metadata shown by the buyer console.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiKeyStatus {
    Active,
    Revoked,
}

impl ApiKeyStatus {
    pub const fn is_active(self) -> bool {
        matches!(self, Self::Active)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuyerApiKey {
    pub id: String,
    pub name: String,
    pub key_prefix: String,
    pub masked_key: String,
    pub created: String,
    pub last_used: String,
    pub tier: String,
    pub rate_limit_rpm: u32,
    pub monthly_spend_cap: f64,
    pub spend_this_month: f64,
    pub status: ApiKeyStatus,
}

impl BuyerApiKey {
    pub fn mock_keys() -> Vec<Self> {
        vec![
            Self {
                id: "key-prod-01".to_string(),
                name: "Production Kubernetes Cluster (US-West)".to_string(),
                key_prefix: "bc_live_9a7b".to_string(),
                masked_key: "bc_live_9a7b\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}3d8f".to_string(),
                created: "2026-06-12".to_string(),
                last_used: "Just now".to_string(),
                tier: "All Tiers".to_string(),
                rate_limit_rpm: 1_200,
                monthly_spend_cap: 2_500.0,
                spend_this_month: 842.10,
                status: ApiKeyStatus::Active,
            },
            Self {
                id: "key-dev-agent".to_string(),
                name: "Dev Agent Sandbox (CI/CD)".to_string(),
                key_prefix: "bc_live_44f1".to_string(),
                masked_key: "bc_live_44f1\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}89c2".to_string(),
                created: "2026-07-04".to_string(),
                last_used: "14 mins ago".to_string(),
                tier: "Standard & Economy".to_string(),
                rate_limit_rpm: 300,
                monthly_spend_cap: 500.0,
                spend_this_month: 128.40,
                status: ApiKeyStatus::Active,
            },
            Self {
                id: "key-eval-bench".to_string(),
                name: "Model Benchmark & Automated Evaluation".to_string(),
                key_prefix: "bc_live_109e".to_string(),
                masked_key: "bc_live_109e\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}1120".to_string(),
                created: "2026-08-01".to_string(),
                last_used: "2 days ago".to_string(),
                tier: "All Tiers".to_string(),
                rate_limit_rpm: 600,
                monthly_spend_cap: 300.0,
                spend_this_month: 42.15,
                status: ApiKeyStatus::Active,
            },
        ]
    }
}
