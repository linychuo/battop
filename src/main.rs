mod charge_control;
mod config;
mod cpu_governor;
mod power_supply;

use charge_control::ChargeControl;
use config::Config;
use cpu_governor::{set_governor_all, set_turbo, Governor};


use log::info;
use signal_hook::{consts::SIGTERM, iterator::Signals};
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
        --dry-run          Load config and validate, then exit
    -h, --help             Print this help message
    -v, --version          Print version

Config file (TOML format):
    [battery]
    governor = \"powersave\"
    turbo = \"never\"
    charge_stop_threshold = 80

    [charger]
    governor = \"performance\"
    turbo = \"auto\"
    charge_stop_threshold = 100

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
}

fn parse_args() -> Result<CliArgs, String> {
    let args: Vec<String> = std::env::args().collect();
    let mut result = CliArgs {
        config_path: None,
        dry_run: false,
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
            eprintln!("Usage: battop [--help|--version|--config <PATH>|--dry-run]");
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

    // Notify systemd we're ready
    sd_notify_ready();

    let mut last_on_battery: Option<bool> = None;

    while running.load(Ordering::SeqCst) {
        // Scan sysfs once per poll cycle — pass results to all consumers
        let supplies = power_supply::detect_supplies();
        let battery = power_supply::on_battery_with(&supplies);

        // Only act on state change
        if last_on_battery != Some(battery) {
            last_on_battery = Some(battery);

            if battery {
                info!("switched to battery mode");
                apply_profile(&config.battery, &charge);
            } else {
                info!("switched to charger mode");
                apply_profile(&config.charger, &charge);
            }
        }

        // Log power draw periodically (use cached supplies)
        if let Some(power) = power_supply::battery_power_draw_watts(&supplies) {
            let capacity = power_supply::battery_capacity(&supplies).unwrap_or(0);
            info!("power: {:.1}W, capacity: {}%", power, capacity);
        }

        thread::sleep(Duration::from_secs(config.poll_interval_secs()));
    }

    info!("battop shutting down, restoring charger profile");
    apply_profile(&config.charger, &charge);

    info!("battop stopped");
}

fn apply_profile(profile: &config::ProfileConfig, charge: &ChargeControl) {
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

    // Apply charge thresholds if hardware supports it
    apply_charge_thresholds(profile, charge);
}

fn apply_charge_thresholds(profile: &config::ProfileConfig, charge: &ChargeControl) {
    if !charge.supported {
        return;
    }

    if let Some(threshold) = profile.charge_stop_threshold
        && charge.has_end_threshold
    {
        if let Err(e) = charge.set_end_threshold(threshold) {
            log::warn!("failed to set charge stop threshold: {}", e);
        } else {
            info!("charge stop threshold set to {}%", threshold);
        }
    }

    if let Some(threshold) = profile.charge_start_threshold
        && charge.has_start_threshold
    {
        if let Err(e) = charge.set_start_threshold(threshold) {
            log::warn!("failed to set charge start threshold: {}", e);
        } else {
            info!("charge start threshold set to {}%", threshold);
        }
    }

    if let Some(ref behaviour) = profile.charge_behaviour
        && charge.has_behaviour
    {
        if let Err(e) = charge.set_behaviour(behaviour) {
            log::warn!("failed to set charge behaviour: {}", e);
        } else {
            info!("charge behaviour set to {}", behaviour);
        }
    }
}

/// Notify systemd that the service is ready.
/// Uses the systemd notification socket (NOTIFY_SOCKET).
fn sd_notify_ready() {
    if let Ok(socket) = std::env::var("NOTIFY_SOCKET") {
        let addr = std::path::Path::new(&socket);
        if let Ok(socket) = std::os::unix::net::UnixDatagram::unbound() {
            if socket.send_to(b"READY=1\n", addr).is_ok() {
                info!("notified systemd: READY=1");
            } else {
                log::warn!("failed to send sd_notify READY");
            }
        }
    }
}
