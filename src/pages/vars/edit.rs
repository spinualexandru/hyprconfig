//! Writes from the variables pages: inline edits, deletes and the add dialog.

use rune::TextEdit;

use super::*;
use crate::app::{self, State};
use crate::backend::hyprland as hypr;

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
pub(super) fn write_value(s: &mut State, kind: Kind, entry: Entry, value: String) {
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
