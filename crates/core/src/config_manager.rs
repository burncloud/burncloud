use anyhow::Result;
use burncloud_common::{load_config, save_config, Config};

pub struct ConfigManager {
    config: Config,
    config_path: String,
}

impl ConfigManager {
    pub fn new(config_path: String) -> Result<Self> {
        let config = load_config(&config_path)?;
        Ok(Self {
            config,
            config_path,
        })
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn update_config(&mut self, new_config: Config) -> Result<()> {
        self.config = new_config;
        save_config(&self.config_path, &self.config)?;
        Ok(())
    }

    pub fn get_models_dir(&self) -> &str {
        &self.config.models_dir
    }

    pub fn get_server_port(&self) -> u16 {
        self.config.server_port
    }
}
