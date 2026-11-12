//! An interface's detail screen, with Tailscale's switch, status and peers.

use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Kit, Switch};

use super::view::{interface_title, state_pill};
use super::{NetworkState, set_tailscale};
use crate::app::State;
use crate::backend::hyprland::NetworkInterface;
use crate::backend::tailscale::{self, TailscalePeer, TailscaleStatus};
use crate::icons;
use crate::ui::{self, Card, Tone};

/// Tailscale's status, when `i` is its tunnel and the CLI answered.
pub(super) fn tailscale_for<'a>(
    n: &'a NetworkState,
    i: &NetworkInterface,
) -> Option<&'a TailscaleStatus> {
    if !tailscale::is_interface(&i.name) {
        return None;
    }
    n.tailscale.data.as_ref()?.as_ref()
}

/// The on/off switch for Tailscale. Shows where a switch in flight is going.
pub(super) fn tailscale_switch(n: &NetworkState, k: &Kit, st: &TailscaleStatus) -> El {
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

pub fn format_bytes(bytes: u64) -> String {
    if bytes == 0 {
        return "0 B".into();
    }
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let i = ((bytes as f64).ln() / 1024f64.ln()).floor().min(4.0) as usize;
    format!("{:.2} {}", bytes as f64 / 1024f64.powi(i as i32), UNITS[i])
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

pub(super) fn detail(n: &NetworkState, k: &Kit, i: &NetworkInterface) -> El {
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
