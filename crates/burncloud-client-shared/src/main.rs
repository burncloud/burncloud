use dioxus::prelude::*;
use dioxus_router::prelude::*;
use burncloud_client_shared::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<CoreRoute> {}
    }
}