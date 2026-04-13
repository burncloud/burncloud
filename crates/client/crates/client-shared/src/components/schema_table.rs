use dioxus::prelude::*;

use crate::components::{BCBadge, BCTable, BadgeVariant};

/// 渲染单个单元格
fn render_cell(
    col: &serde_json::Value,
    row: &serde_json::Value,
) -> Element {
    let key = col["key"].as_str().unwrap_or("");
    let render_type = col["render"].as_str().unwrap_or("text");
    let val = row.get(key);

    match render_type {
        "status_badge" => {
            let active_value = col["active_value"].as_str().unwrap_or("1");
            let active_label = col["active_label"].as_str().unwrap_or("Active");
            let inactive_label = col["inactive_label"].as_str().unwrap_or("Inactive");
            let val_str = value_to_string(val);
            let is_active = val_str == active_value;

            let (variant, label) = if is_active {
                (BadgeVariant::Success, active_label)
            } else {
                (BadgeVariant::Neutral, inactive_label)
            };

            rsx! {
                BCBadge { variant: variant, dot: true, "{label}" }
            }
        }
        "monospace" => {
            let text = value_to_string(val);
            rsx! {
                span {
                    style: "font-family: 'Cascadia Code', 'Fira Code', 'Monaco', 'Consolas', monospace; font-size: var(--bc-font-sm);",
                    "{text}"
                }
            }
        }
        "tags" => {
            let separator = col["separator"].as_str().unwrap_or(",");
            let text = value_to_string(val);
            let tags: Vec<&str> = if text.is_empty() {
                vec![]
            } else {
                text.split(separator).collect()
            };
            rsx! {
                div { class: "flex flex-wrap gap-xs",
                    for tag in tags {
                        span { class: "bc-badge-neutral", style: "padding: var(--bc-space-1) var(--bc-space-2); border-radius: var(--bc-radius-full); font-size: var(--bc-font-xs);",
                            "{tag}"
                        }
                    }
                }
            }
        }
        "money" => {
            let text = value_to_string(val);
            rsx! {
                span { class: "font-medium", "{text}" }
            }
        }
        "datetime" => {
            let text = value_to_string(val);
            rsx! {
                span { class: "text-secondary", "{text}" }
            }
        }
        _ => {
            let text = value_to_string(val);
            rsx! {
                span { "{text}" }
            }
        }
    }
}

fn value_to_string(val: Option<&serde_json::Value>) -> String {
    match val {
        Some(serde_json::Value::String(s)) => s.clone(),
        Some(serde_json::Value::Number(n)) => n.to_string(),
        Some(serde_json::Value::Bool(b)) => b.to_string(),
        _ => String::new(),
    }
}

/// JSON Schema 驱动的通用表格组件
///
/// 接收 JSON Schema 定义和数据列表，自动渲染表格列。
/// 支持 text/status_badge/monospace/tags/money/datetime 渲染器。
/// 操作按钮通过 `actions` + `on_action` 回调实现。
#[component]
pub fn SchemaTable(
    schema: serde_json::Value,
    data: Vec<serde_json::Value>,
    #[props(default)] loading: bool,
    #[props(default)] class: String,
    #[props(default)] pagination: Option<Element>,
    #[props(default)] on_row_click: EventHandler<serde_json::Value>,
    #[props(default)] on_action: EventHandler<ActionEvent>,
    #[props(default)] actions: Vec<ActionDef>,
) -> Element {
    let columns = schema["table_columns"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let has_actions = !actions.is_empty();

    rsx! {
        BCTable {
            class: class,
            pagination: pagination,
            table {
                thead {
                    tr {
                        for col in &columns {
                            {
                                let label = col["label"].as_str().unwrap_or("");
                                rsx! {
                                    th { class: "text-left text-caption font-semibold text-secondary px-md py-sm",
                                        "{label}"
                                    }
                                }
                            }
                        }
                        if has_actions {
                            th { class: "text-right text-caption font-semibold text-secondary px-md py-sm",
                                "操作"
                            }
                        }
                    }
                }
                tbody {
                    if loading {
                        tr {
                            td {
                                colspan: "{columns.len() + usize::from(has_actions)}",
                                class: "text-center py-lg text-secondary",
                                "加载中..."
                            }
                        }
                    } else if data.is_empty() {
                        tr {
                            td {
                                colspan: "{columns.len() + usize::from(has_actions)}",
                                class: "text-center py-lg text-secondary",
                                "暂无数据"
                            }
                        }
                    } else {
                        for row in data.iter() {
                            {
                                let row_clone = row.clone();
                                let cols = columns.clone();
                                let acts = actions.clone();
                                let has_acts = has_actions;
                                rsx! {
                                    tr {
                                        class: "border-t border-[var(--bc-border)] hover:bg-[var(--bc-bg-hover)] cursor-pointer",
                                        onclick: move |_| on_row_click.call(row_clone.clone()),
                                        for col in cols.iter() {
                                            {render_cell(col, row)}
                                        }
                                        if has_acts {
                                            td { class: "text-right px-md py-sm",
                                                div { class: "flex gap-sm justify-end",
                                                    for act in acts.iter() {
                                                        {
                                                            let act_id = act.action_id.clone();
                                                            let act_label = act.label.clone();
                                                            let act_color = act.color.clone();
                                                            let row_data = row.clone();
                                                            rsx! {
                                                                button {
                                                                    class: "btn btn-subtle text-caption",
                                                                    style: "min-height: auto; padding: var(--bc-space-1) var(--bc-space-2); color: {act_color};",
                                                                    onclick: move |e: MouseEvent| {
                                                                        e.stop_propagation();
                                                                        on_action.call(ActionEvent {
                                                                            action_id: act_id.clone(),
                                                                            row: row_data.clone(),
                                                                        });
                                                                    },
                                                                    "{act_label}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// 操作按钮定义
#[derive(Clone, PartialEq, Debug)]
pub struct ActionDef {
    pub action_id: String,
    pub label: String,
    pub color: String,
}

/// 操作事件
#[derive(Clone, Debug, PartialEq)]
pub struct ActionEvent {
    pub action_id: String,
    pub row: serde_json::Value,
}
