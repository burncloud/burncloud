use super::console::{PlaceholderPage, PlaceholderPageWithRole};
use crate::domains::buyer::{
    api_keys::BuyerApiKeys, billing::BuyerBilling, marketplace::BuyerMarketplace,
    overview::BuyerOverview, playground::BuyerPlayground, usage::BuyerUsage,
};
use crate::shared::types::Role;
use dioxus::prelude::*;

#[derive(Clone, Routable, Debug, PartialEq)]
pub enum Route {
    #[route("/")]
    Home {},
    #[route("/home")]
    PublicHome {},
    #[route("/landing")]
    Landing {},
    #[route("/login")]
    Login {},
    #[route("/register")]
    Register {},
    #[route("/buyer")]
    Buyer {},
    #[route("/buyer/overview")]
    BuyerOverviewRoute {},
    #[route("/console")]
    Console {},
    #[route("/console/dashboard")]
    ConsoleDashboard {},
    #[route("/console/buyer/overview")]
    ConsoleBuyerOverview {},
    #[route("/buyer/playground")]
    Playground {},
    #[route("/buyer/marketplace")]
    Marketplace {},
    #[route("/buyer/billing")]
    Billing {},
    #[route("/buyer/logs")]
    Logs {},
    #[route("/buyer/api-keys")]
    ApiKeys {},
    #[route("/buyer/usage")]
    Usage {},
    #[route("/playground")]
    PublicPlayground {},
    #[route("/marketplace")]
    PublicMarketplace {},
    #[route("/models")]
    Models {},
    #[route("/keys")]
    Keys {},
    #[route("/usage")]
    PublicUsage {},
    #[route("/billing")]
    PublicBilling {},
    #[route("/logs")]
    PublicLogs {},
    #[route("/supplier")]
    Supplier {},
    #[route("/supplier/overview")]
    SupplierOverview {},
    #[route("/supplier/resources")]
    SupplierResources {},
    #[route("/supplier/deployments")]
    SupplierDeployments {},
    #[route("/supplier/earnings")]
    SupplierEarnings {},
    #[route("/supplier/settlements")]
    SupplierSettlements {},
    #[route("/supplier/reliability")]
    SupplierReliability {},
    #[route("/supplier/settings")]
    SupplierSettings {},
    #[route("/admin")]
    Admin {},
    #[route("/admin/overview")]
    AdminOverview {},
    #[route("/admin/supply")]
    AdminSupply {},
    #[route("/admin/capacity")]
    AdminCapacity {},
    #[route("/admin/demand")]
    AdminDemand {},
    #[route("/admin/models")]
    AdminModels {},
    #[route("/admin/revenue")]
    AdminRevenue {},
    #[route("/admin/settlements")]
    AdminSettlements {},
    #[route("/admin/suppliers")]
    AdminSuppliers {},
    #[route("/admin/customers")]
    AdminCustomers {},
    #[route("/admin/operations")]
    AdminOperations {},
    #[route("/admin/settings")]
    AdminSettings {},
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

#[component]
pub fn Home() -> Element {
    rsx! { BuyerOverview {} }
}
#[component]
pub fn PublicHome() -> Element {
    rsx! { PlaceholderPage { title: "Public Portal".to_string() } }
}
#[component]
pub fn Landing() -> Element {
    rsx! { PlaceholderPage { title: "Landing".to_string() } }
}
#[component]
pub fn Login() -> Element {
    rsx! { PlaceholderPage { title: "Login".to_string() } }
}
#[component]
pub fn Register() -> Element {
    rsx! { PlaceholderPage { title: "Register".to_string() } }
}
#[component]
pub fn Buyer() -> Element {
    rsx! { BuyerOverview {} }
}
#[component]
pub fn BuyerOverviewRoute() -> Element {
    rsx! { BuyerOverview {} }
}
#[component]
pub fn Console() -> Element {
    rsx! { BuyerOverview {} }
}
#[component]
pub fn ConsoleDashboard() -> Element {
    rsx! { BuyerOverview {} }
}
#[component]
pub fn ConsoleBuyerOverview() -> Element {
    rsx! { BuyerOverview {} }
}
#[component]
pub fn Playground() -> Element {
    rsx! { BuyerPlayground {} }
}
#[component]
pub fn Marketplace() -> Element {
    rsx! { BuyerMarketplace {} }
}
#[component]
pub fn Billing() -> Element {
    rsx! { BuyerBilling {} }
}
#[component]
pub fn Logs() -> Element {
    rsx! { PlaceholderPage { title: "Logs".to_string() } }
}
#[component]
pub fn ApiKeys() -> Element {
    rsx! { BuyerApiKeys {} }
}
#[component]
pub fn Usage() -> Element {
    rsx! { BuyerUsage {} }
}
#[component]
pub fn PublicPlayground() -> Element {
    rsx! { PlaceholderPage { title: "Playground".to_string() } }
}
#[component]
pub fn PublicMarketplace() -> Element {
    rsx! { PlaceholderPage { title: "Marketplace".to_string() } }
}
#[component]
pub fn Models() -> Element {
    rsx! { PlaceholderPage { title: "Models".to_string() } }
}
#[component]
pub fn Keys() -> Element {
    rsx! { PlaceholderPage { title: "Keys".to_string() } }
}
#[component]
pub fn PublicUsage() -> Element {
    rsx! { PlaceholderPage { title: "Usage".to_string() } }
}
#[component]
pub fn PublicBilling() -> Element {
    rsx! { PlaceholderPage { title: "Billing".to_string() } }
}
#[component]
pub fn PublicLogs() -> Element {
    rsx! { PlaceholderPage { title: "Logs".to_string() } }
}
#[component]
pub fn Supplier() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Supplier Overview".to_string(), role: Role::Supplier } }
}
#[component]
pub fn SupplierOverview() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Supplier Overview".to_string(), role: Role::Supplier } }
}
#[component]
pub fn SupplierResources() -> Element {
    rsx! { PlaceholderPageWithRole { title: "GPU Resources".to_string(), role: Role::Supplier } }
}
#[component]
pub fn SupplierDeployments() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Autopilot Deployments".to_string(), role: Role::Supplier } }
}
#[component]
pub fn SupplierEarnings() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Revenue & Payouts".to_string(), role: Role::Supplier } }
}
#[component]
pub fn SupplierSettlements() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Settlement Batches".to_string(), role: Role::Supplier } }
}
#[component]
pub fn SupplierReliability() -> Element {
    rsx! { PlaceholderPageWithRole { title: "SLA & Reliability".to_string(), role: Role::Supplier } }
}
#[component]
pub fn SupplierSettings() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Settings".to_string(), role: Role::Supplier } }
}
#[component]
pub fn Admin() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Admin Overview".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminOverview() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Admin Overview".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminSupply() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Supply Fleet".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminCapacity() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Capacity & Autoscale".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminDemand() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Token Demand".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminModels() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Model Catalog".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminRevenue() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Platform Revenue".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminSettlements() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Settlement Batches".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminSuppliers() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Supplier Accounts".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminCustomers() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Enterprise Customers".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminOperations() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Emergency Controls".to_string(), role: Role::Admin } }
}
#[component]
pub fn AdminSettings() -> Element {
    rsx! { PlaceholderPageWithRole { title: "Settings".to_string(), role: Role::Admin } }
}
#[component]
pub fn NotFound(segments: Vec<String>) -> Element {
    rsx! { PlaceholderPage { title: format!("Not found: /{}", segments.join("/")) } }
}
