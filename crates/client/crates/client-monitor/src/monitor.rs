// Threat data uses serde_json::Value for mock demo; no typed alternative.
#![allow(clippy::disallowed_types)]

use burncloud_client_shared::components::{
    PageHeader, StatKpi, Sparkline, StatusPill, ColumnDef, PageTable,
    SkeletonCard, SkeletonVariant,
};
use burncloud_client_shared::services::monitor_service::MonitorService;
use dioxus::prelude::*;
use serde_json::json;

#[component]
pub fn ServiceMonitor() -> Element {
    let metrics = use_resource(move || async move {
        MonitorService::get_system_metrics().await.ok()
    });

    let m = metrics.read().clone();
    let loading = m.is_none();
    let m_flat = m.clone().flatten();

    let (cpu_pct, mem_pct, mem_used_gb, mem_total_gb, disk_count) = match m_flat {
        Some(data) => {
            let cpu = data.cpu.usage_percent as f64;
            let mem_pct = data.memory.usage_percent as f64;
            let mem_used = data.memory.used as f64 / 1_073_741_824.0;
            let mem_total = data.memory.total as f64 / 1_073_741_824.0;
            let disks = data.disks.len();
            (cpu, mem_pct, mem_used, mem_total, disks)
        }
        None => (0.0, 0.0, 0.0, 0.0, 0),
    };

    // Mock threat data for demo
    let threats = vec![
        json!({"id": "T-001", "type": "Rate Limit", "severity": "high", "source": "192.168.1.100", "time": "2 min ago", "status": "active"}),
        json!({"id": "T-002", "type": "Suspicious Pattern", "severity": "medium", "source": "10.0.0.55", "time": "15 min ago", "status": "investigating"}),
        json!({"id": "T-003", "type": "Auth Failure", "severity": "low", "source": "172.16.0.10", "time": "1 hr ago", "status": "resolved"}),
    ];

    let threat_columns = vec![
        ColumnDef { key: "id".to_string(), label: "ID".to_string(), width: Some("80px".to_string()) },
        ColumnDef { key: "type".to_string(), label: "Type".to_string(), width: None },
        ColumnDef { key: "severity".to_string(), label: "Severity".to_string(), width: Some("100px".to_string()) },
        ColumnDef { key: "source".to_string(), label: "Source".to_string(), width: Some("140px".to_string()) },
        ColumnDef { key: "time".to_string(), label: "Time".to_string(), width: Some("120px".to_string()) },
        ColumnDef { key: "status".to_string(), label: "Status".to_string(), width: Some("120px".to_string()) },
    ];

    let cpu_data = vec![45.0, 52.0, 48.0, 61.0, 55.0, 43.0, 50.0];
    let mem_data = vec![62.0, 65.0, 68.0, 64.0, 70.0, 66.0, 63.0];

    rsx! {
        PageHeader {
            title: "风控雷达",
            subtitle: Some("实时威胁检测与内容安全防御".to_string()),
        }

        div { class: "page-content",
            // System metrics HUD
            div { class: "stats-grid cols-4",
                if loading {
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                    SkeletonCard { variant: Some(SkeletonVariant::Kpi) }
                } else {
                    StatKpi {
                        label: "CPU 使用率",
                        value: "{cpu_pct:.0}%",
                        chart: rsx! { Sparkline { data: cpu_data.clone(), tone: None } }
                    }
                    StatKpi {
                        label: "内存使用率",
                        value: "{mem_pct:.0}%",
                        chart: rsx! { Sparkline { data: mem_data.clone(), tone: Some("danger".to_string()) } }
                    }
                    StatKpi {
                        label: "内存用量",
                        value: "{mem_used_gb:.1}/{mem_total_gb:.1} GB",
                    }
                    StatKpi {
                        label: "磁盘挂载",
                        value: "{disk_count}",
                    }
                }
            }

            // Threat table
            div { class: "section-h lg",
                div { class: "lead",
                    span { class: "lead-title", "威胁事件" }
                    span { class: "lead-sub", "最近 24 小时" }
                }
            }

            PageTable {
                columns: threat_columns,
                for threat in &threats {
                    tr {
                        key: "{threat[\"id\"]}",
                        td { class: "mono", "{threat[\"id\"]}" }
                        td { "{threat[\"type\"]}" }
                        td {
                            {
                                let sev = threat.get("severity").and_then(|s: &serde_json::Value| s.as_str()).unwrap_or("low");
                                let class = match sev {
                                    "high" => "sev-high",
                                    "medium" => "sev-medium",
                                    _ => "sev-low",
                                };
                                rsx! { span { class: "pill {class}", style: "font-size:11px; text-transform:uppercase; letter-spacing:0.08em", "{sev}" } }
                            }
                        }
                        td { class: "mono", style: "font-size:13px", "{threat[\"source\"]}" }
                        td { style: "color:var(--bc-text-secondary); font-size:13px", "{threat[\"time\"]}" }
                        td {
                            StatusPill {
                                value: threat.get("status")
                                    .and_then(|s: &serde_json::Value| s.as_str())
                                    .unwrap_or("unknown")
                                    .to_string()
                            }
                        }
                    }
                }
            }
        }
    }
}
