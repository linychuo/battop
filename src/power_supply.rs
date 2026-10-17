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
    pub energy_full: u64,
    pub energy_now: u64,
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

                let energy_full = read_file(&path.join("energy_full"))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);

                let energy_now = read_file(&path.join("energy_now"))
                    .and_then(|s| s.trim().parse().ok())
                    .unwrap_or(0);

                supplies.push(PowerSupply {
                    name,
                    supply_type,
                    status,
                    capacity,
                    power_now,
                    energy_full,
                    energy_now,
                });
            }
        }
    }

    supplies
}

pub fn on_battery() -> bool {
    let supplies = detect_supplies();
    !supplies
        .iter()
        .any(|s| matches!(s.supply_type, SupplyType::Mains) && s.status == PowerStatus::Charging)
        && supplies
            .iter()
            .any(|s| matches!(s.supply_type, SupplyType::Battery))
}

pub fn battery_power_draw_watts() -> Option<f64> {
    detect_supplies()
        .iter()
        .find(|s| matches!(s.supply_type, SupplyType::Battery))
        .map(|s| s.power_now as f64 / 1_000_000.0)
}

pub fn battery_capacity() -> Option<u32> {
    detect_supplies()
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
