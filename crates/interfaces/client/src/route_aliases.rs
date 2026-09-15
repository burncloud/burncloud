use dioxus::prelude::*;

use crate::app::Route;

#[component]
pub fn Dashboard() -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        navigator.replace(Route::Overview {});
    });
    rsx! {}
}

#[component]
pub fn Users() -> Element {
    let navigator = use_navigator();
    use_effect(move || {
        navigator.replace(Route::Customers {});
    });
    rsx! {}
}
