use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct LogEntry {
    #[serde(default)]
    id: String,
    #[serde(default)]
    timestamp: String,
    #[serde(default = "default_level")]
    level: String,
    #[serde(default)]
    message: String,
}

fn default_level() -> String {
    "INFO".to_string()
}

#[component]
pub fn LogPage() -> Element {
    let mut search_query = use_signal(|| "".to_string());

    let logs_resource = use_resource(move || async move {
        let client = reqwest::Client::new();
        // Use relative URL for WASM/Web compatibility
        let res = client.get("/console/api/logs").send().await;

        match res {
            Ok(r) => {
                if let Ok(data) = r.json::<serde_json::Value>().await {
                    if let Some(arr) = data.get("data").and_then(|d| d.as_array()) {
                        let mut entries = vec![];
                        for item in arr {
                            if let Ok(entry) = serde_json::from_value::<LogEntry>(item.clone()) {
                                entries.push(entry);
                            }
                        }
                        return entries;
                    }
                }
            }
            Err(e) => {
                println!("Error fetching logs: {}", e);
            }
        }
        vec![]
    });

    let filtered_logs = use_memo(move || {
        let logs = logs_resource.read().clone().unwrap_or_default();
        let query = search_query.read();
        if query.is_empty() {
            return logs;
        }
        let q = query.to_lowercase();
        logs.into_iter()
            .filter(|log| {
                log.message.to_lowercase().contains(&q) || log.level.to_lowercase().contains(&q)
            })
            .collect::<Vec<_>>()
    });

    rsx! {
        div { class: "flex flex-col h-full gap-md p-lg",
            div { class: "flex justify-between items-center",
                h1 { class: "text-title font-bold text-primary", "Logs" }
                input {
                    class: "bc-input p-md rounded-lg w-64",
                    style: "max-width: 16rem;",
                    placeholder: "Search logs...",
                    value: "{search_query}",
                    oninput: move |evt| search_query.set(evt.value())
                }
            }

            div {
                class: "flex-1 overflow-auto bc-card-solid rounded-xl",
                id: "log-container",
                if let Some(logs) = logs_resource.read().as_ref() {
                    if logs.is_empty() {
                         div { class: "p-lg text-center text-secondary", "No logs found" }
                    } else {
                        table { class: "w-full text-left text-caption",
                            thead {
                                style: "background: var(--bc-bg-hover); position: sticky; top: 0;",
                                tr {
                                    th { class: "p-md font-medium text-secondary", "Time" }
                                    th { class: "p-md font-medium text-secondary", "Level" }
                                    th { class: "p-md font-medium text-secondary", "Message" }
                                }
                            }
                            tbody {
                                for log in filtered_logs.read().iter() {
                                    tr { class: "border-b transition-colors",
                                        style: "border-color: var(--bc-border);",
                                        td { class: "p-md text-secondary font-mono whitespace-nowrap", "{log.timestamp}" }
                                        td { class: "p-md",
                                            span {
                                                class: match log.level.as_str() {
                                                    "ERROR" => "bc-badge-danger px-sm py-xs rounded text-xxs font-bold",
                                                    "WARN" => "bc-badge-warning px-sm py-xs rounded text-xxs font-bold",
                                                    _ => "bc-badge-info px-sm py-xs rounded text-xxs font-bold"
                                                },
                                                "{log.level}"
                                            }
                                        }
                                        td { class: "p-md text-primary break-all font-mono", "{log.message}" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "p-lg text-center text-secondary", "Loading logs..." }
                }
            }
        }
    }
}
