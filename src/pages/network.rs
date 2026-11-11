use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Switch};

use crate::app::{self, State};
use crate::backend::hyprland::{self as hypr, NetworkInterface, WifiNetwork};
use crate::backend::tailscale::{self, TailscalePeer, TailscaleStatus};
use crate::icons;
use crate::jobs::{self, Remote};
use crate::ui::{self, Card, Tone};

#[derive(Default)]
pub struct NetworkState {
    pub interfaces: Remote<Vec<NetworkInterface>>,
    pub wifi: Remote<Vec<WifiNetwork>>,
    /// The interface whose details show, by name.
    pub selected: Option<String>,
    /// Tailscale's state; `None` inside when the CLI isn't installed.
    pub tailscale: Remote<Option<TailscaleStatus>>,
    /// Where a `tailscale up` (true) or `down` (false) in flight is taking it.
    pub tailscale_pending: Option<bool>,
}

pub fn load(s: &mut State) {
    load_interfaces(s);
    load_tailscale(s);
    scan_wifi(s);
}

fn load_tailscale(s: &mut State) {
    jobs::load(
        s,
        "tailscale-status",
        |s| &mut s.network.tailscale,
        tailscale::status,
    );
}

fn set_tailscale(s: &mut State, on: bool) {
    if s.network.tailscale_pending.is_some() {
        return;
    }
    s.network.tailscale_pending = Some(on);
    s.jobs.write(
        "tailscale-set",
        move || tailscale::set_running(on),
        move |s, r, tx| {
            s.network.tailscale_pending = None;
            match r {
                Ok(()) if on => s.toasts.success(tx, "Tailscale connected"),
                Ok(()) => s.toasts.success(tx, "Tailscale disconnected"),
                Err(e) if on => app::fail(s, tx, "Couldn't connect Tailscale", e),
                Err(e) => app::fail(s, tx, "Couldn't disconnect Tailscale", e),
            }
            // Its addresses come and go with it.
            load_tailscale(s);
            load_interfaces(s);
        },
    );
}

/// Tailscale's status, when `i` is its tunnel and the CLI answered.
fn tailscale_for<'a>(n: &'a NetworkState, i: &NetworkInterface) -> Option<&'a TailscaleStatus> {
    if !tailscale::is_interface(&i.name) {
        return None;
    }
    n.tailscale.data.as_ref()?.as_ref()
}

/// The on/off switch for Tailscale. Shows where a switch in flight is going.
fn tailscale_switch(n: &NetworkState, k: &Kit, st: &TailscaleStatus) -> El {
    let on = n.tailscale_pending.unwrap_or_else(|| st.running());
    let busy = n.tailscale_pending.is_some() || n.tailscale.loading;
    Switch::new(on)
        .label("Tailscale")
        .hide_label()
        .disabled(busy || !st.can_toggle())
        .on_toggle(move |s: &mut State| set_tailscale(s, !on))
        .build(k)
        .key("tailscale-switch")
}

fn tailscale_summary(n: &NetworkState, st: &TailscaleStatus) -> String {
    match (n.tailscale_pending, st.backend_state.as_str()) {
        (Some(true), _) => "Connecting…".into(),
        (Some(false), _) => "Disconnecting…".into(),
        (None, "Running") => match &st.dns_name {
            Some(name) => format!("Connected as {name}"),
            None => "Connected".into(),
        },
        (None, "Stopped") => "Disconnected".into(),
        (None, "NeedsLogin") => "Not logged in. Run `tailscale up` in a terminal to log in.".into(),
        (None, "NeedsMachineAuth") => "Waiting for an admin to approve this machine".into(),
        (None, other) => other.to_owned(),
    }
}

fn load_interfaces(s: &mut State) {
    jobs::load(
        s,
        "network-interfaces",
        |s| &mut s.network.interfaces,
        hypr::get_network_info,
    );
}

