//! Static display model for the administrator enterprise customer accounts page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CustomerRisk {
    Low,
    LowBalance,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminCustomerRow {
    pub name: &'static str,
    pub email: &'static str,
    pub keys: u8,
    pub spend: &'static str,
    pub balance: &'static str,
    pub risk: CustomerRisk,
    pub status: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminCustomersModel {
    pub customers: [AdminCustomerRow; 4],
}

impl AdminCustomersModel {
    pub const fn mock() -> Self {
        Self {
            customers: [
                AdminCustomerRow {
                    name: "HyperScale AI Labs",
                    email: "infra@hyperscale.ai",
                    keys: 8,
                    spend: "$48,200",
                    balance: "$18,400.00",
                    risk: CustomerRisk::Low,
                    status: "Active",
                },
                AdminCustomerRow {
                    name: "Apex Robotics Co.",
                    email: "ops@apexrobotics.com",
                    keys: 4,
                    spend: "$24,100",
                    balance: "$6,200.00",
                    risk: CustomerRisk::Low,
                    status: "Active",
                },
                AdminCustomerRow {
                    name: "FinTech Intelligence Inc.",
                    email: "security@fintech-intel.io",
                    keys: 12,
                    spend: "$19,800",
                    balance: "$12,850.00",
                    risk: CustomerRisk::Low,
                    status: "Active",
                },
                AdminCustomerRow {
                    name: "AutoAgent Studio",
                    email: "dev@autoagent.ai",
                    keys: 2,
                    spend: "$4,120",
                    balance: "$128.50",
                    risk: CustomerRisk::LowBalance,
                    status: "Active",
                },
            ],
        }
    }
}
