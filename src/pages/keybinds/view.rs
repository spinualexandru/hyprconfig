//! The keybinds page view: the bind tables with their rows, and the page shell.

use std::collections::HashMap;

use rune::{El, row, text};
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant, Kit, SegmentedControl};

use super::form::form_dialog;
use super::*;
use crate::app::State;
use crate::backend::hyprland::{Keybind, Variable};
use crate::dispatchers;
use crate::icons;
use crate::ui::{self, Card, Tone};

fn modifier_tone(m: &str) -> Tone {
    match m.to_uppercase().as_str() {
        "SUPER" | "MOD4" | "WIN" => Tone::Accent,
        "SHIFT" => Tone::Neutral,
        "CTRL" | "CONTROL" => Tone::Danger,
        _ => Tone::Outline,
    }
}

/// The config's variables by name, for `command`.
type VarValues<'a> = HashMap<&'a str, &'a str>;

fn var_values(vars: &[Variable]) -> VarValues<'_> {
    vars.iter()
        .map(|v| (v.name.as_str(), v.value.as_str()))
        .collect()
}

fn command(k: &Kit, params: &str, vars: &VarValues) -> El {
    let parts = dispatchers::resolve_command(params, |name| vars.contains_key(name));
    if !parts.iter().any(|p| p.variable) {
        let shown = if params.is_empty() {
            "-".to_owned()
        } else {
            params.to_owned()
        };
        return text(shown)
            .key("command")
            .font(k.ty.body)
            .color(k.c.text_muted)
            .wrap()
            .w_full()
            .selectable(true)
            .selection_color(k.selection());
    }
    row()
        .key("command")
        .w_full()
        .flex_wrap()
        .children(parts.into_iter().enumerate().map(|(i, p)| {
            let value = p
                .variable
                .then(|| vars.get(dispatchers::variable_name(&p.text)))
                .flatten();
            let t = text(p.text).key(format!("p{i}")).font(k.ty.body);
            match value {
                Some(&v) => t
                    .font(k.ty.body_medium)
                    .color(k.c.text_accent)
                    .help(v.to_owned()),
                None => t.color(k.c.text_muted),
            }
        }))
}

const WIDTHS: [Option<f32>; 5] = [Some(170.0), Some(110.0), Some(170.0), None, Some(76.0)];

fn bind_row(k: &Kit, tab: Tab, i: usize, b: &Keybind, vars: &VarValues, settling: bool) -> El {
    let mods = if b.modifiers.is_empty() {
        text("None").font(k.ty.body).color(k.c.text_muted)
    } else {
        row().gap(4.0).flex_wrap().children(
            b.modifiers
                .iter()
                .map(|m| ui::pill(k, m.clone(), modifier_tone(m))),
        )
    };
    let mut actions = row().gap(2.0);
    if tab == Tab::Binds {
        actions = actions.child(
            IconButton::new(&icons::PENCIL)
                .variant(IconButtonVariant::Ghost)
                .label(format!("Edit bind {}", i + 1))
                .help("Edit")
                .disabled(settling)
                .on_press(move |s: &mut State| open_form(s, FormMode::Edit(i)))
                .build(k)
                .key("edit"),
        );
    }
    actions = actions.child(
        IconButton::new(&icons::TRASH_2)
            .variant(IconButtonVariant::Ghost)
            .label(format!("Delete bind {}", i + 1))
            .help("Delete")
            .disabled(settling)
            .on_press(move |s: &mut State| delete(s, tab, i))
            .build(k)
            .key("delete"),
    );
    ui::table_row(
        k,
        format!("bind-{i}"),
        &WIDTHS,
        vec![
            mods,
            ui::keycap(k, &b.key),
            ui::code(k, b.dispatcher.clone()).help(dispatchers::description(&b.dispatcher)),
            command(k, &b.params, vars),
            actions,
        ],
    )
}

fn bind_table(s: &State, k: &Kit, tab: Tab) -> Option<El> {
    let (r, icon, title, description, empty) = match tab {
        Tab::Binds => (
            &s.keybinds.binds,
            &icons::KEYBOARD,
            "Keybinds",
            "Your Hyprland keyboard shortcuts",
            "No keybinds found in your Hyprland configuration.",
        ),
        Tab::Universal => (
            &s.keybinds.universal,
            &icons::GLOBE,
            "Universal Binds",
            "Keybinds that work across all submaps",
            "No universal binds found. Universal binds work across all submaps.",
        ),
    };
    let settling = r.settling();
    let vars = var_values(&s.keybinds.variables);
    Some(match r.data.as_deref() {
        None if r.first_load() => Card::new()
            .icon(icon)
            .title(title)
            .description(description)
            .child(ui::skeleton_rows(
                k,
                8,
                &[Some(128.0), Some(64.0), Some(96.0), None],
            ))
            .build(k)
            .key("loading"),
        // The error card says why.
        None => return None,
        Some([]) => ui::note_card(k, "empty", &[empty]),
        Some(binds) => Card::new()
            .icon(icon)
            .title(format!("{title} ({})", binds.len()))
            .description(description)
            .child(ui::table(
                k,
                "table",
                &[
                    ("Modifiers", WIDTHS[0]),
                    ("Key", WIDTHS[1]),
                    ("Action", WIDTHS[2]),
                    ("Command", WIDTHS[3]),
                    ("", WIDTHS[4]),
                ],
                binds
                    .iter()
                    .enumerate()
                    .map(|(i, b)| bind_row(k, tab, i, b, &vars, settling))
                    .collect(),
            ))
            .build(k)
            .key(if tab == Tab::Binds {
                "binds"
            } else {
                "universal"
            }),
    })
}

pub fn view(s: &State, k: &Kit, layers: &mut Vec<El>) -> El {
    let kb = &s.keybinds;
    let tab = kb.tab;
    let loading = match tab {
        Tab::Binds => kb.binds.loading,
        Tab::Universal => kb.universal.loading,
    };
    let add = Button::new(if tab == Tab::Binds {
        "Add Keybind"
    } else {
        "Add Universal Bind"
    })
    .icon(&icons::PLUS)
    .variant(ButtonVariant::Contrast)
    .on_press(move |s: &mut State| {
        open_form(
            s,
            if tab == Tab::Binds {
                FormMode::Add
            } else {
                FormMode::AddUniversal
            },
        )
    })
    .build(k)
    .key("add");
    let refresh = ui::refresh_button(loading, move |s, _| reload(s, tab))
        .build(k)
        .key("refresh");
    let shown = match tab {
        Tab::Binds => &kb.binds,
        Tab::Universal => &kb.universal,
    };
    let mut body = Vec::new();
    if let Some(e) = &shown.error {
        body.push(ui::error_card(k, "error", "Error", e));
    }
    body.push(
        row().key("tabs-row").child(
            SegmentedControl::new(
                &["Keybinds", "Universal"],
                Some(if tab == Tab::Binds { 0 } else { 1 }),
            )
            .label("Bind type")
            .width(Some(280.0))
            .on_pick(|s: &mut State, i, tx| {
                s.keybinds.tab = if i == 0 { Tab::Binds } else { Tab::Universal };
                tx.instant();
            })
            .build(k)
            .key("tabs"),
        ),
    );
    body.extend(bind_table(s, k, tab));
    form_dialog(s, k, layers);
    ui::page(
        k,
        "keybinds",
        "Keybinds",
        "View and change your Hyprland keyboard shortcuts and keybindings",
        vec![add, refresh],
        body,
    )
}
