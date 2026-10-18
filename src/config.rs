use crate::cpu_governor::Governor;
use serde::Deserialize;
use std::fs;

/// Print a config warning to stderr (works before logger is initialized)
fn config_warn(msg: &str) {
    eprintln!("config warning: {}", msg);
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProfileConfig {
    pub governor: String,
    pub turbo: String,
    pub charge_stop_threshold: Option<u32>,
    pub charge_start_threshold: Option<u32>,
    pub charge_behaviour: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub battery: ProfileConfig,
    pub charger: ProfileConfig,
    pub general: GeneralConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GeneralConfig {
    pub poll_interval_secs: u64,
    pub log_level: String,
}

impl Config {
    pub fn poll_interval_secs(&self) -> u64 {
        self.general.poll_interval_secs
    }

    pub fn log_level(&self) -> &str {
        &self.general.log_level
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            battery: ProfileConfig {
                governor: "powersave".to_string(),
                turbo: "never".to_string(),
                charge_stop_threshold: None,
                charge_start_threshold: None,
                charge_behaviour: None,
            },
            charger: ProfileConfig {
                governor: "performance".to_string(),
                turbo: "auto".to_string(),
                charge_stop_threshold: None,
                charge_start_threshold: None,
                charge_behaviour: None,
            },
            general: GeneralConfig {
                poll_interval_secs: 5,
                log_level: "info".to_string(),
            },
        }
    }
}

impl Config {
    pub fn from_file(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => match toml::from_str::<Config>(&content) {
                Ok(config) => {
                    config.validate();
                    config
                }
                Err(e) => {
                    config_warn(&format!(
                        "failed to parse config {}: {}, using defaults",
                        path, e
                    ));
                    Config::default()
                }
            },
            Err(e) => {
                config_warn(&format!(
                    "failed to read config {}: {}, using defaults",
                    path, e
                ));
                Config::default()
            }
        }
    }

    pub fn default_path() -> String {
        dirs::config_dir()
            .map(|p| p.join("battop/battop.toml").to_string_lossy().to_string())
            .unwrap_or_else(|| "/etc/battop/battop.toml".to_string())
    }

    pub fn validate(&self) {
        // Validate governor values
        Self::validate_governor("battery.governor", &self.battery.governor);
        Self::validate_governor("charger.governor", &self.charger.governor);

        // Validate turbo values
        Self::validate_turbo("battery.turbo", &self.battery.turbo);
        Self::validate_turbo("charger.turbo", &self.charger.turbo);

        // Validate poll interval
        if self.general.poll_interval_secs == 0 {
            config_warn("poll_interval_secs is 0, busy-loop possible; consider setting >= 1");
        }

        // Validate charge thresholds
        Self::validate_threshold(
            "battery.charge_stop_threshold",
            self.battery.charge_stop_threshold,
        );
        Self::validate_threshold(
            "battery.charge_start_threshold",
            self.battery.charge_start_threshold,
        );
        Self::validate_threshold(
            "charger.charge_stop_threshold",
            self.charger.charge_stop_threshold,
        );
        Self::validate_threshold(
            "charger.charge_start_threshold",
            self.charger.charge_start_threshold,
        );
    }

    fn validate_governor(name: &str, value: &str) {
        let gov = Governor::from_str(value);
        if matches!(gov, Governor::Unknown) {
            config_warn(&format!(
                "{} = '{}' is not a valid governor; expected one of: performance, powersave, conservative, ondemand, userspace, schedutil",
                name, value
            ));
        }
    }

    fn validate_turbo(name: &str, value: &str) {
        match value {
            "never" | "auto" => {}
            _ => config_warn(&format!(
                "{} = '{}' is not valid; expected 'never' or 'auto'",
                name, value
            )),
        }
    }

    fn validate_threshold(name: &str, value: Option<u32>) {
        if let Some(v) = value
            && v > 100
        {
            config_warn(&format!("{} is {}%, should be 0-100", name, v));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.battery.governor, "powersave");
        assert_eq!(config.charger.governor, "performance");
        assert_eq!(config.general.poll_interval_secs, 5);
    }

    #[test]
    fn test_parse_valid_config() {
        let toml_str = r#"
[battery]
governor = "powersave"
turbo = "never"
charge_stop_threshold = 80

[charger]
governor = "performance"
turbo = "auto"
charge_stop_threshold = 100

[general]
poll_interval_secs = 10
log_level = "debug"
"#;
        let config = toml::from_str::<Config>(toml_str).expect("should parse");
        assert_eq!(config.battery.governor, "powersave");
        assert_eq!(config.battery.charge_stop_threshold, Some(80));
        assert_eq!(config.charger.governor, "performance");
        assert_eq!(config.battery.turbo, "never");
        assert_eq!(config.charger.turbo, "auto");
        assert_eq!(config.general.poll_interval_secs, 10);
        assert_eq!(config.general.log_level, "debug");
    }

    #[test]
    fn test_parse_partial_config_uses_defaults() {
        // Missing optional fields should still parse
        let toml_str = r#"
[battery]
governor = "performance"
turbo = "auto"

[charger]
governor = "powersave"
turbo = "never"

[general]
poll_interval_secs = 3
log_level = "warn"
"#;
        let config = toml::from_str::<Config>(toml_str).expect("should parse");
        assert_eq!(config.battery.governor, "performance");
        assert_eq!(config.battery.charge_stop_threshold, None);
        assert_eq!(config.general.poll_interval_secs, 3);
    }

    #[test]
    fn test_parse_invalid_falls_back_to_default() {
        let result = toml::from_str::<Config>("this is not valid toml");
        assert!(result.is_err());
    }

    #[test]
    fn test_default_path_uses_toml_extension() {
        let path = Config::default_path();
        assert!(
            path.ends_with(".toml"),
            "expected .toml extension, got: {}",
            path
        );
    }
}
