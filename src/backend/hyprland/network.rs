//! Network interfaces from `/sys/class/net`, their addresses and SSIDs, and Wi-Fi scans.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

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
        } else if crate::backend::tailscale::is_interface(&interface_name) {
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
