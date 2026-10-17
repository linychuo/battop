use std::fs;
use std::path::Path;

const CPUFREQ_PATH: &str = "/sys/devices/system/cpu/cpu0/cpufreq";

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
                    e.file_name()
                        .to_string_lossy()
                        .starts_with("cpu") && e.file_name().to_string_lossy().len() > 3
                })
                .count()
        })
        .unwrap_or(1)
}

/// Get available governors for the CPU
pub fn available_governors() -> Vec<Governor> {
    let path = Path::new(CPUFREQ_PATH).join("scaling_available_governors");
    fs::read_to_string(path)
        .map(|s| s.split_whitespace().map(Governor::from_str).collect())
        .unwrap_or_default()
}

/// Get current governor for a specific CPU core
pub fn get_governor(cpu: usize) -> Option<Governor> {
    let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_governor", cpu);
    fs::read_to_string(&path).ok().map(|s| Governor::from_str(&s))
}

/// Set governor for all CPU cores
pub fn set_governor_all(governor: Governor) -> std::io::Result<()> {
    let gov_str = governor.as_str();
    if gov_str.is_empty() {
        return Ok(());
    }

    for cpu in 0..cpu_count() {
        let path = format!("/sys/devices/system/cpu/cpu{}/cpufreq/scaling_governor", cpu);
        fs::write(&path, gov_str)?;
    }
    Ok(())
}

/// Get current turbo boost state
pub fn get_turbo() -> Option<bool> {
    let path = "/sys/devices/system/cpu/intel_pstate/no_turbo";
    fs::read_to_string(path).ok().map(|s| s.trim() == "0")
}

/// Set turbo boost (true = enabled, false = disabled)
pub fn set_turbo(enabled: bool) -> std::io::Result<()> {
    let path = "/sys/devices/system/cpu/intel_pstate/no_turbo";
    if enabled {
        fs::write(path, "0")
    } else {
        fs::write(path, "1")
    }
}
