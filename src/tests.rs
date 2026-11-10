//! Behaviour on Rune's headless runtime, with a virtual clock. Jobs don't reach the system
//! here: `jobs.skipped` records what each interaction asked for.

use rune::{Env, Key, KeyEvent, Mods, Runtime};

use crate::app::{self, State};
use crate::backend::hyprland::Variable;
use crate::backend::palette::Generator;
use crate::jobs::Remote;
use crate::pages::Page;
use crate::pages::displays::{self, Phase};
use crate::pages::keybinds::{FormMode, Tab};
use crate::{dispatchers, fixtures};

const FRAME: f64 = 1.0 / 60.0;

fn runtime(state: State) -> Runtime<State> {
    let mut rt = Runtime::new(state, app::view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.poll = Some(app::poll);
    rt.on_key = Some(app::on_key);
    rt.frame(0.0);
    rt
}

fn on(page: Page, fill: fn(&mut State)) -> Runtime<State> {
    let mut s = State::new();
    s.page = page;
    fill(&mut s);
    runtime(s)
}

fn run(rt: &mut Runtime<State>, secs: f64) {
    for _ in 0..(secs / FRAME).round() as usize {
        rt.frame(FRAME);
    }
}

fn tap(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("{key} is mounted"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    run(rt, 0.5);
}

fn press(rt: &mut Runtime<State>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    run(rt, 0.3);
}

fn asked(rt: &Runtime<State>, job: &str) -> bool {
    rt.state.jobs.skipped.contains(&job)
}

#[test]
fn the_sidebar_opens_a_page_and_asks_for_its_data() {
    let mut rt = runtime(State::new());
    assert_eq!(rt.state.page, Page::General);
    tap(&mut rt, "keybinds");
    assert_eq!(rt.state.page, Page::Keybinds);
    for job in ["config-info", "keybinds", "bindu", "variables"] {
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
    rt.update(|s, _| app::unpin_dark(s));
    assert_eq!(rt.state.dark, None);
}

#[test]
fn the_general_page_switches_the_colour_generator() {
    let mut rt = runtime(State::new());
    assert_eq!(rt.state.general.generator, Generator::Chromasync);
    rt.state.jobs.skipped.clear();
    tap(&mut rt, "generator/segment1");
    assert_eq!(rt.state.general.generator, Generator::Noctalia);
    assert!(
        asked(&rt, "colors"),
        "picking a generator recolours the app"
    );
}

#[test]
fn the_noctalia_integration_switch_shows_while_noctalia_runs() {
    let mut rt = on(Page::General, |s| {
        s.general.integration = Remote::ready(Some(false))
    });
    tap(&mut rt, "noctalia/switch");
    assert!(asked(&rt, "noctalia-integration-set"));
    let rt = on(Page::General, |s| {
        s.general.integration = Remote::ready(None)
    });
    assert!(rt.find("noctalia").is_none(), "no switch without Noctalia");
}

#[test]
fn noctalias_mode_stands_in_for_the_desktops() {
    // The headless desktop is light; Noctalia says dark, so the sidebar offers light.
    let mut rt = on(Page::General, |s| s.noctalia_dark = Some(true));
    tap(&mut rt, "theme");
    assert_eq!(rt.state.dark, Some(false));
}

#[test]
fn the_general_page_explains_a_fallback() {
    let (_, fill) = fixtures::SCENES
        .iter()
        .find(|(n, _)| *n == "general-fallback")
        .expect("the scene exists");
    let rt = on(Page::General, *fill);
    assert!(rt.find("skipped-noctalia").is_some());
    assert!(rt.find("skipped-chromasync").is_some());
}

#[test]
fn display_changes_show_the_bar_and_cancel_restores_them() {
    let mut rt = on(Page::Displays, fixtures::land_monitors);
    assert!(rt.find("changes-bar").is_none());
    rt.update(|s, _| displays::choose_resolution(s, 0, "1920x1080"));
    run(&mut rt, 0.1);
    // 60 Hz exists at 1080p, so the rate stays.
    assert_eq!(rt.state.displays.chosen[0].rate, "60.00");
    assert!(rt.find("changes-bar").is_some());
    tap(&mut rt, "cancel");
    assert!(!displays::dirty(&rt.state.displays));
    assert!(rt.find("changes-bar").is_none());
}

#[test]
fn a_resolution_without_the_current_rate_takes_its_first_rate() {
    let mut rt = on(Page::Displays, fixtures::land_monitors);
    displays::choose_resolution(&mut rt.state, 1, "1920x1080");
    assert_eq!(rt.state.displays.chosen[1].rate, "120.00");
}

#[test]
fn applying_waits_for_hyprctl() {
    let mut rt = on(Page::Displays, fixtures::land_monitors);
    rt.update(|s, _| displays::choose_resolution(s, 0, "1920x1080"));
    run(&mut rt, 0.1);
    tap(&mut rt, "apply");
    assert_eq!(rt.state.displays.phase, Phase::Applying);
    assert!(asked(&rt, "apply-monitors"));
}

/// As a successful apply lands: the new modes run and the countdown starts.
fn confirming(rt: &mut Runtime<State>) {
    rt.update(|s, tx| {
        displays::choose_resolution(s, 0, "1920x1080");
        displays::start_countdown(s, tx);
    });
    run(rt, 0.2);
}

#[test]
fn unconfirmed_display_settings_revert_after_ten_seconds() {
    let mut rt = on(Page::Displays, fixtures::land_monitors);
    confirming(&mut rt);
    assert!(rt.find("confirm-displays-dialog").is_some());
    run(&mut rt, 4.0);
    assert!(
        matches!(
            rt.state.displays.phase,
            Phase::Confirming { remaining: 6, .. }
        ),
        "{:?}",
        rt.state.displays.phase
    );
    assert!(rt.find("seconds").is_some());
    run(&mut rt, 6.0);
    assert_eq!(rt.state.displays.phase, Phase::Reverting);
    assert!(asked(&rt, "revert-monitors"));
}

#[test]
fn escape_on_the_confirmation_reverts() {
    let mut rt = on(Page::Displays, fixtures::land_monitors);
    confirming(&mut rt);
    press(&mut rt, Key::Escape);
    assert_eq!(rt.state.displays.phase, Phase::Reverting);
}

#[test]
fn keeping_saves_and_becomes_the_new_baseline() {
    let mut rt = on(Page::Displays, fixtures::land_monitors);
    confirming(&mut rt);
    tap(&mut rt, "keep");
    assert_eq!(rt.state.displays.phase, Phase::Saving);
    assert!(asked(&rt, "save-monitors"));
    // As the save's answer does:
    rt.update(|s, _| {
        s.displays.phase = Phase::Editing;
        displays::kept(s);
    });
    assert!(!displays::dirty(&rt.state.displays));
    assert_eq!(rt.state.displays.original[0].width, 1920);
}

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
fn a_volume_is_sent_once_the_slider_rests() {
    let mut rt = on(Page::Audio, |s| {
        s.audio.mixer = Remote::ready(fixtures::mixer())
    });
    let (x, y) = rt.find("device-42/volume/slider").expect("slider");
    rt.pointer_down(x - 100.0, y);
    rt.pointer_move(x - 120.0, y);
    rt.frame(FRAME);
    let v = rt.state.audio.mixer.data.as_ref().unwrap().sinks[1].volume;
    assert!(v < 0.75, "the knob follows the pointer ({v})");
    rt.pointer_up(x - 120.0, y);
    rt.frame(FRAME);
    assert!(!asked(&rt, "set-volume"), "not before the debounce");
    run(&mut rt, 0.2);
    assert!(asked(&rt, "set-volume"));
    assert!(rt.state.audio.pending.is_empty());
}

#[test]
fn a_muted_device_disables_its_slider_and_unmutes() {
    let mut rt = on(Page::Audio, |s| {
        s.audio.mixer = Remote::ready(fixtures::mixer())
    });
    let before = rt.state.audio.mixer.data.as_ref().unwrap().sources[0].volume;
    let (x, y) = rt.find("device-51/volume/slider").expect("slider");
    rt.pointer_down(x + 100.0, y);
    rt.pointer_up(x + 100.0, y);
    run(&mut rt, 0.2);
    assert_eq!(
        rt.state.audio.mixer.data.as_ref().unwrap().sources[0].volume,
        before
    );
    tap(&mut rt, "device-51/volume/mute");
    assert!(!rt.state.audio.mixer.data.as_ref().unwrap().sources[0].muted);
    assert!(asked(&rt, "set-mute"));
}

#[test]
fn opening_a_rule_loads_its_details_once() {
    let mut rt = on(Page::Windowrules, fixtures::rules_data);
    tap(&mut rt, "rule-float-pavucontrol/bar/head");
    assert!(
        rt.state
            .windowrules
            .open
            .contains(&"float-pavucontrol".to_owned())
    );
    assert_eq!(
        rt.state
            .jobs
            .skipped
            .iter()
            .filter(|j| **j == "rule-detail")
            .count(),
        1
    );
    // Closing and reopening a loading rule doesn't ask again.
    tap(&mut rt, "rule-float-pavucontrol/bar/head");
    tap(&mut rt, "rule-float-pavucontrol/bar/head");
    assert_eq!(
        rt.state
            .jobs
            .skipped
            .iter()
            .filter(|j| **j == "rule-detail")
            .count(),
        1
    );
}

#[test]
fn network_details_open_and_go_back() {
    let mut rt = on(Page::Network, |s| {
        s.network.interfaces = Remote::ready(fixtures::interfaces());
        s.network.wifi = Remote::ready(fixtures::wifi());
    });
    tap(&mut rt, "iface-wlan0");
    assert_eq!(rt.state.network.selected.as_deref(), Some("wlan0"));
    assert!(rt.find("page-network-detail").is_some());
    tap(&mut rt, "back");
    assert_eq!(rt.state.network.selected, None);
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
fn every_scene_builds() {
    for (name, scene) in fixtures::SCENES {
        let mut s = State::new();
        scene(&mut s);
        let mut rt = runtime(s);
        run(&mut rt, 0.2);
        assert!(rt.find("brand").is_some(), "{name}");
    }
}

/// As a load does: begin, then land `r`.
fn land<T>(r: &mut Remote<T>, answer: Result<T, String>) {
    let a = r.begin();
    r.land(a, answer);
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

fn count(rt: &Runtime<State>, job: &str) -> usize {
    rt.state.jobs.skipped.iter().filter(|j| **j == job).count()
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
        s.keybinds.binds.written();
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
fn a_panicking_job_still_lands_and_the_writer_goes_on() {
    let mut rt = runtime(State::new());
    rt.update(|s, _| {
        s.jobs.go_live();
        s.appearance.updating = true;
        s.jobs.write(
            "boom",
            || -> Result<(), String> { panic!("kaboom") },
            |s, r, tx| {
                s.appearance.updating = false;
                if let Err(e) = r {
                    app::fail(s, tx, "Failed", e);
                }
            },
        );
        s.jobs.write(
            "after",
            || Ok(7),
            |s, r, _| s.about.info.error = Some(format!("{r:?}")),
        );
    });
    for _ in 0..200 {
        rt.frame(FRAME);
        if rt.state.about.info.error.is_some() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(!rt.state.appearance.updating, "the busy flag cleared");
    assert_eq!(rt.state.about.info.error.as_deref(), Some("Ok(7)"));
    let toast = &rt.state.toasts.items[0].text;
    assert!(toast.contains("boom crashed: kaboom"), "{toast}");
}
