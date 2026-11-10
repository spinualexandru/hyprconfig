//! The app's state, its shell (sidebar and page) and its lifecycle hooks.

use rune::prefs::Prefs;
use rune::{AppSpec, Cx, El, ImageCache, KeyEvent, Tx, Waker, col, row, text};
use rune_kit::Kit;
use rune_kit::shell::{self, PaneState, Sidebar, SidebarRow, SplitView};

use crate::backend::hyprland::{self as hypr, ConfigInfo};
use crate::jobs::{Jobs, Remote};
use crate::pages::{self, Page};
use crate::theme::Palettes;
use crate::ui::Toasts;
use crate::{backend, fixtures, icons};

pub const APP_ID: &str = "hyprconfig";

pub struct State {
    pub page: Page,
    pub sidebar: PaneState,
    /// The appearance the user chose; `None` follows the desktop.
    pub dark: Option<bool>,
    pub palettes: Palettes,
    pub prefs: Option<Prefs>,
    pub jobs: Jobs,
    pub toasts: Toasts,
    pub images: Option<ImageCache>,
    /// Whether the user's config is `hyprland.lua` or `hyprland.conf`.
    pub config_info: Remote<ConfigInfo>,
    pub about: pages::about::AboutState,
    pub appearance: pages::appearance::AppearanceState,
    pub network: pages::network::NetworkState,
    pub audio: pages::audio::AudioState,
    pub displays: pages::displays::DisplaysState,
    pub keybinds: pages::keybinds::KeybindsState,
    pub variables: pages::vars::VarsState,
    pub envvars: pages::vars::VarsState,
    pub windowrules: pages::rules::RulesState,
    pub layerrules: pages::rules::RulesState,
}

impl State {
    /// A fresh state that touches nothing on disk (tests, headless fixtures).
    pub fn new() -> State {
        State {
            page: Page::General,
            sidebar: PaneState::new(232.0).range(180.0, 360.0),
            dark: None,
            palettes: Palettes::default(),
            prefs: None,
            jobs: Jobs::new(),
            toasts: Toasts::default(),
            images: None,
            config_info: Remote::default(),
            about: Default::default(),
            appearance: Default::default(),
            network: Default::default(),
            audio: Default::default(),
            displays: Default::default(),
            keybinds: Default::default(),
            variables: Default::default(),
            envvars: Default::default(),
            windowrules: Default::default(),
            layerrules: Default::default(),
        }
    }

    /// The state the app starts with: the saved appearance and the theme file's colours. Both
    /// are small local reads, done before the first frame so it draws in the right colours.
    pub fn launch() -> State {
        let mut s = State::new();
        let prefs = Prefs::open(APP_ID);
        s.dark = prefs.get::<bool>("dark");
        s.prefs = Some(prefs);
        if let Ok(css) = backend::config::get_theme_css() {
            s.palettes = Palettes::parse(&css);
        }
        s
    }

    pub fn is_lua(&self) -> bool {
        self.config_info
            .data
            .as_ref()
            .is_some_and(|c| c.format == "lua")
    }
}

impl Default for State {
    fn default() -> Self {
        State::new()
    }
}

/// Runs once the native window has drawn: from here on, jobs reach the system.
pub fn ready(s: &mut State, _tx: &mut Tx) {
    s.jobs.go_live();
    load_config_info(s);
    pages::enter(s, s.page);
}

pub fn load_config_info(s: &mut State) {
    let attempt = s.config_info.begin();
    s.jobs
        .run("config-info", hypr::get_config_info, move |s, r, _| {
            s.config_info.land(attempt, r);
        });
}

pub fn navigate(s: &mut State, page: Page, tx: &mut Tx) {
    if s.page != page {
        s.page = page;
        tx.instant();
        pages::enter(s, page);
    }
}

/// Re-reads `theme.css` (after matugen writes it) on a worker.
pub fn reload_theme(s: &mut State) {
    s.jobs
        .run("theme", backend::config::get_theme_css, |s, r, _| {
            if let Ok(css) = r {
                s.palettes = Palettes::parse(&css);
            }
        });
}

