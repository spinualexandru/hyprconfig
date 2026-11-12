//! The colour picker that opens on a row's swatch: its state, drag handling and popover body.

use rune::{Color, El, Key, KeyEvent, TextEdit, Tx, col, row};
use rune_kit::{Button, ButtonVariant, Kit, TextInput};

use super::view::swatch;
use super::*;
use crate::app::State;
use crate::color::{self, Found, Rgba};
use crate::ui;

/// A colour being picked for one row. Nothing is written until Apply.
#[derive(Clone, Debug)]
pub struct Picker {
    pub id: String,
    pub entry: Entry,
    /// The colour in the value as loaded, and where it sits.
    pub found: Found,
    /// Hue (degrees), saturation, value, alpha. Hue is kept even where grey makes it moot.
    pub hsva: [f32; 4],
    pub dirty: bool,
}

impl Picker {
    pub fn rgba(&self) -> Rgba {
        Rgba::from_hsva(self.hsva)
    }

    /// The value with the picked colour in place of the old one.
    pub fn value(&self) -> String {
        if self.dirty {
            color::replace(&self.entry.value, &self.found, self.rgba())
        } else {
            self.entry.value.clone()
        }
    }
}

pub fn open_picker(s: &mut State, kind: Kind, id: String) {
    let v = of(s, kind);
    if v.list.settling() || v.edit_busy {
        return;
    }
    let Some(entry) = v
        .list
        .data
        .iter()
        .flatten()
        .find(|e| e.id(kind) == id)
        .cloned()
    else {
        return;
    };
    let Some(found) = color::find(&entry.value) else {
        return;
    };
    end_edit(v);
    v.picker_hex = TextEdit::new(found.rgba.hex(found.rgba.a < 1.0));
    v.picker = Some(Picker {
        id,
        hsva: found.rgba.hsva(),
        entry,
        found,
        dirty: false,
    });
}

pub fn close_picker(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    if !v.edit_busy {
        v.picker = None;
    }
}

pub fn apply_picker(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    if v.edit_busy {
        return;
    }
    let Some(p) = v.picker.clone() else {
        return;
    };
    if !p.dirty {
        v.picker = None;
        return;
    }
    let value = p.value();
    write_value(s, kind, p.entry, value);
}

/// Changes the picked colour from a drag, and shows it in the hex field.
fn pick(s: &mut State, kind: Kind, f: impl FnOnce(&mut [f32; 4])) {
    let v = of(s, kind);
    let Some(p) = v.picker.as_mut() else {
        return;
    };
    if v.edit_busy {
        return;
    }
    f(&mut p.hsva);
    p.dirty = true;
    let c = p.rgba();
    v.picker_hex = TextEdit::new(c.hex(c.a < 1.0));
}

/// The hex field changed: take the colour once it reads as one.
pub fn hex_typed(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    let (Some(p), Some(c)) = (v.picker.as_mut(), color::parse_hex(&v.picker_hex.text)) else {
        return;
    };
    let [h, sat, val, a] = c.hsva();
    // A grey has no hue of its own: keep the one the sliders show.
    let h = if sat == 0.0 || val == 0.0 {
        p.hsva[0]
    } else {
        h
    };
    p.hsva = [h, sat, val, a];
    p.dirty = true;
}

const PICK_W: f32 = 216.0;
const PICK_H: f32 = 144.0;
const BAR_H: f32 = 12.0;
const THUMB: f32 = 16.0;

/// A round handle centred at `x, y` in its track.
fn thumb(x: f32, y: f32, fill: Color) -> El {
    rune::el()
        .absolute(x - THUMB / 2.0, y - THUMB / 2.0)
        .size(THUMB, THUMB)
        .radius(THUMB / 2.0)
        .fill(fill)
        .border(2.0, Color::WHITE)
        .shadow(1.0, 3.0, Color::BLACK.alpha(0.35))
        .a11y_hidden()
        .key("thumb")
}

/// A drag on a picker track: `f` gets the pointer as fractions of the track's width and height.
fn track_drag(
    kind: Kind,
    f: fn(&mut [f32; 4], f32, f32),
) -> impl Fn(&mut State, &mut rune::Drag, &mut Tx) {
    move |s, d, tx| {
        if d.phase == rune::Phase::Up {
            return;
        }
        let x = (d.x / d.w.max(1.0)).clamp(0.0, 1.0);
        let y = (d.y / d.h.max(1.0)).clamp(0.0, 1.0);
        pick(s, kind, |hsva| f(hsva, x, y));
        tx.instant();
    }
}

