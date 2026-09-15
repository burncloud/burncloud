//! Shared client-side projection types.

/// The three workspace roles exposed by the console shell.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum Role {
    #[default]
    Buyer,
    Supplier,
    Admin,
}

impl Role {
    pub const ALL: [Self; 3] = [Self::Buyer, Self::Supplier, Self::Admin];

    pub const fn code(self) -> &'static str {
        match self {
            Self::Buyer => "buyer",
            Self::Supplier => "supplier",
            Self::Admin => "admin",
        }
    }

    pub const fn label(self) -> &'static str {
        match self {
            Self::Buyer => "Buyer",
            Self::Supplier => "Supplier",
            Self::Admin => "Admin",
        }
    }

    pub fn from_code(value: &str) -> Self {
        match value {
            "supplier" => Self::Supplier,
            "admin" => Self::Admin,
            _ => Self::Buyer,
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

    pub const fn overview_path(self) -> &'static str {
        match self {
            Self::Buyer => "/buyer/overview",
            Self::Supplier => "/supplier/overview",
            Self::Admin => "/admin/overview",
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

/// A navigation item rendered by the role-aware shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NavItem {
    pub key: NavKey,
    pub path: &'static str,
    pub icon: crate::shared::ui::IconName,
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
        icon: crate::shared::ui::IconName::Layout,
        badge: None,
    },
    NavItem {
        key: NavKey::Playground,
        path: "/buyer/playground",
        icon: crate::shared::ui::IconName::Terminal,
        badge: Some("LIVE"),
    },
    NavItem {
        key: NavKey::Marketplace,
        path: "/buyer/marketplace",
        icon: crate::shared::ui::IconName::Store,
        badge: None,
    },
    NavItem {
        key: NavKey::ApiKeys,
        path: "/buyer/api-keys",
        icon: crate::shared::ui::IconName::Key,
        badge: None,
    },
    NavItem {
        key: NavKey::Usage,
        path: "/buyer/usage",
        icon: crate::shared::ui::IconName::Chart,
        badge: None,
    },
    NavItem {
        key: NavKey::Billing,
        path: "/buyer/billing",
        icon: crate::shared::ui::IconName::CreditCard,
        badge: None,
    },
    NavItem {
        key: NavKey::Logs,
        path: "/buyer/logs",
        icon: crate::shared::ui::IconName::Receipt,
        badge: None,
    },
];

const SUPPLIER_NAV: [NavItem; 7] = [
    NavItem {
        key: NavKey::Overview,
        path: "/supplier/overview",
        icon: crate::shared::ui::IconName::Layout,
        badge: None,
    },
    NavItem {
        key: NavKey::Resources,
        path: "/supplier/resources",
        icon: crate::shared::ui::IconName::Server,
        badge: Some("4 NODES"),
    },
    NavItem {
        key: NavKey::Deployments,
        path: "/supplier/deployments",
        icon: crate::shared::ui::IconName::Layers,
        badge: None,
    },
    NavItem {
        key: NavKey::Earnings,
        path: "/supplier/earnings",
        icon: crate::shared::ui::IconName::Coins,
        badge: None,
    },
    NavItem {
        key: NavKey::Settlements,
        path: "/supplier/settlements",
        icon: crate::shared::ui::IconName::Receipt,
        badge: None,
    },
    NavItem {
        key: NavKey::Reliability,
        path: "/supplier/reliability",
        icon: crate::shared::ui::IconName::Shield,
        badge: None,
    },
    NavItem {
        key: NavKey::Settings,
        path: "/supplier/settings",
        icon: crate::shared::ui::IconName::Settings,
        badge: None,
    },
];

const ADMIN_NAV: [NavItem; 11] = [
    NavItem {
        key: NavKey::Overview,
        path: "/admin/overview",
        icon: crate::shared::ui::IconName::Layout,
        badge: None,
    },
    NavItem {
        key: NavKey::Supply,
        path: "/admin/supply",
        icon: crate::shared::ui::IconName::Server,
        badge: None,
    },
    NavItem {
        key: NavKey::Capacity,
        path: "/admin/capacity",
        icon: crate::shared::ui::IconName::Gauge,
        badge: Some("AUTO"),
    },
    NavItem {
        key: NavKey::Demand,
        path: "/admin/demand",
        icon: crate::shared::ui::IconName::Trending,
        badge: None,
    },
    NavItem {
        key: NavKey::Models,
        path: "/admin/models",
        icon: crate::shared::ui::IconName::Cpu,
        badge: None,
    },
    NavItem {
        key: NavKey::Revenue,
        path: "/admin/revenue",
        icon: crate::shared::ui::IconName::Dollar,
        badge: None,
    },
    NavItem {
        key: NavKey::Settlements,
        path: "/admin/settlements",
        icon: crate::shared::ui::IconName::Receipt,
        badge: None,
    },
    NavItem {
        key: NavKey::Suppliers,
        path: "/admin/suppliers",
        icon: crate::shared::ui::IconName::Building,
        badge: None,
    },
    NavItem {
        key: NavKey::Customers,
        path: "/admin/customers",
        icon: crate::shared::ui::IconName::Users,
        badge: None,
    },
    NavItem {
        key: NavKey::Operations,
        path: "/admin/operations",
        icon: crate::shared::ui::IconName::Workflow,
        badge: Some("AUTOPILOT"),
    },
    NavItem {
        key: NavKey::Settings,
        path: "/admin/settings",
        icon: crate::shared::ui::IconName::Settings,
        badge: None,
    },
];
