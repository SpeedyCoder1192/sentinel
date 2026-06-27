use serde::Deserialize;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
use std::time::Duration;
use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};

const BINARY_GLOBAL_PATH: &str = "/usr/local/bin/sentinel";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InitSystem {
    Systemd,
    OpenRc,
    Runit,
    SysVinit,
    Launchd,
    Unknown,
}

fn detect_init_system() -> InitSystem {
    if cfg!(target_os = "macos") {
        return InitSystem::Launchd;
    }

    if Path::new("/run/systemd/system").exists()
        || Command::new("pidof")
            .arg("systemd")
            .output()
            .map(|o| !o.stdout.is_empty())
            .unwrap_or(false)
    {
        InitSystem::Systemd
    } else if Path::new("/sbin/openrc-run").exists()
        || Path::new("/etc/init.d/functions.sh").exists()
    {
        InitSystem::OpenRc
    } else if Path::new("/etc/runit").exists() || Path::new("/var/service").exists() {
        InitSystem::Runit
    } else if Path::new("/etc/init.d").exists() && !Path::new("/run/systemd/system").exists() {
        InitSystem::SysVinit
    } else {
        InitSystem::Unknown
    }
}

#[derive(Debug, Deserialize, Clone, serde::Serialize)]
struct Config {
    #[serde(default = "default_interval")]
    check_interval_secs: u64,
    #[serde(default = "default_cpu_threshold")]
    cpu_threshold_pct: f32,
    #[serde(default = "default_ram_threshold")]
    ram_threshold_pct: f32,
    webhook_url: String,
}

fn default_interval() -> u64 {
    60
}
fn default_cpu_threshold() -> f32 {
    90.0
}
fn default_ram_threshold() -> f32 {
    90.0
}

#[derive(serde::Serialize)]
struct DiscordPayload {
    embeds: Vec<DiscordEmbed>,
}

#[derive(serde::Serialize)]
struct DiscordEmbed {
    title: String,
    description: String,
    color: u32,
    fields: Vec<EmbedField>,
}

#[derive(serde::Serialize)]
struct EmbedField {
    name: String,
    value: String,
    inline: bool,
}

async fn send_discord_alert(
    webhook_url: &str,
    title: &str,
    description: &str,
    color: u32,
    cpu: f32,
    ram: f32,
) {
    let client = reqwest::Client::new();
    let payload = DiscordPayload {
        embeds: vec![DiscordEmbed {
            title: title.to_string(),
            description: description.to_string(),
            color,
            fields: vec![
                EmbedField {
                    name: "Current CPU".to_string(),
                    value: format!("{:.2}%", cpu),
                    inline: true,
                },
                EmbedField {
                    name: "Current RAM".to_string(),
                    value: format!("{:.2}%", ram),
                    inline: true,
                },
            ],
        }],
    };
    let _ = client.post(webhook_url).json(&payload).send().await;
}

async fn send_startup_notification(webhook_url: &str) {
    let client = reqwest::Client::new();
    let payload = DiscordPayload {
        embeds: vec![DiscordEmbed {
            title: "Sentinel Online".to_string(),
            description: "The background resource monitoring daemon has started successfully."
                .to_string(),
            color: 3066993,
            fields: vec![],
        }],
    };
    let _ = client.post(webhook_url).json(&payload).send().await;
}

fn print_help() {
    println!("Sentinel Resource Monitor");
    println!("Usage:");
    println!("  sentinel [webhook_url]   Run manually using a direct webhook URL");
    println!("  sentinel [command]       Execute utility operations\n");
    println!("Commands:");
    println!("  config                   Configure webhook URL and thresholds interactively");
    println!("  --install                Auto-detect init framework and deploy background service (requires 'sudo')");
    println!("  --uninstall              Strip service configurations and remove sentinel paths (requires 'sudo')");
    println!("  --help                   Display execution options and flags");
}

