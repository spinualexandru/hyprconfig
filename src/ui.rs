//! Shared building blocks: page scaffolding, cards, pills, keycaps, table rows, skeletons, a
//! slider and toasts. Everything reads colours and type from the kit.

use rune::{
    Color, Drag, El, Icon, Phase, ScrollSpec, TextStyle, Tx, col, el, icon, row, spacer, text,
};
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant, Kit, Toast, ToastRegion};

use crate::app::State;
use crate::icons;

pub const GUTTER: f32 = 24.0;

pub fn title_style() -> TextStyle {
    TextStyle::new(26.0, 700.0).lh(32.0)
}

pub fn section_style() -> TextStyle {
    TextStyle::new(17.0, 600.0).lh(24.0)
}

pub fn card_title_style() -> TextStyle {
    TextStyle::new(15.0, 600.0).lh(20.0)
}

pub fn mono(k: &Kit) -> TextStyle {
    TextStyle {
        size: 12.5,
        line_height: 18.0,
        ..k.ty.mono
    }
}

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

/// A bordered surface with an optional header (icon tile, title, description, actions).
pub struct Card {
    icon: Option<&'static Icon>,
    title: Option<String>,
    description: Option<String>,
    actions: Vec<El>,
    body: Vec<El>,
    border: Option<Color>,
    gap: f32,
}

impl Card {
    pub fn new() -> Card {
        Card {
            icon: None,
            title: None,
            description: None,
            actions: Vec::new(),
            body: Vec::new(),
            border: None,
            gap: 16.0,
        }
    }
    pub fn icon(mut self, i: &'static Icon) -> Self {
        self.icon = Some(i);
        self
    }
    pub fn title(mut self, t: impl Into<String>) -> Self {
        self.title = Some(t.into());
        self
    }
    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }
    pub fn action(mut self, a: El) -> Self {
        self.actions.push(a);
        self
    }
    pub fn child(mut self, c: El) -> Self {
        self.body.push(c);
        self
    }
    pub fn children(mut self, c: impl IntoIterator<Item = El>) -> Self {
        self.body.extend(c);
        self
    }
    pub fn border(mut self, c: Color) -> Self {
        self.border = Some(c);
        self
    }
    pub fn gap(mut self, g: f32) -> Self {
        self.gap = g;
        self
    }
    pub fn build(self, k: &Kit) -> El {
        let mut root = surface(k).gap(self.gap);
        if let Some(b) = self.border {
            root = root.border(1.0, b);
        }
        if self.title.is_some() || self.description.is_some() || !self.actions.is_empty() {
            let mut head = row().key("card-head").w_full().gap(12.0).center_y();
            if let Some(i) = self.icon {
                head = head.child(icon_tile(k, i));
            }
            let mut titles = col().key("titles").grow(1.0).shrink().min_w(0.0).gap(2.0);
            if let Some(t) = self.title {
                titles = titles.child(
                    text(t)
                        .key("title")
                        .role("heading")
                        .font(card_title_style())
                        .color(k.c.text_strong),
                );
            }
            if let Some(d) = self.description {
                titles = titles.child(
                    text(d)
                        .key("description")
                        .font(k.ty.body)
                        .color(k.c.text_muted)
                        .wrap()
                        .w_full(),
                );
            }
            head = head.child(titles);
            if !self.actions.is_empty() {
                head = head.child(
                    row()
                        .key("card-actions")
                        .gap(8.0)
                        .center_y()
                        .children(self.actions),
                );
            }
            root = root.child(head);
        }
        root.children(self.body)
    }
}

/// A card's surface without the header logic, for custom layouts.
pub fn surface(k: &Kit) -> El {
    col()
        .w_full()
        .padding(20.0)
        .radius(12.0)
        .fill(k.c.surface_raised_base)
        .border(1.0, k.c.border_weak)
}

/// The rounded square behind a card's or row's icon.
pub fn icon_tile(k: &Kit, i: &'static Icon) -> El {
    icon_tile_colored(k, i, k.c.text_accent)
}

