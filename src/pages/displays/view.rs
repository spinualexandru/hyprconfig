//! The displays page view: a card per monitor with its mode selects, the changes bar and
//! the keep-or-revert dialog.

use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Dialog, Kit, Select, SelectAppearance};

use super::*;
use crate::app::State;
use crate::backend::hyprland::MonitorInfo;
use crate::icons;
use crate::ui::{self, Card};

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
