// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use crate::groups::GroupManager;
use crate::tokens::TokenManager;
use burncloud_client_shared::components::{FormMode, PageHeader, SchemaForm};
use burncloud_client_shared::i18n::{t, use_i18n, Language};
use burncloud_client_shared::utils::storage::{ClientState, Theme};
use dioxus::prelude::*;

/// General settings schema
fn settings_schema(lang: Language) -> serde_json::Value {
    serde_json::json!({
        "entity_type": "settings",
        "label": "General Settings",
        "fields": [
            {
                "key": "language",
                "label": t(lang, "settings.general.language_label"),
                "type": "select",
                "required": true,
                "default": "zh",
                "options": [
                    {"value": "zh", "label": t(lang, "settings.general.language_zh")},
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
    let settings_data = use_signal(move || {
        serde_json::json!({
            "language": lang_val,
            "auto_start": "true",
            "auto_update": "true"
        })
    });

    let settings_schema_val = settings_schema(*lang);

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
        PageHeader {
            title: "{t(*lang, \"nav.settings\")}",
            subtitle: Some("Configure system preferences".to_string()),
        }

        div { class: "page-content",
            // Tab Navigation
            div { class: "tabs", style: "margin-bottom:24px",
                button {
                    class: if active_tab() == "general" { "tab active" } else { "tab" },
                    onclick: move |_| active_tab.set("general"),
                    "General"
                }

                button {
                    class: if active_tab() == "groups" { "tab active" } else { "tab" },
                    onclick: move |_| active_tab.set("groups"),
                    "Groups"
                }
                button {
                    class: if active_tab() == "tokens" { "tab active" } else { "tab" },
                    onclick: move |_| active_tab.set("tokens"),
                    "Tokens"
                }
            }

            if active_tab() == "general" {
                div { class: "card flat", style: "padding:24px; max-width:640px",
                    div { style: "display:flex; flex-direction:column; gap:24px",
                        SchemaForm {
                            schema: settings_schema_val.clone(),
                            data: settings_data,
                            mode: FormMode::Create,
                            show_actions: false,
                            on_submit: handle_settings_change,
                        }

                        // Theme toggle
                        div { style: "display:flex; justify-content:space-between; align-items:center; padding-top:16px; border-top:1px solid var(--bc-border)",
                            div {
                                div { style: "font-size:14px; font-weight:500", "{t(*lang, \"settings.general.theme_title\")}" }
                                div { style: "font-size:12px; color:var(--bc-text-secondary); margin-top:4px", "{t(*lang, \"settings.general.theme_desc\")}" }
                            }
                            div { style: "display:flex; gap:8px",
                                {
                                    let cs = ClientState::load();
                                    let ct = cs.theme.clone().unwrap_or_default();
                                    rsx! {
                                        button {
                                            class: if ct == Theme::Light { "tab active" } else { "tab" },
                                            onclick: move |_| {
                                                let mut s = ClientState::load();
                                                s.theme = Some(Theme::Light);
                                                s.save();
                                            },
                                            "{t(*lang, \"settings.general.theme_light\")}"
                                        }
                                        button {
                                            class: if ct == Theme::Dark { "tab active" } else { "tab" },
                                            onclick: move |_| {
                                                let mut s = ClientState::load();
                                                s.theme = Some(Theme::Dark);
                                                s.save();
                                            },
                                            "{t(*lang, \"settings.general.theme_dark\")}"
                                        }
                                        button {
                                            class: if ct == Theme::System { "tab active" } else { "tab" },
                                            onclick: move |_| {
                                                let mut s = ClientState::load();
                                                s.theme = Some(Theme::System);
                                                s.save();
                                            },
                                            "{t(*lang, \"settings.general.theme_system\")}"
                                        }
                                    }
                                }
                            }
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