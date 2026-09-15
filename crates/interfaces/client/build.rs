#[cfg(target_os = "windows")]
fn embed_windows_icon() {
    let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rc_file = manifest_dir.join("assets/burncloud.rc");
    let ico_file = manifest_dir.join("assets/favicon.ico");

    println!("cargo:rerun-if-changed=assets/burncloud.rc");
    println!("cargo:rerun-if-changed=assets/favicon.ico");

    let mut res = winres::WindowsResource::new();
    res.set_icon_with_id(&ico_file.display().to_string(), "1");
    res.set_resource_file(&rc_file.display().to_string());
    res.compile().expect("Failed to compile Windows resources");
}

fn main() {
    #[cfg(target_os = "windows")]
    embed_windows_icon();
}
