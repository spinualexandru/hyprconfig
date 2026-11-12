//! The add/edit keybind dialog.

use rune::{El, col, row, text};
use rune_kit::{Button, ButtonVariant, Checkbox, Dialog, Kit, TextInput};

use super::*;
use crate::app::State;
use crate::dispatchers;
use crate::ui;

fn labelled(k: &Kit, key: &str, label: &str, control: El, hint: Option<String>) -> El {
    let mut c = col()
        .key(key)
        .w_full()
        .gap(6.0)
        .child(
            text(label.to_owned())
                .font(k.ty.body_medium)
                .color(k.c.text_strong),
        )
        .child(control);
    if let Some(h) = hint {
        c = c.child(
            text(h)
                .key("hint")
                .font(k.ty.small)
                .color(k.c.text_muted)
                .wrap()
                .w_full(),
        );
    }
    c
}

pub(super) fn form_dialog(s: &State, k: &Kit, layers: &mut Vec<El>) {
    let kb = &s.keybinds;
    let Some(mode) = kb.form else { return };
    let busy = kb.form_busy;
    let (title, description, submit_label) = match mode {
        FormMode::AddUniversal => (
            "Add Universal Bind",
            "Create a keybind that works across all submaps.",
            "Add Universal Bind",
        ),
        FormMode::Edit(_) => (
            "Edit Keybind",
            "Modify the keybind settings below.",
            "Save Changes",
        ),
        FormMode::Add => (
            "Add New Keybind",
            "Create a new keyboard shortcut for Hyprland.",
            "Add Keybind",
        ),
    };
    let mut mods =
        row()
            .key("mods")
            .gap(16.0)
            .flex_wrap()
            .children(dispatchers::MODIFIERS.iter().map(|m| {
                Checkbox::new(kb.mods.iter().any(|x| x == m))
                    .label(*m)
                    .disabled(busy)
                    .on_toggle(move |s: &mut State| toggle_mod(s, m))
                    .build(k)
                    .key(m.to_lowercase())
            }));
    // Modifiers the form has no checkbox for (`CAPS`, `MOD5`, an unknown variable) keep one
    // of their own, so they're seen and saved rather than dropped.
    let mut others: Vec<&str> = Vec::new();
    for (_, id) in &kb.original_mods {
        if dispatchers::known_modifier(id).is_none() && !others.contains(&id.as_str()) {
            others.push(id);
        }
    }
    for (i, id) in others.into_iter().enumerate() {
        let owned = id.to_owned();
        mods = mods.child(
            Checkbox::new(kb.mods.iter().any(|x| x == id))
                .label(id)
                .disabled(busy)
                .on_toggle(move |s: &mut State| toggle_mod(s, &owned))
                .build(k)
                .key(format!("other-{i}")),
        );
    }
    let aliases: Vec<String> = kb
        .original_mods
        .iter()
        .filter(|(written, id)| written != id && dispatchers::known_modifier(id).is_some())
        .map(|(written, id)| format!("{written} ({id})"))
        .collect();
    let mods_hint = (!aliases.is_empty())
        .then(|| format!("Kept as written in your config: {}", aliases.join(", ")));
    let key_input = TextInput::new(&kb.key)
        .label("Key")
        .placeholder("e.g., Q, Return, 1, left, F1")
        .width(None)
        .disabled(busy)
        .on_edit(|s: &mut State| &mut s.keybinds.key)
        .on_change(|s: &mut State, _| s.keybinds.form_error = None)
        .on_submit(|s: &mut State, _| submit(s))
        .build(k)
        .key("input");

    let dispatcher_input = TextInput::new(&kb.dispatcher)
        .label("Dispatcher")
        .placeholder("e.g., exec, killactive, workspace")
        .width(None)
        .disabled(busy)
        .on_edit(|s: &mut State| &mut s.keybinds.dispatcher)
        .on_change(|s: &mut State, _| s.keybinds.form_error = None)
        .on_submit(|s: &mut State, _| submit(s))
        .build(k)
        .key("input");
    let picks = dispatchers::suggestions(&kb.dispatcher.text);
    let mut dispatcher = col().w_full().gap(6.0).child(dispatcher_input);
    if !picks.is_empty() && !busy {
        dispatcher = dispatcher.child(
            row()
                .key("suggestions")
                .w_full()
                .gap(6.0)
                .flex_wrap()
                .children(picks.into_iter().map(|d| {
                    ui::code(k, d)
                        .key(format!("pick-{d}"))
                        .role("button")
                        .label(format!("Use {d}"))
                        .help(dispatchers::description(d))
                        .focusable(true)
                        .hover(k.c.hover)
                        .press(k.c.pressed)
                        .on_press(move |s: &mut State| {
                            s.keybinds.dispatcher.replace_text(d);
                        })
                })),
        );
    }
    let dispatcher_hint = "Common: exec, killactive, workspace, togglefloating, fullscreen. \
                           Use lua for any other hl.dsp.* call";

    let lua_dispatcher = kb.dispatcher.text.trim() == dispatchers::RAW_LUA;
    let params_input = TextInput::new(&kb.params)
        .label("Parameters")
        .width(None)
        .placeholder(if lua_dispatcher {
            "e.g., hl.dsp.window.fullscreen({ mode = \"maximized\" })"
        } else {
            "e.g., kitty, 1, l"
        })
        .text_style(ui::mono(k))
        .disabled(busy)
        .on_edit(|s: &mut State| &mut s.keybinds.params)
        .on_submit(|s: &mut State, _| submit(s))
        .build(k)
        .key("input");
    let params_hint = if lua_dispatcher {
        "Written to the config as-is: an hl.dsp.* call or a function"
    } else {
        "Command arguments. Can use variables like $terminal or ${terminal}"
    };

    let mut fields = col()
        .key("fields")
        .w_full()
        .gap(16.0)
        .child(labelled(
            k,
            "modifiers",
            "Modifiers (optional)",
            mods,
            mods_hint,
        ))
        .child(labelled(
            k,
            "key",
            "Key *",
            key_input,
            Some("Examples: Q, Return, Escape, left, right, up, down, F1-F12, 1-9".into()),
        ))
        .child(labelled(
            k,
            "dispatcher",
            "Dispatcher (Action) *",
            dispatcher,
            Some(dispatcher_hint.into()),
        ))
        .child(labelled(
            k,
            "params",
            if lua_dispatcher {
                "Lua expression *"
            } else {
                "Parameters (optional)"
            },
            params_input,
            Some(params_hint.into()),
        ));
    if let Some(e) = &kb.form_error {
        fields = fields.child(
            text(e.clone())
                .key("error")
                .font(k.ty.body)
                .color(k.c.danger.icon)
                .wrap()
                .w_full(),
        );
    }
    let buttons = vec![
        Button::new("Cancel")
            .variant(ButtonVariant::Outline)
            .disabled(busy)
            .on_press(close_form)
            .build(k)
            .key("cancel"),
        Button::new(if busy { "Saving…" } else { submit_label })
            .variant(ButtonVariant::Contrast)
            .disabled(busy)
            .on_press(submit)
            .build(k)
            .key("submit"),
    ];
    Dialog::new("keybind-form", title)
        .description(description)
        .width(600.0)
        .open(true)
        .fit()
        .on_close(close_form)
        .child(ui::dialog_body(fields, buttons))
        .build(k, layers);
}
