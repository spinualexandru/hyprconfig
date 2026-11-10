//! Variables (`$name = value`, or `local name = ...` in Lua) and environment variables
//! (`env = NAME,value`, or `hl.env(...)`): the same table, inline editing and add dialog.

use rune::{El, Key, KeyEvent, TextEdit, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Dialog, IconButton, IconButtonVariant, Kit, TextInput};

use crate::app::{self, State};
use crate::backend::hyprland as hypr;
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
    pub edit_value: TextEdit,
    pub edit_busy: bool,
    pub add_open: bool,
    pub add_name: TextEdit,
    pub add_value: TextEdit,
    pub add_error: Option<String>,
    pub add_busy: bool,
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
    let v = of(s, kind);
    if r.is_ok() {
        v.list.written();
        load(s, kind);
    } else {
        v.list.release();
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
                // An edit whose row went away ends.
                let gone = v
                    .editing
                    .as_ref()
                    .is_some_and(|id| !v.list.data.iter().flatten().any(|e| &e.id(kind) == id));
                if gone {
                    v.editing = None;
                }
            }
        },
    );
}

pub fn start_edit(s: &mut State, kind: Kind, id: String, value: &str) {
    let v = of(s, kind);
    if v.list.settling() {
        return;
    }
    v.editing = Some(id);
    v.edit_value = TextEdit::new(value);
    v.edit_value.focused = true;
    v.edit_busy = false;
}

pub fn cancel_edit(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    v.editing = None;
    v.edit_busy = false;
}

pub fn save_edit(s: &mut State, kind: Kind) {
    let v = of(s, kind);
    let Some(id) = v.editing.clone() else { return };
    if v.edit_busy {
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
    let value = v.edit_value.text.clone();
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
            of(s, kind).edit_busy = false;
            if r.is_ok() {
                of(s, kind).editing = None;
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

const WIDTHS: [Option<f32>; 3] = [Some(240.0), None, Some(56.0)];

fn entry_row(s: &State, k: &Kit, kind: Kind, e: &Entry) -> El {
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
        ui::code(
            k,
            if e.value.is_empty() {
                " ".to_owned()
            } else {
                e.value.clone()
            },
        )
        .key("value")
        .role("button")
        .label(format!("Edit {}", e.name))
        .help("Click to edit")
        .focusable(true)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .on_press(move |s: &mut State| start_edit(s, kind, start_id.clone(), &start_value))
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
                    list.iter().map(|e| entry_row(s, k, kind, e)).collect(),
                ))
                .build(k)
                .key("list"),
        ),
    }
    add_dialog(s, k, kind, layers);
    ui::page(k, c.key, c.title, c.subtitle, vec![add, refresh], body)
}
