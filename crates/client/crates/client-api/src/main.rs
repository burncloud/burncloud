use burncloud_client_api::ApiManagement;
use dioxus::prelude::*;

fn main() {
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            dioxus::desktop::Config::new().with_window(
                dioxus::desktop::WindowBuilder::new()
                    .with_title("BurnCloud API管理")
                    .with_inner_size(dioxus::desktop::LogicalSize::new(1200.0, 800.0)),
            ),
        )
        .launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // style { {include_str!("../assets/styles.css")} }
        div { id: "app",
            ApiManagement {}
        }
    }
}
