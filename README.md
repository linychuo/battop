# Battop

Linux battery-aware CPU power management daemon, written in Rust.

## Features

- Automatic CPU governor switching (battery vs charger)
- Turbo boost control (Intel & AMD)
- Energy Performance Preference (EPP) control (Intel Skylake+, AMD Zen 2+)
- CPU frequency limit control (scaling_min_freq / scaling_max_freq)
- Power draw monitoring via sysfs
- Charge threshold control
- Single binary, zero runtime dependencies

## Build

```bash
cargo build --release
```

## Install

```bash
sudo cp target/release/battop /usr/local/bin/
sudo mkdir -p /etc/battop
sudo cp battop.toml /etc/battop/
```

## Run

```bash
sudo battop
```

### CLI Options

```
-c, --config <PATH>    Config file path (default: ~/.config/battop/battop.toml)
    --dry-run          Load config and validate, then exit
-h, --help             Print help message
-v, --version          Print version
```

## Configuration

Edit `~/.config/battop/battop.toml` (or `/etc/battop/battop.toml`):

```toml
[battery]
governor = "powersave"
turbo = "never"
energy_performance_preference = "balance_power"
max_freq = 2000000  # 2.0 GHz in kHz

[charger]
governor = "performance"
turbo = "auto"
energy_performance_preference = "balance_performance"

[charge_control]
stop_threshold = 80
start_threshold = 40

[general]
poll_interval_secs = 5
log_level = "info"
```

### Config Reference

| Section | Key | Values | Default |
|---------|-----|--------|---------|
| battery/charger | governor | performance, powersave, conservative, ondemand, userspace, schedutil | — |
| battery/charger | turbo | never, auto | — |
| battery/charger | energy_performance_preference | performance, balance_performance, balance_power, power | unset (hardware default) |
| battery/charger | max_freq | integer kHz | unset (hardware default) |
| battery/charger | min_freq | integer kHz | unset (hardware default) |
| charge_control | stop_threshold | 0-100 (optional) | none |
| charge_control | start_threshold | 0-100 (optional) | none |
| charge_control | behaviour | auto, inhibit-charge, force-discharge | none |
| general | poll_interval_secs | 1+ | 5 |
| general | log_level | error, warn, info, debug, trace | info |

**Notes:**
- `energy_performance_preference` requires hardware support (Intel Skylake+ or AMD Zen 2+). Battop logs a message and skips if unsupported.
- `max_freq` and `min_freq` are in kHz (e.g., `2000000` = 2.0 GHz). Values must be within `cpuinfo_min_freq`–`cpuinfo_max_freq` range.
- EPP and frequency limits are only applied when the corresponding options are set in config.
