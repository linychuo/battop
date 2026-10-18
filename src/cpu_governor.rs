use std::fs;
use std::path::Path;

// ── Platform Profile ─────────────────────────────────────────────────
// Firmware-level power/performance profile via ACPI
// Path: /sys/firmware/acpi/platform_profile
// Available on most laptops from ~2020+

const PLATFORM_PROFILE_PATH: &str = "/sys/firmware/acpi/platform_profile";
const PLATFORM_PROFILE_CHOICES_PATH: &str = "/sys/firmware/acpi/platform_profile_choices";

/// Check if Platform Profile is supported on this hardware
pub fn platform_profile_is_supported() -> bool {
    Path::new(PLATFORM_PROFILE_PATH).exists()
}

/// Read current Platform Profile
pub fn platform_profile_current() -> Option<String> {
    fs::read_to_string(PLATFORM_PROFILE_PATH)
        .ok()
        .map(|s| s.trim().to_string())
}

/// Read available Platform Profile choices (space-separated)
pub fn platform_profile_choices() -> Vec<String> {
    match fs::read_to_string(PLATFORM_PROFILE_CHOICES_PATH) {
        Ok(content) => content
            .split_whitespace()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect(),
        Err(_) => Vec::new(),
    }
}

/// Set Platform Profile
pub fn set_platform_profile(profile: &str) -> std::io::Result<()> {
    if !platform_profile_is_supported() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "platform profile not supported on this hardware",
        ));
    }
    fs::write(PLATFORM_PROFILE_PATH, profile)
}

// ── EPP (Energy Performance Preference) ──────────────────────────────
// Available on Intel (Skylake+) and AMD (Zen 2+) with `schedutil`/`performance` governors

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnergyPerfPref {
    Performance,
    BalancePerformance,
    BalancePower,
    Power,
    Unknown,
}

impl EnergyPerfPref {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::BalancePerformance => "balance_performance",
            Self::BalancePower => "balance_power",
            Self::Power => "power",
            Self::Unknown => "",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "performance" => Self::Performance,
            "balance_performance" => Self::BalancePerformance,
            "balance_power" => Self::BalancePower,
            "power" => Self::Power,
            _ => Self::Unknown,
        }
    }

    /// Probe the first CPU core for supported EPP values
    pub fn available_values() -> Vec<String> {
        let path = "/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_available_preferences";
        match fs::read_to_string(path) {
            Ok(content) => content
                .split_whitespace()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    pub fn is_supported() -> bool {
        Path::new("/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference").exists()
    }
}

/// Set EPP for all CPU cores. No-op if unsupported.
pub fn set_epp(epp: EnergyPerfPref) -> std::io::Result<()> {
    let value = epp.as_str();
    if value.is_empty() {
        return Ok(());
    }
    for cpu in 0..cpu_count() {
        let path = format!(
            "/sys/devices/system/cpu/cpu{}/cpufreq/energy_performance_preference",
            cpu
        );
        if Path::new(&path).exists() {
            fs::write(&path, value)?;
        }
    }
    Ok(())
}

/// Read current EPP from cpu0
#[allow(dead_code)]
pub fn get_epp() -> Option<EnergyPerfPref> {
    let path = "/sys/devices/system/cpu/cpu0/cpufreq/energy_performance_preference";
    fs::read_to_string(path)
        .ok()
        .map(|s| EnergyPerfPref::from_str(&s))
}

// ── CPU Frequency Limits ──────────────────────────────────────────────
// Set scaling_min_freq / scaling_max_freq in kHz.

/// Get the absolute minimum frequency limit (from cpuinfo_min_freq)
pub fn freq_min_limit_hz() -> Option<u64> {
    fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_min_freq")
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Get the absolute maximum frequency limit (from cpuinfo_max_freq)
pub fn freq_max_limit_hz() -> Option<u64> {
    fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/cpuinfo_max_freq")
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Set scaling_max_freq for all cores (value in kHz)
pub fn set_max_freq(freq_khz: u64) -> std::io::Result<()> {
    for cpu in 0..cpu_count() {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_max_freq", cpu);
        if Path::new(&path).exists() {
            fs::write(&path, freq_khz.to_string())?;
        }
    }
    Ok(())
}

/// Set scaling_min_freq for all cores (value in kHz)
pub fn set_min_freq(freq_khz: u64) -> std::io::Result<()> {
    for cpu in 0..cpu_count() {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_min_freq", cpu);
        if Path::new(&path).exists() {
            fs::write(&path, freq_khz.to_string())?;
        }
    }
    Ok(())
}

/// Read current scaling_max_freq from cpu0 (kHz)
#[allow(dead_code)]
pub fn get_max_freq() -> Option<u64> {
    fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_max_freq")
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Read current scaling_min_freq from cpu0 (kHz)
#[allow(dead_code)]
pub fn get_min_freq() -> Option<u64> {
    fs::read_to_string("/sys/devices/system/cpu/cpu0/cpufreq/scaling_min_freq")
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

// ── Governor & Turbo ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Governor {
    Performance,
    Powersave,
    Conservative,
    Ondemand,
    Userspace,
    Schedutil,
    Unknown,
}

impl Governor {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Performance => "performance",
            Self::Powersave => "powersave",
            Self::Conservative => "conservative",
            Self::Ondemand => "ondemand",
            Self::Userspace => "userspace",
            Self::Schedutil => "schedutil",
            Self::Unknown => "",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "performance" => Self::Performance,
            "powersave" => Self::Powersave,
            "conservative" => Self::Conservative,
            "ondemand" => Self::Ondemand,
            "userspace" => Self::Userspace,
            "schedutil" => Self::Schedutil,
            _ => Self::Unknown,
        }
    }
}

/// Get the number of CPU cores
pub fn cpu_count() -> usize {
    fs::read_dir("/sys/devices/system/cpu")
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    let name = e.file_name();
                    let name = name.to_string_lossy();
                    name.starts_with("cpu") && name[3..].chars().all(|c| c.is_ascii_digit())
                })
                .count()
        })
        .unwrap_or(1)
}

