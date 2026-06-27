# sentinel

![Rust](https://img.shields.io/badge/Rust-%23000000.svg?style=flat&logo=rust&logoColor=white)
![Linux](https://img.shields.io/badge/platform-linux-lightgrey?style=flat-square&logo=linux)
![macOS](https://img.shields.io/badge/platform-macos-lightgrey?style=flat-square&logo=apple)
![Lines of Code](https://sloc.xyz/github/SpeedyCoder1192/sentinel)

![systemd](https://img.shields.io/badge/init-systemd-blue?style=flat-square)
![OpenRC](https://img.shields.io/badge/init-OpenRC-blue?style=flat-square)
![runit](https://img.shields.io/badge/init-runit-blue?style=flat-square)
![SysVinit](https://img.shields.io/badge/init-SysVinit-blue?style=flat-square)
![launchd](https://img.shields.io/badge/init-launchd-blue?style=flat-square)

---

A lightweight, zero-dependency background system monitoring daemon written in Rust. It tracks system resource levels (CPU and RAM) and sends alerts directly to a Discord channel via webhooks. It can run as an instant zero-config binary or you can install it for a systemwide systemd, OpenRC, runit, SysVinit, or launchd service.

## Features

Sentinel runs out of the box with zero configuration by taking a raw webhook URL directly as a command-line argument. For power users, it scales automatically into advanced monitoring profiles through a `config.toml`. You can install the binary to automatically copy itself into the global system path and register it as a persistent background system service. It also includes a built-in terminal setup utility that guides you through creating or modifying your configuration files and webhooks.

## System Requirements

* An internet connection.
* Linux distro that uses systemd, OpenRC, runit, or SysVinit, or macOS utilizing launchd (for --install only, using the compiled binary doesn't need this requirement).
* Sudo access is strictly required for --install, --uninstall, and global config writing routines due to root directory modifications (/usr/local/bin and /etc/).

## Installation

### Method 1: Pre-built Binaries
You can download the pre-compiled binary for your specific architecture and operating system directly from the [releases](https://github.com/SpeedyCoder1192/sentinel/releases) page.

### Method 2: Build from Source
Ensure you have the Rust toolchain installed, then clone and compile the binary:
```bash
cargo build --release
```

### 3. Register Globally
To install the binary permanently into your system \$PATH (`/usr/local/bin/sentinel`) and automatically configure it as a background service, run the install flag with root privileges:
```bash
sudo ./sentinel --install
```

### 4. Configure Parameters
Once installed, run the configuration command systemwide to apply or update your Discord target metrics:
```bash
sudo sentinel config
```

## Usage
```bash
sentinel [webhook_url]   Run manually in the current terminal session
sentinel config          Launch interactive configuration manager
sentinel --install       Deploy binary globally and setup background init service
sentinel --uninstall     Stop service and strip all sentinel traces from the host system
sentinel --help          Display available execution arguments
```

## Configuration

When running via an init system, parameters are managed inside `/etc/sentinel/config.toml`. If it does not exist, you can create it manually or let sentinel config generate it:
```toml
check_interval_secs = 60
cpu_threshold_pct = 90.0
ram_threshold_pct = 90.0
webhook_url = "https://discord.com/api/webhooks/your_webhook_string"
```

## Bugs and Suggestions

If you find any bugs or have suggestions to improve the tool, feel free to get in touch:
* Open an [Issue](https://github.com/speedycoder1192/sentinel/issues/new).
* Message me on [Discord](https://discordapp.com/users/937723659911581777).
* Email me at: https://mail.speedycoder1192.qzz.io/sentinel