fn handle_installation() {
    println!("[*] Checking environment privileges...");
    if !bits_admin::is_admin() {
        eprintln!("[x] Error: Root privileges required for system installation. Run with 'sudo'.");
        std::process::exit(1);
    }

    let init = detect_init_system();
    println!("[*] Target system detection results: {:?}", init);

    if init == InitSystem::Unknown {
        eprintln!("[x] Error: Unsupported init architecture. Sentinel handles systemd, OpenRC, runit, SysVinit, and launchd.");
        std::process::exit(1);
    }

    let current_exe = env::current_exe().expect("[x] Failed to get current binary path");
    if let Err(e) = fs::copy(&current_exe, BINARY_GLOBAL_PATH) {
        eprintln!("[x] Failed to copy binary to {}: {}", BINARY_GLOBAL_PATH, e);
        std::process::exit(1);
    }
    println!("[*] Binary deployed to {}", BINARY_GLOBAL_PATH);

    let _ = fs::create_dir_all("/etc/sentinel");

    match init {
        InitSystem::Launchd => {
            let plist_content = format!(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
                 <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
                 <plist version=\"1.0\">\n\
                 <dict>\n\
                 \t<key>Label</key>\n\
                 \t<string>com.speedycoder.sentinel</string>\n\
                 \t<key>ProgramArguments</key>\n\
                 \t<array>\n\
                 \t\t<string>{}</string>\n\
                 \t</array>\n\
                 \t<key>WorkingDirectory</key>\n\
                 \t<string>/etc/sentinel</string>\n\
                 \t<key>RunAtLoad</key>\n\
                 \t<true/>\n\
                 \t<key>KeepAlive</key>\n\
                 \t<true/>\n\
                 </dict>\n\
                 </plist>\n",
                BINARY_GLOBAL_PATH
            );
            fs::write(
                "/Library/LaunchDaemons/com.speedycoder.sentinel.plist",
                plist_content,
            )
            .unwrap();
            let _ = Command::new("launchctl")
                .arg("load")
                .arg("/Library/LaunchDaemons/com.speedycoder.sentinel.plist")
                .status();
        }
        InitSystem::Systemd => {
            let service_content = format!(
                "[Unit]\nDescription=Sentinel System Monitor\nAfter=network.target\n\n\
                 [Service]\nType=simple\nExecStart={}\nWorkingDirectory=/etc/sentinel\nRestart=always\n\n\
                 [Install]\nWantedBy=multi-user.target\n",
                BINARY_GLOBAL_PATH
            );
            fs::write("/etc/systemd/system/sentinel.service", service_content).unwrap();
            let _ = Command::new("systemctl").arg("daemon-reload").status();
            let _ = Command::new("systemctl")
                .arg("enable")
                .arg("sentinel")
                .status();
            let _ = Command::new("systemctl")
                .arg("start")
                .arg("sentinel")
                .status();
        }
        InitSystem::OpenRc => {
            let script_content = format!(
                "#!/sbin/openrc-run\n\
                 description=\"Sentinel System Monitor Daemon\"\n\
                 command=\"{}\"\n\
                 command_background=\"true\"\n\
                 pidfile=\"/run/sentinel.pid\"\n\
                 working_directory=\"/etc/sentinel\"\n\
                 depend() {{\n    need net\n}}\n",
                BINARY_GLOBAL_PATH
            );
            fs::write("/etc/init.d/sentinel", script_content).unwrap();
            let _ = Command::new("chmod")
                .arg("+x")
                .arg("/etc/init.d/sentinel")
                .status();
            let _ = Command::new("rc-update")
                .arg("add")
                .arg("sentinel")
                .arg("default")
                .status();
            let _ = Command::new("rc-service")
                .arg("sentinel")
                .arg("start")
                .status();
        }
        InitSystem::Runit => {
            let sv_dir = if Path::new("/etc/sv").exists() {
                "/etc/sv/sentinel"
            } else {
                "/etc/runit/sv/sentinel"
            };
            let _ = fs::create_dir_all(sv_dir);
            let run_script = format!(
                "#!/bin/sh\nexec 2>&1\ncd /etc/sentinel\nexec {}\n",
                BINARY_GLOBAL_PATH
            );
            let run_path = format!("{}/run", sv_dir);
            fs::write(&run_path, run_script).unwrap();
            let _ = Command::new("chmod").arg("+x").arg(&run_path).status();

            let service_link = if Path::new("/var/service").exists() {
                "/var/service/sentinel"
            } else {
                "/service/sentinel"
            };
            let _ = Command::new("ln")
                .arg("-s")
                .arg(sv_dir)
                .arg(service_link)
                .status();
        }
        InitSystem::SysVinit => {
            let script_content = format!(
                "#!/bin/sh\n### BEGIN INIT INFO\n# Provides: sentinel\n# Required-Start: $network\n# Default-Start: 2 3 4 5\n### END INIT INFO\n\
                 case \"$1\" in\n  start)\n    echo \"Starting sentinel...\"\n    cd /etc/sentinel && {} > /dev/null 2>&1 &\n    ;;\n\
                    stop)\n    echo \"Stopping sentinel...\"\n    pkill -f {}\n    ;;\n  *)\n    echo \"Usage: /etc/init.d/sentinel {{start|stop}}\"\n    exit 1\nesac\nexit 0\n",
                BINARY_GLOBAL_PATH, BINARY_GLOBAL_PATH
            );
            fs::write("/etc/init.d/sentinel", script_content).unwrap();
            let _ = Command::new("chmod")
                .arg("+x")
                .arg("/etc/init.d/sentinel")
                .status();
            if Command::new("update-rc.d")
                .arg("sentinel")
                .arg("defaults")
                .status()
                .is_err()
            {
                let _ = Command::new("chkconfig")
                    .arg("--add")
                    .arg("sentinel")
                    .status();
            }
            let _ = Command::new("/etc/init.d/sentinel").arg("start").status();
        }
        _ => {}
    }

    println!(
        "[*] Sentinel service activated successfully for framework: {:?}",
        init
    );
    println!("[*] Run 'sudo sentinel config' to finalize parameters.");
}

