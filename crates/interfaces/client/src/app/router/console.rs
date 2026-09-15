use crate::{
    i18n::{strings, Locale},
    shared::{layout::BuyerShell, types::Role},
};
use dioxus::prelude::*;

#[component]
pub fn PlaceholderPage(title: String) -> Element {
    rsx! { PlaceholderPageWithRole { title, role: Role::Buyer } }
}

#[component]
pub fn PlaceholderPageWithRole(
    title: String,
    #[props(default = Role::Buyer)] role: Role,
) -> Element {
    let locale = use_context::<Signal<Locale>>();
    let copy = strings(locale());
    let _ = role;
    rsx! { BuyerShell { div { class: "placeholder-panel", span { class: "placeholder-icon" } h1 { {title} } p { {copy.placeholder} } } } }
}
