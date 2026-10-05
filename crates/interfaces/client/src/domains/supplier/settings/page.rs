//! Supplier node and environment settings page.

use super::{
    actions::{copy_to_clipboard, load_saved_settings, persist_settings},
    state::SupplierSettingsState,
};
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

fn form_text(values: &[(String, dioxus::events::FormValue)], name: &str) -> Option<String> {
    values.iter().find_map(|(key, value)| {
        if key == name {
            match value {
                dioxus::events::FormValue::Text(value) => Some(value.clone()),
                dioxus::events::FormValue::File(_) => None,
            }
        } else {
            None
        }
    })
}

#[component]
pub fn SupplierSettings() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(SupplierSettingsState::default);

    use_effect(move || {
        let mut state = state;
        spawn(async move {
            if let Some(saved) = load_saved_settings().await {
                state.with_mut(|value| value.apply_saved(saved));
            }
        });
    });

    let snapshot = state.read().clone();
    let copied = snapshot.command_copied;
    let saved = snapshot.saved;

    rsx! {
        BuyerShell {
            div { class: "supplier-settings-stack",
                section { class: "page-header supplier-settings-page-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.supplier_settings_title} }
                        p { {copy.supplier_settings_subtitle} }
                    }
                }
                div { class: "conclusion conclusion-healthy supplier-settings-conclusion", role: "status",
                    Icon { name: IconName::CheckCircle, size: 16 }
                    span { class: "conclusion-text", {copy.supplier_settings_conclusion} }
                }
                div { class: "supplier-settings-grid",
                    section { class: "panel supplier-settings-card",
                        Icon { name: IconName::Key, size: 18 }
                        div { class: "supplier-settings-token-row",
                            code { class: "supplier-settings-token", {snapshot.model.node_token} }
                            button {
                                r#type: "button",
                                class: if copied { "supplier-settings-copy copied" } else { "supplier-settings-copy" },
                                title: if copied { copy.supplier_copied } else { copy.supplier_settings_copy },
                                aria_label: if copied { copy.supplier_copied } else { copy.supplier_settings_copy },
                                onclick: move |_| {
                                    copy_to_clipboard(snapshot.model.install_command);
                                    state.with_mut(SupplierSettingsState::mark_command_copied);
                                    let mut copied_state = state;
                                    spawn(async move {
                                        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                        copied_state.with_mut(SupplierSettingsState::clear_command_copied);
                                    });
                                },
                                Icon { name: if copied { IconName::Check } else { IconName::Copy }, size: 13 }
                                span { if copied { {copy.supplier_copied} } else { {copy.supplier_settings_copy} } }
                            }
                        }
                        code { class: "supplier-settings-command",
                            {snapshot.model.install_command}
                        }
                    }
                    section { class: "panel supplier-settings-card",
                        Icon { name: IconName::Bell, size: 18 }
                        form {
                            class: "supplier-settings-form",
                            action: "javascript:void(0)",
                            onsubmit: move |event| {
                                event.prevent_default();
                                let current = state.read().clone();
                                let values = event.values();
                                let email = form_text(&values, "alert_email")
                                    .filter(|value| !value.is_empty())
                                    .unwrap_or_else(|| current.effective_alert_email().to_string());
                                let webhook = form_text(&values, "webhook_url")
                                    .filter(|value| !value.is_empty())
                                    .unwrap_or_else(|| current.webhook_url.clone());
                                persist_settings(&email, &webhook);
                                state.with_mut(|value| {
                                    value.alert_email = email;
                                    value.webhook_url = webhook;
                                    value.mark_saved();
                                });
                                let mut saved_state = state;
                                spawn(async move {
                                    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
                                    saved_state.with_mut(SupplierSettingsState::clear_saved);
                                });
                            },
                            div { class: "supplier-settings-field",
                                label { class: "supplier-settings-visually-hidden", r#for: "supplier-settings-email",
                                    {copy.supplier_settings_alert_email}
                                }
                                input {
                                    id: "supplier-settings-email",
                                    name: "alert_email",
                                    class: "supplier-settings-input",
                                    r#type: "email",
                                    required: true,
                                    value: snapshot.effective_alert_email().to_string(),
                                    oninput: move |event| {
                                        state.with_mut(|value| value.set_alert_email(event.value()));
                                    },
                                }
                            }
                            div { class: "supplier-settings-field",
                                label { r#for: "supplier-settings-webhook", {copy.supplier_settings_webhook} }
                                input {
                                    id: "supplier-settings-webhook",
                                    name: "webhook_url",
                                    class: "supplier-settings-input",
                                    r#type: "url",
                                    required: true,
                                    placeholder: "https://...",
                                    initial_value: snapshot.webhook_url.clone(),
                                }
                                p { class: "supplier-settings-helper",
                                    {copy.supplier_settings_webhook_description}
                                }
                            }
                            button {
                                r#type: "submit",
                                class: if saved { "supplier-settings-save saved" } else { "supplier-settings-save" },
                                title: if saved { copy.supplier_settings_saved } else { copy.supplier_settings_save },
                                aria_label: if saved { copy.supplier_settings_saved } else { copy.supplier_settings_save },
                                Icon { name: if saved { IconName::Check } else { IconName::Save }, size: 15 }
                            }
                        }
                    }
                }
            }
        }
    }
}
