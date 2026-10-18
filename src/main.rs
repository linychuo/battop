mod charge_control;
mod config;
mod cpu_governor;
mod power_supply;

use charge_control::ChargeControl;
use config::Config;
use cpu_governor::{set_epp, set_governor_all, set_max_freq, set_min_freq, set_platform_profile, set_turbo, EnergyPerfPref, Governor};
use power_supply::PowerStatus;

fn print_stats() {
    let supplies = power_supply::detect_supplies();
    let on_battery = power_supply::on_battery_with(&supplies);

    println!("\n  Battop {} — System Stats\n", VERSION);

    // Power source
    let status_str = if on_battery { "battery" } else { "AC/charger" };
    if let Some(power) = power_supply::battery_power_draw_watts(&supplies) {
        if on_battery {
            println!("  Power source:   {} ({:.1}W discharge)", status_str, power);
        } else {
            println!("  Power source:   {} ({:.1}W draw)", status_str, power);
        }
    } else {
        println!("  Power source:   {}", status_str);
    }

    // Battery capacity
    if let Some(cap) = power_supply::battery_capacity(&supplies) {
        let battery = supplies.iter().find(|s| matches!(s.supply_type, power_supply::SupplyType::Battery));
        let status = battery.map(|b| &b.status).unwrap_or(&PowerStatus::Unknown);
        let status_icon = match status {
            PowerStatus::Charging => "⚡",
            PowerStatus::Discharging => "🔋",
            PowerStatus::Full => "✓",
            _ => "",
        };
        println!("  Battery:        {} {}% {}", status_icon, cap, battery_status_str(status));
    }

    // CPU info
    println!("\n  CPU:\n");

    // Governor
    if let Some(gov) = cpu_governor::get_governor() {
        println!("  Governor:       {}", gov);
    } else {
        println!("  Governor:       unavailable");
    }

    // Turbo
    if let Some(turbo) = cpu_governor::get_turbo() {
        println!("  Turbo:          {}", if turbo { "on" } else { "off" });
    }

    // Current frequency
    if let Some(freq) = cpu_governor::get_cur_freq() {
        println!("  Frequency:      {:.0} MHz", freq as f64 / 1000.0);
    }

    // Frequency limits
    if let (Some(min), Some(max)) = (cpu_governor::get_min_freq(), cpu_governor::get_max_freq()) {
        println!("  Freq range:     {:.0} – {:.0} MHz", min as f64 / 1000.0, max as f64 / 1000.0);
    }
    if let (Some(min_limit), Some(max_limit)) = (cpu_governor::freq_min_limit_hz(), cpu_governor::freq_max_limit_hz()) {
        println!("  Hardware limit: {:.0} – {:.0} MHz", min_limit as f64 / 1000.0, max_limit as f64 / 1000.0);
    }

    // EPP
    if EnergyPerfPref::is_supported() {
        if let Some(epp) = cpu_governor::get_epp() {
            if !matches!(epp, EnergyPerfPref::Unknown) {
                println!("  EPP:            {}", epp.as_str());
            }
        }
        println!("  EPP choices:    [{}]", EnergyPerfPref::available_values().join(", "));
    }

    // Platform Profile
    if cpu_governor::platform_profile_is_supported() {
        if let Some(pp) = cpu_governor::platform_profile_current() {
            println!("  Platform Prof:  {}", pp);
        }
        println!("  PP choices:     [{}]", cpu_governor::platform_profile_choices().join(", "));
    }

    println!();
}

fn battery_status_str(status: &PowerStatus) -> &'static str {
    match status {
        PowerStatus::Charging => "(charging)",
        PowerStatus::Discharging => "(discharging)",
        PowerStatus::Full => "(full)",
        PowerStatus::NotCharging => "(not charging)",
        PowerStatus::Unknown => "",
    }
}

