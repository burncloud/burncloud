use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = manifest_dir.join("tailwind.config.js");
    let input = manifest_dir.join("input.css");
    let output = manifest_dir.join("src/assets/tailwind.css");
    let cli = manifest_dir.join("tailwindcss");

    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=input.css");
    println!("cargo:rerun-if-changed=tailwindcss");

    if !cli.exists() {
        println!(
            "cargo:warning=tailwindcss CLI not found at {} — skipping CSS rebuild. \
             Download the standalone v3.4 binary from \
             https://github.com/tailwindlabs/tailwindcss/releases and save it as \
             crates/client/tailwindcss (chmod +x).",
            cli.display()
        );
        return;
    }

    let status = Command::new(&cli)
        .arg("-c").arg(&config)
        .arg("-i").arg(&input)
        .arg("-o").arg(&output)
        .arg("--minify")
        .current_dir(&manifest_dir)
        .status()
        .expect("failed to invoke tailwindcss CLI");

    if !status.success() {
        panic!("tailwindcss build failed with status: {status}");
    }
}
