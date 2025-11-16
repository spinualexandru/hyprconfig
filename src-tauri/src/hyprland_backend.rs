use hyprland::data::Monitors;
use hyprland::shared::HyprData;
use serde::{Deserialize, Serialize};
use std::panic;
use std::fs;
use std::path::Path;
use std::process::Command;
use hyprlang::Config;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayMode {
    pub width: u16,
    pub height: u16,
    pub refresh_rate: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id: i128,
    pub name: String,
    pub description: String,
    pub width: u16,
    pub height: u16,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub transform: String,
    pub active_workspace_id: i32,
    pub active_workspace_name: String,
    pub available_modes: Vec<DisplayMode>,
}

#[tauri::command]
pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    // Wrap in catch_unwind to prevent panics from crossing FFI boundary
    let result = panic::catch_unwind(|| {
        Monitors::get()
    });

    match result {
        Ok(Ok(monitors)) => {
            // Get available modes for all monitors using hyprctl
            let available_modes_map = get_available_modes_for_all_monitors();

            let monitor_infos: Vec<MonitorInfo> = monitors
                .into_iter()
                .map(|m| {
                    let available_modes = available_modes_map
                        .get(&m.name)
                        .cloned()
                        .unwrap_or_else(Vec::new);

                    MonitorInfo {
                        id: m.id,
                        name: m.name,
                        description: m.description,
                        width: m.width,
                        height: m.height,
                        refresh_rate: m.refresh_rate,
                        x: m.x,
                        y: m.y,
                        scale: m.scale,
                        transform: format!("{:?}", m.transform),
                        active_workspace_id: m.active_workspace.id,
                        active_workspace_name: m.active_workspace.name,
                        available_modes,
                    }
                })
                .collect();
            Ok(monitor_infos)
        }
        Ok(Err(e)) => Err(format!("Failed to get monitor info: {}. Make sure Hyprland is running.", e)),
        Err(_) => Err("Failed to get monitor info: Internal panic occurred. Make sure Hyprland is running and accessible.".to_string()),
    }
}

fn get_available_modes_for_all_monitors() -> std::collections::HashMap<String, Vec<DisplayMode>> {
    let mut modes_map = std::collections::HashMap::new();

    // Run hyprctl monitors all to get available modes
    let output = Command::new("hyprctl")
        .args(&["monitors", "all"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut current_monitor: Option<String> = None;

            for line in output_str.lines() {
                let line = line.trim();

                // Detect monitor name (e.g., "Monitor eDP-1 (ID 0):")
                if line.starts_with("Monitor ") && line.contains("(ID") {
                    if let Some(name) = line.split_whitespace().nth(1) {
                        current_monitor = Some(name.to_string());
                    }
                }

                // Parse availableModes line
                if line.starts_with("availableModes:") {
                    if let Some(monitor_name) = &current_monitor {
                        let modes_str = line.strip_prefix("availableModes:").unwrap_or("").trim();
                        let modes = parse_available_modes(modes_str);
                        modes_map.insert(monitor_name.clone(), modes);
                    }
                }
            }
        }
    }

    modes_map
}

fn parse_available_modes(modes_str: &str) -> Vec<DisplayMode> {
    let mut modes = Vec::new();

    // Parse modes like "2560x1600@240.00Hz 2560x1600@60.00Hz"
    for mode_str in modes_str.split_whitespace() {
        if let Some((resolution, refresh)) = mode_str.split_once('@') {
            if let Some((width_str, height_str)) = resolution.split_once('x') {
                if let (Ok(width), Ok(height)) = (width_str.parse::<u16>(), height_str.parse::<u16>()) {
                    // Parse refresh rate (remove "Hz" suffix)
                    let refresh_str = refresh.trim_end_matches("Hz");
                    if let Ok(refresh_rate) = refresh_str.parse::<f32>() {
                        modes.push(DisplayMode {
                            width,
                            height,
                            refresh_rate,
                        });
                    }
                }
            }
        }
    }

    modes
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub state: String,
    pub mac_address: String,
    pub ip_addresses: Vec<String>,
    pub interface_type: String,
    pub mtu: String,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub ssid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: i32,
    pub security: String,
    pub connected: bool,
    pub bssid: String,
    pub frequency: String,
}

