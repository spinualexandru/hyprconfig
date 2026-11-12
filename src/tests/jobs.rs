//! The job runner as the app sees it.

use super::*;

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
