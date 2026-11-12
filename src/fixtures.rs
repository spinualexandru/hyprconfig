//! Named scenes for headless runs (`--scene NAME --snap out.png`): each page with made-up data,
//! so snapshots never depend on the machine they run on.

use rune::TextEdit;

use crate::app::State;
use crate::backend::audio::{AudioDevice, AudioState as Mixer, AudioStream};
use crate::backend::hyprland::{
    DisplayMode, Keybind, MonitorInfo, NetworkInterface, SystemInfo, Variable, WifiNetwork,
};
use crate::backend::hyprpaper::{Daemon, HyprpaperConfig, Wallpaper};
use crate::backend::palette::{Colors, Generator};
use crate::backend::tailscale::{TailscalePeer, TailscaleStatus};
use crate::jobs::Remote;
use crate::pages::Page;
use crate::pages::displays::{self, Phase};
use crate::pages::keybinds::{self, FormMode, Tab};
use crate::pages::rules::RuleDetail;
use crate::pages::vars::{self, Entry, Kind};
use crate::ui::{ToastItem, ToastKind};

pub type Scene = (&'static str, fn(&mut State));

pub const SCENES: &[Scene] = &[
    ("general", |s| {
        s.general.colors = Remote::ready(Colors {
            used: Some(Generator::Chromasync),
            css: String::new(),
            skipped: Vec::new(),
        });
        s.general.integration = Remote::ready(Some(false));
    }),
    ("general-fallback", |s| {
        s.general.generator = Generator::Noctalia;
        s.general.colors = Remote::ready(Colors {
            used: None,
            css: String::new(),
            skipped: vec![
                (
                    Generator::Noctalia,
                    "builtin scheme \"Noctalia\" needs the integration".into(),
                ),
                (Generator::Chromasync, "not installed".into()),
            ],
        });
    }),
    ("dark", |s| s.dark = Some(true)),
    ("light", |s| s.dark = Some(false)),
    ("about", |s| {
        s.page = Page::About;
        s.about.info = Remote::ready(system_info());
    }),
    ("about-loading", |s| s.page = Page::About),
    ("appearance", |s| {
        s.page = Page::Appearance;
        s.appearance.config = Remote::ready(HyprpaperConfig {
            wallpapers: vec![Wallpaper {
                monitor: String::new(),
                path: "/nonexistent/wallpapers/aurora.png".into(),
                fit_mode: "cover".into(),
            }],
            source: Some(Daemon::Hyprpaper),
        });
        s.appearance.screens = vec![("DP-1".into(), 16.0 / 10.0)];
        s.appearance.matugen = true;
        s.appearance.prefs.enable = true;
    }),
    ("appearance-noctalia", |s| {
        s.page = Page::Appearance;
        s.appearance.config = Remote::ready(HyprpaperConfig {
            wallpapers: vec![Wallpaper {
                monitor: String::new(),
                path: "/nonexistent/wallpapers/aurora.png".into(),
                fit_mode: "crop".into(),
            }],
            source: Some(Daemon::Noctalia),
        });
    }),
    ("appearance-empty", |s| {
        s.page = Page::Appearance;
        s.appearance.config = Remote::ready(HyprpaperConfig {
            wallpapers: Vec::new(),
            source: None,
        });
    }),
    ("network", |s| {
        s.page = Page::Network;
        s.network.interfaces = Remote::ready(interfaces());
        s.network.wifi = Remote::ready(wifi());
    }),
    ("network-detail", |s| {
        s.page = Page::Network;
        s.network.interfaces = Remote::ready(interfaces());
        s.network.wifi = Remote::ready(wifi());
        s.network.selected = Some("wlan0".into());
    }),
    ("network-tailscale", |s| {
        s.page = Page::Network;
        s.network.interfaces = Remote::ready(with_tailscale());
        s.network.wifi = Remote::ready(wifi());
        s.network.tailscale = Remote::ready(Some(tailscale(true)));
    }),
    ("network-tailscale-detail", |s| {
        s.page = Page::Network;
        s.network.interfaces = Remote::ready(with_tailscale());
        s.network.wifi = Remote::ready(wifi());
        s.network.tailscale = Remote::ready(Some(tailscale(true)));
        s.network.selected = Some("tailscale0".into());
    }),
    ("network-loading", |s| s.page = Page::Network),
    ("audio", |s| {
        s.page = Page::Audio;
        s.audio.mixer = Remote::ready(mixer());
    }),
    ("audio-error", |s| {
        s.page = Page::Audio;
        s.audio.mixer = Remote::failed("wpctl not found. Is WirePlumber installed?");
    }),
    ("displays", |s| {
        s.page = Page::Displays;
        land_monitors(s);
    }),
    ("displays-changed", |s| {
        s.page = Page::Displays;
        land_monitors(s);
        displays::choose_resolution(s, 0, "1920x1080");
    }),
    ("displays-confirm", |s| {
        s.page = Page::Displays;
        land_monitors(s);
        displays::choose_resolution(s, 0, "1920x1080");
        s.displays.phase = Phase::Confirming {
            remaining: 7,
            next_tick: f64::MAX,
        };
    }),
    ("keybinds", |s| {
        s.page = Page::Keybinds;
        keybind_data(s);
    }),
    ("keybinds-universal", |s| {
        s.page = Page::Keybinds;
        keybind_data(s);
        s.keybinds.tab = Tab::Universal;
    }),
    ("keybinds-form", |s| {
        s.page = Page::Keybinds;
        keybind_data(s);
        keybinds::open_form(s, FormMode::Edit(1));
        s.keybinds.dispatcher = TextEdit::new("move");
    }),
    ("keybinds-loading", |s| s.page = Page::Keybinds),
    ("keybinds-universal-error", |s| {
        s.page = Page::Keybinds;
        keybind_data(s);
        s.keybinds.tab = Tab::Universal;
        s.keybinds.universal = Remote::failed("Failed to parse Hyprland config: line 12");
    }),
    ("keybinds-form-modifiers", |s| {
        s.page = Page::Keybinds;
        keybind_data(s);
        s.keybinds.variables.push(Variable {
            name: "mainMod".into(),
            value: "SUPER".into(),
            source_file: None,
        });
        if let Some(b) = s.keybinds.binds.data.as_mut().and_then(|b| b.get_mut(3)) {
            b.modifiers = vec!["$mainMod".into(), "CONTROL".into(), "MOD5".into()];
        }
        keybinds::open_form(s, FormMode::Edit(3));
    }),
    ("variables", |s| {
        s.page = Page::Variables;
        s.variables.list = Remote::ready(variable_entries());
    }),
    ("variables-stale", |s| {
        s.page = Page::Variables;
        s.variables.list = Remote::ready(variable_entries());
        let a = s.variables.list.begin();
        s.variables.list.land(
            a,
            Err("Hyprland Lua config file not found at \"~/.config/hypr/hyprland.lua\"".into()),
        );
    }),
    ("displays-stale", |s| {
        s.page = Page::Displays;
        land_monitors(s);
        let a = s.displays.monitors.begin();
        s.displays.monitors.land(
            a,
            Err("Failed to run hyprctl: No such file or directory".into()),
        );
    }),
    ("variables-editing", |s| {
        s.page = Page::Variables;
        s.variables.list = Remote::ready(variable_entries());
        vars::start_edit(s, Kind::Variables, "terminal".into(), "kitty");
    }),
    ("variables-picker", |s| {
        s.page = Page::Variables;
        s.variables.list = Remote::ready(variable_entries());
        vars::open_picker(s, Kind::Variables, "activeBorder".into());
    }),
    ("variables-add", |s| {
        s.page = Page::Variables;
        s.variables.list = Remote::ready(variable_entries());
        vars::open_add(s, Kind::Variables);
        s.variables.add_error = Some("Variable name is required".into());
    }),
    ("envvars", |s| {
        s.page = Page::EnvVars;
        s.envvars.list = Remote::ready(env_entries());
    }),
    ("envvars-empty", |s| {
        s.page = Page::EnvVars;
        s.envvars.list = Remote::ready(Vec::new());
    }),
    ("windowrules", |s| {
        s.page = Page::Windowrules;
        rules_data(s);
    }),
    ("layerrules-empty", |s| {
        s.page = Page::Layerrules;
        s.layerrules.names = Remote::ready(Vec::new());
    }),
    ("toasts", |s| {
        s.toasts.items = vec![
            ToastItem {
                id: 1,
                kind: ToastKind::Success,
                text: "Display settings saved".into(),
                due: f64::MAX,
            },
            ToastItem {
                id: 2,
                kind: ToastKind::Error,
                text: "Failed to set volume: wpctl exited with status 1".into(),
                due: f64::MAX,
            },
        ];
    }),
    ("collapsed", |s| s.sidebar.set_open(false)),
];

pub fn system_info() -> SystemInfo {
    SystemInfo {
        os: "CachyOS Linux".into(),
        hostname: "workstation".into(),
        kernel: "7.3.3-1-cachyos".into(),
        uptime: "3 hours, 12 minutes".into(),
        shell: "fish 4.2".into(),
        hyprland_version: "Hyprland 0.56.0".into(),
        gpus: vec![
            "AMD Radeon RX 7900 XTX".into(),
            "AMD Radeon Graphics (integrated)".into(),
        ],
        ram_used: "11.4 GB".into(),
        ram_total: "31.2 GB".into(),
        disk_used: "412G".into(),
        disk_total: "931G".into(),
        cpu: "AMD Ryzen 9 7950X (32) @ 5.88 GHz".into(),
    }
}

pub fn interfaces() -> Vec<NetworkInterface> {
    vec![
        NetworkInterface {
            name: "wlan0".into(),
            state: "up".into(),
            mac_address: "a4:5e:60:12:34:56".into(),
            ip_addresses: vec![
                "192.168.1.42/24".into(),
                "fe80::a65e:60ff:fe12:3456/64".into(),
            ],
            interface_type: "WiFi".into(),
            mtu: "1500".into(),
            rx_bytes: 3_221_225_472,
            tx_bytes: 412_316_860,
            ssid: Some("Home".into()),
        },
        NetworkInterface {
            name: "enp5s0".into(),
            state: "down".into(),
            mac_address: "00:d8:61:ab:cd:ef".into(),
            ip_addresses: Vec::new(),
            interface_type: "Ethernet".into(),
            mtu: "1500".into(),
            rx_bytes: 0,
            tx_bytes: 0,
            ssid: None,
        },
    ]
}

/// `interfaces()` with Tailscale's tunnel.
pub fn with_tailscale() -> Vec<NetworkInterface> {
    let mut all = interfaces();
    all.insert(
        1,
        NetworkInterface {
            name: "tailscale0".into(),
            state: "unknown".into(),
            mac_address: "".into(),
            ip_addresses: vec!["100.101.102.103/32".into(), "fd7a:115c:a1e0::1/128".into()],
            interface_type: "Tailscale".into(),
            mtu: "1280".into(),
            rx_bytes: 48_234_496,
            tx_bytes: 9_437_184,
            ssid: None,
        },
    );
    all
}

pub fn tailscale(running: bool) -> TailscaleStatus {
    TailscaleStatus {
        backend_state: if running { "Running" } else { "Stopped" }.into(),
        dns_name: Some("desk.tail1234.ts.net".into()),
        peers: if running { peers() } else { Vec::new() },
    }
}

pub fn peers() -> Vec<TailscalePeer> {
    let p = |name: &str, os: &str, ip: &str, online, exit_node, last_seen: Option<&str>| {
        TailscalePeer {
            name: name.into(),
            os: os.into(),
            ips: vec![ip.into()],
            online,
            exit_node,
            last_seen: last_seen.map(Into::into),
        }
    };
    vec![
        p("laptop", "linux", "100.101.102.104", true, false, None),
        p("nas", "linux", "100.101.102.105", true, true, None),
        p(
            "phone",
            "android",
            "100.101.102.106",
            false,
            false,
            Some("8 days ago"),
        ),
    ]
}

pub fn wifi() -> Vec<WifiNetwork> {
    let n = |ssid: &str, signal, security: &str, connected, bssid: &str, freq: &str| WifiNetwork {
        ssid: ssid.into(),
        signal_strength: signal,
        security: security.into(),
        connected,
        bssid: bssid.into(),
        frequency: freq.into(),
    };
    vec![
        n("Home", 82, "WPA3", true, "aa:bb:cc:00:00:01", "5180 MHz"),
        n(
            "Neighbour",
            58,
            "WPA2",
            false,
            "aa:bb:cc:00:00:02",
            "2437 MHz",
        ),
        n("Café", 31, "Open", false, "aa:bb:cc:00:00:03", "2412 MHz"),
    ]
}

pub fn mixer() -> Mixer {
    let dev = |id, description: &str, kind: &str, volume, muted, is_default| AudioDevice {
        id,
        name: format!("node{id}"),
        description: description.into(),
        device_type: kind.into(),
        volume,
        muted,
        is_default,
    };
    Mixer {
        sinks: vec![
            dev(
                41,
                "Starship/Matisse HD Audio Controller",
                "sink",
                0.62,
                false,
                true,
            ),
            dev(42, "USB Headphones", "sink", 1.2, false, false),
        ],
        sources: vec![dev(51, "Blue Yeti Microphone", "source", 0.8, true, true)],
        streams: vec![AudioStream {
            id: 71,
            app_name: "Firefox".into(),
            media_name: Some("Lo-fi beats to configure Hyprland to".into()),
            volume: 0.9,
            muted: false,
        }],
    }
}

pub fn monitors() -> Vec<MonitorInfo> {
    let mode = |w, h, r| DisplayMode {
        width: w,
        height: h,
        refresh_rate: r,
    };
    vec![
        MonitorInfo {
            id: 0,
            name: "DP-1".into(),
            description: "Dell Inc. DELL U2723QE".into(),
            width: 3840,
            height: 2160,
            refresh_rate: 60.0,
            x: 0,
            y: 0,
            scale: 1.5,
            transform: "Normal".into(),
            active_workspace_id: 1,
            active_workspace_name: "1".into(),
            available_modes: vec![
                mode(3840, 2160, 60.0),
                mode(3840, 2160, 30.0),
                mode(2560, 1440, 59.95),
                mode(1920, 1080, 60.0),
                mode(1920, 1080, 50.0),
            ],
        },
        MonitorInfo {
            id: 1,
            name: "HDMI-A-1".into(),
            description: "LG Electronics LG ULTRAGEAR".into(),
            width: 2560,
            height: 1440,
            refresh_rate: 143.97,
            x: 2560,
            y: 0,
            scale: 1.0,
            transform: "Normal".into(),
            active_workspace_id: 2,
            active_workspace_name: "2".into(),
            available_modes: vec![
                mode(2560, 1440, 143.97),
                mode(2560, 1440, 59.95),
                mode(1920, 1080, 120.0),
            ],
        },
    ]
}

pub fn land_monitors(s: &mut State) {
    s.displays.monitors = Remote::ready(monitors());
    s.displays.original = monitors();
    s.displays.chosen = monitors()
        .iter()
        .map(|m| displays::Chosen {
            id: m.id,
            resolution: displays::resolution_of(m),
            rate: displays::rate_label(m.refresh_rate),
        })
        .collect();
}

pub fn binds() -> Vec<Keybind> {
    let b = |mods: &[&str], key: &str, dispatcher: &str, params: &str| Keybind {
        modifiers: mods.iter().map(|m| (*m).to_owned()).collect(),
        key: key.into(),
        dispatcher: dispatcher.into(),
        params: params.into(),
    };
    vec![
        b(&["SUPER"], "Return", "exec", "$terminal"),
        b(&["SUPER"], "Q", "killactive", ""),
        b(&["SUPER", "SHIFT"], "E", "exec", "${menu} --show drun"),
        b(&["SUPER", "CTRL"], "left", "movefocus", "l"),
        b(
            &[],
            "XF86AudioRaiseVolume",
            "exec",
            "wpctl set-volume -l 1.5 @DEFAULT_AUDIO_SINK@ 5%+",
        ),
    ]
}

pub fn keybind_data(s: &mut State) {
    s.keybinds.binds = Remote::ready(binds());
    s.keybinds.universal = Remote::ready(vec![Keybind {
        modifiers: vec!["SUPER".into()],
        key: "Escape".into(),
        dispatcher: "submap".into(),
        params: "reset".into(),
    }]);
    s.keybinds.variables = vec![
        Variable {
            name: "terminal".into(),
            value: "kitty".into(),
            source_file: None,
        },
        Variable {
            name: "menu".into(),
            value: "wofi".into(),
            source_file: None,
        },
    ];
}

pub fn env_entries() -> Vec<Entry> {
    vec![
        Entry {
            name: "XCURSOR_SIZE".into(),
            value: "24".into(),
            index: 0,
        },
        Entry {
            name: "QT_QPA_PLATFORM".into(),
            value: "wayland;xcb".into(),
            index: 1,
        },
        Entry {
            name: "QT_QPA_PLATFORMTHEME".into(),
            value: "qt6ct".into(),
            index: 2,
        },
    ]
}

pub fn variable_entries() -> Vec<Entry> {
    vec![
        Entry {
            name: "terminal".into(),
            value: "kitty".into(),
            index: 0,
        },
        Entry {
            name: "menu".into(),
            value: "wofi".into(),
            index: 1,
        },
        Entry {
            name: "mainMod".into(),
            value: "SUPER".into(),
            index: 2,
        },
        Entry {
            name: "gaps".into(),
            value: "5".into(),
            index: 3,
        },
        Entry {
            name: "accent".into(),
            value: "rgb(e2e2e9)".into(),
            index: 4,
        },
        Entry {
            name: "activeBorder".into(),
            value: "rgba(33ccffee) rgba(00ff99ee) 45deg".into(),
            index: 5,
        },
    ]
}

pub fn rules_data(s: &mut State) {
    s.windowrules.names = Remote::ready(vec![
        "float-pavucontrol".into(),
        "firefox-pip".into(),
        "steam-games".into(),
    ]);
    s.windowrules.open = vec!["firefox-pip".into(), "steam-games".into()];
    s.windowrules.details = vec![
        (
            "firefox-pip".into(),
            Remote::ready(RuleDetail {
                enabled: true,
                matches: vec![
                    ("class".into(), "^(firefox)$".into()),
                    ("title".into(), "^(Picture-in-Picture)$".into()),
                ],
                effects: vec![
                    ("float".into(), "true".into()),
                    ("pin".into(), "true".into()),
                    ("size".into(), "640 360".into()),
                ],
            }),
        ),
        (
            "steam-games".into(),
            Remote::ready(RuleDetail {
                enabled: false,
                matches: vec![("class".into(), "^(steam_app_.*)$".into())],
                effects: vec![("immediate".into(), "true".into())],
            }),
        ),
    ];
}