#[tauri::command]
pub fn get_network_info() -> Result<Vec<NetworkInterface>, String> {
    let net_path = Path::new("/sys/class/net");

    if !net_path.exists() {
        return Err("Network information not available on this system".to_string());
    }

    let entries = fs::read_dir(net_path)
        .map_err(|e| format!("Failed to read network interfaces: {}", e))?;

    let mut interfaces = Vec::new();

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {}", e))?;
        let interface_name = entry.file_name().to_string_lossy().to_string();

        // Skip loopback interface
        if interface_name == "lo" {
            continue;
        }

        let interface_path = entry.path();

        // Read operstate (up/down)
        let state = fs::read_to_string(interface_path.join("operstate"))
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();

        // Read MAC address
        let mac_address = fs::read_to_string(interface_path.join("address"))
            .unwrap_or_else(|_| "unknown".to_string())
            .trim()
            .to_string();

        // Read MTU
        let mtu = fs::read_to_string(interface_path.join("mtu"))
            .unwrap_or_else(|_| "0".to_string())
            .trim()
            .to_string();

        // Read RX bytes
        let rx_bytes = fs::read_to_string(interface_path.join("statistics/rx_bytes"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        // Read TX bytes
        let tx_bytes = fs::read_to_string(interface_path.join("statistics/tx_bytes"))
            .ok()
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0);

        // Determine interface type
        let interface_type = if interface_name.starts_with("wl") {
            "WiFi".to_string()
        } else if interface_name.starts_with("en") || interface_name.starts_with("eth") {
            "Ethernet".to_string()
        } else if interface_name.starts_with("br") {
            "Bridge".to_string()
        } else if interface_name.starts_with("docker") || interface_name.starts_with("veth") {
            "Virtual".to_string()
        } else {
            "Other".to_string()
        };

        // Get IP addresses using ip addr show command
        let ip_addresses = get_ip_addresses(&interface_name);

        // Get WiFi SSID if this is a WiFi interface
        let ssid = if interface_type == "WiFi" {
            get_wifi_ssid(&interface_name)
        } else {
            None
        };

        interfaces.push(NetworkInterface {
            name: interface_name,
            state,
            mac_address,
            ip_addresses,
            interface_type,
            mtu,
            rx_bytes,
            tx_bytes,
            ssid,
        });
    }

    // Sort interfaces: connected first, then by name
    interfaces.sort_by(|a, b| {
        match (a.state.as_str(), b.state.as_str()) {
            ("up", "up") | ("down", "down") => a.name.cmp(&b.name),
            ("up", _) => std::cmp::Ordering::Less,
            (_, "up") => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        }
    });

    Ok(interfaces)
}

fn get_ip_addresses(interface_name: &str) -> Vec<String> {
    let output = Command::new("ip")
        .args(&["addr", "show", interface_name])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut ips = Vec::new();

            for line in output_str.lines() {
                let line = line.trim();
                if line.starts_with("inet ") {
                    if let Some(ip) = line.split_whitespace().nth(1) {
                        ips.push(ip.to_string());
                    }
                } else if line.starts_with("inet6 ") {
                    if let Some(ip) = line.split_whitespace().nth(1) {
                        // Skip link-local IPv6 addresses (fe80::)
                        if !ip.starts_with("fe80:") {
                            ips.push(ip.to_string());
                        }
                    }
                }
            }

            return ips;
        }
    }

    Vec::new()
}

fn get_wifi_ssid(interface_name: &str) -> Option<String> {
    // Try using nmcli (NetworkManager) first
    let nmcli_output = Command::new("nmcli")
        .args(&["-t", "-f", "active,ssid", "dev", "wifi"])
        .output();

    if let Ok(output) = nmcli_output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                if line.starts_with("yes:") {
                    let ssid = line.strip_prefix("yes:").unwrap_or("").trim();
                    if !ssid.is_empty() {
                        return Some(ssid.to_string());
                    }
                }
            }
        }
    }

    // Fallback to iwgetid
    let iwgetid_output = Command::new("iwgetid")
        .args(&["-r", interface_name])
        .output();

    if let Ok(output) = iwgetid_output {
        if output.status.success() {
            let ssid = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ssid.is_empty() {
                return Some(ssid);
            }
        }
    }

    // Last fallback: try iw command
    let iw_output = Command::new("iw")
        .args(&["dev", interface_name, "link"])
        .output();

    if let Ok(output) = iw_output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            for line in output_str.lines() {
                let line = line.trim();
                if line.starts_with("SSID:") {
                    let ssid = line.strip_prefix("SSID:").unwrap_or("").trim();
                    if !ssid.is_empty() {
                        return Some(ssid.to_string());
                    }
                }
            }
        }
    }

    None
}

