#[cfg(feature = "desktop")]
fn main() {
    burncloud_client::launch_gui_with_tray();
}

#[cfg(all(not(feature = "desktop"), feature = "web"))]
fn main() {
    burncloud_client::launch_web();
}

#[cfg(all(not(feature = "desktop"), not(feature = "web")))]
fn main() {
    panic!("Please enable either the 'desktop' or 'web' feature");
}
