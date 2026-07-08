// JSON Schema-driven UI — serde_json::Value is the schema wire format; no typed alternative.
#![allow(clippy::disallowed_types)]

use crate::groups::GroupManager;
use crate::tokens::TokenManager;
use burncloud_client_shared::components::{FormMode, PageHeader, SchemaForm};
use burncloud_client_shared::i18n::{t, use_i18n, Language};
use burncloud_client_shared::utils::storage::Theme;
use burncloud_client_shared::use_theme;
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
    let theme_ctx = use_theme();
    let current_theme = theme_ctx.theme.read().clone();

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
            subtitle: Some(t(*lang, "settings.subtitle").to_string()),
        }

        div { class: "page-content",
            // Tab Navigation
            div { class: "tabs mb-bc-8",
                button {
                    class: if active_tab() == "general" { "tab active" } else { "tab" },
                    onclick: move |_| active_tab.set("general"),
                    {t(*lang, "settings.tab.general")}
                }

                button {
                    class: if active_tab() == "groups" { "tab active" } else { "tab" },
                    onclick: move |_| active_tab.set("groups"),
                    {t(*lang, "settings.tab.groups")}
                }
                button {
                    class: if active_tab() == "tokens" { "tab active" } else { "tab" },
                    onclick: move |_| active_tab.set("tokens"),
                    {t(*lang, "settings.tab.tokens")}
                }
            }

            if active_tab() == "general" {
                div { class: "settings-card flex flex-col gap-bc-8",
                        SchemaForm {
                            schema: settings_schema_val.clone(),
                            data: settings_data,
                            mode: FormMode::Create,
                            show_actions: false,
                            on_submit: handle_settings_change,
                        }

                        // Theme toggle
                        div { class: "settings-theme-row",
                            div {
                                div { class: "text-body font-medium", "{t(*lang, \"settings.general.theme_title\")}" }
                                div { class: "text-caption text-bc-text-secondary mt-bc-1", "{t(*lang, \"settings.general.theme_desc\")}" }
                            }
                            div { class: "flex flex-wrap gap-bc-2 shrink-0",
                                button {
                                    class: if current_theme == Theme::Light { "chip active" } else { "chip" },
                                    onclick: move |_| theme_ctx.set_theme(Theme::Light),
                                    "{t(*lang, \"settings.general.theme_light\")}"
                                }
                                button {
                                    class: if current_theme == Theme::Dark { "chip active" } else { "chip" },
                                    onclick: move |_| theme_ctx.set_theme(Theme::Dark),
                                    "{t(*lang, \"settings.general.theme_dark\")}"
                                }
                                button {
                                    class: if current_theme == Theme::System { "chip active" } else { "chip" },
                                    onclick: move |_| theme_ctx.set_theme(Theme::System),
                                    "{t(*lang, \"settings.general.theme_system\")}"
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
