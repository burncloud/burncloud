use super::state::AdminSettingsState;
use crate::{
    i18n::{strings, Locale},
    shared::{
        layout::BuyerShell,
        ui::{Icon, IconName},
    },
};
use dioxus::prelude::*;

#[component]
pub fn AdminSettings() -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let mut state = use_signal(AdminSettingsState::default);
    let snapshot = state.read().clone();
    let saved = snapshot.saved;

    rsx! {
        BuyerShell {
            div { class: "admin-settings-stack",
                section { class: "page-header admin-settings-header",
                    div { class: "page-heading-copy",
                        h1 { {copy.admin_settings_title} }
                        p { {copy.admin_settings_subtitle} }
                    }
                    div { class: "conclusion conclusion-healthy admin-settings-conclusion", role: "status",
                        Icon { name: IconName::CheckCircle, size: 16 }
                        span { class: "conclusion-text", {copy.admin_settings_conclusion} }
                    }
                }
                section { class: "panel admin-settings-card",
                    form {
                        class: "admin-settings-form",
                        onsubmit: move |event| {
                            event.prevent_default();
                            state.with_mut(AdminSettingsState::mark_saved);
                            let mut feedback_state = state;
                            spawn(async move {
                                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                                feedback_state.with_mut(AdminSettingsState::clear_saved);
                            });
                        },
                        div { class: "admin-settings-field",
                            label { r#for: "admin-settings-base-rev-share", {copy.admin_settings_base_rev_share} }
                            input {
                                id: "admin-settings-base-rev-share",
                                name: "base_rev_share",
                                r#type: "number",
                                value: snapshot.base_rev_share,
                                step: "0.5",
                                min: "50",
                                max: "95",
                                oninput: move |event| state.with_mut(|value| value.set_base_rev_share(event.value())),
                            }
                            span { class: "admin-settings-helper",
                                "Default share paid to L1/L2 verified GPU providers on metered token earnings."
                            }
                        }
                        div { class: "admin-settings-field",
                            label { r#for: "admin-settings-l4-rev-share", {copy.admin_settings_l4_rev_share} }
                            input {
                                id: "admin-settings-l4-rev-share",
                                name: "l4_rev_share",
                                r#type: "number",
                                value: snapshot.l4_rev_share,
                                step: "0.5",
                                min: "70",
                                max: "95",
                                oninput: move |event| state.with_mut(|value| value.set_l4_rev_share(event.value())),
                            }
                        }
                        div { class: "admin-settings-field",
                            label { r#for: "admin-settings-target-headroom", {copy.admin_settings_target_headroom} }
                            input {
                                id: "admin-settings-target-headroom",
                                name: "target_headroom",
                                r#type: "number",
                                value: snapshot.target_headroom,
                                step: "1.0",
                                min: "10",
                                max: "60",
                                oninput: move |event| state.with_mut(|value| value.set_target_headroom(event.value())),
                            }
                            span { class: "admin-settings-helper",
                                "Autopilot provisions additional standby compute when live peak demand exceeds this safety buffer."
                            }
                        }
                        div { class: "admin-settings-actions",
                            button {
                                r#type: "submit",
                                class: "button button-primary admin-settings-save",
                                aria_label: if saved { copy.admin_settings_saved } else { copy.admin_settings_save_policies },
                                if saved {
                                    span { class: "admin-settings-saved-icon", Icon { name: IconName::Check, size: 14 } }
                                    span { {copy.admin_settings_saved} }
                                } else {
                                    Icon { name: IconName::Save, size: 14 }
                                    span { {copy.admin_settings_save_policies} }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
