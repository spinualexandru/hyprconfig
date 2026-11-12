//! The shell: the sidebar, open or collapsed to a rail of icons, around the current page.

use rune::{Cx, El, Side, Tx, col, el, row, text};
use rune_kit::shell::{self, PaneState, Sidebar, SidebarRow, SplitView};
use rune_kit::{IconButton, IconButtonVariant, Kit};

use super::{LOGO_SIZE, State, kit, navigate, set_dark};
use crate::icons;
use crate::pages::{self, Page};

/// The window background around the main pane.
const FRAME: f32 = 8.0;
/// How much of the shell's title bar band to cut, keeping a frame-wide strip.
const TITLEBAR_TRIM: f32 = shell::TOOLBAR_HEIGHT - FRAME;
/// The collapsed sidebar's width: its icons, and the list's inset either side.
const RAIL: f32 = 56.0;
/// The height of a page's icon in the collapsed sidebar.
const RAIL_ROW: f32 = 32.0;

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
