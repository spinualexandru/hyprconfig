//! The app's state, its shell (sidebar and page) and its lifecycle hooks.

use std::sync::atomic::Ordering;
use std::time::Duration;

use rune::prefs::Prefs;
use rune::{AppSpec, Cx, El, Image, ImageCache, KeyEvent, Side, Tx, Waker, col, el, row, text};
use rune_kit::shell::{self, PaneState, Sidebar, SidebarRow, SplitView};
use rune_kit::{IconButton, IconButtonVariant, Kit};

use crate::backend::palette;
use crate::ipc;
use crate::jobs::Jobs;
use crate::pages::{self, Page};
use crate::theme::{Kits, Palettes};
use crate::ui::Toasts;
use crate::{backend, fixtures, icons};

pub const APP_ID: &str = "hyprconfig";
/// The window background around the main pane.
const FRAME: f32 = 8.0;
/// How much of the shell's title bar band to cut, keeping a frame-wide strip.
const TITLEBAR_TRIM: f32 = shell::TOOLBAR_HEIGHT - FRAME;
/// The collapsed sidebar's width: its icons, and the list's inset either side.
const RAIL: f32 = 56.0;
/// The height of a page's icon in the collapsed sidebar.
const RAIL_ROW: f32 = 32.0;

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

pub fn view(s: &State, cx: &mut Cx<State>) -> El {
    let (k, dark) = kit(s, cx);
    let mut layers = Vec::new();
    let main = pages::view(s, k, cx, &mut layers);

    // Collapsed, the sidebar narrows to a rail of icons rather than leaving the window. Both
    // write to the same pane state, so its shortcut, and dragging its edge, still open and
    // close it.
    let rail = PaneState::new(RAIL).range(RAIL, RAIL);
    let (pane, content) = if s.sidebar.is_open() {
        (&s.sidebar, sidebar(s, k, dark))
    } else {
        (&rail, sidebar_rail(s, k, dark))
    };
    let shell = SplitView::new(main)
        .sidebar(pane, |s: &mut State| &mut s.sidebar, content)
        .hide_sidebar_toggle()
        .frame_inset(FRAME)
        .titlebar_inset(cx.titlebar_inset())
        .reduced_motion(cx.is_reduced_motion())
        .build(k);
    // Hyprland draws no title bar, so the shell's empty band above the panes is pushed out of
    // the window, leaving the usual frame inset at the top.
    let shell = el().key("shell-frame").w_full().h_full().child(
        el().key("shell-trim")
            .pin(Some(0.0), Some(-TITLEBAR_TRIM), Some(0.0), Some(0.0))
            .child(shell),
    );
    if let Some(t) = s.toasts.build(k) {
        layers.push(t);
    }
    rune_kit::root(shell, layers)
}

/// The app's logo, embedded so it shows wherever the binary runs.
const LOGO_SVG: &[u8] = include_bytes!("../assets/hyprconfig.svg");
/// The logo's size in the sidebar, in points.
const LOGO_SIZE: f32 = 32.0;

/// The app's mark: its logo, or Hyprland's on an accent tile where no image cache runs
/// (headless tests).
fn logo(s: &State, k: &Kit) -> El {
    let mark = el().key("logo").size(LOGO_SIZE, LOGO_SIZE).a11y_hidden();
    match &s.logo {
        Some(img) => mark.pixels(img.pixels()),
        None => mark
            .radius(8.0)
            .fill(k.c.bg_accent)
            .center()
            .child(rune::icon(&icons::HYPRLAND, 18.0).color(k.c.text_contrast)),
    }
}

/// The button flipping light and dark mode.
fn theme_button(k: &Kit, dark: bool) -> El {
    let (icon, help) = if dark {
        (&icons::MOON, "Switch to light mode")
    } else {
        (&icons::SUN, "Switch to dark mode")
    };
    IconButton::new(icon)
        .variant(IconButtonVariant::GhostMuted)
        .label(help)
        .help(help)
        .on_press(move |s: &mut State| set_dark(s, Some(!dark)))
        .build(k)
        .key("theme")
}

/// The button collapsing the sidebar to its rail, or expanding the rail back.
fn sidebar_button(k: &Kit, open: bool) -> El {
    let (key, help) = if open {
        ("collapse", "Collapse sidebar")
    } else {
        ("expand", "Expand sidebar")
    };
    IconButton::new(&shell::SIDEBAR)
        .variant(IconButtonVariant::GhostMuted)
        .label(help)
        .help(help)
        .on_press(move |s: &mut State| s.sidebar.set_open(!open))
        .build(k)
        .key(key)
        .expanded(open)
}

/// The open sidebar: the app's name, a row per page, and the appearance and collapse buttons.
fn sidebar(s: &State, k: &Kit, dark: bool) -> El {
    let brand = row()
        .key("brand")
        .w_full()
        .gap(10.0)
        .px(4.0)
        .py(6.0)
        .center_y()
        .child(logo(s, k))
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
                .build(k)
                .key(p.id()),
        );
    }
    let footer = row()
        .key("footer")
        .w_full()
        .gap(4.0)
        .center_y()
        .child(theme_button(k, dark))
        .child(sidebar_button(k, true));
    sidebar.footer(footer).build(k)
}

/// The collapsed sidebar: the same pages and buttons as icons, each named by its tooltip.
fn sidebar_rail(s: &State, k: &Kit, dark: bool) -> El {
    let brand = row()
        .key("brand")
        .w_full()
        .py(6.0)
        .center()
        .child(logo(s, k).help("Hyprconfig").help_side(Side::Right));
    let mut rail = Sidebar::new().label("Pages").header(brand);
    for &p in Page::ALL {
        rail = rail.child(rail_item(k, p, s.page == p));
    }
    let footer = col()
        .key("footer")
        .w_full()
        .gap(4.0)
        .center()
        .child(theme_button(k, dark).help_side(Side::Right))
        .child(sidebar_button(k, false).help_side(Side::Right));
    rail.footer(footer).build(k)
}

/// A page's icon in the rail, selected as its sidebar row is, with its title as a tooltip.
fn rail_item(k: &Kit, p: Page, selected: bool) -> El {
    let c = &k.c;
    let glyph = if selected { c.icon_base } else { c.icon_muted };
    let mut item = row()
        .key(p.id())
        .label(p.title())
        .help(p.title())
        .help_side(Side::Right)
        .w_full()
        .h(RAIL_ROW)
        .center()
        .radius(k.radius.md)
        .motion(k.motion.control)
        .focusable(true)
        .focus_selected(selected)
        .child(
            rune::icon(p.icon(), 16.0)
                .key("icon")
                .color(glyph)
                .a11y_hidden(),
        );
    item = k.interactive(item);
    if selected {
        item = item.fill(c.pressed);
    }
    item.on_press_tx(move |s: &mut State, tx: &mut Tx| navigate(s, p, tx))
        .role("listitem")
}

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
