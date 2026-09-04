#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Role {
    #[default]
    Buyer,
    Supplier,
    Admin,
}

impl Role {
    pub const ALL: [Self; 3] = [Self::Buyer, Self::Supplier, Self::Admin];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Buyer => "buyer",
            Self::Supplier => "supplier",
            Self::Admin => "admin",
        }
    }

    pub const fn overview_path(self) -> &'static str {
        match self {
            Self::Buyer => "/buyer/overview",
            Self::Supplier => "/supplier/overview",
            Self::Admin => "/admin/overview",
        }
    }

    pub fn from_path(path: &str) -> Self {
        if path.starts_with("/supplier") {
            Self::Supplier
        } else if path.starts_with("/admin") {
            Self::Admin
        } else {
            Self::Buyer
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NavKey {
    Overview,
    Playground,
    Marketplace,
    ApiKeys,
    Usage,
    Billing,
    Logs,
    Resources,
    Deployments,
    Earnings,
    Settlements,
    Reliability,
    Settings,
    Supply,
    Capacity,
    Demand,
    Models,
    Revenue,
    Suppliers,
    Customers,
    Operations,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IconKind {
    AlertTriangle,
    ArrowRight,
    Bell,
    Building,
    Chart,
    Check,
    CheckCircle,
    ChevronDown,
    Coins,
    Cpu,
    CreditCard,
    Dollar,
    Gauge,
    Globe,
    Key,
    Layers,
    Layout,
    Menu,
    Receipt,
    Search,
    Server,
    Settings,
    Shield,
    Store,
    Terminal,
    Trending,
    Users,
    Workflow,
    X,
    Zap,
}

#[derive(Clone, Copy, Debug)]
pub struct NavItem {
    pub key: NavKey,
    pub path: &'static str,
    pub icon: IconKind,
    pub badge: Option<&'static str>,
}

pub const fn nav_items(role: Role) -> &'static [NavItem] {
    match role {
        Role::Buyer => &BUYER_NAV,
        Role::Supplier => &SUPPLIER_NAV,
        Role::Admin => &ADMIN_NAV,
    }
}

const BUYER_NAV: [NavItem; 7] = [
    NavItem {
        key: NavKey::Overview,
        path: "/buyer/overview",
        icon: IconKind::Layout,
        badge: None,
    },
    NavItem {
        key: NavKey::Playground,
        path: "/buyer/playground",
        icon: IconKind::Terminal,
        badge: Some("LIVE"),
    },
    NavItem {
        key: NavKey::Marketplace,
        path: "/buyer/marketplace",
        icon: IconKind::Store,
        badge: None,
    },
    NavItem {
        key: NavKey::ApiKeys,
        path: "/buyer/api-keys",
        icon: IconKind::Key,
        badge: None,
    },
    NavItem {
        key: NavKey::Usage,
        path: "/buyer/usage",
        icon: IconKind::Chart,
        badge: None,
    },
    NavItem {
        key: NavKey::Billing,
        path: "/buyer/billing",
        icon: IconKind::CreditCard,
        badge: None,
    },
    NavItem {
        key: NavKey::Logs,
        path: "/buyer/logs",
        icon: IconKind::Receipt,
        badge: None,
    },
];

const SUPPLIER_NAV: [NavItem; 7] = [
    NavItem {
        key: NavKey::Overview,
        path: "/supplier/overview",
        icon: IconKind::Layout,
        badge: None,
    },
    NavItem {
        key: NavKey::Resources,
        path: "/supplier/resources",
        icon: IconKind::Server,
        badge: Some("4 NODES"),
    },
    NavItem {
        key: NavKey::Deployments,
        path: "/supplier/deployments",
        icon: IconKind::Layers,
        badge: None,
    },
    NavItem {
        key: NavKey::Earnings,
        path: "/supplier/earnings",
        icon: IconKind::Coins,
        badge: None,
    },
    NavItem {
        key: NavKey::Settlements,
        path: "/supplier/settlements",
        icon: IconKind::Receipt,
        badge: None,
    },
    NavItem {
        key: NavKey::Reliability,
        path: "/supplier/reliability",
        icon: IconKind::Shield,
        badge: None,
    },
    NavItem {
        key: NavKey::Settings,
        path: "/supplier/settings",
        icon: IconKind::Settings,
        badge: None,
    },
];

const ADMIN_NAV: [NavItem; 11] = [
    NavItem {
        key: NavKey::Overview,
        path: "/admin/overview",
        icon: IconKind::Layout,
        badge: None,
    },
    NavItem {
        key: NavKey::Supply,
        path: "/admin/supply",
        icon: IconKind::Server,
        badge: None,
    },
    NavItem {
        key: NavKey::Capacity,
        path: "/admin/capacity",
        icon: IconKind::Gauge,
        badge: Some("AUTO"),
    },
    NavItem {
        key: NavKey::Demand,
        path: "/admin/demand",
        icon: IconKind::Trending,
        badge: None,
    },
    NavItem {
        key: NavKey::Models,
        path: "/admin/models",
        icon: IconKind::Cpu,
        badge: None,
    },
    NavItem {
        key: NavKey::Revenue,
        path: "/admin/revenue",
        icon: IconKind::Dollar,
        badge: None,
    },
    NavItem {
        key: NavKey::Settlements,
        path: "/admin/settlements",
        icon: IconKind::Receipt,
        badge: None,
    },
    NavItem {
        key: NavKey::Suppliers,
        path: "/admin/suppliers",
        icon: IconKind::Building,
        badge: None,
    },
    NavItem {
        key: NavKey::Customers,
        path: "/admin/customers",
        icon: IconKind::Users,
        badge: None,
    },
    NavItem {
        key: NavKey::Operations,
        path: "/admin/operations",
        icon: IconKind::Workflow,
        badge: Some("AUTOPILOT"),
    },
    NavItem {
        key: NavKey::Settings,
        path: "/admin/settings",
        icon: IconKind::Settings,
        badge: None,
    },
];

#[derive(Clone, Debug)]
pub struct Metric {
    pub value: String,
    pub unit: Option<&'static str>,
    pub trend: Option<&'static str>,
    pub positive: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelUsage {
    pub name: &'static str,
    pub family: &'static str,
    pub tier: &'static str,
    pub tokens: &'static str,
    pub latency: &'static str,
    pub cost: &'static str,
}

pub const MODEL_USAGE: [ModelUsage; 3] = [
    ModelUsage {
        name: "DeepSeek V3 (671B MoE)",
        family: "DeepSeek",
        tier: "STANDARD",
        tokens: "1,120,400",
        latency: "380 ms",
        cost: "$0.28",
    },
    ModelUsage {
        name: "DeepSeek R1 Reasoning",
        family: "DeepSeek",
        tier: "PERFORMANCE",
        tokens: "410,200",
        latency: "620 ms",
        cost: "$0.89",
    },
    ModelUsage {
        name: "Qwen 2.5 72B Instruct",
        family: "Qwen",
        tier: "STANDARD",
        tokens: "310,000",
        latency: "410 ms",
        cost: "$0.18",
    },
];

#[derive(Clone, Copy, Debug)]
pub struct ActivityEvent {
    pub kind: IconKind,
    pub title: &'static str,
    pub description: &'static str,
    pub time: &'static str,
}

pub const ACTIVITY: [ActivityEvent; 3] = [
    ActivityEvent {
        kind: IconKind::Dollar,
        title: "Prepaid balance top-up completed ($100.00)",
        description: "Receipt #REC-8921 generated. Payment method: Visa ending in 4242.",
        time: "12 mins ago",
    },
    ActivityEvent {
        kind: IconKind::Zap,
        title: "Sub-150ms smart fallback verified for DeepSeek V3",
        description: "BurnCloud automatically provisioned additional capacity in US-West cluster.",
        time: "1 hour ago",
    },
    ActivityEvent {
        kind: IconKind::Key,
        title: "New API Key \"Production Kubernetes Cluster\" generated",
        description: "Associated with Standard & Performance tiers.",
        time: "1 day ago",
    },
];

pub const TODAY_SPEND: f64 = 14.28;
pub const BALANCE: f64 = 128.50;

pub const fn is_low_balance(balance: f64) -> bool {
    balance < 20.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn balance_threshold_matches_product_rule() {
        assert!(is_low_balance(19.99));
        assert!(!is_low_balance(20.0));
        assert!(!is_low_balance(BALANCE));
    }

    #[test]
    fn role_paths_round_trip() {
        for role in Role::ALL {
            assert_eq!(Role::from_path(role.overview_path()), role);
        }
    }
}
