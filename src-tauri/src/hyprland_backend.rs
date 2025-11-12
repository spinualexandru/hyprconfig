use hyprland::data::Monitors;
use hyprland::shared::HyprData;
use serde::{Deserialize, Serialize};
use std::panic;
use std::fs;
use std::path::Path;
use std::process::Command;

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
}

#[tauri::command]
pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    // Wrap in catch_unwind to prevent panics from crossing FFI boundary
    let result = panic::catch_unwind(|| {
        Monitors::get()
    });

    match result {
        Ok(Ok(monitors)) => {
            let monitor_infos: Vec<MonitorInfo> = monitors
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
                    transform: format!("{:?}", m.transform),
                    active_workspace_id: m.active_workspace.id,
                    active_workspace_name: m.active_workspace.name,
                })
                .collect();
            Ok(monitor_infos)
        }
        Ok(Err(e)) => Err(format!("Failed to get monitor info: {}. Make sure Hyprland is running.", e)),
        Err(_) => Err("Failed to get monitor info: Internal panic occurred. Make sure Hyprland is running and accessible.".to_string()),
    }
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