fn handle_uninstallation() {
    println!("[*] Disabling and removing sentinel daemon configuration profiles...");
    if !bits_admin::is_admin() {
        eprintln!("[x] Error: Root privileges required for uninstallation. Run with 'sudo'.");
        std::process::exit(1);
    }

    let init = detect_init_system();

    match init {
        InitSystem::Launchd => {
            let plist_path = "/Library/LaunchDaemons/com.speedycoder.sentinel.plist";
            let _ = Command::new("launchctl")
                .arg("unload")
                .arg(plist_path)
                .status();
            let _ = fs::remove_file(plist_path);
        }
        InitSystem::Systemd => {
            let _ = Command::new("systemctl")
                .arg("stop")
                .arg("sentinel")
                .status();
            let _ = Command::new("systemctl")
                .arg("disable")
                .arg("sentinel")
                .status();
            let _ = fs::remove_file("/etc/systemd/system/sentinel.service");
            let _ = Command::new("systemctl").arg("daemon-reload").status();
        }
        InitSystem::OpenRc => {
            let _ = Command::new("rc-service")
                .arg("sentinel")
                .arg("stop")
                .status();
            let _ = Command::new("rc-update")
                .arg("del")
                .arg("sentinel")
                .status();
            let _ = fs::remove_file("/etc/init.d/sentinel");
        }
        InitSystem::Runit => {
            let service_link = if Path::new("/var/service/sentinel").exists() {
                "/var/service/sentinel"
            } else {
                "/service/sentinel"
            };
            let _ = fs::remove_file(service_link);
            let _ = Command::new("pkill")
                .arg("-f")
                .arg(BINARY_GLOBAL_PATH)
                .status();
            let _ = fs::remove_dir_all("/etc/sv/sentinel");
            let _ = fs::remove_dir_all("/etc/runit/sv/sentinel");
        }
        InitSystem::SysVinit => {
            let _ = Command::new("/etc/init.d/sentinel").arg("stop").status();
            if Command::new("update-rc.d")
                .arg("sentinel")
                .arg("remove")
                .status()
                .is_err()
            {
                let _ = Command::new("chkconfig")
                    .arg("--del")
                    .arg("sentinel")
                    .status();
            }
            let _ = fs::remove_file("/etc/init.d/sentinel");
        }
        _ => {
            let _ = Command::new("pkill")
                .arg("-f")
                .arg(BINARY_GLOBAL_PATH)
                .status();
        }
    }

    if Path::new(BINARY_GLOBAL_PATH).exists() {
        let _ = fs::remove_file(BINARY_GLOBAL_PATH);
    }
    let _ = fs::remove_dir_all("/etc/sentinel");

    println!("[*] Uninstallation clean and complete.");
}

