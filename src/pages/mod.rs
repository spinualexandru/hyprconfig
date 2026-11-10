//! One module per sidebar page. Each owns its state, its loads and its view.

pub mod about;
pub mod appearance;
pub mod audio;
pub mod displays;
pub mod general;
pub mod keybinds;
pub mod network;
pub mod rules;
pub mod vars;

use rune::{Cx, El, Icon};
use rune_kit::Kit;

use crate::app::State;
use crate::icons;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    General,
    Appearance,
    Network,
    Audio,
    Displays,
    Keybinds,
    Variables,
    EnvVars,
    Windowrules,
    Layerrules,
    About,
}

impl Page {
    pub const ALL: &[Page] = &[
        Page::General,
        Page::Appearance,
        Page::Network,
        Page::Audio,
        Page::Displays,
        Page::Keybinds,
        Page::Variables,
        Page::EnvVars,
        Page::Windowrules,
        Page::Layerrules,
        Page::About,
    ];

    pub fn id(self) -> &'static str {
        match self {
            Page::General => "general",
            Page::Appearance => "appearance",
            Page::Network => "network",
            Page::Audio => "audio",
            Page::Displays => "displays",
            Page::Keybinds => "keybinds",
            Page::Variables => "variables",
            Page::EnvVars => "envvars",
            Page::Windowrules => "windowrules",
            Page::Layerrules => "layerrules",
            Page::About => "about",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Page::General => "General",
            Page::Appearance => "Appearance",
            Page::Network => "Network",
            Page::Audio => "Audio",
            Page::Displays => "Displays",
            Page::Keybinds => "Keybinds",
            Page::Variables => "Variables",
            Page::EnvVars => "Env Vars",
            Page::Windowrules => "Windowrules",
            Page::Layerrules => "Layerrules",
            Page::About => "About",
        }
    }

    pub fn icon(self) -> &'static Icon {
        match self {
            Page::General => &icons::SETTINGS,
            Page::Appearance => &icons::PALETTE,
            Page::Network => &icons::WIFI,
            Page::Audio => &icons::VOLUME_2,
            Page::Displays => &icons::MONITOR,
            Page::Keybinds => &icons::KEYBOARD,
            Page::Variables => &icons::DOLLAR_SIGN,
            Page::EnvVars => &icons::TERMINAL,
            Page::Windowrules => &icons::APP_WINDOW,
            Page::Layerrules => &icons::LAYERS,
            Page::About => &icons::INFO,
        }
    }
}

/// Loads what a page shows each time it opens; data from the last visit shows meanwhile.
pub fn enter(s: &mut State, page: Page) {
    match page {
        Page::General => {}
        Page::Appearance => appearance::load(s),
        Page::Network => network::load(s),
        Page::Audio => audio::load(s),
        Page::Displays => displays::load(s),
        Page::Keybinds => keybinds::load(s),
        Page::Variables => vars::load(s, vars::Kind::Variables),
        Page::EnvVars => vars::load(s, vars::Kind::Env),
        Page::Windowrules => rules::load(s, rules::Kind::Window),
        Page::Layerrules => rules::load(s, rules::Kind::Layer),
        Page::About => about::load(s),
    }
}

pub fn view(s: &State, k: &Kit, cx: &mut Cx<State>, layers: &mut Vec<El>) -> El {
    match s.page {
        Page::General => general::view(k),
        Page::Appearance => appearance::view(s, k, cx, layers),
        Page::Network => network::view(s, k),
        Page::Audio => audio::view(s, k),
        Page::Displays => displays::view(s, k, layers),
        Page::Keybinds => keybinds::view(s, k, layers),
        Page::Variables => vars::view(s, k, layers, vars::Kind::Variables),
        Page::EnvVars => vars::view(s, k, layers, vars::Kind::Env),
        Page::Windowrules => rules::view(s, k, rules::Kind::Window),
        Page::Layerrules => rules::view(s, k, rules::Kind::Layer),
        Page::About => about::view(s, k),
    }
}
