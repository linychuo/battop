# Battop

> **Linux battery & CPU power management daemon — Rust, zero dependencies, ~1MB binary**

[![Build](https://img.shields.io/badge/build-rust-orange)](https://rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux-blue)](https://kernel.org)
[![Binary](https://img.shields.io/badge/binary-single%20binary-brightgreen)]()
[![Deps](https://img.shields.io/badge/runtime%20dependencies-zero-success)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

English | [简体中文](README.zh.md)

Battop is a Linux battery-aware CPU power management daemon that automatically switches power profiles on AC/battery transitions.

**Why Battop over auto-cpufreq?**

| | Battop (Rust) | auto-cpufreq (Python) |
|---|---|---|
| Memory | ~1 MB | ~40 MB+ |
| Dependencies | zero (single binary) | Python + psutil + pyinotify + bash |
| Governor switching | ✅ | ✅ |
| Turbo control | ✅ | ✅ |
| EPP | ✅ | ✅ |
| Platform Profile | ✅ | ❌ |
| Frequency limits | ✅ | ✅ |
| Charge thresholds | ✅ | ✅ |

---

## Quick Start

```bash
# Build
cargo build --release

# Install
sudo cp target/release/battop /usr/local/bin/
sudo mkdir -p /etc/battop
sudo cp battop.toml /etc/battop/

# Run daemon
sudo battop

# Display live power stats (no daemon needed)
sudo battop --stats
```

## NixOS

```nix
# flake.nix
{
  inputs.battop.url = "github:linychuo/battop";

  outputs = { self, nixpkgs, battop, ... }: {
    nixosConfigurations.myhost = nixpkgs.lib.nixosSystem {
      modules = [
        battop.nixosModules.default
        {
          services.battop = {
            enable = true;
            battery.governor = "powersave";
            battery.turbo = "never";
            charger.governor = "performance";
            charger.turbo = "auto";
            charge_control.stop_threshold = 80;
            charge_control.start_threshold = 70;
          };
        }
      ];
    };
  };
}
```

```bash
sudo nixos-rebuild switch
```

## Features

- **Governor switching** — auto switch CPU governor on AC/battery transitions
- **Turbo control** — disable boost on battery, enable on charger
- **Platform Profile** — firmware-level power profile (CPU + fan + TDP coordination)
- **EPP** — Energy Performance Preference tuning (Intel Skylake+, AMD Zen 2+)
- **Frequency limits** — manually cap CPU frequency range
- **Charge thresholds** — limit max charge (e.g., 80%) to preserve battery health
- **Zero dependencies** — single statically-linked binary, no Python or library deps
- **systemd integration** — watchdog keepalive, graceful shutdown
- **`--stats`** — real-time power/frequency/temperature display

## Configuration

Config file: `~/.config/battop/battop.toml` or `/etc/battop/battop.toml`

```toml
[battery]
governor = "powersave"
turbo = "never"
energy_performance_preference = "balance_power"
platform_profile = "low-power"
max_freq = 2000000  # 2.0 GHz (kHz)

[charger]
governor = "performance"
turbo = "auto"
energy_performance_preference = "balance_performance"
platform_profile = "performance"

[charge_control]
stop_threshold = 80   # stop charging at 80%
start_threshold = 70  # start charging at 70%
behaviour = "auto"

[general]
poll_interval_secs = 5
log_level = "info"
```

### Config Reference

| Option | Values | Default |
|---|---|---|
| `battery/charger.governor` | performance, powersave, conservative, ondemand, userspace, schedutil | — |
| `battery/charger.turbo` | never, auto | — |
| `battery/charger.energy_performance_preference` | performance, balance_performance, balance_power, power | hardware default |
| `battery/charger.platform_profile` | low-power, balanced, balanced-performance, performance, quiet, cool, power | hardware default |
| `battery/charger.max_freq` | integer kHz | hardware default |
| `battery/charger.min_freq` | integer kHz | hardware default |
| `charge_control.stop_threshold` | 0-100 | disabled |
| `charge_control.start_threshold` | 0-100 | disabled |
| `charge_control.behaviour` | auto, inhibit-charge, force-discharge | — |
| `general.poll_interval_secs` | ≥1 | 5 |

## CLI

```
-c, --config <PATH>    Config file path
    --stats            Display live power stats (no daemon needed)
    --dry-run          Validate config and exit
-h, --help             Help
-v, --version          Version
```

### `--stats` Example

```
  Battop 0.1.0 — System Stats

  Power source:   battery (8.2W discharge)
  Battery:        🔋 67% (discharging)

  CPU:
  Governor:       powersave
  Turbo:          off
  Frequency:      1800 MHz
  Freq range:     1200 – 2500 MHz
  Hardware limit: 1200 – 2500 MHz
  EPP:            balance_power
```

## Hardware Compatibility

| Feature | Requirement |
|---|---|
| Governor | All Linux systems |
| Turbo | Intel / AMD CPUs with boost |
| EPP | Intel Skylake+ or AMD Zen 2+ |
| Platform Profile | ~2020+ laptops (ACPI 6.3+) |
| Charge thresholds | Laptop kernel driver with charge_control_threshold |

Unsupported features are auto-detected and silently skipped. Your hardware won't break — battop logs what's available and uses what it can.

## License

MIT