pub fn icon_tile_colored(_k: &Kit, i: &'static Icon, ink: Color) -> El {
    el().key("tile")
        .size(40.0, 40.0)
        .radius(10.0)
        .fill(ink.alpha(0.12))
        .center()
        .child(icon(i, 20.0).color(ink))
        .a11y_hidden()
}

/// A muted card holding one line of text (empty states).
pub fn note_card(k: &Kit, key: &str, lines: &[&str]) -> El {
    surface(k)
        .key(key)
        .gap(8.0)
        .children(lines.iter().enumerate().map(|(i, l)| {
            text((*l).to_owned())
                .key(format!("line{i}"))
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .w_full()
        }))
}

/// A danger-bordered card with an error message.
pub fn error_card(k: &Kit, key: &str, title: &str, message: &str) -> El {
    Card::new()
        .title(title)
        .border(k.c.danger.icon)
        .gap(8.0)
        .child(
            text(message.to_owned())
                .key("message")
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .w_full()
                .selectable(true),
        )
        .build(k)
        .key(key)
}

/// A label over a value (About, Network details).
pub fn field(k: &Kit, key: &str, label: &str, value: El) -> El {
    col()
        .key(key)
        .gap(4.0)
        .min_w(0.0)
        .child(
            text(label.to_owned())
                .key("label")
                .font(k.ty.body_medium)
                .color(k.c.text_strong),
        )
        .child(value)
}

pub fn value_text(k: &Kit, v: impl Into<String>) -> El {
    text(v.into())
        .key("value")
        .font(k.ty.body)
        .color(k.c.text_muted)
        .wrap()
        .w_full()
        .selectable(true)
        .selection_color(k.selection())
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Accent,
    Outline,
    Success,
    Warning,
    Danger,
}

/// A small status label.
pub fn pill(k: &Kit, label: impl Into<String>, tone: Tone) -> El {
    let c = &k.c;
    let (fill, ink, border) = match tone {
        Tone::Neutral => (c.layer_03, c.text_base, None),
        Tone::Accent => (c.bg_accent, c.text_contrast, None),
        Tone::Outline => (Color::TRANSPARENT, c.text_base, Some(c.border_base)),
        Tone::Success => (c.success.icon.alpha(0.16), c.success.icon, None),
        Tone::Warning => (c.warning.icon.alpha(0.16), c.warning.icon, None),
        Tone::Danger => (c.danger.icon.alpha(0.14), c.danger.icon, None),
    };
    let label = label.into();
    let e = row()
        .role("status")
        .label(label.clone())
        .h(22.0)
        .px(8.0)
        .center()
        .radius(6.0)
        .fill(fill)
        .child(
            text(label)
                .font(k.ty.text_small_medium)
                .color(ink)
                .a11y_hidden(),
        );
    match border {
        Some(b) => e.border(1.0, b),
        None => e,
    }
}

/// Monospace text on a muted chip.
pub fn code(k: &Kit, s: impl Into<String>) -> El {
    row()
        .px(6.0)
        .py(2.0)
        .radius(5.0)
        .fill(k.c.layer_02)
        .child(text(s.into()).font(mono(k)).color(k.c.text_base))
}

