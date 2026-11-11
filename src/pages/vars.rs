//! Variables (`$name = value`, or `local name = ...` in Lua) and environment variables
//! (`env = NAME,value`, or `hl.env(...)`): the same table, inline editing and add dialog.

use rune::{Color, El, Key, KeyEvent, TextEdit, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Dialog, IconButton, IconButtonVariant, Kit, TextInput};

use crate::app::{self, State};
use crate::backend::hyprland as hypr;
use crate::color::{self, Found, Rgba};
use crate::icons;
use crate::jobs::Remote;
use crate::ui::{self, Card};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Variables,
    Env,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub name: String,
    pub value: String,
    /// The env var's position among `env` lines (its identity); variables go by name.
    pub index: usize,
}

impl Entry {
    pub fn id(&self, kind: Kind) -> String {
        match kind {
            Kind::Variables => self.name.clone(),
            Kind::Env => self.index.to_string(),
        }
    }
}

#[derive(Default)]
pub struct VarsState {
    pub list: Remote<Vec<Entry>>,
    /// The row being edited in place, by id.
    pub editing: Option<String>,
    /// That row as it was when the edit began. An env var's id is its position, which a delete
    /// elsewhere shifts onto another entry, so the edit follows the entry rather than the id.
    pub editing_entry: Option<Entry>,
    pub edit_value: TextEdit,
    pub edit_busy: bool,
    pub add_open: bool,
    pub add_name: TextEdit,
    pub add_value: TextEdit,
    pub add_error: Option<String>,
    pub add_busy: bool,
    /// The colour picker open on a row's swatch.
    pub picker: Option<Picker>,
    /// The picker's hex field.
    pub picker_hex: TextEdit,
}

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

struct Texts {
    key: &'static str,
    title: &'static str,
    subtitle: &'static str,
    add: &'static str,
    card: &'static str,
    card_description: &'static str,
    icon: &'static rune::Icon,
    empty: &'static str,
    error: &'static str,
    dialog: &'static str,
    dialog_description: &'static str,
    name_placeholder: &'static str,
    name_hint: &'static str,
    value_placeholder: &'static str,
    name_required: &'static str,
    submit: &'static str,
}

static VARIABLE_TEXTS: Texts = Texts {
    key: "variables",
    title: "Variables",
    subtitle: "View and manage your Hyprland configuration variables",
    add: "Add Variable",
    card: "Variables",
    card_description: "Your Hyprland configuration variables",
    icon: &icons::DOLLAR_SIGN,
    empty: "No variables found in your Hyprland configuration.",
    error: "Error Loading Variables",
    dialog: "Add New Variable",
    dialog_description: "Create a new variable in your Hyprland configuration.",
    name_placeholder: "e.g., terminal, gaps, mainMod",
    name_hint: "Letters, numbers, and underscores only. No spaces.",
    value_placeholder: "e.g., kitty, 5, SUPER",
    name_required: "Variable name is required",
    submit: "Add Variable",
};

static ENV_TEXTS: Texts = Texts {
    key: "envvars",
    title: "Environment Variables",
    subtitle: "View and manage your Hyprland environment variables",
    add: "Add Env Var",
    card: "Environment Variables",
    card_description: "Your Hyprland environment variables",
    icon: &icons::TERMINAL,
    empty: "No environment variables found in your Hyprland configuration.",
    error: "Error Loading Environment Variables",
    dialog: "Add New Environment Variable",
    dialog_description: "Create a new environment variable in your Hyprland configuration.",
    name_placeholder: "e.g., XCURSOR_SIZE, QT_QPA_PLATFORM",
    name_hint: "Letters, numbers, and underscores only. Convention: UPPER_CASE.",
    value_placeholder: "e.g., 24, wayland, qt6ct",
    name_required: "Environment variable name is required",
    submit: "Add Environment Variable",
};

fn texts(kind: Kind) -> &'static Texts {
    match kind {
        Kind::Variables => &VARIABLE_TEXTS,
        Kind::Env => &ENV_TEXTS,
    }
}