#[tauri::command]
pub async fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    // First, trigger a rescan
    let _ = Command::new("nmcli")
        .args(&["device", "wifi", "rescan"])
        .output();

    // Small delay to allow scan to complete (non-blocking)
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Get list of available networks
    let output = Command::new("nmcli")
        .args(&["-t", "-f", "IN-USE,SSID,SIGNAL,SECURITY,BSSID,FREQ", "device", "wifi", "list"])
        .output()
        .map_err(|e| format!("Failed to scan WiFi networks: {}. Make sure NetworkManager is installed.", e))?;

    if !output.status.success() {
        return Err("Failed to scan WiFi networks. Make sure NetworkManager is running.".to_string());
    }

    let output_str = String::from_utf8_lossy(&output.stdout);
    let mut networks = Vec::new();
    let mut seen_ssids = std::collections::HashSet::new();

    for line in output_str.lines() {
        let parts: Vec<&str> = line.split(':').collect();
        if parts.len() >= 6 {
            let in_use = parts[0];
            let ssid = parts[1].trim();
            let signal_str = parts[2].trim();
            let security = parts[3].trim();
            let bssid = parts[4].trim();
            let freq = parts[5].trim();

            // Skip hidden networks and duplicates (show only strongest signal for each SSID)
            if ssid.is_empty() || seen_ssids.contains(ssid) {
                continue;
            }

            seen_ssids.insert(ssid.to_string());

            let signal_strength = signal_str.parse::<i32>().unwrap_or(0);
            let connected = in_use == "*";

            let security_type = if security.is_empty() {
                "Open".to_string()
            } else if security.contains("WPA3") {
                "WPA3".to_string()
            } else if security.contains("WPA2") {
                "WPA2".to_string()
            } else if security.contains("WPA") {
                "WPA".to_string()
            } else if security.contains("WEP") {
                "WEP".to_string()
            } else {
                security.to_string()
            };

            networks.push(WifiNetwork {
                ssid: ssid.to_string(),
                signal_strength,
                security: security_type,
                connected,
                bssid: bssid.to_string(),
                frequency: freq.to_string(),
            });
        }
    }

    // Sort networks: connected first, then by signal strength
    networks.sort_by(|a, b| {
        match (a.connected, b.connected) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => b.signal_strength.cmp(&a.signal_strength),
        }
    });

    Ok(networks)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SystemInfo {
    pub os: String,
    pub hostname: String,
    pub kernel: String,
    pub uptime: String,
    pub shell: String,
    pub hyprland_version: String,
    pub gpus: Vec<String>,
    pub ram_used: String,
    pub ram_total: String,
    pub disk_used: String,
    pub disk_total: String,
    pub cpu: String,
}

#[tauri::command]
pub fn get_system_info() -> Result<SystemInfo, String> {
    Ok(SystemInfo {
        os: get_os_info(),
        hostname: get_hostname(),
        kernel: get_kernel_version(),
        uptime: get_uptime(),
        shell: get_shell_info(),
        hyprland_version: get_hyprland_version(),
        gpus: get_gpu_info(),
        ram_used: get_ram_used(),
        ram_total: get_ram_total(),
        disk_used: get_disk_used(),
        disk_total: get_disk_total(),
        cpu: get_cpu_info(),
    })
}

fn get_os_info() -> String {
    // Try to read /etc/os-release
    if let Ok(contents) = fs::read_to_string("/etc/os-release") {
        for line in contents.lines() {
            if line.starts_with("PRETTY_NAME=") {
                let name = line.strip_prefix("PRETTY_NAME=").unwrap_or("");
                return name.trim_matches('"').to_string();
            }
        }
    }
    "Unknown".to_string()
}

fn get_hostname() -> String {
    Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_kernel_version() -> String {
    Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_uptime() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/uptime") {
        if let Some(uptime_seconds) = contents.split_whitespace().next() {
            if let Ok(seconds) = uptime_seconds.parse::<f64>() {
                let days = (seconds / 86400.0).floor() as u64;
                let hours = ((seconds % 86400.0) / 3600.0).floor() as u64;
                let minutes = ((seconds % 3600.0) / 60.0).floor() as u64;

                if days > 0 {
                    return format!("{}d {}h {}m", days, hours, minutes);
                } else if hours > 0 {
                    return format!("{}h {}m", hours, minutes);
                } else {
                    return format!("{}m", minutes);
                }
            }
        }
    }
    "Unknown".to_string()
}

