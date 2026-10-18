# Battop

Linux battery-aware CPU power management daemon, written in Rust.

## Features

- Automatic CPU governor switching (battery vs charger)
- Turbo boost control (Intel & AMD)
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

[charger]
governor = "performance"
turbo = "auto"
charge_stop_threshold = 80
charge_start_threshold = 40

[general]
poll_interval_secs = 5
log_level = "info"
```

### Config Reference

| Section | Key | Values | Default |
|---------|-----|--------|---------|
| battery/charger | governor | performance, powersave, conservative, ondemand, userspace, schedutil | — |
| battery/charger | turbo | never, auto | — |
| battery/charger | charge_stop_threshold | 0-100 (optional) | none |
| battery/charger | charge_start_threshold | 0-100 (optional) | none |
| battery/charger | charge_behaviour | auto, inhibit-charge, force-discharge | none |
| general | poll_interval_secs | 1+ | 5 |
| general | log_level | error, warn, info, debug, trace | info |
