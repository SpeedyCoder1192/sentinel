# sentinel

A lightweight, zero-dependency background system monitoring daemon written in Rust. It tracks system resource thresholds (CPU and RAM) and sends alerts directly to a Discord channel via webhooks. It can run as an instant zero-config binary or you can install it for a systemwide systemd service.

## Features

Sentinel runs out of the box with zero configuration by taking a raw webhook URL directly as a command-line argument. For power users, it scales automatically into advanced monitoring profiles through a declarative config.toml layout. The binary contains built-in deployment hooks that automatically copy itself into the global system path and register it as a persistent background systemd service. It also includes a built-in terminal setup utility that guides you through creating or modifying your configuration files and webhook targets interactively.

## System Requirements

* An internet connection.
* Linux distro that uses systemd, OpenRC, runit, or SysVinit (for --install, using the compiled binary doesn't need this requirement).
* Sudo access is strictly required for --install, --uninstall, and global config writing routines due to root directory modifications (/usr/local/bin and /etc/).

## Installation

### 1. Build from Source
Ensure you have the Rust toolchain installed, then clone and compile the optimized production release binary:
```bash
cargo build --release
```

### 2. Register Globally (systemd)
To install the binary permanently into your system \$PATH (/usr/local/bin/sentinel) and automatically configure it as a background service, run the install flag with root privileges:
```bash
sudo ./sentinel --install
```

### 3. Configure Parameters
Once installed, run the interactive configuration engine systemwide to apply or update your Discord target metrics:
```bash
sudo sentinel config
```
## Usage
```bash
sentinel [webhook_url]   Run manually in the current terminal session
sentinel config          Launch interactive configuration manager
sentinel --status        Collect current vitals and push an on-demand report to Discord
sentinel --install       Deploy binary globally and setup background systemd service
sentinel --uninstall     Stop service and strip all sentinel traces from the host system
sentinel --help          Display available execution arguments
```
## Configuration

When running via systemd, parameters are managed inside /etc/sentinel/config.toml. If it does not exist, you can create it manually or let sentinel config generate it:
```toml
check_interval_secs = 60
cpu_threshold_pct = 90.0
ram_threshold_pct = 90.0
webhook_url = "https://discord.com/api/webhooks/your_webhook_string"
```
