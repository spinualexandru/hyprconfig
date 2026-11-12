//! Writes from the keybinds page: the add/edit form's state changes, saving and deleting.

use rune::TextEdit;

use super::*;
use crate::app::{self, State};
use crate::backend::hyprland::{self as hypr, Keybind};
use crate::dispatchers;

pub fn open_form(s: &mut State, mode: FormMode) {
    let kb = &mut s.keybinds;
    let existing = match mode {
        FormMode::Edit(i) => {
            // Positions shift while a change is written; wait for the list to reload.
            if kb.binds.settling() {
                return;
            }
            let Some(b) = kb.binds.data.as_ref().and_then(|b| b.get(i)).cloned() else {
                return;
            };
            Some(b)
        }
        _ => None,
    };
    let b = existing.clone().unwrap_or(Keybind {
        modifiers: Vec::new(),
        key: String::new(),
        dispatcher: String::new(),
        params: String::new(),
    });
    let vars: Vec<(&str, &str)> = kb
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.value.as_str()))
        .collect();
    kb.original_mods = b
        .modifiers
        .iter()
        .map(|m| (m.clone(), dispatchers::modifier_id(m, &vars)))
        .collect();
    kb.mods = Vec::new();
    for (_, id) in &kb.original_mods {
        if !kb.mods.contains(id) {
            kb.mods.push(id.clone());
        }
    }
    kb.original = existing;
    kb.form = Some(mode);
    kb.key = TextEdit::new(b.key);
    kb.key.focused = true;
    kb.dispatcher = TextEdit::new(b.dispatcher);
    kb.params = TextEdit::new(b.params);
    kb.form_error = None;
    kb.form_busy = false;
}

pub fn close_form(s: &mut State) {
    if !s.keybinds.form_busy {
        s.keybinds.form = None;
    }
}

pub fn toggle_mod(s: &mut State, m: &str) {
    let mods = &mut s.keybinds.mods;
    match mods.iter().position(|x| x == m) {
        Some(i) => {
            mods.remove(i);
        }
        None => mods.push(m.to_owned()),
    }
}

pub fn submit(s: &mut State) {
    let kb = &mut s.keybinds;
    let Some(mode) = kb.form else { return };
    if kb.form_busy {
        return;
    }
    let key = kb.key.text.trim().to_owned();
    let dispatcher = kb.dispatcher.text.trim().to_owned();
    let params = kb.params.text.trim().to_owned();
    if key.is_empty() {
        kb.form_error = Some("Key is required".into());
        return;
    }
    if dispatcher.is_empty() {
        kb.form_error = Some("Dispatcher is required".into());
        return;
    }
    // Modifiers the user left alone keep their spelling, order and any the form has no
    // checkbox for.
    let mods = dispatchers::modifiers_to_save(&kb.original_mods, &kb.mods);
    let original = kb.original.clone();
    if matches!(mode, FormMode::Edit(_)) && original.is_none() {
        return;
    }
    kb.form_busy = true;
    kb.form_error = None;
    let tab = if mode == FormMode::AddUniversal {
        Tab::Universal
    } else {
        Tab::Binds
    };
    remote(s, tab).hold();
    s.jobs.write(
        "save-keybind",
        move || match (mode, original) {
            (FormMode::Edit(i), Some(expected)) => {
                check_bind(hypr::get_keybinds(), i, &expected)?;
                hypr::edit_keybind(i, mods, key, dispatcher, params)
            }
            (FormMode::AddUniversal, _) => hypr::add_bindu(mods, key, dispatcher, params),
            _ => hypr::add_keybind(mods, key, dispatcher, params),
        },
        move |s, r, _| {
            s.keybinds.form_busy = false;
            remote(s, tab).after_write(r.is_ok());
            match r {
                Ok(()) => {
                    s.keybinds.form = None;
                    reload(s, tab);
                }
                Err(e) => s.keybinds.form_error = Some(e),
            }
        },
    );
}

pub fn delete(s: &mut State, tab: Tab, index: usize) {
    let r = remote(s, tab);
    // One change at a time: positions shift until the list reloads.
    if r.settling() {
        return;
    }
    let Some(expected) = r.data.as_ref().and_then(|b| b.get(index)).cloned() else {
        return;
    };
    r.hold();
    s.jobs.write(
        "delete-keybind",
        move || match tab {
            Tab::Binds => {
                check_bind(hypr::get_keybinds(), index, &expected)?;
                hypr::delete_keybind(index)
            }
            Tab::Universal => {
                check_bind(hypr::get_all_bindu(), index, &expected)?;
                hypr::delete_bindu(index)
            }
        },
        move |s, r, tx| {
            remote(s, tab).after_write(r.is_ok());
            match r {
                Ok(()) => reload(s, tab),
                Err(e) => app::fail(s, tx, "Failed to delete the keybind", e),
            }
        },
    );
}
