use std::fs;
use std::path::Path;

const BATTERY_PATH: &str = "/sys/class/power_supply/BAT0";

/// Charge control capability probed from hardware
#[derive(Debug)]
pub struct ChargeControl {
    pub supported: bool,
    pub has_end_threshold: bool,
    pub has_start_threshold: bool,
    pub has_behaviour: bool,
    pub current_end_threshold: Option<u32>,
    pub current_start_threshold: Option<u32>,
    pub available_behaviours: Vec<String>,
}

impl ChargeControl {
    /// Probe the battery hardware for charge control capabilities
    pub fn probe() -> Self {
        let end_path = format!("{}/charge_control_end_threshold", BATTERY_PATH);
        let start_path = format!("{}/charge_control_start_threshold", BATTERY_PATH);
        let behaviour_path = format!("{}/charge_behaviour", BATTERY_PATH);

        let has_end_threshold = Path::new(&end_path).exists();
        let has_start_threshold = Path::new(&start_path).exists();
        let has_behaviour = Path::new(&behaviour_path).exists();

        let supported = has_end_threshold || has_start_threshold;

        let current_end_threshold = if has_end_threshold {
            fs::read_to_string(&end_path)
                .ok()
                .and_then(|s| s.trim().parse().ok())
        } else {
            None
        };

        let current_start_threshold = if has_start_threshold {
            fs::read_to_string(&start_path)
                .ok()
                .and_then(|s| s.trim().parse().ok())
        } else {
            None
        };

        let available_behaviours = if has_behaviour {
            fs::read_to_string(&behaviour_path)
                .map(|s| {
                    // Parse format: "[auto] inhibit-charge force-discharge"
                    s.trim()
                        .trim_start_matches('[')
                        .trim_end_matches(']')
                        .split_whitespace()
                        .map(|b| {
                            // Remove brackets from selected option
                            b.trim_start_matches('[').trim_end_matches(']').to_string()
                        })
                        .collect()
                })
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        Self {
            supported,
            has_end_threshold,
            has_start_threshold,
            has_behaviour,
            current_end_threshold,
            current_start_threshold,
            available_behaviours,
        }
    }

    /// Set charge end threshold (stop charging at this percentage)
    pub fn set_end_threshold(threshold: u32) -> std::io::Result<()> {
        let path = format!("{}/charge_control_end_threshold", BATTERY_PATH);
        fs::write(&path, threshold.to_string())
    }

    /// Set charge start threshold (start charging at this percentage)
    pub fn set_start_threshold(threshold: u32) -> std::io::Result<()> {
        let path = format!("{}/charge_control_start_threshold", BATTERY_PATH);
        fs::write(&path, threshold.to_string())
    }

    /// Set charge behaviour (auto, inhibit-charge, force-discharge)
    pub fn set_behaviour(behaviour: &str) -> std::io::Result<()> {
        let path = format!("{}/charge_behaviour", BATTERY_PATH);
        fs::write(&path, behaviour)
    }
}
