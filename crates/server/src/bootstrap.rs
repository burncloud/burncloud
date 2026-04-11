use anyhow::Context;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MasterKeySource {
    Env,
    File,
    Generated,
}

impl MasterKeySource {
    pub fn as_str(self) -> &'static str {
        match self {
            MasterKeySource::Env => "env",
            MasterKeySource::File => "file",
            MasterKeySource::Generated => "generated",
        }
    }
}

pub fn ensure_master_key() -> anyhow::Result<MasterKeySource> {
    if let Ok(existing) = std::env::var("MASTER_KEY") {
        std::env::set_var("MASTER_KEY", &existing);
        tracing::info!(source = "env", "MASTER_KEY loaded from environment variable");
        return Ok(MasterKeySource::Env);
    }

    let key_path = master_key_path()?;

    if key_path.exists() {
        let raw = std::fs::read_to_string(&key_path).with_context(|| {
            format!(
                "failed to read existing MASTER_KEY file at {} (check file permissions)",
                key_path.display()
            )
        })?;
        let hex_key = raw.trim().to_string();
        std::env::set_var("MASTER_KEY", &hex_key);
        tracing::info!(
            source = "file",
            key_path = %key_path.display(),
            "MASTER_KEY loaded from existing key file"
        );
        return Ok(MasterKeySource::File);
    }

    let hex_key = generate_master_key()?;
    write_master_key(&key_path, &hex_key).with_context(|| {
        format!(
            "failed to persist newly generated MASTER_KEY to {} \
             (check directory permissions and available disk space); \
             refusing to start with an in-memory key to avoid silent data loss on restart",
            key_path.display()
        )
    })?;
    std::env::set_var("MASTER_KEY", &hex_key);
    tracing::warn!(
        source = "generated",
        key_path = %key_path.display(),
        "generated a new MASTER_KEY and stored it at {}. \
         BACK UP THIS FILE NOW: if it is lost, all encrypted data becomes permanently unrecoverable. \
         To migrate between machines, copy this file or set the MASTER_KEY environment variable to its contents.",
        key_path.display()
    );

    Ok(MasterKeySource::Generated)
}

fn master_key_path() -> anyhow::Result<PathBuf> {
    let base = dirs::config_dir()
        .ok_or_else(|| anyhow::anyhow!("unable to resolve user config directory"))?;
    let dir = base.join("burncloud");
    std::fs::create_dir_all(&dir).with_context(|| {
        format!(
            "failed to create MASTER_KEY config directory at {} (check parent permissions)",
            dir.display()
        )
    })?;
    Ok(dir.join("master.key"))
}

fn generate_master_key() -> anyhow::Result<String> {
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::rngs::OsRng
        .try_fill_bytes(&mut bytes)
        .context("failed to generate MASTER_KEY: OS random number generator unavailable")?;
    Ok(hex::encode(bytes))
}

fn write_master_key(path: &Path, key: &str) -> anyhow::Result<()> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = options
        .open(path)
        .with_context(|| format!("failed to create key file at {}", path.display()))?;
    file.write_all(key.as_bytes())
        .with_context(|| format!("failed to write key bytes to {}", path.display()))?;
    file.sync_all()
        .with_context(|| format!("failed to fsync key file at {}", path.display()))?;
    Ok(())
}
