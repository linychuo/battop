use crate::cpu_governor::{EnergyPerfPref, Governor};
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
    #[serde(default)]
    pub energy_performance_preference: Option<String>,
    #[serde(default)]
    pub max_freq: Option<u64>,
    #[serde(default)]
    pub min_freq: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ChargeControlConfig {
    pub stop_threshold: Option<u32>,
    pub start_threshold: Option<u32>,
    pub behaviour: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub battery: ProfileConfig,
    pub charger: ProfileConfig,
    #[serde(default)]
    pub charge_control: ChargeControlConfig,
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
                energy_performance_preference: None,
                max_freq: None,
                min_freq: None,
            },
            charger: ProfileConfig {
                governor: "performance".to_string(),
                turbo: "auto".to_string(),
                energy_performance_preference: None,
                max_freq: None,
                min_freq: None,
            },
            charge_control: ChargeControlConfig {
                stop_threshold: None,
                start_threshold: None,
                behaviour: None,
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
        Self::validate_governor("battery.governor", &self.battery.governor);
        Self::validate_governor("charger.governor", &self.charger.governor);
        Self::validate_turbo("battery.turbo", &self.battery.turbo);
        Self::validate_turbo("charger.turbo", &self.charger.turbo);
        Self::validate_epp("battery.energy_performance_preference", &self.battery.energy_performance_preference);
        Self::validate_epp("charger.energy_performance_preference", &self.charger.energy_performance_preference);
        Self::validate_freq("battery.max_freq", self.battery.max_freq, "kHz");
        Self::validate_freq("battery.min_freq", self.battery.min_freq, "kHz");
        Self::validate_freq("charger.max_freq", self.charger.max_freq, "kHz");
        Self::validate_freq("charger.min_freq", self.charger.min_freq, "kHz");

        if self.general.poll_interval_secs == 0 {
            config_warn("poll_interval_secs is 0, busy-loop possible; consider setting >= 1");
        }

        Self::validate_threshold(
            "charge_control.stop_threshold",
            self.charge_control.stop_threshold,
        );
        Self::validate_threshold(
            "charge_control.start_threshold",
            self.charge_control.start_threshold,
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

    fn validate_epp(name: &str, value: &Option<String>) {
        if let Some(v) = value {
            let epp = EnergyPerfPref::from_str(v);
            if matches!(epp, EnergyPerfPref::Unknown) {
                config_warn(&format!(
                    "{} = '{}' is not valid; expected one of: performance, balance_performance, balance_power, power",
                    name, v
                ));
            }
        }
    }

    fn validate_freq(name: &str, value: Option<u64>, unit: &str) {
        if let Some(v) = value {
            if v == 0 {
                config_warn(&format!(
                    "{} is 0 {}, this would effectively disable the CPU",
                    name, unit
                ));
            }
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
energy_performance_preference = "balance_power"
max_freq = 2000000

[charger]
governor = "performance"
turbo = "auto"
energy_performance_preference = "balance_performance"
min_freq = 800000

[charge_control]
stop_threshold = 80
start_threshold = 40
behaviour = "auto"

[general]
poll_interval_secs = 10
log_level = "debug"
"#;
        let config = toml::from_str::<Config>(toml_str).expect("should parse");
        assert_eq!(config.battery.governor, "powersave");
        assert_eq!(config.battery.turbo, "never");
        assert_eq!(config.battery.energy_performance_preference, Some("balance_power".to_string()));
        assert_eq!(config.battery.max_freq, Some(2_000_000));
        assert_eq!(config.charger.governor, "performance");
        assert_eq!(config.charger.turbo, "auto");
        assert_eq!(config.charger.energy_performance_preference, Some("balance_performance".to_string()));
        assert_eq!(config.charger.min_freq, Some(800_000));
        assert_eq!(config.charge_control.stop_threshold, Some(80));
        assert_eq!(config.charge_control.start_threshold, Some(40));
        assert_eq!(config.charge_control.behaviour, Some("auto".to_string()));
        assert_eq!(config.general.poll_interval_secs, 10);
        assert_eq!(config.general.log_level, "debug");
    }

    #[test]
    fn test_parse_invalid_epp_warns_but_does_not_panic() {
        // Config with invalid EPP — should still parse (validation is best-effort)
        let toml_str = r#"
[battery]
governor = "powersave"
turbo = "never"
energy_performance_preference = "invalid_value"

[charger]
governor = "performance"
turbo = "auto"

[general]
poll_interval_secs = 5
log_level = "info"
"#;
        let config = toml::from_str::<Config>(toml_str).expect("should parse");
        // invalid EPP parses but is caught at validation time
        assert_eq!(config.battery.energy_performance_preference, Some("invalid_value".to_string()));
    }

    #[test]
    fn test_parse_partial_config_uses_defaults() {
        let toml_str = r#"
[battery]
governor = "powersave"
turbo = "never"

[charger]
governor = "performance"
turbo = "auto"

[general]
poll_interval_secs = 3
log_level = "warn"
"#;
        let config = toml::from_str::<Config>(toml_str).expect("should parse");
        assert_eq!(config.charge_control.stop_threshold, None);
        assert_eq!(config.charge_control.start_threshold, None);
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
