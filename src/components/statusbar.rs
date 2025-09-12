use dioxus::prelude::*;
use crate::types::SystemStatus;

#[component]
pub fn StatusBar(system_status: Signal<SystemStatus>) -> Element {
    let status = system_status.read();
    
    rsx! {
        div { 
            class: "status-bar",
            div { class: "status-item", "服务状态: 运行中" }
            div { class: "status-separator", "|" }
            div { class: "status-item", "CPU: {status.cpu_usage:.1}%" }
            div { class: "status-separator", "|" }
            div { class: "status-item", "内存: {format_bytes(status.memory_used)}" }
            div { class: "status-separator", "|" }
            div { class: "status-item", "活跃模型: {status.active_models}" }
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const GB: u64 = 1_073_741_824;
    const MB: u64 = 1_048_576;
    
    if bytes >= GB {
        format!("{:.1}GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1}MB", bytes as f64 / MB as f64)
    } else {
        format!("{}B", bytes)
    }
}