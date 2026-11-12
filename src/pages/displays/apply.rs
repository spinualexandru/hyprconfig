//! Applying chosen modes: the confirmation countdown, then reverting or keeping them.

use rune::Tx;

use super::*;
use crate::app::{self, State};
use crate::backend::hyprland as hypr;

/// A monitor's settings as `hyprctl`/the config want them.
#[derive(Clone)]
struct Mode {
    name: String,
    width: u16,
    height: u16,
    rate: f32,
    x: i32,
    y: i32,
    scale: f32,
}

fn modes(d: &DisplaysState, use_original: bool) -> Vec<Mode> {
    d.original
        .iter()
        .filter_map(|m| {
            let base = Mode {
                name: m.name.clone(),
                width: m.width,
                height: m.height,
                rate: m.refresh_rate,
                x: m.x,
                y: m.y,
                scale: m.scale,
            };
            if use_original {
                return Some(base);
            }
            let c = d.chosen.iter().find(|c| c.id == m.id)?;
            let (width, height) = parse_resolution(&c.resolution)?;
            Some(Mode {
                width,
                height,
                rate: c.rate.parse().ok()?,
                ..base
            })
        })
        .collect()
}

fn apply_one(m: &Mode, save: bool) -> Result<(), String> {
    let r = if save {
        hypr::save_monitor_settings(m.name.clone(), m.width, m.height, m.rate, m.x, m.y, m.scale)
    } else {
        hypr::apply_monitor_settings(m.name.clone(), m.width, m.height, m.rate, m.x, m.y, m.scale)
    };
    r.map_err(|e| format!("{}: {e}", m.name))
}

/// Applies or saves every mode, stopping at the first failure.
fn apply_all(modes: Vec<Mode>, save: bool) -> Result<(), String> {
    modes.iter().try_for_each(|m| apply_one(m, save))
}

/// Applies every mode, going on past failures (a revert must put back every monitor it can);
/// the failures, joined.
fn apply_each(
    modes: &[Mode],
    mut apply: impl FnMut(&Mode) -> Result<(), String>,
) -> Result<(), String> {
    let failures: Vec<String> = modes.iter().filter_map(|m| apply(m).err()).collect();
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    }
}

pub fn apply(s: &mut State) {
    if s.displays.phase != Phase::Editing {
        return;
    }
    s.displays.phase = Phase::Applying;
    let modes = modes(&s.displays, false);
    s.jobs.write(
        "apply-monitors",
        move || apply_all(modes, false),
        |s, r, tx| match r {
            Ok(()) => start_countdown(s, tx),
            Err(e) => {
                app::fail(s, tx, "Failed to apply settings", e);
                // The monitors applied before the failing one run their new modes.
                begin_revert(s);
            }
        },
    );
}

pub fn start_countdown(s: &mut State, tx: &mut Tx) {
    let next_tick = tx.clock() + 1.0;
    s.displays.phase = Phase::Confirming {
        remaining: CONFIRM_SECS,
        next_tick,
    };
    tx.wake_at(next_tick);
}

/// Counts the confirmation down; reverts at zero.
pub fn tick(s: &mut State, now: f64, tx: &mut Tx) -> bool {
    let Phase::Confirming {
        remaining,
        next_tick,
    } = s.displays.phase
    else {
        return false;
    };
    if now < next_tick {
        return false;
    }
    if remaining <= 1 {
        revert(s);
    } else {
        let next_tick = next_tick + 1.0;
        s.displays.phase = Phase::Confirming {
            remaining: remaining - 1,
            next_tick,
        };
        tx.wake_at(next_tick);
    }
    true
}

pub fn revert(s: &mut State) {
    if matches!(s.displays.phase, Phase::Confirming { .. }) {
        begin_revert(s);
    }
}

/// Puts every monitor back on its original mode.
fn begin_revert(s: &mut State) {
    s.displays.phase = Phase::Reverting;
    let modes = modes(&s.displays, true);
    s.jobs.write(
        "revert-monitors",
        move || apply_each(&modes, |m| apply_one(m, false)),
        |s, r, tx| {
            s.displays.phase = Phase::Editing;
            cancel(s);
            match r {
                Ok(()) => s.toasts.success(tx, "Display settings reverted"),
                Err(e) => {
                    app::fail(s, tx, "Some displays didn't revert", e);
                    // Show what the monitors run now, not what they were meant to return to.
                    load(s);
                }
            }
        },
    );
}

pub fn keep(s: &mut State) {
    if !matches!(s.displays.phase, Phase::Confirming { .. }) {
        return;
    }
    s.displays.phase = Phase::Saving;
    let modes = modes(&s.displays, false);
    s.jobs.write(
        "save-monitors",
        move || apply_all(modes, true),
        |s, r, tx| {
            s.displays.phase = Phase::Editing;
            match r {
                Ok(()) => {
                    kept(s);
                    s.toasts.success(tx, "Display settings saved");
                }
                Err(e) => app::fail(s, tx, "Failed to save", e),
            }
        },
    );
}

/// The chosen modes become what reverting returns to.
pub fn kept(s: &mut State) {
    let d = &mut s.displays;
    for m in d
        .original
        .iter_mut()
        .chain(d.monitors.data.iter_mut().flatten())
    {
        if let Some(c) = d.chosen.iter().find(|c| c.id == m.id)
            && let (Some((w, h)), Ok(rate)) = (parse_resolution(&c.resolution), c.rate.parse())
        {
            m.width = w;
            m.height = h;
            m.refresh_rate = rate;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Mode, apply_each};

    fn mode(name: &str) -> Mode {
        Mode {
            name: name.into(),
            width: 1920,
            height: 1080,
            rate: 60.0,
            x: 0,
            y: 0,
            scale: 1.0,
        }
    }

    #[test]
    fn a_revert_tries_every_monitor_and_reports_each_failure() {
        let modes = [mode("DP-1"), mode("DP-2"), mode("HDMI-A-1")];
        let mut tried = Vec::new();
        let r = apply_each(&modes, |m| {
            tried.push(m.name.clone());
            if m.name == "HDMI-A-1" {
                Ok(())
            } else {
                Err(format!("{}: rejected", m.name))
            }
        });
        assert_eq!(tried, ["DP-1", "DP-2", "HDMI-A-1"]);
        assert_eq!(r, Err("DP-1: rejected; DP-2: rejected".to_owned()));
        assert_eq!(apply_each(&modes, |_| Ok(())), Ok(()));
    }
}