fn get_shell_info() -> String {
    let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sh");

    // Try to get shell version
    let version_output = Command::new(&shell_path)
        .arg("--version")
        .output();

    if let Ok(output) = version_output {
        if output.status.success() {
            let version_str = String::from_utf8_lossy(&output.stdout);
            // Get first line which usually contains version info
            if let Some(first_line) = version_str.lines().next() {
                return first_line.trim().to_string();
            }
        }
    }

    shell_name.to_string()
}

fn get_hyprland_version() -> String {
    Command::new("hyprctl")
        .arg("version")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .and_then(|s| s.lines().next().map(|l| l.to_string()))
        .unwrap_or_else(|| "Unknown".to_string())
}

fn get_gpu_info() -> Vec<String> {
    let output = Command::new("lspci")
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            let mut gpus = Vec::new();

            for line in output_str.lines() {
                if line.contains("VGA compatible controller") || line.contains("3D controller") {
                    // Extract GPU name after the controller type
                    if let Some(gpu_name) = line.split(':').nth(2) {
                        gpus.push(gpu_name.trim().to_string());
                    }
                }
            }

            if !gpus.is_empty() {
                return gpus;
            }
        }
    }

    vec!["Unknown".to_string()]
}

fn get_ram_used() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        let mut total = 0u64;
        let mut available = 0u64;

        for line in contents.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    total = value.parse::<u64>().unwrap_or(0);
                }
            } else if line.starts_with("MemAvailable:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    available = value.parse::<u64>().unwrap_or(0);
                }
            }
        }

        if total > 0 && available > 0 {
            let used = total - available;
            return format!("{:.2} GB", used as f64 / 1024.0 / 1024.0);
        }
    }

    "Unknown".to_string()
}

fn get_ram_total() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/meminfo") {
        for line in contents.lines() {
            if line.starts_with("MemTotal:") {
                if let Some(value) = line.split_whitespace().nth(1) {
                    if let Ok(kb) = value.parse::<u64>() {
                        return format!("{:.2} GB", kb as f64 / 1024.0 / 1024.0);
                    }
                }
            }
        }
    }

    "Unknown".to_string()
}

fn get_disk_used() -> String {
    let output = Command::new("df")
        .args(&["-h", "/"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = output_str.lines().nth(1) {
                if let Some(used) = line.split_whitespace().nth(2) {
                    return used.to_string();
                }
            }
        }
    }

    "Unknown".to_string()
}

fn get_disk_total() -> String {
    let output = Command::new("df")
        .args(&["-h", "/"])
        .output();

    if let Ok(output) = output {
        if output.status.success() {
            let output_str = String::from_utf8_lossy(&output.stdout);
            if let Some(line) = output_str.lines().nth(1) {
                if let Some(total) = line.split_whitespace().nth(1) {
                    return total.to_string();
                }
            }
        }
    }

    "Unknown".to_string()
}

fn get_cpu_info() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/cpuinfo") {
        for line in contents.lines() {
            if line.starts_with("model name") {
                if let Some(name) = line.split(':').nth(1) {
                    return name.trim().to_string();
                }
            }
        }
    }

    "Unknown".to_string()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Keybind {
    pub modifiers: Vec<String>,
    pub key: String,
    pub dispatcher: String,
    pub params: String,
}
fn register_hyprland_handlers(config: &mut Config) {
    // Register root-level Hyprland keywords as handlers
    let root_keywords = vec![
        "monitor",
        "env",
        "bind",
        "bindm",
        "bindel",
        "bindl",
        "bindr",
        "windowrule",
        "windowrulev2",
        "workspace",
        "exec",
        "exec-once",
        "permission",
        "blurls",
        "layerrule",
        "gesture",
        "source",
    ];

    for keyword in root_keywords {
        config.register_handler_fn(keyword, |_ctx| Ok(()));
    }

    // Register category-specific handlers for animations
    config.register_category_handler_fn("animations", "animation", |_ctx| Ok(()));
    config.register_category_handler_fn("animations", "bezier", |_ctx| Ok(()));
}
#[tauri::command]
pub fn get_keybinds() -> Result<Vec<Keybind>, String> {
    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/keybinds.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Get all bind handler calls
    let binds = config.all_handler_calls().get("bind")
        .cloned()
        .unwrap_or_else(Vec::new);

    println!("binds: {:?}", binds);


    let mut keybinds = Vec::new();

    for bind_str in binds {
        // Parse bind format: "MODS, KEY, dispatcher, params"
        // Example: "SUPER, Q, exec, kitty"
        let parts: Vec<&str> = bind_str.split(',').map(|s| s.trim()).collect();

        if parts.len() >= 3 {
            let mods_str = parts[0];
            let key = parts[1].to_string();
            let dispatcher = parts[2].to_string();
            let params = if parts.len() > 3 {
                parts[3..].join(", ")
            } else {
                String::new()
            };

            // Split modifiers by space or underscore
            let modifiers: Vec<String> = mods_str
                .split(|c: char| c.is_whitespace() || c == '_')
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();

            keybinds.push(Keybind {
                modifiers,
                key,
                dispatcher,
                params,
            });
        }
    }

    Ok(keybinds)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
}

