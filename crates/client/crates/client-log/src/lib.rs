use burncloud_client_shared::components::SchemaTable;
use burncloud_client_shared::schema::log_schema;
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

    let schema = log_schema();

    let table_data: Vec<serde_json::Value> = filtered_logs
        .read()
        .iter()
        .filter_map(|log| serde_json::to_value(log).ok())
        .collect();

    let loading = logs_resource.read().is_none();

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
                SchemaTable {
                    schema: schema.clone(),
                    data: table_data,
                    loading: loading,
                    on_row_click: move |_| {},
                }
            }
        }
    }
}