pub fn of(s: &mut State, kind: Kind) -> &mut VarsState {
    match kind {
        Kind::Variables => &mut s.variables,
        Kind::Env => &mut s.envvars,
    }
}

fn of_ref(s: &State, kind: Kind) -> &VarsState {
    match kind {
        Kind::Variables => &s.variables,
        Kind::Env => &s.envvars,
    }
}

// Field accessors as function pointers, one per kind and field.
type Access = fn(&mut State) -> &mut TextEdit;
fn edit_access(kind: Kind) -> Access {
    match kind {
        Kind::Variables => |s| &mut s.variables.edit_value,
        Kind::Env => |s| &mut s.envvars.edit_value,
    }
}
fn hex_access(kind: Kind) -> Access {
    match kind {
        Kind::Variables => |s| &mut s.variables.picker_hex,
        Kind::Env => |s| &mut s.envvars.picker_hex,
    }
}
fn name_access(kind: Kind) -> Access {
    match kind {
        Kind::Variables => |s| &mut s.variables.add_name,
        Kind::Env => |s| &mut s.envvars.add_name,
    }
}
fn value_access(kind: Kind) -> Access {
    match kind {
        Kind::Variables => |s| &mut s.variables.add_value,
        Kind::Env => |s| &mut s.envvars.add_value,
    }
}

fn fetch(kind: Kind) -> Result<Vec<Entry>, String> {
    match kind {
        Kind::Variables => hypr::get_variables().map(|v| {
            v.into_iter()
                .enumerate()
                .map(|(i, v)| Entry {
                    name: v.name,
                    value: v.value,
                    index: i,
                })
                .collect()
        }),
        Kind::Env => hypr::get_env_vars().map(|v| {
            v.into_iter()
                .map(|e| Entry {
                    name: e.name,
                    value: e.value,
                    index: e.index,
                })
                .collect()
        }),
    }
}

/// Fails unless the config still has `expected` where it was loaded: env vars go by their
/// position among `env` lines, so one that moved must not be overwritten or deleted.
pub fn check_entry(current: Result<Vec<Entry>, String>, expected: &Entry) -> Result<(), String> {
    if current?.iter().any(|e| e == expected) {
        Ok(())
    } else {
        Err(format!(
            "{} changed in your config since it was loaded. Refresh and try again.",
            expected.name
        ))
    }
}

/// The write for an entry that's shown, once the config still has it.
fn checked(
    kind: Kind,
    expected: &Entry,
    write: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    if kind == Kind::Env {
        check_entry(fetch(kind), expected)?;
    }
    write()
}

/// A write to the list landed: settle and reload, or keep the rows and say why.
fn written(s: &mut State, kind: Kind, r: &Result<(), String>) {
    of(s, kind).list.after_write(r.is_ok());
    if r.is_ok() {
        load(s, kind);
    }
}

pub fn load(s: &mut State, kind: Kind) {
    let attempt = of(s, kind).list.begin();
    s.jobs.run(
        texts(kind).key,
        move || fetch(kind),
        move |s, r, _| {
            let v = of(s, kind);
            if v.list.land(attempt, r) {
                end_stale_edit(v, kind);
            }
        },
    );
}

/// After a reload: an edit whose row went away ends, and for an env var, so does one whose
/// position now holds another entry.
pub fn end_stale_edit(v: &mut VarsState, kind: Kind) {
    let rows = || v.list.data.iter().flatten();
    let gone = match kind {
        Kind::Variables => v
            .editing
            .as_ref()
            .is_some_and(|id| !rows().any(|e| &e.id(kind) == id)),
        Kind::Env => v
            .editing_entry
            .as_ref()
            .is_some_and(|orig| !rows().any(|e| e == orig)),
    };
    if gone {
        end_edit(v);
    }
}