/// One key on a keycap; a long key name truncates and shows whole on hover.
pub fn keycap(k: &Kit, label: &str) -> El {
    row()
        .h(24.0)
        .min_w(24.0)
        .shrink()
        .px(7.0)
        .center()
        .radius(6.0)
        .fill(k.c.layer_02)
        .border(1.0, k.c.border_base)
        .child(
            text(label.to_owned())
                .font(TextStyle {
                    size: 12.0,
                    line_height: 16.0,
                    ..k.ty.ui_medium
                })
                .color(k.c.text_base)
                .shrink()
                .min_w(0.0)
                .truncate()
                .help_when_truncated(label.to_owned()),
        )
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

/// A shimmer-free placeholder block.
pub fn skeleton(k: &Kit, w: Option<f32>, h: f32) -> El {
    let e = el().h(h).radius(6.0).fill(k.c.layer_03).a11y_hidden();
    match w {
        Some(w) => e.w(w),
        None => e.w(0.0).grow(1.0),
    }
}

/// Rows of placeholder blocks: each row lists fixed widths, `None` grows.
pub fn skeleton_rows(k: &Kit, rows: usize, shape: &[Option<f32>]) -> El {
    col()
        .key("skeleton")
        .w_full()
        .gap(16.0)
        .label("Loading")
        .children((0..rows).map(|i| {
            row()
                .key(format!("s{i}"))
                .w_full()
                .gap(16.0)
                .children(shape.iter().map(|w| skeleton(k, *w, 22.0)))
        }))
}

/// The refresh button pages share; it spins nothing, it disables while loading.
pub fn refresh_button(loading: bool, f: impl Fn(&mut State, &mut Tx) + 'static) -> Button {
    Button::new(if loading { "Refreshing…" } else { "Refresh" })
        .icon(&icons::REFRESH_CW)
        .variant(ButtonVariant::Outline)
        .disabled(loading)
        .on_press_tx(f)
}

pub fn icon_button(i: &'static Icon, label: &str) -> IconButton {
    IconButton::new(i)
        .variant(IconButtonVariant::Ghost)
        .label(label)
        .help(label)
}

/// A table: a header row, then rows. `widths` are fixed points or `None` to share the rest.
pub fn table(k: &Kit, key: &str, headers: &[(&str, Option<f32>)], rows: Vec<El>) -> El {
    let head = row()
        .key("thead")
        .w_full()
        .h(36.0)
        .px(12.0)
        .gap(12.0)
        .center_y()
        .fill(k.c.layer_01)
        .children(headers.iter().map(|(h, w)| {
            cell(
                text((*h).to_owned())
                    .font(k.ty.text_small_medium)
                    .color(k.c.text_muted),
                *w,
            )
        }));
    col()
        .key(key)
        .role("table")
        .w_full()
        .radius(8.0)
        .border(1.0, k.c.border_weak)
        .clip_rounded(8.0)
        .child(head)
        .children(rows.into_iter().map(|r| {
            col()
                .w_full()
                .child(el().w_full().h(1.0).fill(k.c.border_weak).a11y_hidden())
                .child(r)
        }))
}

/// A table row's cells laid out like its header.
pub fn table_row(k: &Kit, key: impl Into<String>, widths: &[Option<f32>], cells: Vec<El>) -> El {
    row()
        .key(key.into())
        .role("row")
        .w_full()
        .min_h(48.0)
        .px(12.0)
        .py(6.0)
        .gap(12.0)
        .center_y()
        .hover(k.c.hover)
        .children(
            cells
                .into_iter()
                .zip(widths.iter())
                .map(|(c, w)| cell(c, *w)),
        )
}

fn cell(content: El, w: Option<f32>) -> El {
    let c = row().min_w(0.0).center_y().child(content);
    match w {
        Some(w) => c.w(w).shrink(),
        None => c.w(0.0).grow(1.0),
    }
}

/// A horizontal slider over `0..=max` that fills its row. `on_change` runs as it drags (the
/// value lands at once). Escape during a drag puts the old value back.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    k: &Kit,
    key: &str,
    label: &str,
    value: f32,
    max: f32,
    step: f32,
    disabled: bool,
    on_change: impl Fn(&mut State, f32, &mut Tx) + Clone + 'static,
) -> El {
    const KNOB: f32 = 18.0;
    const BAR: f32 = 4.0;
    let v = (value / max).clamp(0.0, 1.0);
    let to_value =
        move |x: f32, w: f32| ((x - KNOB / 2.0) / (w - KNOB).max(1.0)).clamp(0.0, 1.0) * max;
    let accent = if disabled {
        k.c.border_strong
    } else {
        k.c.bg_accent
    };
    let mut s = row()
        .key(key)
        .role("slider")
        .label(label.to_owned())
        .range_value(f64::from(value), 0.0, f64::from(max))
        .range_step(f64::from(step))
        .h(22.0)
        .w(0.0)
        .grow(1.0)
        .center_y()
        .opacity(if disabled { 0.5 } else { 1.0 })
        .child(
            el().key("filled")
                .w(0.0)
                .grow(v)
                .h(BAR)
                .radius(BAR / 2.0)
                .fill(accent)
                .a11y_hidden(),
        )
        .child(
            el().key("knob")
                .size(KNOB, KNOB)
                .radius(KNOB / 2.0)
                .fill(k.c.knob)
                .border(1.0, k.c.border_base)
                .shadow(1.0, 3.0, Color::BLACK.alpha(0.18))
                .a11y_hidden(),
        )
        .child(
            el().key("rest")
                .w(0.0)
                .grow(1.0 - v)
                .h(BAR)
                .radius(BAR / 2.0)
                .fill(k.c.layer_03)
                .a11y_hidden(),
        );
    if !disabled {
        let step_change = on_change.clone();
        s = s
            .focusable(true)
            .on_step(move |st: &mut State, steps, tx| {
                let next = (value + steps as f32 * step).clamp(0.0, max);
                step_change(st, next, tx);
            })
            .on_drag(move |st: &mut State, d: &mut Drag, tx| match d.phase {
                Phase::Down => {
                    d.data = value;
                    on_change(st, to_value(d.x, d.w), tx);
                    tx.instant();
                }
                Phase::Move => {
                    on_change(st, to_value(d.x, d.w), tx);
                    tx.instant();
                }
                Phase::Up => {
                    if d.cancelled() {
                        on_change(st, d.data, tx);
                    }
                }
            });
    }
    s
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastKind {
    Success,
    Warning,
    Error,
}

#[derive(Clone, Debug)]
pub struct ToastItem {
    pub id: u64,
    pub kind: ToastKind,
    pub text: String,
    pub due: f64,
}

/// Notifications in the corner; each closes after a few seconds or on its close button.
#[derive(Default)]
pub struct Toasts {
    next: u64,
    pub items: Vec<ToastItem>,
}

impl Toasts {
    const SECS: f64 = 4.5;
    const MAX: usize = 3;

    pub fn push(&mut self, tx: &mut Tx, kind: ToastKind, text: impl Into<String>) {
        self.next += 1;
        let due = tx.clock() + Self::SECS;
        self.items.push(ToastItem {
            id: self.next,
            kind,
            text: text.into(),
            due,
        });
        if self.items.len() > Self::MAX {
            self.items.remove(0);
        }
        tx.wake_at(due);
    }

    pub fn success(&mut self, tx: &mut Tx, text: impl Into<String>) {
        self.push(tx, ToastKind::Success, text);
    }
    pub fn warning(&mut self, tx: &mut Tx, text: impl Into<String>) {
        self.push(tx, ToastKind::Warning, text);
    }
    pub fn error(&mut self, tx: &mut Tx, text: impl Into<String>) {
        self.push(tx, ToastKind::Error, text);
    }

    /// Drops expired toasts; true when any went.
    pub fn expire(&mut self, now: f64) -> bool {
        let before = self.items.len();
        self.items.retain(|t| t.due > now);
        self.items.len() != before
    }

    pub fn build(&self, k: &Kit) -> Option<El> {
        if self.items.is_empty() {
            return None;
        }
        let mut region = ToastRegion::new();
        for t in &self.items {
            let (title, glyph, ink) = match t.kind {
                ToastKind::Success => ("Done", &icons::CHECK, k.c.success.icon),
                ToastKind::Warning => ("Warning", &icons::INFO, k.c.warning.icon),
                ToastKind::Error => ("Error", &icons::X, k.c.danger.icon),
            };
            let id = t.id;
            region = region.toast(
                Toast::new(format!("toast-{id}"), title)
                    .description(t.text.clone())
                    .lead(icon(glyph, 16.0).color(ink))
                    .on_close(move |s: &mut State| s.toasts.items.retain(|t| t.id != id))
                    .build(k),
            );
        }
        Some(region.build(k))
    }
}
