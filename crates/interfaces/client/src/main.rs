#[cfg(feature = "desktop")]
fn main() {
    burncloud_client::launch_gui_with_tray();
}

#[cfg(all(not(feature = "desktop"), feature = "web"))]
fn main() {
    dioxus::launch(burncloud_client::app::App);
}

#[cfg(not(any(feature = "desktop", feature = "web")))]
fn main() {}