pub fn start_edit(s: &mut State, kind: Kind, id: String, value: &str) {
    let v = of(s, kind);
    if v.list.settling() {
        return;
    }
    v.editing_entry = v
        .list
        .data
        .iter()
        .flatten()
        .find(|e| e.id(kind) == id)
        .cloned();
    v.editing = Some(id);
    v.edit_value = TextEdit::new(value);
    v.edit_value.focused = true;
    v.edit_busy = false;
}

pub fn cancel_edit(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    end_edit(v);
    v.edit_busy = false;
}

fn end_edit(v: &mut VarsState) {
    v.editing = None;
    v.editing_entry = None;
}

pub fn save_edit(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    if v.edit_busy {
        return;
    }
    // The entry the edit began on: an env var's write checks the file still has it there.
    let Some(entry) = v.editing_entry.clone() else {
        return;
    };
    let value = v.edit_value.text.clone();
    write_value(s, kind, entry, value);
}

/// Writes `value` for `entry`; once it lands, the edit or picker on it closes.
fn write_value(s: &mut State, kind: Kind, entry: Entry, value: String) {
    let v = of(s, kind);
    v.edit_busy = true;
    v.list.hold();
    s.jobs.write(
        "save-var",
        move || {
            checked(kind, &entry, || match kind {
                Kind::Variables => hypr::set_variable(entry.name.clone(), value),
                Kind::Env => hypr::edit_env_var(entry.index, entry.name.clone(), value),
            })
        },
        move |s, r, tx| {
            let v = of(s, kind);
            v.edit_busy = false;
            if r.is_ok() {
                end_edit(v);
                v.picker = None;
            }
            written(s, kind, &r);
            if let Err(e) = r {
                app::fail(s, tx, "Failed to save", e);
            }
        },
    );
}

