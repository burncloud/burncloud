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
        div { class: "flex flex-col h-full gap-4 p-6",
            div { class: "flex justify-between items-center",
                h1 { class: "text-2xl font-bold text-gray-800", "Logs" }
                input {
                    class: "border border-gray-300 p-2 rounded-lg w-64 focus:outline-none focus:ring-2 focus:ring-blue-500",
                    placeholder: "Search logs...",
                    value: "{search_query}",
                    oninput: move |evt| search_query.set(evt.value())
                }
            }

            div {
                class: "flex-1 overflow-auto border border-gray-200 rounded-xl bg-white shadow-sm",
                id: "log-container",
                if let Some(logs) = logs_resource.read().as_ref() {
                    if logs.is_empty() {
                         div { class: "p-4 text-center text-gray-500", "No logs found" }
                    } else {
                        table { class: "w-full text-left text-sm",
                            thead { class: "bg-gray-50 sticky top-0",
                                tr {
                                    th { class: "p-3 font-medium text-gray-500", "Time" }
                                    th { class: "p-3 font-medium text-gray-500", "Level" }
                                    th { class: "p-3 font-medium text-gray-500", "Message" }
                                }
                            }
                            tbody {
                                for log in filtered_logs.read().iter() {
                                    tr {
                                        class: "border-b border-gray-100 hover:bg-gray-50 log-entry",
                                        td { class: "p-3 text-gray-600 font-mono whitespace-nowrap", "{log.timestamp}" }
                                        td { class: "p-3",
                                            span {
                                                class: match log.level.as_str() {
                                                    "ERROR" => "px-2 py-1 rounded bg-red-100 text-red-700 text-xs font-bold",
                                                    "WARN" => "px-2 py-1 rounded bg-yellow-100 text-yellow-700 text-xs font-bold",
                                                    _ => "px-2 py-1 rounded bg-blue-100 text-blue-700 text-xs font-bold"
                                                },
                                                "{log.level}"
                                            }
                                        }
                                        td { class: "p-3 text-gray-800 break-all font-mono", "{log.message}" }
                                    }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "p-4 text-center text-gray-500", "Loading logs..." }
                }
            }
        }
    }
}
