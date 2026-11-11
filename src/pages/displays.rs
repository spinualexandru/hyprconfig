use rune::{El, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Dialog, Kit, Select, SelectAppearance};

use crate::app::{self, State};
use crate::backend::hyprland::{self as hypr, MonitorInfo};
use crate::icons;
use crate::jobs::{self, Remote};
use crate::ui::{self, Card};

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

fn toggle_select(s: &mut State, key: &str) {
    s.displays.open_select = if s.displays.open_select.as_deref() == Some(key) {
        None
    } else {
        Some(key.to_owned())
    };
}

fn monitor_card(s: &State, k: &Kit, m: &MonitorInfo, layers: &mut Vec<El>) -> El {
    let d = &s.displays;
    let id = m.id;
    let current = d
        .chosen
        .iter()
        .find(|c| c.id == id)
        .cloned()
        .unwrap_or(Chosen {
            id,
            resolution: resolution_of(m),
            rate: rate_label(m.refresh_rate),
        });
    let editable = d.phase == Phase::Editing;

    let res_key = format!("res-{id}");
    let res_labels: Vec<String> = resolutions(m)
        .iter()
        .map(|r| r.replace('x', " × "))
        .collect();
    let res_current = current.resolution.replace('x', " × ");
    let rk = res_key.clone();
    let res_select = Select::new(res_key.clone(), &res_labels)
        .label("Resolution")
        .current(Some(&res_current))
        .open(d.open_select.as_deref() == Some(res_key.as_str()))
        .disabled(!editable)
        .appearance(SelectAppearance::Field)
        .width(260.0)
        .on_toggle(move |s: &mut State| toggle_select(s, &rk))
        .on_close(|s: &mut State| s.displays.open_select = None)
        .on_choose(move |s: &mut State, v: &str, _| {
            s.displays.open_select = None;
            choose_resolution(s, id, &v.replace(" × ", "x"));
        })
        .build(k, layers);

    let rate_key = format!("rate-{id}");
    let rate_labels: Vec<String> = rates_for(m, &current.resolution)
        .iter()
        .map(|r| format!("{r} Hz"))
        .collect();
    let rate_current = format!("{} Hz", current.rate);
    let rk = rate_key.clone();
    let rate_select = Select::new(rate_key.clone(), &rate_labels)
        .label("Refresh Rate")
        .current(Some(&rate_current))
        .open(d.open_select.as_deref() == Some(rate_key.as_str()))
        .disabled(!editable)
        .appearance(SelectAppearance::Field)
        .width(260.0)
        .on_toggle(move |s: &mut State| toggle_select(s, &rk))
        .on_close(|s: &mut State| s.displays.open_select = None)
        .on_choose(move |s: &mut State, v: &str, _| {
            s.displays.open_select = None;
            choose_rate(s, id, v.trim_end_matches(" Hz"));
        })
        .build(k, layers);

    Card::new()
        .icon(&icons::MONITOR)
        .title(m.name.clone())
        .description(m.description.clone())
        .child(ui::grid2(
            "settings",
            vec![
                ui::field(k, "resolution", "Resolution", res_select),
                ui::field(k, "rate", "Refresh Rate", rate_select),
                ui::field(
                    k,
                    "position",
                    "Position",
                    ui::value_text(k, format!("({}, {})", m.x, m.y)),
                ),
                ui::field(
                    k,
                    "scale",
                    "Scale",
                    ui::value_text(k, format!("{}×", m.scale)),
                ),
                ui::field(
                    k,
                    "transform",
                    "Transform",
                    ui::value_text(k, m.transform.clone()),
                ),
                ui::field(
                    k,
                    "workspace",
                    "Workspace",
                    ui::value_text(
                        k,
                        format!("{} (#{})", m.active_workspace_name, m.active_workspace_id),
                    ),
                ),
            ],
        ))
        .build(k)
        .key(format!("monitor-{id}"))
}

