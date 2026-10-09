//! Static display model for the administrator supply fleet page.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SupplyCluster {
    pub name: &'static str,
    pub source: &'static str,
    pub hardware: &'static str,
    pub region: &'static str,
    pub utilization: &'static str,
    pub attestation: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AdminSupplyModel {
    pub total_gpus: &'static str,
    pub providers: &'static str,
    pub core_dc_gpus: &'static str,
    pub standby_gpus: &'static str,
    pub clusters: [SupplyCluster; 4],
}

impl AdminSupplyModel {
    pub const fn mock() -> Self {
        Self {
            total_gpus: "420",
            providers: "24",
            core_dc_gpus: "96",
            standby_gpus: "44",
            clusters: [
                SupplyCluster {
                    name: "Silicon-Bay Core Pod 1-4",
                    source: "Owned Bare-Metal IDC",
                    hardware: "64x H100 SXM5 80GB",
                    region: "us-west-sjc",
                    utilization: "92.4%",
                    attestation: "Confidential Nitro",
                },
                SupplyCluster {
                    name: "Frankfurt EuroNode Alpha",
                    source: "Supplier (L3 Verified)",
                    hardware: "32x A100-SXM4 80GB",
                    region: "eu-central-fra",
                    utilization: "78.1%",
                    attestation: "Hardware TPM",
                },
                SupplyCluster {
                    name: "Tokyo Enterprise Cluster",
                    source: "Supplier (L4 Strategic)",
                    hardware: "48x H100 SXM5 80GB",
                    region: "ap-east-hkg",
                    utilization: "88.6%",
                    attestation: "Confidential Nitro",
                },
                SupplyCluster {
                    name: "Lambda Labs Elastic Burst",
                    source: "External Cloud Reservation",
                    hardware: "24x H100 PCIe",
                    region: "us-east-va",
                    utilization: "32.0%",
                    attestation: "Cloud Attested",
                },
            ],
        }
    }
}
