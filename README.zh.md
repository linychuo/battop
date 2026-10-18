# Battop

> **Linux 电池 & CPU 电源管理 daemon — Rust 实现，零依赖，~1MB**

[![Build](https://img.shields.io/badge/build-rust-orange)](https://rust-lang.org)
[![Platform](https://img.shields.io/badge/platform-linux-blue)](https://kernel.org)
[![Binary](https://img.shields.io/badge/binary-single%20binary-brightgreen)]()
[![Deps](https://img.shields.io/badge/runtime%20dependencies-zero-success)]()
[![License](https://img.shields.io/badge/license-MIT-blue)]()

[English](README.md) | 简体中文

Battop 是一个 Linux 电池感知的 CPU 电源管理守护进程，在插拔电时自动切换电源策略。

**为什么选 Battop 而不是 auto-cpufreq？**

| | Battop (Rust) | auto-cpufreq (Python) |
|---|---|---|
| 内存占用 | ~1 MB | ~40 MB+ |
| 依赖 | 零（单个 binary） | Python + psutil + pyinotify + bash |
| Governor 切换 | ✅ | ✅ |
| Turbo 控制 | ✅ | ✅ |
| EPP | ✅ | ✅ |
| Platform Profile | ✅ | ❌ |
| 频率上下限 | ✅ | ✅ |
| 充电阈值 | ✅ | ✅ |

---

## 快速开始

```bash
# 编译
cargo build --release

# 安装
sudo cp target/release/battop /usr/local/bin/
sudo mkdir -p /etc/battop
sudo cp battop.toml /etc/battop/

# 运行 daemon
sudo battop

# 查看当前功耗状态（不需要 daemon）
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

## 特性

- **Governor 切换** — 电池/插电自动切换 CPU 调度策略
- **Turbo 控制** — 电池时关闭睿频省电，插电时开启
- **Platform Profile** — 固件级整机电源策略（CPU + 风扇 + 功耗墙协调）
- **EPP** — 能效偏好微调（Intel Skylake+, AMD Zen 2+）
- **频率限制** — 手动设定 CPU 频率上下限
- **充电阈值** — 限制充电上限（如 80%）延长电池寿命（ThinkPad 上持久化）
- **零依赖** — 单个静态链接 binary，无 Python/库依赖
- **systemd 集成** — watchdog 保活，优雅关闭
- **`--stats`** — 实时查看功率/频率/温度

## 配置

配置文件：`~/.config/battop/battop.toml` 或 `/etc/battop/battop.toml`

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
stop_threshold = 80   # 充到 80% 停止
start_threshold = 70  # 用到 70% 开始充
behaviour = "auto"

[general]
poll_interval_secs = 5
log_level = "info"
```

### 配置项参考

| 选项 | 值 | 默认 |
|---|---|---|
| `battery/charger.governor` | performance, powersave, conservative, ondemand, userspace, schedutil | — |
| `battery/charger.turbo` | never, auto | — |
| `battery/charger.energy_performance_preference` | performance, balance_performance, balance_power, power | 硬件默认 |
| `battery/charger.platform_profile` | low-power, balanced, balanced-performance, performance, quiet, cool, power | 硬件默认 |
| `battery/charger.max_freq` | 整数 kHz | 硬件默认 |
| `battery/charger.min_freq` | 整数 kHz | 硬件默认 |
| `charge_control.stop_threshold` | 0-100 | 禁用 |
| `charge_control.start_threshold` | 0-100 | 禁用 |
| `charge_control.behaviour` | auto, inhibit-charge, force-discharge | — |
| `general.poll_interval_secs` | ≥1 | 5 |

## CLI

```
-c, --config <PATH>    配置文件路径
    --stats            显示实时功耗状态（不需要 daemon）
    --dry-run          验证配置并退出
-h, --help             帮助
-v, --version          版本
```

### `--stats` 示例输出

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

## 硬件兼容性

| 功能 | 要求 |
|---|---|
| Governor | 所有 Linux 系统 |
| Turbo | Intel / AMD 带睿频的 CPU |
| EPP | Intel Skylake+ 或 AMD Zen 2+ |
| Platform Profile | ~2020 年后的笔记本（ACPI 6.3+） |
| 充电阈值 | 部分笔记本内核驱动支持（charge_control_threshold） |

不支持的功能会在启动时自动检测并跳过，不会影响其他功能。

## 充电阈值持久化

充电阈值的行为取决于笔记本的内核驱动：

| 驱动 | 持久性 | 关机后的行为 |
|---|---|---|
| `thinkpad_acpi`（ThinkPad） | EC NVRAM（持久） | EC 固件遵守阈值，即使关机插电也不会超过限制 |
| 其他驱动 | 仅运行时 | 关机后阈值重置，关机插电可能充到 100% |

### 如何检查

```bash
# 查看哪个内核驱动提供了电池接口
lsmod | grep -E "thinkpad_acpi|battery"

# 查看硬件是否支持充电阈值
ls /sys/class/power_supply/BAT0/charge_control_end_threshold

# 查看当前阈值
cat /sys/class/power_supply/BAT0/charge_control_end_threshold
```

### battop 在不同平台上的行为

- **ThinkPad**：battop 通过 sysfs 写入阈值，`thinkpad_acpi` 存储到 EC NVRAM。设置可以跨越重启、断电、关机充电。
- **其他笔记本**：battop 启动时写入阈值，仅在系统运行时有效。每次启动 battop 都会重新应用阈值。

### 已验证机型

| 型号 | 驱动 | 持久化 |
|---|---|---|
| ThinkPad T14 / T16 / X1 Carbon / P 系列 | `thinkpad_acpi` | ✅ 是 |
| 其他 ThinkPad 机型 | `thinkpad_acpi` | ✅ 是 |
| Dell / HP / ASUS 等 | 视驱动而定 | 请用 `lsmod` 确认 |

> **注意**：即使在非 ThinkPad 笔记本上，battop 仍然能在正常使用时保护电池。限制仅在笔记本关机且插电充电时才会出现。

## License

MIT
