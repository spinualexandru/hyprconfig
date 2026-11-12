//! The Network page: interface details and the Tailscale switch.

use super::*;

#[test]
fn network_details_open_and_go_back() {
    let mut rt = on(Page::Network, |s| {
        s.network.interfaces = Remote::ready(fixtures::interfaces());
        s.network.wifi = Remote::ready(fixtures::wifi());
    });
    tap(&mut rt, "iface-wlan0");
    assert_eq!(rt.state.network.selected.as_deref(), Some("wlan0"));
    assert!(rt.find("page-network-detail").is_some());
    tap(&mut rt, "back");
    assert_eq!(rt.state.network.selected, None);
}

#[test]
fn tailscale_switches_from_the_list_without_opening_details() {
    let mut rt = on(Page::Network, |s| {
        s.network.interfaces = Remote::ready(fixtures::with_tailscale());
        s.network.wifi = Remote::ready(fixtures::wifi());
        s.network.tailscale = Remote::ready(Some(fixtures::tailscale(true)));
    });
    assert!(
        rt.find("iface-wlan0/tailscale-switch").is_none(),
        "only on its tunnel"
    );
    tap(&mut rt, "iface-tailscale0/tailscale-switch");
    assert!(asked(&rt, "tailscale-set"));
    assert_eq!(rt.state.network.tailscale_pending, Some(false));
    assert_eq!(
        rt.state.network.selected, None,
        "the switch doesn't open details"
    );
}

#[test]
fn tailscale_switches_from_its_details() {
    let mut rt = on(Page::Network, |s| {
        s.network.interfaces = Remote::ready(fixtures::with_tailscale());
        s.network.tailscale = Remote::ready(Some(fixtures::tailscale(false)));
        s.network.selected = Some("tailscale0".into());
    });
    assert!(rt.find("peers").is_none(), "no devices while disconnected");
    tap(&mut rt, "tailscale/tailscale-switch");
    assert_eq!(rt.state.network.tailscale_pending, Some(true));

    let rt = on(Page::Network, |s| {
        s.network.interfaces = Remote::ready(fixtures::with_tailscale());
        s.network.tailscale = Remote::ready(Some(fixtures::tailscale(true)));
        s.network.selected = Some("tailscale0".into());
    });
    assert!(rt.find("peers/items/peer-nas").is_some());
    assert!(rt.find("peers/items/peer-phone").is_some());

    // No switch without the CLI's answer, or on other interfaces' details.
    let rt = on(Page::Network, |s| {
        s.network.interfaces = Remote::ready(fixtures::with_tailscale());
        s.network.selected = Some("tailscale0".into());
    });
    assert!(rt.find("tailscale-switch").is_none());
    let rt = on(Page::Network, |s| {
        s.network.interfaces = Remote::ready(fixtures::with_tailscale());
        s.network.tailscale = Remote::ready(Some(fixtures::tailscale(true)));
        s.network.selected = Some("wlan0".into());
    });
    assert!(rt.find("tailscale-switch").is_none());
}
