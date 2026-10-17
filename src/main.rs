mod charge_control;
mod config;
mod cpu_governor;
mod power_supply;

use charge_control::ChargeControl;
use config::Config;
use cpu_governor::{Governor, set_governor_all, set_turbo};
use power_supply::on_battery;

use log::info;
use signal_hook::{consts::SIGTERM, iterator::Signals};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    let config = Config::from_file(&Config::default_path());

    // Initialize logger with configured level
    if let Ok(level) = config.log_level.parse::<log::LevelFilter>() {
        env_logger::Builder::from_default_env().filter_level(level).init();
    } else {
        env_logger::init();
    }

    info!("battop starting...");

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

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    // Handle SIGTERM/SIGINT gracefully
    let mut signals = Signals::new([SIGTERM, signal_hook::consts::SIGINT]).unwrap();
    thread::spawn(move || {
        for _ in signals.forever() {
            info!("received shutdown signal");
            r.store(false, Ordering::SeqCst);
            break;
        }
    });

    let mut last_on_battery: Option<bool> = None;

    while running.load(Ordering::SeqCst) {
        let battery = on_battery();

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

        // Log power draw periodically
        if let Some(power) = power_supply::battery_power_draw_watts() {
            info!(
                "power: {:.1}W, capacity: {}%",
                power,
                power_supply::battery_capacity().unwrap_or(0)
            );
        }

        thread::sleep(Duration::from_secs(config.poll_interval_secs));
    }

    info!("battop shutting down");
}

fn apply_profile(profile: &config::ProfileConfig, charge: &ChargeControl) {
    let governor = Governor::from_str(&profile.governor);
    info!(
        "applying profile: governor={}, turbo={}",
        profile.governor, profile.turbo
    );

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
        _ => {}
    }

    // Apply charge thresholds if hardware supports it
    apply_charge_thresholds(profile, charge);
}

fn apply_charge_thresholds(profile: &config::ProfileConfig, charge: &ChargeControl) {
    if !charge.supported {
        return;
    }

    if let Some(threshold) = profile.charge_stop_threshold {
        if charge.has_end_threshold {
            if let Err(e) = ChargeControl::set_end_threshold(threshold) {
                log::warn!("failed to set charge stop threshold: {}", e);
            } else {
                info!("charge stop threshold set to {}%", threshold);
            }
        }
    }

    if let Some(threshold) = profile.charge_start_threshold {
        if charge.has_start_threshold {
            if let Err(e) = ChargeControl::set_start_threshold(threshold) {
                log::warn!("failed to set charge start threshold: {}", e);
            } else {
                info!("charge start threshold set to {}%", threshold);
            }
        }
    }

    if let Some(ref behaviour) = profile.charge_behaviour {
        if charge.has_behaviour {
            if let Err(e) = ChargeControl::set_behaviour(behaviour) {
                log::warn!("failed to set charge behaviour: {}", e);
            } else {
                info!("charge behaviour set to {}", behaviour);
            }
        }
    }
}
