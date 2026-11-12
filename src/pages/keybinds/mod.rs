//! The keybinds page: binds and universal (`bindu`) binds, with an add/edit form.

mod edit;
mod form;
mod view;

pub use edit::*;
pub use view::view;

use rune::TextEdit;

use crate::app::State;
use crate::backend::hyprland::{self as hypr, Keybind, Variable};
use crate::jobs::{self, Remote};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Binds,
    /// `bindu`: binds that work in every submap.
    Universal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormMode {
    Add,
    /// Editing the bind at this index.
    Edit(usize),
    AddUniversal,
}

#[derive(Default)]
pub struct KeybindsState {
    pub binds: Remote<Vec<Keybind>>,
    pub universal: Remote<Vec<Keybind>>,
    /// The config's variables, to show what `$terminal` stands for.
    pub variables: Vec<Variable>,
    pub tab: Tab,
    pub form: Option<FormMode>,
    /// The bind being edited, as loaded: the save checks the config still has it there.
    pub original: Option<Keybind>,
    /// Its modifiers as written, each with its `dispatchers::modifier_id`.
    pub original_mods: Vec<(String, String)>,
    /// Ticked modifiers, by `dispatchers::modifier_id`.
    pub mods: Vec<String>,
    pub key: TextEdit,
    pub dispatcher: TextEdit,
    pub params: TextEdit,
    pub form_error: Option<String>,
    pub form_busy: bool,
}

pub fn load(s: &mut State) {
    load_binds(s);
    load_universal(s);
    s.jobs.run("variables", hypr::get_variables, |s, r, _| {
        // Optional: the table just doesn't explain variables without them.
        if let Ok(v) = r {
            s.keybinds.variables = v;
        }
    });
}

fn remote(s: &mut State, tab: Tab) -> &mut Remote<Vec<Keybind>> {
    match tab {
        Tab::Binds => &mut s.keybinds.binds,
        Tab::Universal => &mut s.keybinds.universal,
    }
}

fn reload(s: &mut State, tab: Tab) {
    match tab {
        Tab::Binds => load_binds(s),
        Tab::Universal => load_universal(s),
    }
}

/// Fails unless the bind at `index` in `current` is still `expected`: binds go by position, so
/// a bind that moved (another edit, the file changed) must not be overwritten or deleted.
pub fn check_bind(
    current: Result<Vec<Keybind>, String>,
    index: usize,
    expected: &Keybind,
) -> Result<(), String> {
    match current?.get(index) {
        Some(b) if b == expected => Ok(()),
        _ => Err("The keybind changed in your config since it was loaded. \
                  Refresh and try again."
            .into()),
    }
}

fn load_binds(s: &mut State) {
    jobs::load(s, "keybinds", |s| &mut s.keybinds.binds, hypr::get_keybinds);
}

fn load_universal(s: &mut State) {
    jobs::load(
        s,
        "bindu",
        |s| &mut s.keybinds.universal,
        hypr::get_all_bindu,
    );
}