pub fn delete(s: &mut State, kind: Kind, entry: Entry) {
    let v = of(s, kind);
    // One change at a time: env var positions shift until the list reloads.
    if v.list.settling() {
        return;
    }
    v.list.hold();
    s.jobs.write(
        "delete-var",
        move || {
            checked(kind, &entry, || match kind {
                Kind::Variables => hypr::delete_variable(entry.name.clone()),
                Kind::Env => hypr::delete_env_var(entry.index),
            })
        },
        move |s, r, tx| {
            written(s, kind, &r);
            if let Err(e) = r {
                app::fail(s, tx, "Failed to delete", e);
            }
        },
    );
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

pub fn open_add(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    v.add_open = true;
    v.add_name = TextEdit::new("");
    v.add_name.focused = true;
    v.add_value = TextEdit::new("");
    v.add_error = None;
    v.add_busy = false;
}

pub fn close_add(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    if !v.add_busy {
        v.add_open = false;
    }
}

pub fn submit_add(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    if v.add_busy {
        return;
    }
    let name = v.add_name.text.trim().to_owned();
    if name.is_empty() {
        v.add_error = Some(texts(kind).name_required.into());
        return;
    }
    let value = v.add_value.text.clone();
    v.add_busy = true;
    v.add_error = None;
    s.jobs.write(
        "add-var",
        move || match kind {
            Kind::Variables => hypr::add_variable(name, value),
            Kind::Env => hypr::add_env_var(name, value),
        },
        move |s, r, _| {
            let v = of(s, kind);
            v.add_busy = false;
            match r {
                Ok(()) => {
                    v.add_open = false;
                    load(s, kind);
                }
                Err(e) => v.add_error = Some(e),
            }
        },
    );
}

fn on_escape(kind: Kind) -> impl Fn(&mut State, &KeyEvent, &mut Tx) -> bool {
    move |s, e, _| {
        if e.key == Key::Escape {
            cancel_edit(s, kind);
            true
        } else {
            false
        }
    }
}

/// A small rounded square filled with `c`, outlined so it shows on any background.
fn swatch(k: &Kit, c: Color) -> El {
    row()
        .w(14.0)
        .h(14.0)
        .shrink()
        .radius(4.0)
        .fill(c)
        .border(1.0, k.c.border_strong)
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
fn swatch_button(s: &State, k: &Kit, kind: Kind, e: &Entry, c: Color, layers: &mut Vec<El>) -> El {
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

const WIDTHS: [Option<f32>; 3] = [Some(240.0), None, Some(56.0)];

fn entry_row(s: &State, k: &Kit, kind: Kind, e: &Entry, layers: &mut Vec<El>) -> El {
    let v = of_ref(s, kind);
    let settling = v.list.settling();
    let id = e.id(kind);
    let editing = v.editing.as_deref() == Some(id.as_str());
    let shown_name = match kind {
        Kind::Variables => format!("${}", e.name),
        Kind::Env => e.name.clone(),
    };
    let name = ui::code(k, shown_name);
    let value = if editing {
        row()
            .key("editor")
            .w_full()
            .gap(4.0)
            .center_y()
            .children(
                color::value_color(&v.edit_value.text)
                    .map(|c| swatch(k, c).key("swatch").mr(4.0).a11y_hidden()),
            )
            .child(
                TextInput::new(&v.edit_value)
                    .label(format!("Value of {}", e.name))
                    .text_style(ui::mono(k))
                    .width(None)
                    .disabled(v.edit_busy)
                    .on_edit(edit_access(kind))
                    .on_submit(move |s: &mut State, _| save_edit(s, kind))
                    .on_key(on_escape(kind))
                    .build(k)
                    .key("input")
                    .w(0.0)
                    .grow(1.0),
            )
            .child(
                ui::icon_button(&icons::CHECK, "Save")
                    .disabled(v.edit_busy)
                    .on_press(move |s: &mut State| save_edit(s, kind))
                    .build(k)
                    .key("save"),
            )
            .child(
                ui::icon_button(&icons::X, "Cancel")
                    .disabled(v.edit_busy)
                    .on_press(move |s: &mut State| cancel_edit(s, kind))
                    .build(k)
                    .key("cancel"),
            )
    } else {
        let (start_id, start_value) = (id.clone(), e.value.clone());
        // While its colour is being picked, the row previews the value it would write.
        let picking = v.picker.as_ref().filter(|p| p.id == id);
        let shown = picking.map_or_else(|| e.value.clone(), Picker::value);
        let code = ui::code(
            k,
            if shown.is_empty() {
                " ".to_owned()
            } else {
                shown
            },
        )
        .key("value")
        .role("button")
        .label(format!("Edit {}", e.name))
        .help("Click to edit")
        .focusable(true)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .on_press(move |s: &mut State| start_edit(s, kind, start_id.clone(), &start_value));
        let c = match picking {
            Some(p) => Some(p.rgba().color()),
            None => color::value_color(&e.value),
        };
        match c {
            Some(c) => row()
                .key("value-cell")
                .gap(8.0)
                .center_y()
                .child(swatch_button(s, k, kind, e, c, layers))
                .child(code),
            None => code,
        }
    };
    let actions = if editing {
        rune::el()
    } else {
        let entry = e.clone();
        IconButton::new(&icons::TRASH_2)
            .variant(IconButtonVariant::Ghost)
            .label(format!("Delete {}", e.name))
            .help("Delete")
            .disabled(settling)
            .on_press(move |s: &mut State| delete(s, kind, entry.clone()))
            .build(k)
            .key("delete")
    };
    ui::table_row(k, format!("row-{id}"), &WIDTHS, vec![name, value, actions])
}

fn add_dialog(s: &State, k: &Kit, kind: Kind, layers: &mut Vec<El>) {
    let v = of_ref(s, kind);
    if !v.add_open {
        return;
    }
    let c = texts(kind);
    let label = |t: &str| {
        text(t.to_owned())
            .font(k.ty.body_medium)
            .color(k.c.text_strong)
    };
    let mut fields = col()
        .key("fields")
        .w_full()
        .gap(16.0)
        .child(
            col()
                .key("name-field")
                .w_full()
                .gap(6.0)
                .child(label("Variable Name"))
                .child(
                    TextInput::new(&v.add_name)
                        .label("Variable Name")
                        .placeholder(c.name_placeholder)
                        .width(None)
                        .disabled(v.add_busy)
                        .invalid(v.add_error.is_some())
                        .on_edit(name_access(kind))
                        .on_change(move |s: &mut State, _| of(s, kind).add_error = None)
                        .on_submit(move |s: &mut State, _| submit_add(s, kind))
                        .build(k)
                        .key("name"),
                )
                .child(text(c.name_hint).font(k.ty.small).color(k.c.text_muted)),
        )
        .child(
            col()
                .key("value-field")
                .w_full()
                .gap(6.0)
                .child(label("Value"))
                .child(
                    TextInput::new(&v.add_value)
                        .label("Value")
                        .placeholder(c.value_placeholder)
                        .width(None)
                        .disabled(v.add_busy)
                        .on_edit(value_access(kind))
                        .on_submit(move |s: &mut State, _| submit_add(s, kind))
                        .build(k)
                        .key("value"),
                ),
        );
    if let Some(e) = &v.add_error {
        fields = fields.child(
            text(e.clone())
                .key("error")
                .font(k.ty.body)
                .color(k.c.danger.icon)
                .wrap()
                .w_full(),
        );
    }
    let buttons = vec![
        Button::new("Cancel")
            .variant(ButtonVariant::Outline)
            .disabled(v.add_busy)
            .on_press(move |s: &mut State| close_add(s, kind))
            .build(k)
            .key("cancel"),
        Button::new(if v.add_busy { "Adding…" } else { c.submit })
            .variant(ButtonVariant::Contrast)
            .disabled(v.add_busy)
            .on_press(move |s: &mut State| submit_add(s, kind))
            .build(k)
            .key("submit"),
    ];
    Dialog::new(format!("add-{}", c.key), c.dialog)
        .description(c.dialog_description)
        .open(true)
        .fit()
        .on_close(move |s: &mut State| close_add(s, kind))
        .child(ui::dialog_body(fields, buttons))
        .build(k, layers);
}

pub fn view(s: &State, k: &Kit, layers: &mut Vec<El>, kind: Kind) -> El {
    let v = of_ref(s, kind);
    let c = texts(kind);
    let add = Button::new(c.add)
        .icon(&icons::PLUS)
        .variant(ButtonVariant::Contrast)
        .on_press(move |s: &mut State| open_add(s, kind))
        .build(k)
        .key("add");
    let refresh = ui::refresh_button(v.list.loading, move |s, _| load(s, kind))
        .build(k)
        .key("refresh");
    let mut body = Vec::new();
    // A failed reload shows above the rows from before.
    if let Some(e) = &v.list.error {
        body.push(ui::error_card(k, "error", c.error, e));
    }
    match v.list.data.as_deref() {
        None if v.list.first_load() => body.push(
            Card::new()
                .icon(c.icon)
                .title(c.card)
                .description(c.card_description)
                .child(ui::skeleton_rows(k, 6, &[Some(128.0), None]))
                .build(k)
                .key("loading"),
        ),
        None => {}
        Some([]) => body.push(ui::note_card(k, "empty", &[c.empty])),
        Some(list) => body.push(
            Card::new()
                .icon(c.icon)
                .title(format!("{} ({})", c.card, list.len()))
                .description(c.card_description)
                .child(ui::table(
                    k,
                    "table",
                    &[("Name", WIDTHS[0]), ("Value", WIDTHS[1]), ("", WIDTHS[2])],
                    list.iter()
                        .map(|e| entry_row(s, k, kind, e, layers))
                        .collect(),
                ))
                .build(k)
                .key("list"),
        ),
    }
    add_dialog(s, k, kind, layers);
    ui::page(k, c.key, c.title, c.subtitle, vec![add, refresh], body)
}
