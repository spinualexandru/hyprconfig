//! Behaviour on Rune's headless runtime, with a virtual clock. Jobs don't reach the system
//! here: `jobs.skipped` records what each interaction asked for.

mod audio;
mod displays;
mod general;
mod jobs;
mod keybinds;
mod network;
mod rules;
mod shell;
mod vars;

use rune::{Env, Key, KeyEvent, Mods, Runtime};

use crate::app::{self, State};
use crate::fixtures;
use crate::jobs::Remote;
use crate::pages::Page;

pub(super) const FRAME: f64 = 1.0 / 60.0;

pub(super) fn runtime(state: State) -> Runtime<State> {
    let mut rt = Runtime::new(state, app::view, (1100.0, 760.0), Env::new(false, 1.0));
    rt.poll = Some(app::poll);
    rt.on_key = Some(app::on_key);
    rt.frame(0.0);
    rt
}

pub(super) fn on(page: Page, fill: fn(&mut State)) -> Runtime<State> {
    let mut s = State::new();
    s.page = page;
    fill(&mut s);
    runtime(s)
}

pub(super) fn run(rt: &mut Runtime<State>, secs: f64) {
    for _ in 0..(secs / FRAME).round() as usize {
        rt.frame(FRAME);
    }
}

pub(super) fn tap(rt: &mut Runtime<State>, key: &str) {
    let (x, y) = rt.find(key).unwrap_or_else(|| panic!("{key} is mounted"));
    rt.pointer_down(x, y);
    rt.pointer_up(x, y);
    run(rt, 0.5);
}

pub(super) fn press(rt: &mut Runtime<State>, key: Key) {
    rt.key(KeyEvent {
        key,
        mods: Mods::default(),
    });
    run(rt, 0.3);
}

pub(super) fn asked(rt: &Runtime<State>, job: &str) -> bool {
    rt.state.jobs.skipped.contains(&job)
}

/// As a load does: begin, then land `r`.
pub(super) fn land<T>(r: &mut Remote<T>, answer: Result<T, String>) {
    let a = r.begin();
    r.land(a, answer);
}

pub(super) fn count(rt: &Runtime<State>, job: &str) -> usize {
    rt.state.jobs.skipped.iter().filter(|j| **j == job).count()
}
