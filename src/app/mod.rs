//! The app's state, its shell (sidebar and page) and its lifecycle hooks.

use std::sync::atomic::Ordering;
use std::time::Duration;

use rune::prefs::Prefs;
use rune::{AppSpec, Cx, Image, ImageCache, KeyEvent, Tx, Waker};
use rune_kit::Kit;
use rune_kit::shell::PaneState;

use crate::backend::palette;
use crate::ipc;
use crate::jobs::Jobs;
use crate::pages::{self, Page};
use crate::theme::{Kits, Palettes};
use crate::ui::Toasts;
use crate::{backend, fixtures};

mod shell;

pub use shell::view;

pub const APP_ID: &str = "hyprconfig";

/// The pref holding the light or dark choice made in the sidebar.
const DARK_PREF: &str = "dark";

pub struct State {
    pub page: Page,
    pub sidebar: PaneState,
    /// The appearance the user chose; `None` follows the desktop.
    pub dark: Option<bool>,
    /// Noctalia's light or dark mode while it runs; it stands in for the desktop's.
    pub noctalia_dark: Option<bool>,
    /// The kits for the current colours, light and dark.
    pub kits: Kits,
    pub prefs: Option<Prefs>,
    pub jobs: Jobs,
    pub toasts: Toasts,
    pub images: Option<ImageCache>,
    /// The app's logo, rasterized once the image cache exists.
    pub logo: Option<Image>,
    pub general: pages::general::GeneralState,
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
        State::with_kits(Kits::default())
    }

    fn with_kits(kits: Kits) -> State {
        State {
            page: Page::General,
            sidebar: PaneState::new(232.0).range(180.0, 360.0),
            dark: None,
            noctalia_dark: None,
            kits,
            prefs: None,
            jobs: Jobs::new(),
            toasts: Toasts::default(),
            images: None,
            logo: None,
            general: Default::default(),
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

    /// The state the app starts with: the saved appearance, the preferred colour generator and
    /// the colours it made last time (or the theme file's). All are small local reads, done
    /// before the first frame so it draws in the right colours; the generator reruns once the
    /// window is up.
    pub fn launch() -> State {
        let palettes = palette::cached_css()
            .or_else(|| backend::config::get_theme_css().ok())
            .map(|css| Palettes::parse(&css))
            .unwrap_or_default();
        let mut s = State::with_kits(Kits::new(&palettes));
        let prefs = Prefs::open(APP_ID);
        s.dark = prefs.get::<bool>(DARK_PREF);
        s.general.generator = prefs
            .get(pages::general::GENERATOR_PREF)
            .unwrap_or_default();
        s.prefs = Some(prefs);
        s
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
    pages::general::refresh_colors(s);
    load_noctalia_mode(s);
    pages::general::load_integration(s);
    listen(s);
    watch_theme(s);
    pages::enter(s, s.page);
}

pub fn navigate(s: &mut State, page: Page, tx: &mut Tx) {
    if s.page != page {
        s.page = page;
        tx.instant();
        pages::enter(s, page);
    }
}

/// Recolours the app when its inputs change on disk: chromasync's config, hyprpaper's
/// wallpaper, `theme.css` and Noctalia's settings, or with the Noctalia integration on, the
/// palette Noctalia renders. Noctalia's hook often reports the same change first; recolouring
/// is idempotent, so the second report costs a resolve and nothing else, while a change the
/// hook missed or read too early still lands.
fn watch_theme(s: &mut State) {
    let hooked = s.general.hooked.clone();
    s.jobs.watch(
        "theme-files",
        Duration::from_millis(750),
        move || palette::stamps(&palette::watched_paths(hooked.load(Ordering::Relaxed))),
        |s, _| {
            // Noctalia's settings hold its light or dark mode as well as its scheme.
            load_noctalia_mode(s);
            pages::general::refresh_colors(s);
        },
    );
}

/// Listens for `hyprconfig notify` (Noctalia's template and hooks) and acts on what changed.
fn listen(s: &mut State) {
    s.jobs.stream(
        "ipc",
        |emit| match ipc::bind() {
            Ok(listener) => ipc::serve_batched(listener, Duration::from_millis(300), emit),
            Err(e) => eprintln!("hyprconfig: not listening for changes: {e}"),
        },
        |s, batch: Vec<ipc::Event>, _| {
            if batch.contains(&ipc::Event::Mode) {
                // Switching mode in Noctalia is newer than a choice made in the sidebar.
                set_dark(s, None);
                load_noctalia_mode(s);
            }
            if batch
                .iter()
                .any(|e| matches!(e, ipc::Event::Colors | ipc::Event::Wallpaper))
            {
                pages::general::refresh_colors(s);
            }
        },
    );
}

/// Pins light or dark mode and saves the choice; `None` forgets it, so the desktop's (or
/// Noctalia's) shows.
pub fn set_dark(s: &mut State, dark: Option<bool>) {
    s.dark = dark;
    let Some(p) = s.prefs.as_mut() else { return };
    let saved = match dark {
        Some(dark) => p.set(DARK_PREF, &dark),
        None => p.remove(DARK_PREF),
    };
    if let Err(e) = saved {
        eprintln!("hyprconfig: couldn't save the appearance: {e}");
    }
}

/// Follows Noctalia's light or dark mode, unless the user picked one in the sidebar since
/// Noctalia's last change.
pub fn load_noctalia_mode(s: &mut State) {
    s.jobs
        .run("noctalia-mode", palette::noctalia_dark, |s, r, _| match r {
            Ok(dark) => s.noctalia_dark = dark,
            Err(e) => eprintln!("hyprconfig: couldn't read Noctalia's mode: {e}"),
        });
}

pub fn kit<'a>(s: &'a State, cx: &Cx<State>) -> (&'a Kit, bool) {
    let dark = s
        .dark
        .or(s.noctalia_dark)
        .unwrap_or_else(|| cx.system_dark());
    (s.kits.get(dark), dark)
}

/// The app's logo, embedded so it shows wherever the binary runs.
const LOGO_SVG: &[u8] = include_bytes!("../../assets/hyprconfig.svg");
/// The logo's size in the sidebar, in points.
const LOGO_SIZE: f32 = 32.0;

fn setup(s: &mut State, waker: Waker) {
    let images = ImageCache::new(waker.clone());
    // Rasterized at 3× so it stays crisp on HiDPI screens; smaller scales draw from its mips.
    s.logo = Some(images.svg(LOGO_SVG, LOGO_SIZE, 3.0));
    s.images = Some(images);
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
    if rune_kit::shell::is_sidebar_toggle(event) {
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
