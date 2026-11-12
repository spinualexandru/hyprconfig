//! The Displays page: pending changes, apply, and the revert countdown.

use super::*;
use crate::pages::displays::{self, Phase};

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
