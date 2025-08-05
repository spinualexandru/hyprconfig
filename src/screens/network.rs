use gtk4::{prelude::*, Box as GtkBox, Entry, Label, Orientation, Switch};
use std::fs;

fn get_wireless_interfaces() -> Vec<String> {
    let mut interfaces = Vec::new();

    if let Ok(entries) = fs::read_dir("/sys/class/net/") {
        for entry in entries.flatten() {
            let path = entry.path().join("wireless");
            if path.exists() {
                if let Some(iface) = entry.file_name().to_str() {
                    interfaces.push(iface.to_string());
                }
            }
        }
    }
    interfaces
}

fn get_interface_status(iface: &str) -> Option<String> {
    let path = format!("/sys/class/net/{}/operstate", iface);
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

pub fn network_page() -> GtkBox {
    let box_ = GtkBox::new(Orientation::Vertical, 12);
    box_.append(&Label::new(Some("Wi-Fi Enabled:")));

    let wifi_switch = Switch::new();
    box_.append(&wifi_switch);

    box_.append(&Label::new(Some("Current Network:")));

    let network_entry = Entry::new();
    network_entry.set_placeholder_text(Some("SSID"));

    let interfaces = get_wireless_interfaces();

    if let Some(iface) = interfaces.first() {
        let status = get_interface_status(iface);
        if let Some(status) = status {
            if status == "up" {
                wifi_switch.set_active(true);
                network_entry.set_text("Connected (SSID unavailable)");
            } else {
                wifi_switch.set_active(false);
                network_entry.set_text("Not connected");
            }
        }
    } else {
        network_entry.set_text("No wireless interface found");
    }

    box_.append(&network_entry);
    box_
}
