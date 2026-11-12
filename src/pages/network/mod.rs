mod detail;
mod view;

pub use view::view;

use crate::app::{self, State};
use crate::backend::hyprland::{self as hypr, NetworkInterface, WifiNetwork};
use crate::backend::tailscale::{self, TailscaleStatus};
use crate::jobs::{self, Remote};

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
