use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct SpotlightConfig {
    pub is_grid_view: bool,
    pub clipboard_max_items: usize,
    pub clipboard_auto_paste: bool,
}

impl Default for SpotlightConfig {
    fn default() -> Self {
        Self {
            is_grid_view: false,
            clipboard_max_items: 50,
            clipboard_auto_paste: true,
        }
    }
}

impl SpotlightConfig {
    fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("/home/jaime/.config"))
            .join("pulsaros-spotlight")
    }

    fn config_file() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> Self {
        let file = Self::config_file();
        if file.exists() {
            if let Ok(content) = fs::read_to_string(&file) {
                if let Ok(config) = serde_json::from_str::<SpotlightConfig>(&content) {
                    return config;
                }
            }
        }
        Self::default()
    }

    pub fn save(&self) {
        let dir = Self::config_dir();
        let _ = fs::create_dir_all(&dir);
        let file = Self::config_file();
        if let Ok(content) = serde_json::to_string_pretty(self) {
            let _ = fs::write(&file, content);
        }
    }
}