/// Set governor for all CPU cores
pub fn set_governor_all(governor: Governor) -> std::io::Result<()> {
    let gov_str = governor.as_str();
    if gov_str.is_empty() {
        return Ok(());
    }

    for cpu in 0..cpu_count() {
        let path = format!(
            "/sys/devices/system/cpu/cpu{}/cpufreq/scaling_governor",
            cpu
        );
        fs::write(&path, gov_str)?;
    }
    Ok(())
}

/// Get the sysfs path for turbo boost control.
/// Intel: /sys/devices/system/cpu/intel_pstate/no_turbo (inverted: 0 = turbo on)
/// AMD:   /sys/devices/system/cpu/cpufreq/boost (1 = boost on)
fn turbo_path() -> Option<&'static str> {
    let intel = "/sys/devices/system/cpu/intel_pstate/no_turbo";
    let amd = "/sys/devices/system/cpu/cpufreq/boost";
    if Path::new(intel).exists() {
        Some(intel)
    } else if Path::new(amd).exists() {
        Some(amd)
    } else {
        None
    }
}

/// Detect whether we're on Intel or AMD for turbo control
fn is_intel_turbo() -> bool {
    Path::new("/sys/devices/system/cpu/intel_pstate/no_turbo").exists()
}

/// Get current turbo boost state
#[allow(dead_code)]
pub fn get_turbo() -> Option<bool> {
    let path = turbo_path()?;
    fs::read_to_string(path).ok().map(|s| {
        if is_intel_turbo() {
            s.trim() == "0" // Intel: no_turbo=0 means turbo enabled
        } else {
            s.trim() == "1" // AMD: boost=1 means boost enabled
        }
    })
}

/// Set turbo boost (true = enabled, false = disabled)
/// Supports both Intel (no_turbo) and AMD (boost) sysfs interfaces.
pub fn set_turbo(enabled: bool) -> std::io::Result<()> {
    let path = turbo_path().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "turbo boost control not supported on this CPU",
        )
    })?;

    if is_intel_turbo() {
        // Intel: no_turbo. 0 = turbo on, 1 = turbo off
        fs::write(path, if enabled { "0" } else { "1" })
    } else {
        // AMD: boost. 1 = boost on, 0 = boost off
        fs::write(path, if enabled { "1" } else { "0" })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_profile_not_supported_on_this_machine() {
        // i3-3120M doesn't have platform profile, but the functions shouldn't panic
        let _ = platform_profile_is_supported();
        let _ = platform_profile_current();
        let _ = platform_profile_choices();
    }

    #[test]
    fn test_governor_as_str() {
        assert_eq!(Governor::Performance.as_str(), "performance");
        assert_eq!(Governor::Powersave.as_str(), "powersave");
        assert_eq!(Governor::Unknown.as_str(), "");
    }

    #[test]
    fn test_governor_from_str() {
        assert_eq!(Governor::from_str("performance"), Governor::Performance);
        assert_eq!(Governor::from_str("powersave"), Governor::Powersave);
        assert_eq!(Governor::from_str("  PERFORMANCE  "), Governor::Performance);
        assert_eq!(Governor::from_str("invalid"), Governor::Unknown);
    }

    #[test]
    fn test_epp_as_str() {
        assert_eq!(EnergyPerfPref::Performance.as_str(), "performance");
        assert_eq!(EnergyPerfPref::BalancePower.as_str(), "balance_power");
        assert_eq!(EnergyPerfPref::Unknown.as_str(), "");
    }

    #[test]
    fn test_epp_from_str() {
        assert_eq!(EnergyPerfPref::from_str("performance"), EnergyPerfPref::Performance);
        assert_eq!(EnergyPerfPref::from_str("balance_performance"), EnergyPerfPref::BalancePerformance);
        assert_eq!(EnergyPerfPref::from_str("balance_power"), EnergyPerfPref::BalancePower);
        assert_eq!(EnergyPerfPref::from_str("power"), EnergyPerfPref::Power);
        assert_eq!(EnergyPerfPref::from_str("invalid"), EnergyPerfPref::Unknown);
        assert_eq!(EnergyPerfPref::from_str("  BALANCE_POWER  "), EnergyPerfPref::BalancePower);
    }

    #[test]
    fn test_epp_unknown_produces_empty() {
        let epp = EnergyPerfPref::Unknown;
        assert!(epp.as_str().is_empty());
    }
}
