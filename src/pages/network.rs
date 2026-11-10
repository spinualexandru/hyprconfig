use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit};

use crate::app::State;
use crate::backend::hyprland::{self as hypr, NetworkInterface, WifiNetwork};
use crate::icons;
use crate::jobs::Remote;
use crate::ui::{self, Card, Tone};

#[derive(Default)]
pub struct NetworkState {
    pub interfaces: Remote<Vec<NetworkInterface>>,
    pub wifi: Remote<Vec<WifiNetwork>>,
    /// The interface whose details show, by name.
    pub selected: Option<String>,
}

pub fn load(s: &mut State) {
    load_interfaces(s);
    scan_wifi(s);
}

fn load_interfaces(s: &mut State) {
    let attempt = s.network.interfaces.begin();
    s.jobs.run(
        "network-interfaces",
        hypr::get_network_info,
        move |s, r, _| {
            s.network.interfaces.land(attempt, r);
        },
    );
}

/// A scan takes a second or so (NetworkManager rescans first); one at a time.
fn scan_wifi(s: &mut State) {
    if s.network.wifi.loading {
        return;
    }
    let attempt = s.network.wifi.begin();
    s.jobs
        .run("wifi-scan", hypr::scan_wifi_networks, move |s, r, _| {
            // The scan fails quietly where there's no Wi-Fi or NetworkManager.
            s.network.wifi.land(attempt, r.or_else(|_| Ok(Vec::new())));
        });
}

fn interface_title(i: &NetworkInterface) -> String {
    match &i.ssid {
        Some(ssid) => format!("{ssid} ({})", i.interface_type),
        None => i.interface_type.clone(),
    }
}

fn interface_icon(i: &NetworkInterface) -> &'static rune::Icon {
    match i.interface_type.as_str() {
        "WiFi" => &icons::WIFI,
        "Ethernet" => &icons::CABLE,
        _ => &icons::NETWORK,
    }
}

fn state_pill(k: &Kit, state: &str) -> El {
    ui::pill(
        k,
        state.to_uppercase(),
        if state == "up" {
            Tone::Success
        } else {
            Tone::Neutral
        },
    )
}

pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let i = ((bytes as f64).ln() / 1024f64.ln()).floor().min(4.0) as usize;
    format!("{:.2} {}", bytes as f64 / 1024f64.powi(i as i32), UNITS[i])
}

fn interface_card(k: &Kit, i: &NetworkInterface) -> El {
    let name = i.name.clone();
    let mut titles = col()
        .key("titles")
        .grow(1.0)
        .shrink()
        .min_w(0.0)
        .gap(2.0)
        .child(
            text(interface_title(i))
                .font(ui::card_title_style())
                .color(k.c.text_strong)
                .truncate(),
        )
        .child(text(i.name.clone()).font(k.ty.body).color(k.c.text_muted));
    if let Some(ip) = i.ip_addresses.first() {
        titles = titles.child(text(ip.clone()).font(ui::mono(k)).color(k.c.text_muted));
    }
    ui::surface(k)
        .key(format!("iface-{}", i.name))
        .role("button")
        .label(format!("{} details", interface_title(i)))
        .focusable(true)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .on_press(move |s: &mut State| s.network.selected = Some(name.clone()))
        .child(
            row()
                .w_full()
                .gap(12.0)
                .center_y()
                .child(ui::icon_tile(k, interface_icon(i)))
                .child(titles)
                .child(state_pill(k, &i.state))
                .child(rune::icon(&icons::CHEVRON_RIGHT, 18.0).color(k.c.icon_muted)),
        )
}

fn signal(strength: i32) -> (&'static str, Tone) {
    match strength {
        75.. => ("Excellent", Tone::Success),
        50..=74 => ("Good", Tone::Warning),
        25..=49 => ("Fair", Tone::Warning),
        _ => ("Weak", Tone::Danger),
    }
}

fn wifi_card(k: &Kit, n: &WifiNetwork) -> El {
    let (quality, tone) = signal(n.signal_strength);
    let ink = match tone {
        Tone::Success => k.c.success.icon,
        Tone::Warning => k.c.warning.icon,
        _ => k.c.danger.icon,
    };
    let mut r = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(ui::icon_tile(k, &icons::WIFI))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(n.ssid.clone())
                        .font(ui::card_title_style())
                        .color(k.c.text_strong)
                        .truncate(),
                )
                .child(
                    text(format!("{} • {}", n.security, n.frequency))
                        .font(k.ty.body)
                        .color(k.c.text_muted),
                ),
        )
        .child(
            col()
                .items_end()
                .child(
                    text(format!("{}%", n.signal_strength))
                        .font(k.ty.body_medium)
                        .color(ink),
                )
                .child(text(quality).font(k.ty.small).color(k.c.text_muted)),
        );
    if n.connected {
        r = r.child(ui::pill(k, "CONNECTED", Tone::Success));
    }
    ui::surface(k).key(format!("wifi-{}", n.bssid)).child(r)
}

fn skeleton_card(k: &Kit, key: String) -> El {
    ui::surface(k).key(key).child(
        row()
            .w_full()
            .gap(12.0)
            .center_y()
            .child(ui::skeleton(k, Some(40.0), 40.0))
            .child(
                col()
                    .grow(1.0)
                    .gap(8.0)
                    .child(ui::skeleton(k, Some(160.0), 16.0))
                    .child(ui::skeleton(k, Some(100.0), 14.0)),
            ),
    )
}

