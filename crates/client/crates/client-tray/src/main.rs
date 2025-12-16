#[cfg(target_os = "windows")]
use burncloud_client_tray::start_tray;

#[cfg(target_os = "windows")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    start_tray()
}

#[cfg(not(target_os = "windows"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!("System tray is only supported on Windows");
    Ok(())
}
