use dioxus::prelude::*;
use crate::channels::ChannelManager;
use crate::tokens::TokenManager;
use crate::groups::GroupManager;
use burncloud_client_shared::i18n::{use_i18n, t, Language};

#[component]
pub fn SystemSettings() -> Element {
    let mut active_tab = use_signal(|| "general");
    let i18n = use_i18n();
    let lang = i18n.language.read();

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
            div { class: "flex gap-md mb-lg border-b border-border",
                button { 
                    class: if active_tab() == "general" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("general"),
                    "General"
                }
                button { 
                    class: if active_tab() == "channels" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("channels"),
                    "{t(*lang, \"nav.channels\")}"
                }
                button { 
                    class: if active_tab() == "groups" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("groups"),
                    "Groups"
                }
                button { 
                    class: if active_tab() == "tokens" { "btn btn-text text-primary font-bold border-b-2 border-primary rounded-none px-md py-sm" } else { "btn btn-text text-secondary px-md py-sm" },
                    onclick: move |_| active_tab.set("tokens"),
                    "Tokens"
                }
            }

            if active_tab() == "general" {
                div { class: "card",
                    div { class: "p-lg",
                        h3 { class: "text-subtitle font-semibold mb-md", "General Settings" }
                        div { class: "flex flex-col gap-md",
                            // Language Switcher
                            div { class: "flex justify-between items-center",
                                div {
                                    div { class: "font-medium", "Language / 语言" }
                                    div { class: "text-caption text-secondary", "Display language" }
                                }
                                select {
                                    class: "input input-sm w-32",
                                    value: match *lang { Language::Zh => "zh", Language::En => "en" },
                                    onchange: move |evt| {
                                        let mut l = i18n.language.write();
                                        match evt.value().as_str() {
                                            "en" => *l = Language::En,
                                            _ => *l = Language::Zh,
                                        }
                                    },
                                    option { value: "zh", "中文" }
                                    option { value: "en", "English" }
                                }
                            }

                            div { class: "flex justify-between items-center",
                                div {
                                    div { class: "font-medium", "Auto Start" }
                                    div { class: "text-caption text-secondary", "Launch BurnCloud on login" }
                                }
                                input {
                                    r#type: "checkbox",
                                    checked: true
                                }
                            }
                            div { class: "flex justify-between items-center",
                                div {
                                    div { class: "font-medium", "Updates" }
                                    div { class: "text-caption text-secondary", "Check for updates automatically" }
                                }
                                input {
                                    r#type: "checkbox",
                                    checked: true
                                }
                            }
                        }
                    }
                }
            } else if active_tab() == "channels" {
                ChannelManager {}
            } else if active_tab() == "groups" {
                GroupManager {}
            } else if active_tab() == "tokens" {
                TokenManager {}
            }
        }
    }
}