fn detail(k: &Kit, i: &NetworkInterface) -> El {
    let back = Button::new("Back")
        .icon(&icons::ARROW_LEFT)
        .variant(ButtonVariant::Ghost)
        .on_press(|s: &mut State| s.network.selected = None)
        .build(k)
        .key("back");
    let ips = if i.ip_addresses.is_empty() {
        ui::value_text(k, "No IP addresses assigned")
    } else {
        row().key("ips").gap(8.0).flex_wrap().children(
            i.ip_addresses
                .iter()
                .map(|ip| ui::pill(k, ip.clone(), Tone::Outline)),
        )
    };
    let mut fields = vec![ui::field(
        k,
        "name",
        "Interface Name",
        ui::value_text(k, i.name.clone()),
    )];
    if let Some(ssid) = &i.ssid {
        fields.push(ui::field(
            k,
            "ssid",
            "Network Name (SSID)",
            ui::value_text(k, ssid.clone()),
        ));
    }
    fields.push(ui::field(
        k,
        "type",
        "Interface Type",
        ui::value_text(k, i.interface_type.clone()),
    ));
    fields.push(ui::field(
        k,
        "mac",
        "MAC Address",
        text(i.mac_address.clone())
            .key("value")
            .font(ui::mono(k))
            .color(k.c.text_muted)
            .selectable(true),
    ));
    fields.push(ui::field(k, "ips", "IP Addresses", ips));
    fields.push(ui::field(
        k,
        "mtu",
        "MTU (Maximum Transmission Unit)",
        ui::value_text(k, format!("{} bytes", i.mtu)),
    ));
    fields.push(ui::grid2(
        "traffic",
        vec![
            ui::field(
                k,
                "rx",
                "Data Received",
                ui::value_text(k, format_bytes(i.rx_bytes)),
            ),
            ui::field(
                k,
                "tx",
                "Data Transmitted",
                ui::value_text(k, format_bytes(i.tx_bytes)),
            ),
        ],
    ));
    ui::page(
        k,
        "network-detail",
        &interface_title(i),
        &i.name,
        vec![back],
        vec![
            Card::new()
                .title("Connection Status")
                .action(state_pill(k, &i.state))
                .children(fields)
                .gap(20.0)
                .build(k)
                .key("status"),
        ],
    )
}

pub fn view(s: &State, k: &Kit) -> El {
    let n = &s.network;
    if let Some(i) = n
        .selected
        .as_ref()
        .and_then(|name| n.interfaces.data.as_ref()?.iter().find(|i| &i.name == name))
    {
        return detail(k, i);
    }
    let busy = n.interfaces.loading || n.wifi.loading;
    let refresh = Button::new("Refresh")
        .icon(&icons::REFRESH_CW)
        .variant(ButtonVariant::Outline)
        .disabled(busy)
        .on_press(|s: &mut State| {
            load_interfaces(s);
            scan_wifi(s);
        })
        .build(k)
        .key("refresh");
    let mut body = Vec::new();
    if let Some(e) = &n.interfaces.error {
        body.push(ui::error_card(
            k,
            "error",
            "Error Loading Network Information",
            e,
        ));
    }

    let ifaces = n.interfaces.data.as_deref().unwrap_or_default();
    if n.interfaces.first_load() {
        body.push(ui::section(
            k,
            "interfaces",
            None,
            "Network Interfaces",
            None,
            (0..2).map(|i| skeleton_card(k, format!("sk{i}"))).collect(),
        ));
    } else if !ifaces.is_empty() {
        body.push(ui::section(
            k,
            "interfaces",
            None,
            "Network Interfaces",
            None,
            ifaces.iter().map(|i| interface_card(k, i)).collect(),
        ));
    }

    let wifi = n.wifi.data.as_deref().unwrap_or_default();
    let scanning_first = n.wifi.first_load();
    let (connected, available): (Vec<_>, Vec<_>) = wifi.iter().partition(|w| w.connected);
    if scanning_first {
        body.push(ui::section(
            k,
            "connected",
            None,
            "Connected",
            None,
            vec![skeleton_card(k, "sk-c".into())],
        ));
    } else if !connected.is_empty() {
        body.push(ui::section(
            k,
            "connected",
            None,
            "Connected",
            None,
            connected.iter().map(|w| wifi_card(k, w)).collect(),
        ));
    }
    let rescan = ui::icon_button(&icons::REFRESH_CW, "Scan for networks")
        .disabled(n.wifi.loading)
        .on_press(scan_wifi)
        .build(k)
        .key("rescan");
    if scanning_first {
        body.push(ui::section(
            k,
            "available",
            None,
            "Available Networks",
            Some(rescan),
            (0..3)
                .map(|i| skeleton_card(k, format!("sk-a{i}")))
                .collect(),
        ));
    } else if !available.is_empty() {
        let mut items: Vec<El> = available.iter().map(|w| wifi_card(k, w)).collect();
        if n.wifi.loading {
            items.insert(
                0,
                text("Scanning…")
                    .key("scanning")
                    .font(k.ty.small)
                    .color(k.c.text_muted),
            );
        }
        body.push(ui::section(
            k,
            "available",
            None,
            "Available Networks",
            Some(rescan),
            items,
        ));
    }

    if !n.interfaces.loading
        && !n.wifi.loading
        && !scanning_first
        && ifaces.is_empty()
        && wifi.is_empty()
        && n.interfaces.data.is_some()
    {
        body.push(ui::note_card(
            k,
            "empty",
            &["No network interfaces or WiFi networks detected."],
        ));
    }
    ui::page(
        k,
        "network",
        "Network",
        "Network settings and configuration",
        vec![refresh],
        body,
    )
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn bytes_read_like_the_web_version() {
        assert_eq!(format_bytes(0), "0 B");
        assert_eq!(format_bytes(512), "512.00 B");
        assert_eq!(format_bytes(1536), "1.50 KB");
        assert_eq!(format_bytes(5 * 1024 * 1024 * 1024), "5.00 GB");
    }
}
