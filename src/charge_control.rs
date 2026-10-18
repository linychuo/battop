use std::fs;
use std::path::Path;

const POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";

/// Find the first battery device name (BAT0, BAT1, etc.)
fn find_battery_name() -> Option<String> {
    if let Ok(entries) = fs::read_dir(POWER_SUPPLY_PATH) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().to_string();
            if name.starts_with("BAT") {
                // Verify it's actually a battery
                let type_path = path.join("type");
                if let Ok(content) = fs::read_to_string(&type_path)
                && content.trim().eq_ignore_ascii_case("Battery")
            {
                return Some(name);
            }
            }
        }
    }
    None
}

/// Build the sysfs path for a battery attribute
fn battery_sysfs_attr(battery_name: &str, attr: &str) -> String {
    format!("{}/{}/{}", POWER_SUPPLY_PATH, battery_name, attr)
}

/// Read a battery attribute as string
fn read_battery_attr(battery_name: &str, attr: &str) -> Option<String> {
    fs::read_to_string(battery_sysfs_attr(battery_name, attr))
        .ok()
        .map(|s| s.trim().to_string())
}

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
    battery_name: Option<String>,
}

impl ChargeControl {
    /// Probe the battery hardware for charge control capabilities
    pub fn probe() -> Self {
        let battery_name = find_battery_name();

        let (has_end_threshold, has_start_threshold, has_behaviour) =
            if let Some(ref name) = battery_name {
                (
                    Path::new(&battery_sysfs_attr(name, "charge_control_end_threshold")).exists(),
                    Path::new(&battery_sysfs_attr(name, "charge_control_start_threshold")).exists(),
                    Path::new(&battery_sysfs_attr(name, "charge_behaviour")).exists(),
                )
            } else {
                (false, false, false)
            };

        let supported = has_end_threshold || has_start_threshold;

        let current_end_threshold = if has_end_threshold {
            battery_name
                .as_ref()
                .and_then(|n| read_battery_attr(n, "charge_control_end_threshold"))
                .and_then(|s| s.parse().ok())
        } else {
            None
        };

        let current_start_threshold = if has_start_threshold {
            battery_name
                .as_ref()
                .and_then(|n| read_battery_attr(n, "charge_control_start_threshold"))
                .and_then(|s| s.parse().ok())
        } else {
            None
        };

        let available_behaviours = if has_behaviour {
            battery_name
                .as_ref()
                .and_then(|n| read_battery_attr(n, "charge_behaviour"))
                .map(|s| {
                    // Format: "[auto] inhibit-charge force-discharge"
                    s.split_whitespace()
                        .map(|b| b.trim_matches(&['[', ']'][..]).to_string())
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
            battery_name,
        }
    }

    /// Set charge end threshold (stop charging at this percentage)
    pub fn set_end_threshold(&self, threshold: u32) -> std::io::Result<()> {
        if threshold > 100 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "charge end threshold {}% is out of range (0-100)",
                    threshold
                ),
            ));
        }
        match &self.battery_name {
            Some(name) => {
                let path = battery_sysfs_attr(name, "charge_control_end_threshold");
                fs::write(&path, threshold.to_string())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no battery found",
            )),
        }
    }

    /// Set charge start threshold (start charging at this percentage)
    pub fn set_start_threshold(&self, threshold: u32) -> std::io::Result<()> {
        if threshold > 100 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "charge start threshold {}% is out of range (0-100)",
                    threshold
                ),
            ));
        }
        match &self.battery_name {
            Some(name) => {
                let path = battery_sysfs_attr(name, "charge_control_start_threshold");
                fs::write(&path, threshold.to_string())
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no battery found",
            )),
        }
    }

    /// Set charge behaviour (auto, inhibit-charge, force-discharge)
    pub fn set_behaviour(&self, behaviour: &str) -> std::io::Result<()> {
        let valid = ["auto", "inhibit-charge", "force-discharge"];
        if !valid.contains(&behaviour) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "invalid charge behaviour '{}', expected one of: {}",
                    behaviour,
                    valid.join(", ")
                ),
            ));
        }
        match &self.battery_name {
            Some(name) => {
                let path = battery_sysfs_attr(name, "charge_behaviour");
                fs::write(&path, behaviour)
            }
            None => Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no battery found",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_threshold_range_validation() {
        // Create a dummy ChargeControl for testing validation logic
        let cc = ChargeControl {
            supported: true,
            has_end_threshold: true,
            has_start_threshold: true,
            has_behaviour: false,
            current_end_threshold: None,
            current_start_threshold: None,
            available_behaviours: Vec::new(),
            battery_name: None,
        };
        let result = cc.set_end_threshold(101);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn test_valid_threshold_no_invalid_input_error() {
        let cc = ChargeControl {
            supported: true,
            has_end_threshold: true,
            has_start_threshold: true,
            has_behaviour: false,
            current_end_threshold: None,
            current_start_threshold: None,
            available_behaviours: Vec::new(),
            battery_name: None,
        };
        let result = cc.set_end_threshold(80);
        // May fail with NotFound (no battery), but NOT InvalidInput
        if let Err(e) = result {
            assert_ne!(e.kind(), std::io::ErrorKind::InvalidInput);
        }
    }

    #[test]
    fn test_invalid_behaviour() {
        let cc = ChargeControl {
            supported: true,
            has_end_threshold: false,
            has_start_threshold: false,
            has_behaviour: true,
            current_end_threshold: None,
            current_start_threshold: None,
            available_behaviours: Vec::new(),
            battery_name: None,
        };
        let result = cc.set_behaviour("invalid");
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().kind(),
            std::io::ErrorKind::InvalidInput
        );
    }
}
