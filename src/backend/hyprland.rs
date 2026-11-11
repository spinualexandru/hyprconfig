use crate::backend::hyprland_lua as lua;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayMode {
    pub width: u16,
    pub height: u16,
    pub refresh_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .map_err(|e| format!("Failed to run hyprctl: {e}. Make sure Hyprland is running."))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to get monitor info: {}. Make sure Hyprland is running.",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_monitors_json(&output.stdout)
}

/// `hyprctl monitors -j`, read leniently: Hyprland adds and drops fields between releases
/// (0.57 leaves `specialWorkspace.id` out when none is shown), so only what the app shows is
/// required.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HyprctlMonitor {
    id: i128,
    name: String,
    #[serde(default)]
    description: String,
    width: u16,
    height: u16,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
    #[serde(default)]
    transform: i64,
    #[serde(default)]
    active_workspace: HyprctlWorkspace,
    #[serde(default)]
    available_modes: Vec<String>,
}

#[derive(Deserialize, Default)]
struct HyprctlWorkspace {
    #[serde(default)]
    id: i32,
    #[serde(default)]
    name: String,
}

/// Hyprland's `wl_output_transform` numbers, named as the `hyprland` crate named them.
fn transform_name(t: i64) -> String {
    match t {
        0 => "Normal",
        1 => "Normal90",
        2 => "Normal180",
        3 => "Normal270",
        4 => "Flipped",
        5 => "Flipped90",
        6 => "Flipped180",
        7 => "Flipped270",
        _ => return format!("Unknown ({t})"),
    }
    .to_string()
}

fn parse_monitors_json(json: &[u8]) -> Result<Vec<MonitorInfo>, String> {
    let monitors: Vec<HyprctlMonitor> = serde_json::from_slice(json)
        .map_err(|e| format!("Failed to read hyprctl's monitor list: {e}"))?;
    Ok(monitors
        .into_iter()
        .map(|m| MonitorInfo {
            id: m.id,
            name: m.name,
            description: m.description,
            width: m.width,
            height: m.height,
            refresh_rate: m.refresh_rate,
            x: m.x,
            y: m.y,
            scale: m.scale,
            transform: transform_name(m.transform),
            active_workspace_id: m.active_workspace.id,
            active_workspace_name: m.active_workspace.name,
            available_modes: parse_available_modes(&m.available_modes.join(" ")),
        })
        .collect())
}

fn parse_available_modes(modes_str: &str) -> Vec<DisplayMode> {
    let mut modes = Vec::new();

    // Parse modes like "2560x1600@240.00Hz 2560x1600@60.00Hz"
    for mode_str in modes_str.split_whitespace() {
        if let Some((resolution, refresh)) = mode_str.split_once('@')
            && let Some((width_str, height_str)) = resolution.split_once('x')
            && let (Ok(width), Ok(height)) = (width_str.parse::<u16>(), height_str.parse::<u16>())
        {
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

    modes
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiNetwork {
    pub ssid: String,
    pub signal_strength: i32,
    pub security: String,
    pub connected: bool,
    pub bssid: String,
    pub frequency: String,
}

pub fn get_network_info() -> Result<Vec<NetworkInterface>, String> {
    let net_path = Path::new("/sys/class/net");

    if !net_path.exists() {
        return Err("Network information not available on this system".to_string());
    }

    let entries =
        fs::read_dir(net_path).map_err(|e| format!("Failed to read network interfaces: {}", e))?;

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
        } else if super::tailscale::is_interface(&interface_name) {
            "Tailscale".to_string()
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
    interfaces.sort_by(|a, b| match (a.state.as_str(), b.state.as_str()) {
        ("up", "up") | ("down", "down") => a.name.cmp(&b.name),
        ("up", _) => std::cmp::Ordering::Less,
        (_, "up") => std::cmp::Ordering::Greater,
        _ => a.name.cmp(&b.name),
    });

    Ok(interfaces)
}

fn get_ip_addresses(interface_name: &str) -> Vec<String> {
    let output = Command::new("ip")
        .args(["addr", "show", interface_name])
        .output();

    if let Ok(output) = output
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        let mut ips = Vec::new();

        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("inet ") {
                if let Some(ip) = line.split_whitespace().nth(1) {
                    ips.push(ip.to_string());
                }
            } else if line.starts_with("inet6 ")
                && let Some(ip) = line.split_whitespace().nth(1)
            {
                // Skip link-local IPv6 addresses (fe80::)
                if !ip.starts_with("fe80:") {
                    ips.push(ip.to_string());
                }
            }
        }

        return ips;
    }

    Vec::new()
}

fn get_wifi_ssid(interface_name: &str) -> Option<String> {
    // Try using nmcli (NetworkManager) first
    let nmcli_output = Command::new("nmcli")
        .args(["-t", "-f", "active,ssid", "dev", "wifi"])
        .output();

    if let Ok(output) = nmcli_output
        && output.status.success()
    {
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

    // Fallback to iwgetid
    let iwgetid_output = Command::new("iwgetid")
        .args(["-r", interface_name])
        .output();

    if let Ok(output) = iwgetid_output
        && output.status.success()
    {
        let ssid = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if !ssid.is_empty() {
            return Some(ssid);
        }
    }

    // Last fallback: try iw command
    let iw_output = Command::new("iw")
        .args(["dev", interface_name, "link"])
        .output();

    if let Ok(output) = iw_output
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            let line = line.trim();
            if line.starts_with("SSID:") {
                let ssid = unescape_iw_ssid(line.strip_prefix("SSID:").unwrap_or("").trim());
                if !ssid.is_empty() {
                    return Some(ssid);
                }
            }
        }
    }

    None
}