/// A scan takes a second or so (NetworkManager rescans first); one at a time.
fn scan_wifi(s: &mut State) {
    if s.network.wifi.loading {
        return;
    }
    // The scan fails quietly where there's no Wi-Fi or NetworkManager.
    jobs::load(
        s,
        "wifi-scan",
        |s| &mut s.network.wifi,
        || hypr::scan_wifi_networks().or_else(|_| Ok(Vec::new())),
    );
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
        "Tailscale" => &icons::SHIELD_CHECK,
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

fn interface_card(n: &NetworkState, k: &Kit, i: &NetworkInterface) -> El {
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
    // Tailscale's tunnel reports `unknown` either way; its switch says whether it's on.
    let status = match tailscale_for(n, i) {
        Some(st) => tailscale_switch(n, k, st),
        None => state_pill(k, &i.state),
    };
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
                .child(status)
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

fn peer_card(k: &Kit, p: &TailscalePeer) -> El {
    let sub = [p.ips.first(), Some(&p.os).filter(|os| !os.is_empty())]
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(" • ");
    let mut r = row()
        .w_full()
        .gap(12.0)
        .center_y()
        .child(ui::icon_tile(
            k,
            if p.mobile() {
                &icons::SMARTPHONE
            } else {
                &icons::MONITOR
            },
        ))
        .child(
            col()
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(p.name.clone())
                        .font(ui::card_title_style())
                        .color(k.c.text_strong)
                        .truncate(),
                )
                .child(
                    text(sub)
                        .font(ui::mono(k))
                        .color(k.c.text_muted)
                        .selectable(true),
                ),
        );
    if p.exit_node {
        r = r.child(ui::pill(k, "EXIT NODE", Tone::Outline));
    }
    r = if p.online {
        r.child(ui::pill(k, "ONLINE", Tone::Success))
    } else {
        let mut status = col()
            .items_end()
            .child(ui::pill(k, "OFFLINE", Tone::Neutral));
        if let Some(seen) = &p.last_seen {
            status = status.gap(4.0).child(
                text(format!("Seen {seen}"))
                    .font(k.ty.small)
                    .color(k.c.text_muted),
            );
        }
        r.child(status)
    };
    ui::surface(k).key(format!("peer-{}", p.name)).child(r)
}

/// The other devices on the tailnet, while connected.
fn peers_section(k: &Kit, st: &TailscaleStatus) -> Option<El> {
    if !st.running() {
        return None;
    }
    let online = st.peers.iter().filter(|p| p.online).count();
    let count = text(format!("{online} of {} online", st.peers.len()))
        .key("count")
        .font(k.ty.body)
        .color(k.c.text_muted);
    let items = if st.peers.is_empty() {
        vec![ui::note_card(
            k,
            "no-peers",
            &["No other devices on this tailnet."],
        )]
    } else {
        st.peers.iter().map(|p| peer_card(k, p)).collect()
    };
    Some(ui::section(
        k,
        "peers",
        None,
        "Tailscale Devices",
        Some(count),
        items,
    ))
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

fn detail(n: &NetworkState, k: &Kit, i: &NetworkInterface) -> El {
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
    let mut fields = Vec::new();
    let tailscale = tailscale_for(n, i);
    if let Some(st) = tailscale {
        fields.push(row().key("tailscale-row").w_full().child(ui::setting(
            k,
            "tailscale",
            "Tailscale",
            &tailscale_summary(n, st),
            st.can_toggle(),
            tailscale_switch(n, k, st),
        )));
    }
    fields.push(ui::field(
        k,
        "name",
        "Interface Name",
        ui::value_text(k, i.name.clone()),
    ));
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
    // Tunnels (Tailscale, WireGuard) have none.
    if !i.mac_address.is_empty() {
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
    }
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
        [Card::new()
            .title("Connection Status")
            .action(match tailscale {
                Some(st) => state_pill(k, if st.running() { "up" } else { "down" }),
                None => state_pill(k, &i.state),
            })
            .children(fields)
            .gap(20.0)
            .build(k)
            .key("status")]
        .into_iter()
        .chain(tailscale.and_then(|st| peers_section(k, st)))
        .collect(),
    )
}

pub fn view(s: &State, k: &Kit) -> El {
    let n = &s.network;
    if let Some(i) = n
        .selected
        .as_ref()
        .and_then(|name| n.interfaces.data.as_ref()?.iter().find(|i| &i.name == name))
    {
        return detail(n, k, i);
    }
    let busy = n.interfaces.loading || n.wifi.loading || n.tailscale.loading;
    let refresh = Button::new("Refresh")
        .icon(&icons::REFRESH_CW)
        .variant(ButtonVariant::Outline)
        .disabled(busy)
        .on_press(|s: &mut State| {
            load_interfaces(s);
            load_tailscale(s);
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
            ifaces.iter().map(|i| interface_card(n, k, i)).collect(),
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
