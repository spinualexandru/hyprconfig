//! The Audio page: volume sliders and mute.

use super::*;

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
