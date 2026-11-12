//! Page scaffolding and arrangement: pages, sections, the two-column grid, dialog bodies and
//! labelled settings rows.

use rune::{El, Icon, ScrollSpec, col, el, icon, row, spacer, text};
use rune_kit::Kit;

use super::{GUTTER, section_style, title_style};

/// A scrolling page: title, subtitle and actions over the page's sections.
pub fn page(
    k: &Kit,
    key: &str,
    title: &str,
    subtitle: &str,
    actions: Vec<El>,
    body: Vec<El>,
) -> El {
    col()
        .key(format!("page-{key}"))
        .w_full()
        .h_full()
        .min_h(0.0)
        .scroll(ScrollSpec {
            stick_bottom: false,
        })
        .child(
            col()
                .key("content")
                .w_full()
                .padding(GUTTER)
                .gap(GUTTER)
                .child(page_header(k, title, subtitle, actions))
                .children(body),
        )
}

pub fn page_header(k: &Kit, title: &str, subtitle: &str, actions: Vec<El>) -> El {
    row()
        .key("header")
        .w_full()
        .gap(16.0)
        .center_y()
        .child(
            col()
                .key("titles")
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(6.0)
                .child(
                    text(title.to_owned())
                        .key("title")
                        .role("heading")
                        .font(title_style())
                        .color(k.c.text_strong),
                )
                .child(
                    text(subtitle.to_owned())
                        .key("subtitle")
                        .font(k.ty.body)
                        .color(k.c.text_muted)
                        .wrap()
                        .w_full(),
                ),
        )
        .child(row().key("actions").gap(8.0).center_y().children(actions))
}

/// A titled group of cards (Network's "Connected", Audio's "Output Devices").
pub fn section(
    k: &Kit,
    key: &str,
    glyph: Option<&'static Icon>,
    title: &str,
    trailing: Option<El>,
    items: Vec<El>,
) -> El {
    let mut head = row().key("head").w_full().gap(8.0).center_y();
    if let Some(g) = glyph {
        head = head.child(icon(g, 18.0).color(k.c.icon_muted));
    }
    head = head
        .child(
            text(title.to_owned())
                .role("heading")
                .font(section_style())
                .color(k.c.text_strong),
        )
        .child(spacer());
    if let Some(t) = trailing {
        head = head.child(t);
    }
    col()
        .key(key)
        .w_full()
        .gap(12.0)
        .child(head)
        .child(col().key("items").w_full().gap(12.0).children(items))
}

/// Two equal columns that wrap their cells in reading order.
pub fn grid2(key: &str, cells: Vec<El>) -> El {
    let mut out = col().key(key).w_full().gap(16.0);
    let mut cells = cells.into_iter().enumerate().peekable();
    while let Some((i, a)) = cells.next() {
        let b = cells.next().map(|(_, b)| b);
        let mut r = row()
            .key(format!("r{}", i / 2))
            .w_full()
            .gap(16.0)
            .items_start()
            .child(a.w(0.0).grow(1.0));
        r = match b {
            Some(b) => r.child(b.w(0.0).grow(1.0)),
            None => r.child(el().w(0.0).grow(1.0)),
        };
        out = out.child(r);
    }
    out
}

/// A dialog's padded content over its right-aligned buttons.
pub fn dialog_body(content: El, buttons: Vec<El>) -> El {
    col()
        .key("content")
        .w_full()
        .px(16.0)
        .pb(16.0)
        .gap(20.0)
        .child(content)
        .child(
            row()
                .key("buttons")
                .w_full()
                .gap(8.0)
                .justify_end()
                .children(buttons),
        )
}

/// A labelled control: name and description on the left, the control on the right. Dimmed
/// while `enabled` is false.
pub fn setting(
    k: &Kit,
    key: &str,
    label: &str,
    description: &str,
    enabled: bool,
    control: El,
) -> El {
    row()
        .key(key)
        .w(0.0)
        .grow(1.0)
        .gap(16.0)
        .center_y()
        .opacity(if enabled { 1.0 } else { 0.5 })
        .child(
            col()
                .grow(1.0)
                .shrink()
                .min_w(0.0)
                .gap(2.0)
                .child(
                    text(label.to_owned())
                        .font(k.ty.body_medium)
                        .color(k.c.text_strong),
                )
                .child(
                    text(description.to_owned())
                        .font(k.ty.body)
                        .color(k.c.text_muted)
                        .wrap()
                        .w_full(),
                ),
        )
        .child(control)
}