use log::info;
use signal_hook::{consts::SIGTERM, iterator::Signals};
use std::os::unix::net::UnixDatagram;
use std::process;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!(
        "battop {} — Linux battery-aware CPU power management daemon

Usage: battop [OPTIONS]

Options:
    -c, --config <PATH>    Config file path (default: ~/.config/battop/battop.toml)
        --stats            Display current system power stats
        --dry-run          Load config and validate, then exit
    -h, --help             Print this help message
    -v, --version          Print version

Config file (TOML format):
    [battery]
    governor = \"powersave\"
    turbo = \"never\"

    [charger]
    governor = \"performance\"
    turbo = \"auto\"

    [charge_control]
    stop_threshold = 80
    start_threshold = 40

    [general]
    poll_interval_secs = 5
    log_level = \"info\"
",
        VERSION
    );
}

fn print_version() {
    println!("battop {}", VERSION);
}

struct CliArgs {
    config_path: Option<String>,
    dry_run: bool,
    stats: bool,
}

fn parse_args() -> Result<CliArgs, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut result = CliArgs {
        config_path: None,
        dry_run: false,
        stats: false,
    };

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                print_help();
                process::exit(0);
            }
            "-v" | "--version" => {
                print_version();
                process::exit(0);
            }
            "-c" | "--config" => {
                i += 1;
                if i >= args.len() {
                    return Err(format!("{} requires a path argument", args[i - 1]));
                }
                result.config_path = Some(args[i].clone());
            }
            "--dry-run" => {
                result.dry_run = true;
            }
            "--stats" => {
                result.stats = true;
            }
            other => {
                return Err(format!("unknown argument: {}", other));
            }
        }
        i += 1;
    }

    Ok(result)
}

