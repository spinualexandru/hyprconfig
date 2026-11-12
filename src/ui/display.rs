//! Small display pieces: label/value fields, status pills, code chips and keycaps.

use rune::{Color, El, TextStyle, col, row, text};
use rune_kit::Kit;

use super::mono;

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
