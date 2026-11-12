//! The Keybinds page: the bind form, universal binds and command display.

use super::*;
use crate::backend::hyprland::Variable;
use crate::dispatchers;
use crate::pages::keybinds::{FormMode, Tab};

#[test]
fn the_keybind_form_validates_and_suggests_dispatchers() {
    let mut rt = on(Page::Keybinds, fixtures::keybind_data);
    tap(&mut rt, "add");
    assert_eq!(rt.state.keybinds.form, Some(FormMode::Add));
    tap(&mut rt, "submit");
    assert_eq!(
        rt.state.keybinds.form_error.as_deref(),
        Some("Key is required")
    );

    tap(&mut rt, "key/input");
    rt.paste("Q");
    run(&mut rt, 0.1);
    assert_eq!(rt.state.keybinds.key.text, "Q");
    assert_eq!(rt.state.keybinds.form_error, None);
    tap(&mut rt, "super");
    assert_eq!(rt.state.keybinds.mods, vec!["SUPER".to_owned()]);

    tap(&mut rt, "dispatcher/input");
    rt.paste("killa");
    run(&mut rt, 0.1);
    tap(&mut rt, "pick-killactive");
    assert_eq!(rt.state.keybinds.dispatcher.text, "killactive");
    tap(&mut rt, "submit");
    assert!(rt.state.keybinds.form_busy);
    assert!(asked(&rt, "save-keybind"));
}

#[test]
fn editing_a_keybind_starts_from_it() {
    let mut rt = on(Page::Keybinds, fixtures::keybind_data);
    tap(&mut rt, "bind-2/edit");
    assert_eq!(rt.state.keybinds.form, Some(FormMode::Edit(2)));
    assert_eq!(rt.state.keybinds.key.text, "E");
    assert_eq!(
        rt.state.keybinds.mods,
        vec!["SUPER".to_owned(), "SHIFT".to_owned()]
    );
    assert_eq!(rt.state.keybinds.params.text, "${menu} --show drun");
}

#[test]
fn universal_binds_add_without_edit() {
    let mut rt = on(Page::Keybinds, fixtures::keybind_data);
    rt.update(|s, _| s.keybinds.tab = Tab::Universal);
    run(&mut rt, 0.2);
    assert!(rt.find("bind-0/edit").is_none());
    assert!(rt.find("bind-0/delete").is_some());
    tap(&mut rt, "add");
    assert_eq!(rt.state.keybinds.form, Some(FormMode::AddUniversal));
}

#[test]
fn a_failed_universal_load_shows_its_error() {
    let mut rt = on(Page::Keybinds, fixtures::keybind_data);
    rt.update(|s, _| {
        s.keybinds.tab = Tab::Universal;
        s.keybinds.universal = Remote::failed("bindu: parse error");
    });
    run(&mut rt, 0.2);
    assert!(rt.find("error").is_some());
    assert!(rt.find("empty").is_none(), "not 'No universal binds found'");
    // The binds tab's own error stays on its tab.
    rt.update(|s, _| s.keybinds.tab = Tab::Binds);
    run(&mut rt, 0.2);
    assert!(rt.find("error").is_none());
}

#[test]
fn a_keybind_change_waits_for_the_reload_before_the_next() {
    let mut rt = on(Page::Keybinds, fixtures::keybind_data);
    tap(&mut rt, "bind-2/delete");
    assert_eq!(count(&rt, "delete-keybind"), 1);
    assert!(rt.state.keybinds.binds.settling());
    // A second press (on the bind that now sits at 2) and an edit are ignored.
    tap(&mut rt, "bind-2/delete");
    tap(&mut rt, "bind-1/edit");
    assert_eq!(count(&rt, "delete-keybind"), 1);
    assert_eq!(rt.state.keybinds.form, None);
    // As the delete and then its reload land:
    rt.update(|s, _| {
        s.keybinds.binds.after_write(true);
        let mut binds = fixtures::binds();
        binds.remove(2);
        land(&mut s.keybinds.binds, Ok(binds));
    });
    run(&mut rt, 0.2);
    assert!(!rt.state.keybinds.binds.settling());
    tap(&mut rt, "bind-2/delete");
    assert_eq!(count(&rt, "delete-keybind"), 2);
}

#[test]
fn editing_a_bind_keeps_modifiers_the_form_has_no_box_for() {
    let mut rt = on(Page::Keybinds, |s| {
        fixtures::keybind_data(s);
        s.keybinds.variables.push(Variable {
            name: "mainMod".into(),
            value: "SUPER".into(),
            source_file: None,
        });
        let b = &mut s.keybinds.binds.data.as_mut().unwrap()[1];
        b.modifiers = vec!["$mainMod".into(), "CONTROL".into(), "MOD5".into()];
    });
    tap(&mut rt, "bind-1/edit");
    let kb = &rt.state.keybinds;
    assert_eq!(kb.mods, ["SUPER", "CTRL", "MOD5"]);
    assert!(rt.find("other-0").is_some(), "MOD5 has its own checkbox");
    assert_eq!(
        dispatchers::modifiers_to_save(&kb.original_mods, &kb.mods),
        ["$mainMod", "CONTROL", "MOD5"],
        "untouched modifiers save as written"
    );
    tap(&mut rt, "shift");
    tap(&mut rt, "ctrl");
    let kb = &rt.state.keybinds;
    assert_eq!(
        dispatchers::modifiers_to_save(&kb.original_mods, &kb.mods),
        ["$mainMod", "MOD5", "SHIFT"]
    );
}

#[test]
fn bind_commands_split_out_only_known_variables() {
    let mut rt = on(Page::Keybinds, |s| {
        fixtures::keybind_data(s);
        let binds = s.keybinds.binds.data.as_mut().unwrap();
        binds[0].params = "$terminalx".into();
        binds[1].params = "$nope --flag".into();
    });
    run(&mut rt, 0.2);
    assert!(
        rt.find("bind-0/command/p0").is_some() && rt.find("bind-0/command/p1").is_some(),
        "$terminal, then x"
    );
    assert!(rt.find("bind-0/command/p2").is_none());
    assert!(rt.find("bind-1/command").is_some());
    assert!(
        rt.find("bind-1/command/p0").is_none(),
        "an unknown variable stays plain text"
    );
    assert!(
        rt.find("bind-2/command/p1").is_some(),
        "${{menu}}, then the rest"
    );
}
