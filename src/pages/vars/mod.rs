//! Variables (`$name = value`, or `local name = ...` in Lua) and environment variables
//! (`env = NAME,value`, or `hl.env(...)`): the same table, inline editing and add dialog.

mod edit;
mod picker;
mod view;

pub use edit::*;
pub use picker::*;
pub use view::view;

use rune::TextEdit;

use crate::app::State;
use crate::backend::hyprland as hypr;
use crate::icons;
use crate::jobs::Remote;

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

fn end_edit(v: &mut VarsState) {
    v.editing = None;
    v.editing_entry = None;
}
