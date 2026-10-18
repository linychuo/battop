use std::fs;
use std::path::Path;

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
}
