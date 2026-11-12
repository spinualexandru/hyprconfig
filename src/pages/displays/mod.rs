//! The displays page: each monitor's resolution and refresh rate, applied with a timed
//! confirmation that reverts unless the user keeps the new settings.

mod apply;
mod view;

pub use apply::*;
pub use view::view;

use crate::app::State;
use crate::backend::hyprland::{self as hypr, MonitorInfo};
use crate::jobs::{self, Remote};

/// Seconds the new settings stay unless the user keeps them.
pub const CONFIRM_SECS: u32 = 10;

#[derive(Default)]
pub struct DisplaysState {
    pub monitors: Remote<Vec<MonitorInfo>>,
    /// What each monitor runs as Hyprland last reported it (or as last saved); reverting
    /// returns here.
    pub original: Vec<MonitorInfo>,
    /// The mode picked for each monitor.
    pub chosen: Vec<Chosen>,
    /// The open dropdown: `res-<id>` or `rate-<id>`.
    pub open_select: Option<String>,
    pub phase: Phase,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Chosen {
    pub id: i128,
    /// `WIDTHxHEIGHT`
    pub resolution: String,
    /// Hz with two decimals, as Hyprland lists modes.
    pub rate: String,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Phase {
    #[default]
    Editing,
    /// `hyprctl` is applying the chosen modes.
    Applying,
    /// The new modes run; they revert when `remaining` reaches 0 unless kept.
    Confirming { remaining: u32, next_tick: f64 },
    /// Writing the kept modes to the config.
    Saving,
    /// Putting the original modes back.
    Reverting,
}

pub fn resolution_of(m: &MonitorInfo) -> String {
    format!("{}x{}", m.width, m.height)
}

pub fn rate_label(r: f32) -> String {
    format!("{r:.2}")
}

fn chosen_from(monitors: &[MonitorInfo]) -> Vec<Chosen> {
    monitors
        .iter()
        .map(|m| Chosen {
            id: m.id,
            resolution: resolution_of(m),
            rate: rate_label(m.refresh_rate),
        })
        .collect()
}

/// Distinct resolutions in Hyprland's order.
pub fn resolutions(m: &MonitorInfo) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for mode in &m.available_modes {
        let r = format!("{}x{}", mode.width, mode.height);
        if !out.contains(&r) {
            out.push(r);
        }
    }
    out
}

pub fn rates_for(m: &MonitorInfo, resolution: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for mode in m
        .available_modes
        .iter()
        .filter(|mode| format!("{}x{}", mode.width, mode.height) == resolution)
    {
        let r = rate_label(mode.refresh_rate);
        if !out.contains(&r) {
            out.push(r);
        }
    }
    out
}

fn parse_resolution(r: &str) -> Option<(u16, u16)> {
    let (w, h) = r.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

pub fn load(s: &mut State) {
    if s.displays.phase != Phase::Editing {
        return;
    }
    jobs::load_then(
        s,
        "monitors",
        |s| &mut s.displays.monitors,
        hypr::get_monitors,
        |s| {
            if let Some(m) = &s.displays.monitors.data {
                s.displays.original = m.clone();
                s.displays.chosen = chosen_from(m);
            }
        },
    );
}

pub fn dirty(d: &DisplaysState) -> bool {
    d.chosen != chosen_from(&d.original)
}

pub fn choose_resolution(s: &mut State, id: i128, resolution: &str) {
    let Some(m) = s
        .displays
        .monitors
        .data
        .as_ref()
        .and_then(|ms| ms.iter().find(|m| m.id == id))
    else {
        return;
    };
    let rates = rates_for(m, resolution);
    let Some(c) = s.displays.chosen.iter_mut().find(|c| c.id == id) else {
        return;
    };
    if !rates.contains(&c.rate)
        && let Some(first) = rates.first()
    {
        c.rate = first.clone();
    }
    c.resolution = resolution.to_owned();
}

pub fn choose_rate(s: &mut State, id: i128, rate: &str) {
    if let Some(c) = s.displays.chosen.iter_mut().find(|c| c.id == id) {
        c.rate = rate.to_owned();
    }
}

fn cancel(s: &mut State) {
    s.displays.chosen = chosen_from(&s.displays.original);
}