/// `iw` prints an SSID's raw bytes, writing each byte that isn't printable ASCII (every byte
/// of a UTF-8 "ș" or "î", a backslash, a leading or trailing space) as `\xHH`. This puts the
/// bytes back and reads them as UTF-8.
fn unescape_iw_ssid(escaped: &str) -> String {
    let src = escaped.as_bytes();
    let mut bytes = Vec::with_capacity(src.len());
    let mut i = 0;
    while i < src.len() {
        if src[i] == b'\\'
            && src.get(i + 1) == Some(&b'x')
            && let Some(hex) = escaped.get(i + 2..i + 4)
            && let Ok(b) = u8::from_str_radix(hex, 16)
        {
            bytes.push(b);
            i += 4;
        } else {
            bytes.push(src[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Blocks for about half a second while NetworkManager rescans; call it off the UI thread.
pub fn scan_wifi_networks() -> Result<Vec<WifiNetwork>, String> {
    // First, trigger a rescan
    let _ = Command::new("nmcli")
        .args(["device", "wifi", "rescan"])
        .output();

    // Give the rescan a moment to complete
    std::thread::sleep(std::time::Duration::from_millis(500));

    // Get list of available networks
    let output = Command::new("nmcli")
        .args([
            "-t",
            "-f",
            "IN-USE,SSID,SIGNAL,SECURITY,BSSID,FREQ",
            "device",
            "wifi",
            "list",
        ])
        .output()
        .map_err(|e| {
            format!(
                "Failed to scan WiFi networks: {}. Make sure NetworkManager is installed.",
                e
            )
        })?;

    if !output.status.success() {
        return Err(
            "Failed to scan WiFi networks. Make sure NetworkManager is running.".to_string(),
        );
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
    networks.sort_by(|a, b| match (a.connected, b.connected) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => b.signal_strength.cmp(&a.signal_strength),
    });

    Ok(networks)
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    if let Ok(contents) = fs::read_to_string("/proc/uptime")
        && let Some(uptime_seconds) = contents.split_whitespace().next()
        && let Ok(seconds) = uptime_seconds.parse::<f64>()
    {
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
    "Unknown".to_string()
}

fn get_shell_info() -> String {
    let shell_path = std::env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());
    let shell_name = Path::new(&shell_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("sh");

    // Try to get shell version
    let version_output = Command::new(&shell_path).arg("--version").output();

    if let Ok(output) = version_output
        && output.status.success()
    {
        let version_str = String::from_utf8_lossy(&output.stdout);
        // Get first line which usually contains version info
        if let Some(first_line) = version_str.lines().next() {
            return first_line.trim().to_string();
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
    let output = Command::new("lspci").output();

    if let Ok(output) = output
        && output.status.success()
    {
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
            } else if line.starts_with("MemAvailable:")
                && let Some(value) = line.split_whitespace().nth(1)
            {
                available = value.parse::<u64>().unwrap_or(0);
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
            if line.starts_with("MemTotal:")
                && let Some(value) = line.split_whitespace().nth(1)
                && let Ok(kb) = value.parse::<u64>()
            {
                return format!("{:.2} GB", kb as f64 / 1024.0 / 1024.0);
            }
        }
    }

    "Unknown".to_string()
}

fn get_disk_used() -> String {
    let output = Command::new("df").args(["-h", "/"]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = output_str.lines().nth(1)
            && let Some(used) = line.split_whitespace().nth(2)
        {
            return used.to_string();
        }
    }

    "Unknown".to_string()
}

fn get_disk_total() -> String {
    let output = Command::new("df").args(["-h", "/"]).output();

    if let Ok(output) = output
        && output.status.success()
    {
        let output_str = String::from_utf8_lossy(&output.stdout);
        if let Some(line) = output_str.lines().nth(1)
            && let Some(total) = line.split_whitespace().nth(1)
        {
            return total.to_string();
        }
    }

    "Unknown".to_string()
}

fn get_cpu_info() -> String {
    if let Ok(contents) = fs::read_to_string("/proc/cpuinfo") {
        for line in contents.lines() {
            if line.starts_with("model name")
                && let Some(name) = line.split(':').nth(1)
            {
                return name.trim().to_string();
            }
        }
    }

    "Unknown".to_string()
}

// ============================================================================
// Config: everything below reads or edits the user's `hyprland.lua`, through
// `hyprland_lua`, after resolving which file that is.
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keybind {
    pub modifiers: Vec<String>,
    pub key: String,
    pub dispatcher: String,
    pub params: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub index: usize,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowruleProperty {
    pub key: String,
    pub value: String,
    pub property_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Windowrule {
    pub name: String,
    pub enabled: bool,
    pub match_properties: Vec<WindowruleProperty>,
    pub effect_properties: Vec<WindowruleProperty>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerruleProperty {
    pub key: String,
    pub value: String,
    pub property_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layerrule {
    pub name: String,
    pub enabled: bool,
    pub match_properties: Vec<LayerruleProperty>,
    pub effect_properties: Vec<LayerruleProperty>,
}

pub fn get_keybinds() -> Result<Vec<Keybind>, String> {
    lua::get_keybinds(&lua::config_path()?)
}

pub fn add_keybind(
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    lua::add_keybind(&lua::config_path()?, modifiers, key, dispatcher, params)
}

pub fn edit_keybind(
    index: usize,
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    lua::edit_keybind(
        &lua::config_path()?,
        index,
        modifiers,
        key,
        dispatcher,
        params,
    )
}

pub fn delete_keybind(index: usize) -> Result<(), String> {
    lua::delete_keybind(&lua::config_path()?, index)
}

/// Universal binds: the ones that work in every submap.
pub fn get_all_bindu() -> Result<Vec<Keybind>, String> {
    lua::get_all_bindu(&lua::config_path()?)
}

pub fn add_bindu(
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    lua::add_bindu(&lua::config_path()?, modifiers, key, dispatcher, params)
}

pub fn delete_bindu(index: usize) -> Result<(), String> {
    lua::delete_bindu(&lua::config_path()?, index)
}

pub fn get_variables() -> Result<Vec<Variable>, String> {
    lua::get_variables(&lua::config_path()?)
}

pub fn set_variable(name: String, value: String) -> Result<(), String> {
    lua::set_variable(&lua::config_path()?, name, value)
}

pub fn add_variable(name: String, value: String) -> Result<(), String> {
    lua::add_variable(&lua::config_path()?, name, value)
}

pub fn delete_variable(name: String) -> Result<(), String> {
    lua::delete_variable(&lua::config_path()?, name)
}

pub fn get_env_vars() -> Result<Vec<EnvVar>, String> {
    lua::get_env_vars(&lua::config_path()?)
}

pub fn add_env_var(name: String, value: String) -> Result<(), String> {
    lua::add_env_var(&lua::config_path()?, name, value)
}

pub fn edit_env_var(index: usize, name: String, value: String) -> Result<(), String> {
    lua::edit_env_var(&lua::config_path()?, index, name, value)
}

pub fn delete_env_var(index: usize) -> Result<(), String> {
    lua::delete_env_var(&lua::config_path()?, index)
}

/// Applies a monitor layout to the running session (`hyprctl eval`), without saving it.
pub fn apply_monitor_settings(
    name: String,
    width: u16,
    height: u16,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
) -> Result<(), String> {
    let layout = lua::MonitorLayout {
        width,
        height,
        refresh_rate,
        x,
        y,
        scale,
    };
    let output = Command::new("hyprctl")
        .args(["eval", &lua::monitor_eval_code(&name, &layout)])
        .output()
        .map_err(|e| format!("Failed to run hyprctl: {}", e))?;

    // hyprctl exits 0 even when Hyprland rejects the request, so check the reply too.
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || stdout != "ok" {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stdout.is_empty() { stderr } else { stdout };
        return Err(format!("Failed to apply monitor settings: {}", detail));
    }

    Ok(())
}

pub fn save_monitor_settings(
    name: String,
    width: u16,
    height: u16,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
) -> Result<(), String> {
    lua::save_monitor_settings(
        &lua::config_path()?,
        name,
        &lua::MonitorLayout {
            width,
            height,
            refresh_rate,
            x,
            y,
            scale,
        },
    )
}

pub fn get_windowrule_names() -> Result<Vec<String>, String> {
    lua::get_windowrule_names(&lua::config_path()?)
}

pub fn get_windowrule(name: String) -> Result<Windowrule, String> {
    lua::get_windowrule(&lua::config_path()?, name)
}

pub fn delete_windowrule(name: String) -> Result<(), String> {
    lua::delete_windowrule(&lua::config_path()?, name)
}

pub fn get_layerrule_names() -> Result<Vec<String>, String> {
    lua::get_layerrule_names(&lua::config_path()?)
}

pub fn get_layerrule(name: String) -> Result<Layerrule, String> {
    lua::get_layerrule(&lua::config_path()?, name)
}

pub fn delete_layerrule(name: String) -> Result<(), String> {
    lua::delete_layerrule(&lua::config_path()?, name)
}

#[cfg(test)]
mod monitor_tests {
    use super::parse_monitors_json;

    /// Hyprland 0.57.2's output for a laptop panel with no special workspace shown: that
    /// object has no `id`, which the `hyprland` crate rejected.
    const HYPRLAND_0_57: &str = r#"[{
        "id": 0, "name": "eDP-1", "description": "BOE NE160QDM-NM7", "make": "BOE",
        "width": 2560, "height": 1600, "physicalWidth": 340, "physicalHeight": 220,
        "refreshRate": 240.00000, "x": 0, "y": 0,
        "activeWorkspace": { "address": "2", "id": 2, "type": "normal", "name": "2" },
        "specialWorkspace": { "address": "", "type": "", "name": "" },
        "reserved": [0, 34, 0, 0], "scale": 1, "transform": 0, "focused": true,
        "solitaryBlockedBy": ["WINDOWED","CANDIDATE"], "disabled": false,
        "availableModes": ["2560x1600@240.00Hz","2560x1600@60.00Hz"],
        "sdrBrightness": 1
    }]"#;

    #[test]
    fn reads_hyprland_0_57_monitors() {
        let m = parse_monitors_json(HYPRLAND_0_57.as_bytes()).unwrap();
        assert_eq!(m.len(), 1);
        let m = &m[0];
        assert_eq!(
            (m.id, m.name.as_str(), m.width, m.height),
            (0, "eDP-1", 2560, 1600)
        );
        assert_eq!(m.refresh_rate, 240.0);
        assert_eq!(m.scale, 1.0);
        assert_eq!(m.transform, "Normal");
        assert_eq!(
            (m.active_workspace_id, m.active_workspace_name.as_str()),
            (2, "2")
        );
        assert_eq!(m.available_modes.len(), 2);
        assert_eq!(m.available_modes[1].refresh_rate, 60.0);
    }

    #[test]
    fn a_broken_list_is_an_error_not_a_panic() {
        assert!(parse_monitors_json(b"not json").is_err());
        assert!(parse_monitors_json(b"[]").unwrap().is_empty());
    }
}

#[cfg(test)]
mod ssid_tests {
    use super::unescape_iw_ssid;

    #[test]
    fn iw_escapes_become_utf8() {
        // "Sânziana" as `iw dev wlan0 link` prints it.
        assert_eq!(unescape_iw_ssid(r"S\xc3\xa2nziana"), "Sânziana");
        assert_eq!(unescape_iw_ssid(r"Ca\xc8\x99a"), "Cașa");
        assert_eq!(unescape_iw_ssid(r"a\x5cb\x20"), "a\\b ");
        assert_eq!(unescape_iw_ssid("Plain Home"), "Plain Home");
    }

    #[test]
    fn broken_escapes_stay_as_text() {
        assert_eq!(unescape_iw_ssid(r"odd\xzz\x4"), r"odd\xzz\x4");
        assert_eq!(unescape_iw_ssid(r"\xff"), "\u{fffd}");
    }
}
