//! Build script for BurnCloud client
//!
//! On Windows, this embeds the application icon into the executable
//! so it displays correctly in the taskbar and file explorer.
//!
//! IMPORTANT: Build scripts are compiled for the HOST machine, not the TARGET.
//! We must check the TARGET environment variable to detect Windows builds.

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();

    // Check if we're building for Windows (regardless of host OS)
    if target.contains("windows") {
        // Tell Cargo to re-run this script if the icon changes
        println!("cargo:rerun-if-changed=assets/favicon.ico");

        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/favicon.ico")
            .set("ProductName", "BurnCloud")
            .set("FileDescription", "BurnCloud - AI Local Deployment Platform")
            .set("LegalCopyright", "Copyright © 2024 BurnCloud");

        if let Err(e) = res.compile() {
            eprintln!("cargo:warning=Failed to embed Windows resources: {}", e);
            // Don't fail the build, just warn - icon embedding is not critical
        } else {
            println!("cargo:warning=Windows resources embedded successfully");
        }
    }
}