#[tauri::command]
pub fn get_variables() -> Result<Vec<Variable>, String> {
    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/programs.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Get all variables
    let variables_map = config.variables();
    let mut variables = Vec::new();

    for (name, value) in variables_map {
        variables.push(Variable {
            name: name.clone(),
            value: value.clone(),
        });
    }

    // Sort by name for consistent display
    variables.sort_by(|a, b| a.name.cmp(&b.name));

    Ok(variables)
}

#[tauri::command]
pub fn set_variable(name: String, value: String) -> Result<(), String> {
    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/programs.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Set the variable (mutation API)
    config.set_variable(name.clone(), value.clone());

    // Save the config file
    config.save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn add_variable(name: String, value: String) -> Result<(), String> {
    // Validate variable name (alphanumeric + underscore only)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Variable name must contain only letters, numbers, and underscores".to_string());
    }

    if name.is_empty() {
        return Err("Variable name cannot be empty".to_string());
    }

    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/programs.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Add the variable (same as set_variable in hyprlang)
    config.set_variable(name.clone(), value.clone());

    // Save the config file
    config.save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn delete_variable(name: String) -> Result<(), String> {
    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/programs.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Remove the variable (mutation API)
    config.remove_variable(&name);

    // Save the config file
    config.save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn add_keybind(
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    // Validate inputs
    if key.trim().is_empty() {
        return Err("Key is required".to_string());
    }

    if dispatcher.trim().is_empty() {
        return Err("Dispatcher is required".to_string());
    }

    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/keybinds.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Format bind args: "MODS, KEY, dispatcher, params"
    let mods_str = if modifiers.is_empty() {
        String::new()
    } else {
        modifiers.join(" ")
    };

    let bind_args = if mods_str.is_empty() {
        format!("{}, {}, {}", key.trim(), dispatcher.trim(), params.trim())
    } else {
        format!("{}, {}, {}, {}", mods_str, key.trim(), dispatcher.trim(), params.trim())
    };

    // Add handler call (mutation API)
    config.add_handler_call("bind", bind_args)
        .map_err(|e| format!("Failed to add keybind: {:?}", e))?;

    // Save the config file
    config.save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn edit_keybind(
    index: usize,
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    // Validate inputs
    if key.trim().is_empty() {
        return Err("Key is required".to_string());
    }

    if dispatcher.trim().is_empty() {
        return Err("Dispatcher is required".to_string());
    }

    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/keybinds.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Remove old keybind at index
    config.remove_handler_call("bind", index)
        .map_err(|e| format!("Failed to remove keybind at index {}: {:?}", index, e))?;

    // Format new bind args
    let mods_str = if modifiers.is_empty() {
        String::new()
    } else {
        modifiers.join(" ")
    };

    let bind_args = if mods_str.is_empty() {
        format!("{}, {}, {}", key.trim(), dispatcher.trim(), params.trim())
    } else {
        format!("{}, {}, {}, {}", mods_str, key.trim(), dispatcher.trim(), params.trim())
    };

    // Add new keybind (mutation API)
    config.add_handler_call("bind", bind_args)
        .map_err(|e| format!("Failed to add keybind: {:?}", e))?;

    // Save the config file
    config.save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn delete_keybind(index: usize) -> Result<(), String> {
    // Get Hyprland config path
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/keybinds.conf");

    if !config_path.exists() {
        return Err(format!("Hyprland config file not found at {:?}", config_path));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprland_handlers(&mut config);
    config.parse_file(&config_path)
        .map_err(|e| format!("Failed to parse Hyprland config: {:?}", e))?;

    // Remove handler call at index (mutation API)
    config.remove_handler_call("bind", index)
        .map_err(|e| format!("Failed to remove keybind at index {}: {:?}", index, e))?;

    // Save the config file
    config.save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}
