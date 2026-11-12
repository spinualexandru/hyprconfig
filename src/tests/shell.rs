//! The shell: sidebar, theme, toasts, scenes and cross-page error display.

use super::*;

#[test]
fn the_sidebar_opens_a_page_and_asks_for_its_data() {
    let mut rt = runtime(State::new());
    assert_eq!(rt.state.page, Page::General);
    tap(&mut rt, "keybinds");
    assert_eq!(rt.state.page, Page::Keybinds);
    for job in ["keybinds", "bindu", "variables"] {
        assert!(asked(&rt, job), "{job} wasn't requested");
    }
    // A first load shows placeholders, not an empty state.
    assert!(rt.find("loading").is_some());
    assert!(rt.find("empty").is_none());
}

#[test]
fn reopening_a_page_keeps_its_data_while_it_reloads() {
    let mut rt = on(Page::Variables, |s| {
        s.variables.list = Remote::ready(fixtures::variable_entries())
    });
    tap(&mut rt, "general");
    tap(&mut rt, "variables");
    assert!(rt.state.variables.list.loading);
    assert!(
        rt.find("row-terminal").is_some(),
        "old rows show while reloading"
    );
}

#[test]
fn the_theme_row_flips_the_appearance() {
    let mut rt = runtime(State::new());
    tap(&mut rt, "theme");
    assert_eq!(rt.state.dark, Some(true));
    tap(&mut rt, "theme");
    assert_eq!(rt.state.dark, Some(false));
    // A mode change reported by Noctalia's hook outranks that choice.
    rt.update(|s, _| app::set_dark(s, None));
    assert_eq!(rt.state.dark, None);
}

#[test]
fn toasts_close_on_their_own() {
    let mut rt = runtime(State::new());
    rt.update(|s, tx| s.toasts.success(tx, "Saved"));
    run(&mut rt, 0.5);
    assert!(rt.find("toast-1").is_some());
    run(&mut rt, 4.5);
    assert!(rt.state.toasts.items.is_empty());
    assert!(rt.find("toast-1").is_none());
}

#[test]
fn the_sidebar_toggle_shortcut_collapses_it() {
    let mut rt = runtime(State::new());
    assert!(rt.state.sidebar.is_open());
    rt.key(KeyEvent {
        key: Key::Command('b'),
        mods: Mods {
            shift: true,
            cmd: true,
            ctrl: true,
            ..Mods::default()
        },
    });
    run(&mut rt, 0.5);
    assert!(!rt.state.sidebar.is_open());
}

#[test]
fn the_collapsed_sidebar_keeps_its_icons() {
    let mut rt = runtime(State::new());
    tap(&mut rt, "collapse");
    assert!(!rt.state.sidebar.is_open());
    // The pages stay reachable as icons, named by their tooltips.
    assert!(rt.find("brand").is_some());
    assert!(rt.find("theme").is_some());
    tap(&mut rt, Page::Audio.id());
    assert_eq!(rt.state.page, Page::Audio);
    tap(&mut rt, "expand");
    assert!(rt.state.sidebar.is_open());
    assert!(rt.find("collapse").is_some());
}

#[test]
fn every_scene_builds() {
    for (name, scene) in fixtures::SCENES {
        let mut s = State::new();
        scene(&mut s);
        let mut rt = runtime(s);
        run(&mut rt, 0.2);
        assert!(rt.find("brand").is_some(), "{name}");
    }
}

#[test]
fn a_failed_reload_shows_its_error_over_the_old_rows() {
    let mut rt = on(Page::Variables, |s| {
        s.variables.list = Remote::ready(fixtures::variable_entries())
    });
    rt.update(|s, _| land(&mut s.variables.list, Err("config unreadable".into())));
    run(&mut rt, 0.2);
    assert!(rt.find("error").is_some());
    assert!(rt.find("row-terminal").is_some());

    let mut rt = on(Page::Displays, fixtures::land_monitors);
    rt.update(|s, _| land(&mut s.displays.monitors, Err("hyprctl failed".into())));
    run(&mut rt, 0.2);
    assert!(rt.find("error").is_some());
    assert!(rt.find("monitor-0").is_some());

    let mut rt = on(Page::About, |s| {
        s.about.info = Remote::ready(fixtures::system_info())
    });
    rt.update(|s, _| land(&mut s.about.info, Err("lspci missing".into())));
    run(&mut rt, 0.2);
    assert!(rt.find("error").is_some());
}
