use burncloud_client_shared::components::layout::CoreRoute;
use dioxus::prelude::*;
use dioxus_router::components::Router;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        Router::<CoreRoute> {}
    }
}