fn toggle_dark(s: &mut State, dark_now: bool) {
    let dark = !dark_now;
    s.dark = Some(dark);
    if let Some(p) = s.prefs.as_mut()
        && let Err(e) = p.set("dark", &dark)
    {
        eprintln!("hyprconfig: couldn't save the appearance: {e}");
    }
}

pub fn kit(s: &State, cx: &Cx<State>) -> (Kit, bool) {
    let dark = s.dark.unwrap_or_else(|| cx.system_dark());
    (s.palettes.kit(dark), dark)
}

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let (k, dark) = kit(s, cx);
    let mut layers = Vec::new();
    let main = pages::view(s, &k, cx, &mut layers);

    let brand = row()
        .key("brand")
        .w_full()
        .gap(10.0)
        .px(4.0)
        .py(6.0)
        .center_y()
        .child(
            rune::el()
                .key("logo")
                .size(32.0, 32.0)
                .radius(8.0)
                .fill(k.c.bg_accent)
                .center()
                .child(rune::icon(&icons::HYPRLAND, 18.0).color(k.c.text_contrast))
                .a11y_hidden(),
        )
        .child(
            col()
                .key("names")
                .shrink()
                .min_w(0.0)
                .child(
                    text("Hyprconfig")
                        .font(k.ty.ui_medium)
                        .color(k.c.text_strong)
                        .truncate(),
                )
                .child(
                    text("Hyprland Settings")
                        .font(k.ty.small)
                        .color(k.c.text_muted)
                        .truncate(),
                ),
        );

    let mut sidebar = Sidebar::new().label("Pages").header(brand);
    for &p in Page::ALL {
        sidebar = sidebar.child(
            SidebarRow::new(p.title())
                .icon(p.icon())
                .selected(s.page == p)
                .on_select(move |s: &mut State, tx| navigate(s, p, tx))
                .build(&k)
                .key(p.id()),
        );
    }
    let theme_row = SidebarRow::new(if dark { "Dark Mode" } else { "Light Mode" })
        .icon(if dark { &icons::MOON } else { &icons::SUN })
        .on_select(move |s: &mut State, _| toggle_dark(s, dark))
        .build(&k)
        .key("theme")
        .help(if dark {
            "Switch to light mode"
        } else {
            "Switch to dark mode"
        });
    let sidebar = sidebar.footer(theme_row).build(&k);

    let shell = SplitView::new(main)
        .sidebar(&s.sidebar, |s: &mut State| &mut s.sidebar, sidebar)
        .titlebar_inset(cx.titlebar_inset())
        .reduced_motion(cx.is_reduced_motion())
        .build(&k);
    if let Some(t) = s.toasts.build(&k) {
        layers.push(t);
    }
    rune_kit::root(shell, layers)
}

fn setup(s: &mut State, waker: Waker) {
    s.images = Some(ImageCache::new(waker.clone()));
    s.jobs.set_waker(waker);
}

pub fn poll(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    let mut changed = Jobs::drain(s, tx);
    changed |= s.toasts.expire(now);
    changed |= pages::displays::tick(s, now, tx);
    changed |= pages::audio::flush(s, now);
    changed |= pages::appearance::watch_image(s);
    changed
}

pub fn on_key(s: &mut State, event: &KeyEvent, _: &mut Tx) -> bool {
    if shell::is_sidebar_toggle(event) {
        s.sidebar.toggle();
        return true;
    }
    false
}

pub fn spec() -> AppSpec<State> {
    AppSpec::<State> {
        min_size: Some((760.0, 520.0)),
        families: ("Inter", "ui-monospace"),
        setup: Some(setup),
        poll: Some(poll),
        on_key: Some(on_key),
        scenes: fixtures::SCENES,
        ..AppSpec::new("Hyprconfig", (1100.0, 760.0), State::new, view)
    }
}

/// The text a toast shows for a failed call.
pub fn fail(s: &mut State, tx: &mut Tx, what: &str, e: String) {
    s.toasts.error(tx, format!("{what}: {e}"));
}
