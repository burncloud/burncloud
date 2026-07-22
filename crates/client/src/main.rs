#[cfg(feature = "desktop")]
fn main() {
    burncloud_client::launch_gui_with_tray();
}

// `--all-features` enables both platforms; use desktop as the native default.
#[cfg(all(feature = "web", not(feature = "desktop")))]
fn main() {
    burncloud_client::launch_web();
}

#[cfg(all(not(feature = "desktop"), not(feature = "web")))]
fn main() {
    panic!("Please enable either 'desktop' or 'web' feature");
}
