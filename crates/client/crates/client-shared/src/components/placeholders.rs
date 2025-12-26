use dioxus::prelude::*;

#[component]
pub fn Dashboard() -> Element {
    rsx! {
        div { "Dashboard Placeholder" }
    }
}

#[component]
pub fn DeployConfig() -> Element {
    rsx! {
        div { "Deploy Placeholder" }
    }
}

#[component]
pub fn ServiceMonitor() -> Element {
    rsx! {
        div { "Monitor Placeholder" }
    }
}

#[component]
pub fn ApiManagement() -> Element {
    rsx! {
        div { "API Placeholder" }
    }
}

#[component]
pub fn SystemSettings() -> Element {
    rsx! {
        div { "Settings Placeholder" }
    }
}

#[component]
pub fn ChannelPage() -> Element {
    rsx! {
        div { "Channel Placeholder" }
    }
}

#[component]
pub fn UserPage() -> Element {
    rsx! {
        div { "User Placeholder" }
    }
}

#[component]
pub fn BillingPage() -> Element {
    rsx! {
        div { "Billing Placeholder" }
    }
}

#[component]
pub fn LogPage() -> Element {
    rsx! {
        div { "Logs Placeholder" }
    }
}

#[component]
pub fn ConnectPage() -> Element {
    rsx! {
        div { "Connect Placeholder" }
    }
}

#[component]
pub fn PlaygroundPage() -> Element {
    rsx! {
        div { "Playground Placeholder" }
    }
}

#[component]
pub fn NotFoundPage(segments: Vec<String>) -> Element {
    rsx! {
        div { "Not Found Placeholder" }
    }
}
