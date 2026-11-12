//! The General page: colour generator and Noctalia integration.

use super::*;
use crate::backend::palette::Generator;

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