fn confirm_dialog(s: &State, k: &Kit, layers: &mut Vec<El>) {
    let Phase::Confirming { remaining, .. } = s.displays.phase else {
        return;
    };
    Dialog::new("confirm-displays", "Keep these display settings?")
        .open(true)
        .fit()
        .click_outside(false)
        .on_close(revert)
        .child(ui::dialog_body(
            col()
                .key("countdown")
                .w_full()
                .items_center()
                .gap(4.0)
                .child(
                    text("Your display settings have been changed. If you can see this dialog, the new settings are working. Keep them, or they revert automatically.")
                        .key("explanation")
                        .font(k.ty.body)
                        .color(k.c.text_muted)
                        .wrap()
                        .w_full()
                        .mb(12.0),
                )
                .child(text(remaining.to_string()).key("seconds").font(ui::title_style()).color(k.c.text_strong))
                .child(
                    text(format!("Reverting in {remaining} second{}", if remaining == 1 { "" } else { "s" }))
                        .key("caption")
                        .font(k.ty.body)
                        .color(k.c.text_muted),
                ),
            vec![
                Button::new("Revert").variant(ButtonVariant::Outline).on_press(revert).build(k).key("revert"),
                Button::new("Keep Changes").variant(ButtonVariant::Contrast).on_press(keep).build(k).key("keep"),
            ],
        ))
        .build(k, layers);
}

pub fn view(s: &State, k: &Kit, layers: &mut Vec<El>) -> El {
    let d = &s.displays;
    let r = &d.monitors;
    let refresh = ui::refresh_button(r.loading, |s, _| load(s))
        .disabled(r.loading || d.phase != Phase::Editing)
        .build(k)
        .key("refresh");
    let mut body = Vec::new();
    // A failed reload shows above the monitors from before.
    if let Some(e) = &r.error {
        body.push(ui::error_card(k, "error", "Error Loading Monitors", e));
    }
    match r.data.as_deref() {
        None if r.first_load() => body.push(col().key("loading").w_full().gap(16.0).children(
            (0..2).map(|i| {
                Card::new()
                    .icon(&icons::MONITOR)
                    .child(ui::skeleton_rows(k, 3, &[None, None]))
                    .build(k)
                    .key(format!("sk{i}"))
            }),
        )),
        None => {}
        Some([]) => body.push(ui::note_card(k, "empty", &["No monitors detected."])),
        Some(ms) => {
            for m in ms {
                body.push(monitor_card(s, k, m, layers));
            }
        }
    }
    confirm_dialog(s, k, layers);
    let page = ui::page(
        k,
        "displays",
        "Displays",
        "Configure your display settings",
        vec![refresh],
        body,
    );

    let show_bar = dirty(d) && !r.loading && r.data.as_ref().is_some_and(|m| !m.is_empty());
    let mut root = col()
        .key("displays-root")
        .w_full()
        .h_full()
        .min_h(0.0)
        .child(page.h(0.0).grow(1.0));
    if show_bar {
        let busy = d.phase != Phase::Editing;
        root = root.child(
            row()
                .key("changes-bar")
                .w_full()
                .h(64.0)
                .px(ui::GUTTER)
                .gap(8.0)
                .center_y()
                .fill(k.c.surface_raised_base)
                .border_sides([1.0, 0.0, 0.0, 0.0], k.c.border_weak)
                .child(
                    text(if d.phase == Phase::Applying {
                        "Applying…"
                    } else {
                        "You have unsaved changes to your display settings."
                    })
                    .font(k.ty.body)
                    .color(k.c.text_muted)
                    .grow(1.0)
                    .shrink(),
                )
                .child(
                    Button::new("Cancel")
                        .variant(ButtonVariant::Outline)
                        .disabled(busy)
                        .on_press(cancel)
                        .build(k)
                        .key("cancel"),
                )
                .child(
                    Button::new("Apply Changes")
                        .variant(ButtonVariant::Contrast)
                        .disabled(busy)
                        .on_press(apply)
                        .build(k)
                        .key("apply"),
                ),
        );
    }
    root
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