fn main() {
    let cli = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            eprintln!("error: {}", e);
            eprintln!("Usage: battop [--help|--version|--config <PATH>|--stats|--dry-run]");
            process::exit(1);
        }
    };

    let config_path = cli
        .config_path
        .unwrap_or_else(Config::default_path);
    let config = Config::from_file(&config_path);

    // Initialize logger with configured level
    if let Ok(level) = config.log_level().parse::<log::LevelFilter>() {
        env_logger::Builder::from_default_env().filter_level(level).init();
    } else {
        log::warn!("invalid log_level '{}', using default", config.log_level());
        env_logger::init();
    }

    info!("battop {} starting...", VERSION);

    if cli.stats {
        print_stats();
        process::exit(0);
    }

    if cli.dry_run {
        info!("dry-run mode: config loaded successfully, exiting");
        info!(
            "config: battery governor={}, turbo={}; charger governor={}, turbo={}",
            config.battery.governor,
            config.battery.turbo,
            config.charger.governor,
            config.charger.turbo
        );
        process::exit(0);
    }

    // Probe charge control hardware capabilities
    let charge = ChargeControl::probe();
    if charge.supported {
        info!(
            "charge control: end_threshold={}, start_threshold={}, behaviour={:?}",
            charge.current_end_threshold.unwrap_or(0),
            charge.current_start_threshold.unwrap_or(0),
            charge.available_behaviours
        );
    } else {
        info!("charge control: not supported on this hardware");
    }

    info!(
        "config: battery governor={}, turbo={}; charger governor={}, turbo={}",
        config.battery.governor, config.battery.turbo,
        config.charger.governor, config.charger.turbo
    );

    // Probe and log hardware capabilities
    info!(
        "hardware: EPP supported={}, available values={:?}",
        EnergyPerfPref::is_supported(),
        EnergyPerfPref::available_values()
    );
    info!(
        "hardware: frequency limits: min={:?} kHz, max={:?} kHz",
        cpu_governor::freq_min_limit_hz(),
        cpu_governor::freq_max_limit_hz()
    );
    info!(
        "hardware: platform profile supported={}, current={:?}, choices={:?}",
        cpu_governor::platform_profile_is_supported(),
        cpu_governor::platform_profile_current(),
        cpu_governor::platform_profile_choices()
    );

    // Check if running on a system without battery (desktop)
    let supplies = power_supply::detect_supplies();
    let has_battery = supplies
        .iter()
        .any(|s| matches!(s.supply_type, power_supply::SupplyType::Battery));
    if !has_battery {
        info!("no battery detected (desktop system), defaulting to battery profile");
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Handle SIGTERM/SIGINT gracefully
    let _signal_handle = Signals::new([SIGTERM, signal_hook::consts::SIGINT]).map(
        |mut signals| {
            let handle = thread::spawn(move || {
                if signals.forever().next().is_some() {
                    info!("received shutdown signal");
                    r.store(false, Ordering::SeqCst);
                }
            });
            info!("signal handlers registered");
            handle
        },
    );

    // Open systemd notification socket (used for both READY and WATCHDOG)
    let notify_socket = sd_notify_get_socket();

    let mut last_on_battery: Option<bool> = None;

    // Notify systemd we're fully ready
    sd_notify_ready(&notify_socket);

    while running.load(Ordering::SeqCst) {
        // Send watchdog keepalive
        sd_notify_watchdog(&notify_socket);

        // Scan sysfs once per poll cycle — pass results to all consumers
        let supplies = power_supply::detect_supplies();
        let battery = power_supply::on_battery_with(&supplies);

        // Only act on state change
        if last_on_battery != Some(battery) {
            last_on_battery = Some(battery);

            if battery {
                info!("switched to battery mode");
                apply_profile(&config.battery, &config.charge_control, &charge);
            } else {
                info!("switched to charger mode");
                apply_profile(&config.charger, &config.charge_control, &charge);
            }
        }

        // Log power draw periodically (use cached supplies)
        if let Some(power) = power_supply::battery_power_draw_watts(&supplies) {
            let capacity = power_supply::battery_capacity(&supplies).unwrap_or(0);
            info!("power: {:.1}W, capacity: {}%", power, capacity);
        }

        thread::sleep(Duration::from_secs(config.poll_interval_secs()));
    }

    // Notify systemd about shutdown
    sd_notify_stopping(&notify_socket);

    info!("battop shutting down, restoring charger profile");
    apply_profile(&config.charger, &config.charge_control, &charge);

    info!("battop stopped");
}

fn apply_profile(
    profile: &config::ProfileConfig,
    charge_control: &config::ChargeControlConfig,
    charge: &ChargeControl,
) {
    let governor = Governor::from_str(&profile.governor);
    info!(
        "applying profile: governor={}, turbo={}",
        profile.governor, profile.turbo
    );

    if matches!(governor, Governor::Unknown) {
        log::error!(
            "refusing to apply profile: unknown governor '{}'",
            profile.governor
        );
        return;
    }

    if let Err(e) = set_governor_all(governor) {
        log::error!("failed to set governor: {}", e);
    }

    // ── Platform Profile (firmware-level power/performance) ──────
    if let Some(ref pp_str) = profile.platform_profile {
        if !cpu_governor::platform_profile_is_supported() {
            log::info!("Platform Profile not supported on this hardware, skipping");
        } else if let Err(e) = set_platform_profile(pp_str) {
            log::warn!("failed to set platform profile '{}': {}", pp_str, e);
        } else {
            info!("set platform profile to '{}'", pp_str);
        }
    }

    // ── EPP (Energy Performance Preference) ──────────────────────
    if let Some(ref epp_str) = profile.energy_performance_preference {
        let epp = EnergyPerfPref::from_str(epp_str);
        if matches!(epp, EnergyPerfPref::Unknown) {
            log::warn!("invalid EPP '{}', skipping", epp_str);
        } else if !EnergyPerfPref::is_supported() {
            log::info!("EPP not supported on this hardware, skipping");
        } else if let Err(e) = set_epp(epp) {
            log::warn!("failed to set EPP '{}': {}", epp_str, e);
        } else {
            info!("set EPP to '{}'", epp_str);
        }
    }

    // ── CPU Frequency Limits ───────────────────────────────────────
    if let Some(freq_khz) = profile.max_freq {
        if let Err(e) = set_max_freq(freq_khz) {
            log::warn!("failed to set max_freq {} kHz: {}", freq_khz, e);
        } else {
            info!("set max CPU frequency to {} MHz", freq_khz / 1000);
        }
    }
    if let Some(freq_khz) = profile.min_freq {
        if let Err(e) = set_min_freq(freq_khz) {
            log::warn!("failed to set min_freq {} kHz: {}", freq_khz, e);
        } else {
            info!("set min CPU frequency to {} MHz", freq_khz / 1000);
        }
    }

    match profile.turbo.as_str() {
        "never" => {
            if let Err(e) = set_turbo(false) {
                log::warn!("failed to disable turbo: {}", e);
            }
        }
        "auto" => {
            if let Err(e) = set_turbo(true) {
                log::warn!("failed to enable turbo: {}", e);
            }
        }
        other => {
            log::warn!(
                "invalid turbo value '{}', expected 'never' or 'auto', skipping",
                other
            );
        }
    }

    // Charge thresholds are global — they only take effect when plugged in,
    // so we apply them on every profile switch.
    apply_charge_thresholds(charge_control, charge);
}

