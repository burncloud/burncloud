#[cfg(feature = "desktop")]
fn main() {
    burncloud_client::launch_gui_with_tray();
}

#[cfg(not(feature = "desktop"))]
fn main() {}
