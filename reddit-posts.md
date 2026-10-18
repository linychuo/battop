## r/linux 中文版

标题：Battop — Rust 写的 Linux 电池 & CPU 电源管理 daemon，auto-cpufreq 的零依赖替代品

最近在用的电源管理工具 Battop 更新了不少功能，觉得可以分享一下。

背景：我一直用 auto-cpufreq 管理笔记本功耗，但它依赖 Python + psutil + pyinotify + bash helper，内存常驻 ~40MB。后来切到了 Battop — 一个 Rust 写的同功能 daemon，单个 binary，零运行时依赖，常驻 ~1MB。

**Battop 做的事：**
- 插电/拔电自动切换 CPU governor（performance ↔ powersave）
- Turbo boost 控制（电池关、插电开）
- EPP（Energy Performance Preference，能效偏好微调）
- Platform Profile（固件级整机电源策略，协调 CPU + 风扇 + 功耗墙）
- CPU 频率上下限
- 充电阈值（比如限制充到 80% 保护电池）
- `--stats` 子命令实时看功耗/频率/状态

**内存对比：**
- Battop: 368KB resident
- auto-cpufreq: ~40MB+

**硬件兼容：** 自动探测，不支持的功能静默跳过。比如我的老 i3 没有 EPP 和 Platform Profile，但 governor + turbo + 充电阈值照常工作。

**NixOS 支持：** flake 里加几行配置就行，有完整的 NixOS module。

GitHub: https://github.com/linychuo/battop

核心功能覆盖了 auto-cpufreq 的全部能力，还没有 Platform Profile 支持（auto-cpufreq 没有这个）。如果你也在找一个轻量的 Linux 电源管理方案，可以试试。

---

## r/linux English Version

Title: Battop — battery-aware CPU power manager in Rust, zero-dependency alternative to auto-cpufreq

I've been using Battop for Linux power management and wanted to share it with the community.

Context: I was using auto-cpufreq on my laptop but it pulls in Python + psutil + pyinotify + a bash helper and idles at ~40MB RSS. Battop does the same job in Rust as a single binary with zero runtime dependencies at ~1MB.

**What Battop does:**
- Auto CPU governor switching on AC/battery (performance ↔ powersave)
- Turbo boost control (off on battery, auto on charger)
- EPP (Energy Performance Preference) tuning
- Platform Profile (firmware-level power profile coordinating CPU + fans + TDP)
- CPU frequency limits (min/max)
- Charge thresholds (e.g., stop at 80% to preserve battery)
- `--stats` for real-time power/frequency display

**Memory comparison:**
- Battop: 368KB RSS
- auto-cpufreq: ~40MB+

**Hardware compat:** Auto-detected. Unsupported features (like EPP/Platform Profile on older hardware) are silently skipped — battop logs what's available and applies what it can.

**NixOS:** Full module available, just add a few lines to your flake.

GitHub: https://github.com/linychuo/battop

Feature-complete vs auto-cpufreq (and adds Platform Profile, which auto-cpufreq doesn't have). If you're looking for a lightweight Linux power manager, give it a look.
