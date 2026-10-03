//! Static display model for the supplier reliability page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReliabilityTierStatus {
    Completed,
    Current,
    Eligible,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReliabilityTierKind {
    Community,
    Verified,
    Professional,
    Strategic,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReliabilityTier {
    pub kind: ReliabilityTierKind,
    pub level: &'static str,
    pub revenue_share: &'static str,
    pub status: ReliabilityTierStatus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReliabilityAuditKind {
    Thermal,
    Driver,
    Interconnect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ReliabilityAuditEvent {
    pub kind: ReliabilityAuditKind,
    pub node: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplierReliabilityModel {
    pub score: &'static str,
    pub availability: &'static str,
    pub current_tier: &'static str,
    pub revenue_share: &'static str,
    pub tiers: [ReliabilityTier; 4],
    pub audits: [ReliabilityAuditEvent; 3],
}

pub const REFERENCE_RELIABILITY: SupplierReliabilityModel = SupplierReliabilityModel {
    score: "98.6",
    availability: "99.94%",
    current_tier: "L3 Pro",
    revenue_share: "80.0%",
    tiers: [
        ReliabilityTier {
            kind: ReliabilityTierKind::Community,
            level: "L1",
            revenue_share: "70%",
            status: ReliabilityTierStatus::Completed,
        },
        ReliabilityTier {
            kind: ReliabilityTierKind::Verified,
            level: "L2",
            revenue_share: "75%",
            status: ReliabilityTierStatus::Completed,
        },
        ReliabilityTier {
            kind: ReliabilityTierKind::Professional,
            level: "L3",
            revenue_share: "80%",
            status: ReliabilityTierStatus::Current,
        },
        ReliabilityTier {
            kind: ReliabilityTierKind::Strategic,
            level: "L4",
            revenue_share: "85%",
            status: ReliabilityTierStatus::Eligible,
        },
    ],
    audits: [
        ReliabilityAuditEvent {
            kind: ReliabilityAuditKind::Thermal,
            node: "HKG-Edge-RTX-Pool",
        },
        ReliabilityAuditEvent {
            kind: ReliabilityAuditKind::Driver,
            node: "SJC-Pod-01-Rack4",
        },
        ReliabilityAuditEvent {
            kind: ReliabilityAuditKind::Interconnect,
            node: "8x H100 SXM5",
        },
    ],
};

impl SupplierReliabilityModel {
    pub const fn reference() -> &'static Self {
        &REFERENCE_RELIABILITY
    }
}
