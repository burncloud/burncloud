use std::path::PathBuf;
use std::process::Command;

const TAILWIND_VERSION: &str = "3.4.17";

fn download_tailwind(cli: &std::path::Path, cli_name: &str) -> Result<(), String> {
    // Determine target triple and extension
    let (os, arch, ext) = if cfg!(target_os = "windows") {
        (
            "windows",
            if cfg!(target_arch = "x86_64") {
                "x64"
            } else {
                "x86"
            },
            ".exe",
        )
    } else if cfg!(target_os = "macos") {
        (
            "macos",
            if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "x64"
            },
            "",
        )
    } else {
        (
            "linux",
            if cfg!(target_arch = "x86_64") {
                "x64"
            } else if cfg!(target_arch = "aarch64") {
                "arm64"
            } else {
                "x64"
            },
            "",
        )
    };

    let filename = format!("tailwindcss-{os}-{arch}{ext}");
    let url = format!(
        "https://github.com/tailwindlabs/tailwindcss/releases/download/v{TAILWIND_VERSION}/{filename}"
    );

    println!("cargo:warning=Downloading tailwindcss v{TAILWIND_VERSION} from {url} ...");

    // Download via platform-appropriate command
    let success = if cfg!(target_os = "windows") {
        // PowerShell: download to temp file then rename (avoids partial files)
        let tmp = cli.with_extension("tmp");
        let ps_script = format!(
            "Invoke-WebRequest -Uri '{}' -OutFile '{}' -UseBasicParsing",
            url.replace("'", "''"),
            tmp.display().to_string().replace("'", "''"),
        );
        let status = Command::new("powershell")
            .args(["-NoProfile", "-NonInteractive", "-Command", &ps_script])
            .status()
            .map_err(|e| format!("failed to run powershell: {e}"))?;
        if status.success() {
            std::fs::rename(&tmp, cli).map_err(|e| format!("rename failed: {e}"))?;
            true
        } else {
            let _ = std::fs::remove_file(&tmp);
            false
        }
    } else {
        // macOS/Linux: use curl (universally available)
        let status = Command::new("curl")
            .args(["-fsSL", "-o"])
            .arg(cli)
            .arg(&url)
            .status()
            .map_err(|e| format!("failed to run curl: {e}"))?;
        if status.success() {
            // chmod +x
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(cli, std::fs::Permissions::from_mode(0o755))
                    .map_err(|e| format!("chmod failed: {e}"))?;
            }
            true
        } else {
            false
        }
    };

    if success && cli.exists() {
        println!("cargo:warning=tailwindcss v{TAILWIND_VERSION} downloaded successfully.");
        Ok(())
    } else {
        Err(format!(
            "Download failed. Please manually download from:\n  {url}\n  and save as crates/client/{cli_name}"
        ))
    }
}

fn main() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config = manifest_dir.join("tailwind.config.js");
    let input = manifest_dir.join("input.css");
    let output = manifest_dir.join("src/assets/tailwind.css");
    let cli_name = if cfg!(windows) {
        "tailwindcss.exe"
    } else {
        "tailwindcss"
    };
    let cli = manifest_dir.join(cli_name);

    println!("cargo:rerun-if-changed=tailwind.config.js");
    println!("cargo:rerun-if-changed=input.css");
    println!("cargo:rerun-if-changed={cli_name}");

    // Auto-download if CLI is missing
    if !cli.exists() {
        if let Err(e) = download_tailwind(&cli, cli_name) {
            println!("cargo:warning={e}");
            // Ensure a placeholder CSS exists so include_str! doesn't break compilation
            if !output.exists() {
                if let Some(parent) = output.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                let _ = std::fs::write(&output, "/* tailwind placeholder — download failed */\n");
            }
            return;
        }
    }

    let status = match Command::new(&cli)
        .arg("-c")
        .arg(&config)
        .arg("-i")
        .arg(&input)
        .arg("-o")
        .arg(&output)
        .arg("--minify")
        .current_dir(&manifest_dir)
        .status()
    {
        Ok(s) => s,
        Err(e) => {
            println!("cargo:warning=failed to invoke tailwindcss CLI: {e}");
            return;
        }
    };

    if !status.success() {
        panic!("tailwindcss build failed with status: {status}");
    }
}