fn apply_charge_thresholds(cc: &config::ChargeControlConfig, charge: &ChargeControl) {
    if !charge.supported {
        return;
    }

    if let Some(threshold) = cc.stop_threshold
        && charge.has_end_threshold
    {
        if let Err(e) = charge.set_end_threshold(threshold) {
            log::warn!("failed to set charge stop threshold: {}", e);
        } else {
            info!("charge stop threshold set to {}%", threshold);
        }
    }

    if let Some(threshold) = cc.start_threshold
        && charge.has_start_threshold
    {
        if let Err(e) = charge.set_start_threshold(threshold) {
            log::warn!("failed to set charge start threshold: {}", e);
        } else {
            info!("charge start threshold set to {}%", threshold);
        }
    }

    if let Some(ref behaviour) = cc.behaviour
        && charge.has_behaviour
    {
        if let Err(e) = charge.set_behaviour(behaviour) {
            log::warn!("failed to set charge behaviour: {}", e);
        } else {
            info!("charge behaviour set to {}", behaviour);
        }
    }
}

/// Get the systemd notification socket address from NOTIFY_SOCKET env var.
/// Returns None if NOTIFY_SOCKET is not set or not a valid path.
fn sd_notify_get_socket() -> Option<std::path::PathBuf> {
    std::env::var("NOTIFY_SOCKET").ok().map(std::path::PathBuf::from)
}

/// Send a notification message via the systemd notification socket.
fn sd_notify_send(addr: &std::path::Path, msg: &[u8]) {
    if let Ok(socket) = UnixDatagram::unbound() {
        if socket.send_to(msg, addr).is_err() {
            log::warn!("failed to send sd_notify message");
        }
    }
}

/// Notify systemd that the service is ready.
fn sd_notify_ready(addr: &Option<std::path::PathBuf>) {
    if let Some(addr) = addr {
        sd_notify_send(addr, b"READY=1\n");
        info!("notified systemd: READY=1");
    }
}

/// Send watchdog keepalive to systemd.
fn sd_notify_watchdog(addr: &Option<std::path::PathBuf>) {
    if let Some(addr) = addr {
        sd_notify_send(addr, b"WATCHDOG=1\n");
    }
}

/// Notify systemd that the service is stopping.
fn sd_notify_stopping(addr: &Option<std::path::PathBuf>) {
    if let Some(addr) = addr {
        sd_notify_send(addr, b"STOPPING=1\n");
        info!("notified systemd: STOPPING=1");
    }
}
