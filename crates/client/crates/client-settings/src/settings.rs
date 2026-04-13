// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use crate::groups::GroupManager;
use crate::tokens::TokenManager;
use burncloud_client_shared::components::{FormMode, SchemaForm};
use burncloud_client_shared::i18n::{t, use_i18n, Language};
use burncloud_client_shared::schema;
use dioxus::prelude::*;

/// General settings schema
fn settings_schema() -> serde_json::Value {
    serde_json::json!({
        "entity_type": "settings",
        "label": "General Settings",
        "fields": [
            {
                "key": "language",
                "label": "Language / 语言",
                "type": "select",
                "required": true,
                "default": "zh",
                "options": [
                    {"value": "zh", "label": "中文"},
                    {"value": "en", "label": "English"}
                ]
            },
            {
                "key": "auto_start",
                "label": "Auto Start — Launch BurnCloud on login",
                "type": "toggle",
                "default": "true"
            },
            {
                "key": "auto_update",
                "label": "Updates — Check for updates automatically",
                "type": "toggle",
                "default": "true"
            }
        ],
        "table_columns": [],
        "form_sections": [
            {"title": "General Settings", "fields": ["language", "auto_start", "auto_update"]}
        ]
    })
}

#[component]
pub fn SystemSettings() -> Element {
    let mut active_tab = use_signal(|| "general");
    let mut i18n = use_i18n();
    let lang = i18n.language.read();

    // Settings form data
    let lang_val = match *lang {
        Language::Zh => "zh",
        Language::En => "en",
    };
    let mut settings_data = use_signal(move || serde_json::json!({
        "language": lang_val,
        "auto_start": "true",
        "auto_update": "true"
    }));

    let settings_schema_val = settings_schema();

    // Handle settings change (immediate apply)
    let handle_settings_change = move |value: serde_json::Value| {
        if let Some(lang_str) = value["language"].as_str() {
            let mut l = i18n.language.write();
            match lang_str {
                "en" => *l = Language::En,
                _ => *l = Language::Zh,
            }
        }
    };

    rsx! {
        div { class: "page-header",
            h1 { class: "text-large-title font-bold text-primary m-0",
                "{t(*lang, \"nav.settings\")}"
            }
            p { class: "text-secondary m-0 mt-sm",
                "Configure system preferences"
            }
        }

        div { class: "page-content",
            // Tab Navigation
            div { class: "flex gap-md mb-lg border-b",
                button {
                    class: if active_tab() == "general" { "btn btn-subtle font-bold border-b-2 rounded-none p-md p-sm" } else { "btn btn-subtle p-md p-sm" },
                    style: if active_tab() == "general" { "border-bottom-color: var(--bc-primary); color: var(--bc-primary);" } else { "" },
                    onclick: move |_| active_tab.set("general"),
                    "General"
                }

                button {
                    class: if active_tab() == "groups" { "btn btn-subtle font-bold border-b-2 rounded-none p-md p-sm" } else { "btn btn-subtle p-md p-sm" },
                    style: if active_tab() == "groups" { "border-bottom-color: var(--bc-primary); color: var(--bc-primary);" } else { "" },
                    onclick: move |_| active_tab.set("groups"),
                    "Groups"
                }
                button {
                    class: if active_tab() == "tokens" { "btn btn-subtle font-bold border-b-2 rounded-none p-md p-sm" } else { "btn btn-subtle p-md p-sm" },
                    style: if active_tab() == "tokens" { "border-bottom-color: var(--bc-primary); color: var(--bc-primary);" } else { "" },
                    onclick: move |_| active_tab.set("tokens"),
                    "Tokens"
                }
            }

            if active_tab() == "general" {
                div { class: "bc-card-solid",
                    div { class: "p-lg",
                        SchemaForm {
                            schema: settings_schema_val.clone(),
                            data: settings_data,
                            mode: FormMode::Create,
                            show_actions: false,
                            on_submit: handle_settings_change,
                        }
                    }
                }

            } else if active_tab() == "groups" {
                GroupManager {}
            } else if active_tab() == "tokens" {
                TokenManager {}
            }
        }
    }
}
