use std::path::{Path, PathBuf};

/// Repository root (`burncloud/`), inferred from `crates/loops`.
pub fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .expect("crates/loops must live at crates/loops")
        .to_path_buf()
}

/// All loop runtime artifacts (screenshots, metrics, logs, prompts).
pub fn loops_data_dir(root: &Path) -> PathBuf {
    root.join("data").join("loops")
}

pub fn acceptance_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("acceptance")
}

/// Screenshots + metrics.json + review.json + manifest.json (aesthetic gate).
pub fn aesthetic_artifacts_dir(root: &Path) -> PathBuf {
    loops_data_dir(root).join("aesthetic").join("latest")
}

/// Screenshots + manifest.json (css visual gate).
pub fn css_visual_artifacts_dir(root: &Path) -> PathBuf {
    loops_data_dir(root).join("css-visual").join("latest")
}

/// Per-iteration logs + loop-state + agent prompts (jobs aesthetic loop).
pub fn jobs_aesthetic_run_dir(root: &Path) -> PathBuf {
    loops_data_dir(root).join("jobs-aesthetic")
}

/// Per-iteration logs (css optimize loop).
pub fn css_optimize_run_dir(root: &Path) -> PathBuf {
    loops_data_dir(root).join("css-optimize")
}

pub fn client_crate_dir(root: &Path) -> PathBuf {
    root.join("crates").join("client")
}

pub fn burncloud_bin(root: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        root.join("target").join("debug").join("burncloud.exe")
    }
    #[cfg(not(windows))]
    {
        root.join("target").join("debug").join("burncloud")
    }
}
