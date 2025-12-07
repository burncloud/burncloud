use dioxus::prelude::*;
use dioxus_router::components::Router;
use burncloud_client_shared::components::layout::CoreRoute;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<CoreRoute> {}
    }
}