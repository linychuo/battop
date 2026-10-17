# Battop

Linux battery-aware CPU power management daemon, written in Rust.

## Features

- Automatic CPU governor switching (battery vs charger)
- Turbo boost control
- Power draw monitoring via sysfs
- Single binary, zero runtime dependencies
- ~1MB binary size in release mode

## Build

```bash
cargo build --release
```

## Install

```bash
sudo cp target/release/battop /usr/local/bin/
sudo mkdir -p /etc/battop
sudo cp battop.conf /etc/battop/
```

## Run

```bash
sudo battop
```

## Configuration

Edit `/etc/battop/battop.conf`:

```ini
[battery]
governor = powersave
turbo = never

[charger]
governor = performance
turbo = auto

[general]
poll_interval_secs = 5
log_level = info
```
