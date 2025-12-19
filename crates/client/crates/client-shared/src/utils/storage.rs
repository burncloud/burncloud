use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use std::fs;
#[cfg(not(target_arch = "wasm32"))]
use std::path::PathBuf;

#[cfg(target_arch = "wasm32")]
use gloo_storage::{LocalStorage, Storage};

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct ClientState {
    pub last_username: Option<String>,
    pub last_password: Option<String>,
}

impl ClientState {
    #[cfg(not(target_arch = "wasm32"))]
    fn get_path() -> PathBuf {
        let db_dir = if cfg!(target_os = "windows") {
            // Windows: %USERPROFILE%\AppData\Local\BurnCloud
            let user_profile = std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(user_profile)
                .join("AppData")
                .join("Local")
                .join("BurnCloud")
        } else {
            // Linux: ~/.burncloud
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".burncloud")
        };

        // Create directory if not exists
        if !db_dir.exists() {
            let _ = fs::create_dir_all(&db_dir);
        }

        db_dir.join("client_state.json")
    }

    pub fn load() -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            LocalStorage::get("client_state").unwrap_or_default()
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::get_path();
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    if let Ok(state) = serde_json::from_str(&content) {
                        return state;
                    }
                }
            }
            Self::default()
        }
    }

    pub fn save(&self) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = LocalStorage::set("client_state", self);
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let path = Self::get_path();
            if let Ok(content) = serde_json::to_string_pretty(self) {
                let _ = fs::write(path, content);
            }
        }
    }
}
