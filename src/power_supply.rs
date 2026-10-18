use std::fs;
use std::path::Path;

const POWER_SUPPLY_PATH: &str = "/sys/class/power_supply";

#[derive(Debug)]
pub struct PowerSupply {
    pub name: String,
    pub supply_type: SupplyType,
    pub status: PowerStatus,
    pub capacity: u32,
    pub power_now: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupplyType {
    Battery,
    Mains,
    Usb,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerStatus {
    Charging,
    Discharging,
    Full,
    NotCharging,
    Unknown,
}

impl SupplyType {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "battery" => Self::Battery,
            "mains" => Self::Mains,
            "usb" => Self::Usb,
            _ => Self::Unknown,
        }
    }
}

impl PowerStatus {
    fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "charging" => Self::Charging,
            "discharging" => Self::Discharging,
            "full" => Self::Full,
            "not_charging" | "not charging" => Self::NotCharging,
            _ => Self::Unknown,
        }
    }
}

/// Detect all power supplies. Call once per poll cycle for efficiency.
pub fn detect_supplies() -> Vec<PowerSupply> {
    let mut supplies = Vec::new();

    if let Ok(entries) = fs::read_dir(POWER_SUPPLY_PATH) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            let name = path.file_name().unwrap().to_string_lossy().to_string();
            let supply_type = read_supply_type(&path);

            if matches!(supply_type, SupplyType::Battery | SupplyType::Mains) {
                let status = read_file(&path.join("status"))
                    .map(|s| PowerStatus::from_str(&s))
                    .unwrap_or(PowerStatus::Unknown);

                let capacity = read_file(&path.join("capacity"))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);

                let power_now = read_file(&path.join("power_now"))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);

                supplies.push(PowerSupply {
                    name,
                    supply_type,
                    status,
                    capacity,
                    power_now,
                });
            }
        }
    }

    supplies
}

/// Returns true if running on battery (no AC online).
/// Uses pre-scanned supplies for efficiency.
/// Falls back to battery status if AC online is not available.
pub fn on_battery_with(supplies: &[PowerSupply]) -> bool {
    let has_battery = supplies
        .iter()
        .any(|s| matches!(s.supply_type, SupplyType::Battery));
    if !has_battery {
        // No battery found (desktop) - default to battery profile
        return true;
    }

    // Method 1: Check if any mains supply is online
    let mut found_ac = false;
    for s in supplies {
        if matches!(s.supply_type, SupplyType::Mains) {
            let online_path = format!("{}/{}/online", POWER_SUPPLY_PATH, s.name);
            if let Some(content) = read_file(Path::new(&online_path)) {
                found_ac = true;
                if content.trim() == "1" {
                    return false; // AC is online
                }
            }
        }
    }

    // If we found AC adapters but none are online, we're on battery
    if found_ac {
        return true;
    }

    // Method 2: Fallback - check battery status
    for s in supplies {
        if matches!(s.supply_type, SupplyType::Battery) {
            match s.status {
                PowerStatus::Discharging => return true,
                PowerStatus::Charging | PowerStatus::Full => return false,
                _ => {} // Unknown/NotCharging - keep checking
            }
        }
    }

    // Default: assume battery to be safe
    true
}

/// Get battery power draw in watts (from cached supplies).
pub fn battery_power_draw_watts(supplies: &[PowerSupply]) -> Option<f64> {
    supplies
        .iter()
        .find(|s| matches!(s.supply_type, SupplyType::Battery))
        .map(|s| s.power_now as f64 / 1_000_000.0)
}

/// Get battery capacity percentage (from cached supplies).
pub fn battery_capacity(supplies: &[PowerSupply]) -> Option<u32> {
    supplies
        .iter()
        .find(|s| matches!(s.supply_type, SupplyType::Battery))
        .map(|s| s.capacity)
}

fn read_supply_type(path: &Path) -> SupplyType {
    read_file(&path.join("type"))
        .map(|s| SupplyType::from_str(&s))
        .unwrap_or(SupplyType::Unknown)
}

fn read_file(path: &Path) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supply_type_from_str() {
        assert_eq!(SupplyType::from_str("Battery"), SupplyType::Battery);
        assert_eq!(SupplyType::from_str("Mains"), SupplyType::Mains);
        assert_eq!(SupplyType::from_str("USB"), SupplyType::Usb);
        assert_eq!(SupplyType::from_str("foo"), SupplyType::Unknown);
    }

    #[test]
    fn test_power_status_from_str() {
        assert_eq!(PowerStatus::from_str("Charging"), PowerStatus::Charging);
        assert_eq!(PowerStatus::from_str("Discharging"), PowerStatus::Discharging);
        assert_eq!(PowerStatus::from_str("Full"), PowerStatus::Full);
        assert_eq!(
            PowerStatus::from_str("Not Charging"),
            PowerStatus::NotCharging
        );
    }

    #[test]
    fn test_on_battery_no_battery() {
        let supplies = vec![];
        assert!(on_battery_with(&supplies));
    }

    #[test]
    fn test_on_battery_discharging() {
        let supplies = vec![PowerSupply {
            name: "BAT0".to_string(),
            supply_type: SupplyType::Battery,
            status: PowerStatus::Discharging,
            capacity: 50,
            power_now: 10_000_000,
        }];
        assert!(on_battery_with(&supplies));
    }

    #[test]
    fn test_on_battery_charging_fallback() {
        // No AC online file, battery status says charging
        let supplies = vec![PowerSupply {
            name: "BAT0".to_string(),
            supply_type: SupplyType::Battery,
            status: PowerStatus::Charging,
            capacity: 80,
            power_now: 0,
        }];
        assert!(!on_battery_with(&supplies));
    }
}
