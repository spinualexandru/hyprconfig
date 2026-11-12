//! The Variables and Env Vars pages: inline edits, the colour picker, adds and deletes.

use super::*;
use crate::pages::vars::{self, Kind};

#[test]
fn a_variable_edits_in_place() {
    let mut rt = on(Page::Variables, |s| {
        s.variables.list = Remote::ready(fixtures::variable_entries())
    });
    tap(&mut rt, "row-terminal/value");
    assert_eq!(rt.state.variables.editing.as_deref(), Some("terminal"));
    assert_eq!(rt.state.variables.edit_value.text, "kitty");
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.variables.editing, None);
    assert!(!asked(&rt, "save-var"));

    tap(&mut rt, "row-menu/value");
    rt.paste("-alt");
    run(&mut rt, 0.1);
    assert!(rt.state.variables.edit_value.text.contains("-alt"));
    press(&mut rt, Key::Enter);
    assert!(rt.state.variables.edit_busy);
    assert!(asked(&rt, "save-var"));
}

#[test]
fn the_swatch_opens_a_colour_picker_and_the_text_still_edits() {
    let mut rt = on(Page::Variables, |s| {
        s.variables.list = Remote::ready(fixtures::variable_entries())
    });
    tap(
        &mut rt,
        "row-accent/value-cell/variables-picker-accent-trigger/swatch",
    );
    let v = &rt.state.variables;
    assert_eq!(v.picker.as_ref().map(|p| p.id.as_str()), Some("accent"));
    assert_eq!(v.editing, None, "the swatch picks, it doesn't edit");
    assert_eq!(v.picker_hex.text, "e2e2e9");

    // Drag to the square's bottom-left corner: black.
    let (x, y) = rt.find("picker/sv").expect("the picker shows");
    rt.pointer_down(x, y);
    rt.pointer_move(x - 500.0, y + 500.0);
    rt.frame(FRAME);
    rt.pointer_up(x - 500.0, y + 500.0);
    run(&mut rt, 0.1);
    let p = rt.state.variables.picker.as_ref().unwrap();
    assert_eq!(p.value(), "rgb(000000)", "written as it was: rgb(rrggbb)");
    assert_eq!(rt.state.variables.picker_hex.text, "000000");
    assert!(!asked(&rt, "save-var"), "nothing is written before Apply");

    // Typing a hex moves the picker there.
    rt.state.variables.picker_hex = rune::TextEdit::new("ff000080");
    vars::hex_typed(&mut rt.state, Kind::Variables);
    let p = rt.state.variables.picker.as_ref().unwrap();
    assert_eq!(p.value(), "rgba(ff000080)");

    tap(&mut rt, "picker/buttons/apply");
    assert!(asked(&rt, "save-var"));

    // The label beside the swatch edits the text.
    let mut rt = on(Page::Variables, |s| {
        s.variables.list = Remote::ready(fixtures::variable_entries())
    });
    tap(&mut rt, "row-accent/value-cell/value");
    assert_eq!(rt.state.variables.editing.as_deref(), Some("accent"));
    assert_eq!(rt.state.variables.edit_value.text, "rgb(e2e2e9)");
    assert!(rt.state.variables.picker.is_none());
}

#[test]
fn adding_a_variable_needs_a_name() {
    let mut rt = on(Page::Variables, |s| {
        s.variables.list = Remote::ready(fixtures::variable_entries())
    });
    tap(&mut rt, "add");
    assert!(rt.state.variables.add_open);
    tap(&mut rt, "submit");
    assert_eq!(
        rt.state.variables.add_error.as_deref(),
        Some("Variable name is required")
    );
    assert!(!asked(&rt, "add-var"));
    // Typing in the name field clears the error.
    tap(&mut rt, "name");
    rt.paste("browser");
    run(&mut rt, 0.1);
    assert_eq!(rt.state.variables.add_error, None);
    tap(&mut rt, "submit");
    assert!(asked(&rt, "add-var"));
}

#[test]
fn escape_closes_the_add_dialog() {
    let mut rt = on(Page::EnvVars, |s| {
        s.envvars.list = Remote::ready(Vec::new())
    });
    tap(&mut rt, "add");
    assert!(rt.state.envvars.add_open);
    press(&mut rt, Key::Escape);
    assert!(!rt.state.envvars.add_open);
}

#[test]
fn an_env_var_delete_waits_for_the_reload_before_the_next() {
    let mut rt = on(Page::EnvVars, |s| {
        s.envvars.list = Remote::ready(fixtures::env_entries())
    });
    tap(&mut rt, "row-1/delete");
    tap(&mut rt, "row-2/delete");
    assert_eq!(count(&rt, "delete-var"), 1);
    // The edit of a row's value waits too.
    tap(&mut rt, "row-2/value");
    assert_eq!(rt.state.envvars.editing, None);
}

#[test]
fn an_env_var_edit_ends_when_another_entry_takes_its_place() {
    let mut rt = on(Page::EnvVars, |s| {
        s.envvars.list = Remote::ready(fixtures::env_entries())
    });
    tap(&mut rt, "row-1/value");
    assert_eq!(rt.state.envvars.editing.as_deref(), Some("1"));
    // A reload that leaves the edited entry where it was keeps the edit.
    rt.update(|s, _| {
        land(&mut s.envvars.list, Ok(fixtures::env_entries()));
        vars::end_stale_edit(&mut s.envvars, Kind::Env);
    });
    run(&mut rt, 0.2);
    assert_eq!(rt.state.envvars.editing.as_deref(), Some("1"));
    // Entry 0 was deleted elsewhere, so entry 2 now sits at position 1.
    rt.update(|s, _| {
        let mut rows = fixtures::env_entries();
        rows.remove(0);
        for (i, e) in rows.iter_mut().enumerate() {
            e.index = i;
        }
        land(&mut s.envvars.list, Ok(rows));
        vars::end_stale_edit(&mut s.envvars, Kind::Env);
    });
    run(&mut rt, 0.2);
    assert_eq!(rt.state.envvars.editing, None);
    assert_eq!(rt.state.envvars.editing_entry, None);
}