fn picker_body(k: &Kit, kind: Kind, v: &VarsState, p: &Picker) -> El {
    let [h, sat, val, _] = p.hsva;
    let c = p.rgba();
    let hue = Rgba::from_hsva([h, 1.0, 1.0, 1.0]).color();
    let stops = |n: usize, f: &dyn Fn(f32) -> Color| -> Vec<(f32, Color)> {
        (0..n)
            .map(|i| {
                let t = i as f32 / (n - 1) as f32;
                (t, f(t))
            })
            .collect()
    };
    let right = std::f32::consts::FRAC_PI_2;
    let down = std::f32::consts::PI;
    // Saturation across (sRGB-correct stops), value down as a black veil.
    let square = rune::el()
        .key("sv")
        .size(PICK_W, PICK_H)
        .radius(6.0)
        .fill(hue)
        .linear_gradient(
            right,
            stops(16, &|t| Rgba::from_hsva([h, t, 1.0, 1.0]).color()),
        )
        .cursor(rune::Cursor::Crosshair)
        .label("Saturation and brightness")
        .child(
            rune::el()
                .key("veil")
                .absolute(0.0, 0.0)
                .size(PICK_W, PICK_H)
                .radius(6.0)
                .linear_gradient(
                    down,
                    stops(16, &|t| {
                        Color::BLACK.alpha(1.0 - rune::color::srgb_decode(1.0 - t))
                    }),
                )
                .a11y_hidden(),
        )
        .child(thumb(
            sat * PICK_W,
            (1.0 - val) * PICK_H,
            c.opaque().color(),
        ))
        .on_drag(track_drag(kind, |hsva, x, y| {
            hsva[1] = x;
            hsva[2] = 1.0 - y;
        }));
    let bar = |key: &'static str| {
        rune::el()
            .key(key)
            .size(PICK_W, BAR_H)
            .radius(BAR_H / 2.0)
            .cursor(rune::Cursor::Pointer)
    };
    let hue_bar = bar("hue")
        .label("Hue")
        .fill(hue)
        .linear_gradient(
            right,
            stops(13, &|t| Rgba::from_hsva([t * 360.0, 1.0, 1.0, 1.0]).color()),
        )
        .child(thumb(h / 360.0 * PICK_W, BAR_H / 2.0, hue))
        .on_drag(track_drag(kind, |hsva, x, _| hsva[0] = x * 359.9));
    let alpha_bar = bar("alpha")
        .label("Opacity")
        .fill(k.c.layer_03)
        .border(1.0, k.c.border_base)
        .linear_gradient(
            right,
            [(0.0, c.color().alpha(0.0)), (1.0, c.color().alpha(1.0))],
        )
        .child(thumb(c.a * PICK_W, BAR_H / 2.0, c.color()))
        .on_drag(track_drag(kind, |hsva, x, _| hsva[3] = x));
    let hex = row()
        .key("hex-row")
        .w_full()
        .gap(8.0)
        .center_y()
        .child(
            rune::el()
                .key("preview")
                .size(28.0, 28.0)
                .shrink()
                .radius(6.0)
                .fill(c.color())
                .border(1.0, k.c.border_strong)
                .a11y_hidden(),
        )
        .child(
            TextInput::new(&v.picker_hex)
                .label("Hex colour")
                .placeholder("rrggbb or rrggbbaa")
                .text_style(ui::mono(k))
                .width(None)
                .disabled(v.edit_busy)
                .on_edit(hex_access(kind))
                .on_change(move |s: &mut State, _| hex_typed(s, kind))
                .on_submit(move |s: &mut State, _| apply_picker(s, kind))
                .on_key(move |s: &mut State, e: &KeyEvent, _| {
                    let esc = e.key == Key::Escape;
                    if esc {
                        close_picker(s, kind);
                    }
                    esc
                })
                .build(k)
                .key("hex")
                .w(0.0)
                .grow(1.0),
        );
    let buttons = row()
        .key("buttons")
        .w_full()
        .gap(8.0)
        .justify_end()
        .child(
            Button::new("Cancel")
                .variant(ButtonVariant::Outline)
                .disabled(v.edit_busy)
                .on_press(move |s: &mut State| close_picker(s, kind))
                .build(k)
                .key("cancel"),
        )
        .child(
            Button::new(if v.edit_busy { "Applying…" } else { "Apply" })
                .variant(ButtonVariant::Contrast)
                .disabled(v.edit_busy)
                .on_press(move |s: &mut State| apply_picker(s, kind))
                .build(k)
                .key("apply"),
        );
    col()
        .key("picker")
        .w(PICK_W)
        .gap(12.0)
        .child(square)
        .child(hue_bar)
        .child(alpha_bar)
        .child(hex)
        .child(buttons)
}

/// The row's swatch: pressing it opens the colour picker beside it.
pub(super) fn swatch_button(
    s: &State,
    k: &Kit,
    kind: Kind,
    e: &Entry,
    c: Color,
    layers: &mut Vec<El>,
) -> El {
    let v = of_ref(s, kind);
    let id = e.id(kind);
    let picking = v.picker.as_ref().filter(|p| p.id == id);
    let open_id = id.clone();
    let trigger = swatch(k, c)
        .key("swatch")
        .role("button")
        .label(format!("Pick a colour for {}", e.name))
        .help("Pick a colour")
        .focusable(true)
        .cursor(rune::Cursor::Pointer)
        .on_press(move |s: &mut State| open_picker(s, kind, open_id.clone()));
    let mut pop = rune_kit::Popover::new(format!("{}-picker-{id}", texts(kind).key))
        .trigger(trigger)
        .label(format!("Colour for {}", e.name))
        .placement(rune_kit::Placement::BottomStart)
        .open(picking.is_some())
        .on_close(move |s: &mut State| close_picker(s, kind));
    if let Some(p) = picking {
        pop = pop.child(picker_body(k, kind, v, p));
    }
    pop.build(k, layers)
}