fn handle_configuration() {
    println!("[*] Sentinel interactive configuration engine setup");

    let is_global = Path::new("/etc/sentinel").exists();
    let config_path = if is_global {
        if !bits_admin::is_admin() {
            eprintln!("[x] Error: Write access to /etc/sentinel denied. Run configuration step with 'sudo'.");
            std::process::exit(1);
        }
        "/etc/sentinel/config.toml".to_string()
    } else {
        "config.toml".to_string()
    };

    let mut default_webhook = String::new();
    if Path::new(&config_path).exists() {
        if let Ok(content) = fs::read_to_string(&config_path) {
            if let Ok(existing) = toml::from_str::<Config>(&content) {
                default_webhook = existing.webhook_url;
            }
        }
    }

    print!("Enter Discord Webhook URL [{}]: ", default_webhook);
    io::stdout().flush().unwrap();
    let mut input_webhook = String::new();
    io::stdin().read_line(&mut input_webhook).unwrap();
    let mut final_webhook = input_webhook.trim().to_string();

    if final_webhook.is_empty() {
        final_webhook = default_webhook;
    }

    if final_webhook.is_empty() {
        eprintln!("[x] Config cannot be empty without target webhook definition.");
        std::process::exit(1);
    }

    let config = Config {
        check_interval_secs: default_interval(),
        cpu_threshold_pct: default_cpu_threshold(),
        ram_threshold_pct: default_ram_threshold(),
        webhook_url: final_webhook,
    };

    let toml_string = toml::to_string_pretty(&config).unwrap();
    if let Err(e) = fs::write(&config_path, toml_string) {
        eprintln!("[x] Failed to write configuration file: {}.", e);
        std::process::exit(1);
    }
    println!(
        "[*] Configuration updated successfully inside: {}",
        config_path
    );

    if is_global {
        let init = detect_init_system();
        match init {
            InitSystem::Launchd => {
                let plist_path = "/Library/LaunchDaemons/com.speedycoder.sentinel.plist";
                let _ = Command::new("launchctl")
                    .arg("unload")
                    .arg(plist_path)
                    .status();
                let _ = Command::new("launchctl")
                    .arg("load")
                    .arg(plist_path)
                    .status();
            }
            InitSystem::Systemd => {
                let _ = Command::new("systemctl")
                    .arg("restart")
                    .arg("sentinel")
                    .status();
            }
            InitSystem::OpenRc => {
                let _ = Command::new("rc-service")
                    .arg("sentinel")
                    .arg("restart")
                    .status();
            }
            InitSystem::Runit => {
                let _ = Command::new("sv").arg("restart").arg("sentinel").status();
            }
            InitSystem::SysVinit => {
                let _ = Command::new("/etc/init.d/sentinel").arg("stop").status();
                let _ = Command::new("/etc/init.d/sentinel").arg("start").status();
            }
            _ => {}
        }
    }
}

fn load_existing_config() -> Option<Config> {
    let paths_to_check = ["/etc/sentinel/config.toml", "config.toml"];
    for path in &paths_to_check {
        if Path::new(path).exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(parsed) = toml::from_str(&content) {
                    return Some(parsed);
                }
            }
        }
    }
    None
}

mod bits_admin {
    pub fn is_admin() -> bool {
        unsafe { libc::getuid() == 0 }
    }
}

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 {
        match args[1].as_str() {
            "--install" => {
                handle_installation();
                return;
            }
            "--uninstall" => {
                handle_uninstallation();
                return;
            }
            "config" => {
                handle_configuration();
                return;
            }
            "--help" | "-h" => {
                print_help();
                return;
            }
            invalid if invalid.starts_with('-') || invalid == "uninstall" => {
                eprintln!("[x] Error: Unknown option flag or command '{}'.", invalid);
                println!("Run 'sentinel --help' to see valid operations.");
                std::process::exit(1);
            }
            _ => {}
        }
    }

    let config = match load_existing_config() {
        Some(c) => c,
        None => {
            let url = if args.len() > 1 {
                args[1].clone()
            } else {
                env::var("SENTINEL_WEBHOOK").unwrap_or_else(|_| {
                    eprintln!("[x] Error: No configuration found. Run 'sudo sentinel config' or provide a webhook URL via arguments.");
                    std::process::exit(1);
                })
            };
            Config {
                check_interval_secs: default_interval(),
                cpu_threshold_pct: default_cpu_threshold(),
                ram_threshold_pct: default_ram_threshold(),
                webhook_url: url,
            }
        }
    };

    let mut sys = System::new_with_specifics(
        RefreshKind::new()
            .with_cpu(CpuRefreshKind::everything())
            .with_memory(MemoryRefreshKind::everything()),
    );

    sys.refresh_all();
    tokio::time::sleep(Duration::from_millis(500)).await;

    println!("[*] Sentinel active. Monitoring resources.");
    send_startup_notification(&config.webhook_url).await;

    loop {
        sys.refresh_all();
        tokio::time::sleep(Duration::from_secs(1)).await;

        let cpu_usage = sys.global_cpu_info().cpu_usage();
        let total_ram = sys.total_memory() as f32;
        let used_ram = sys.used_memory() as f32;
        let ram_usage = if total_ram > 0.0 {
            (used_ram / total_ram) * 100.0
        } else {
            0.0
        };

        if cpu_usage > config.cpu_threshold_pct {
            println!("[!] CPU threshold crossed: {:.2}%", cpu_usage);
            send_discord_alert(
                &config.webhook_url,
                "Alert: High CPU Utilization Detected",
                "Operational processing limits exceeded.",
                15158332,
                cpu_usage,
                ram_usage,
            )
            .await;
        }

        if ram_usage > config.ram_threshold_pct {
            println!("[!] RAM threshold crossed: {:.2}%", ram_usage);
            send_discord_alert(
                &config.webhook_url,
                "Alert: Memory Cap Exceeded",
                "Memory buffer crossed.",
                15158332,
                cpu_usage,
                ram_usage,
            )
            .await;
        }

        tokio::time::sleep(Duration::from_secs(config.check_interval_secs)).await;
    }
}
