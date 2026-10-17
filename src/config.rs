use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    pub governor: String,
    pub turbo: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub battery: ProfileConfig,
    pub charger: ProfileConfig,
    pub poll_interval_secs: u64,
    pub log_level: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            battery: ProfileConfig {
                governor: "powersave".to_string(),
                turbo: "never".to_string(),
            },
            charger: ProfileConfig {
                governor: "performance".to_string(),
                turbo: "auto".to_string(),
            },
            poll_interval_secs: 5,
            log_level: "info".to_string(),
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        if let Ok(content) = fs::read_to_string(path) {
            if let Ok(config) = serde_ini::from_str::<Config>(&content) {
                return config;
            }
        }
        Config::default()
    }

    pub fn default_path() -> String {
        dirs::config_dir()
            .map(|p| p.join("battop/battop.conf").to_string_lossy().to_string())
            .unwrap_or_else(|| "/etc/battop/battop.conf".to_string())
    }
}
