use rune::{El, TextEdit, col, row, text};
use rune_kit::{
    Button, ButtonVariant, Checkbox, Dialog, IconButton, IconButtonVariant, Kit, SegmentedControl,
    TextInput,
};

use crate::app::{self, State};
use crate::backend::hyprland::{self as hypr, Keybind, Variable};
use crate::dispatchers;
use crate::icons;
use crate::jobs::Remote;
use crate::ui::{self, Card, Tone};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Binds,
    /// `bindu`: binds that work in every submap.
    Universal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormMode {
    Add,
    /// Editing the bind at this index.
    Edit(usize),
    AddUniversal,
}

#[derive(Default)]
pub struct KeybindsState {
    pub binds: Remote<Vec<Keybind>>,
    pub universal: Remote<Vec<Keybind>>,
    /// The config's variables, to show what `$terminal` stands for.
    pub variables: Vec<Variable>,
    pub tab: Tab,
    pub form: Option<FormMode>,
    /// The bind being edited, as loaded: the save checks the config still has it there.
    pub original: Option<Keybind>,
    /// Its modifiers as written, each with its `dispatchers::modifier_id`.
    pub original_mods: Vec<(String, String)>,
    /// Ticked modifiers, by `dispatchers::modifier_id`.
    pub mods: Vec<String>,
    pub key: TextEdit,
    pub dispatcher: TextEdit,
    pub params: TextEdit,
    pub form_error: Option<String>,
    pub form_busy: bool,
}

pub fn load(s: &mut State) {
    app::load_config_info(s);
    load_binds(s);
    load_universal(s);
    s.jobs.run("variables", hypr::get_variables, |s, r, _| {
        // Optional: the table just doesn't explain variables without them.
        if let Ok(v) = r {
            s.keybinds.variables = v;
        }
    });
}

fn remote(s: &mut State, tab: Tab) -> &mut Remote<Vec<Keybind>> {
    match tab {
        Tab::Binds => &mut s.keybinds.binds,
        Tab::Universal => &mut s.keybinds.universal,
    }
}

fn reload(s: &mut State, tab: Tab) {
    match tab {
        Tab::Binds => load_binds(s),
        Tab::Universal => load_universal(s),
    }
}

/// Fails unless the bind at `index` in `current` is still `expected`: binds go by position, so
/// a bind that moved (another edit, the file changed) must not be overwritten or deleted.
pub fn check_bind(
    current: Result<Vec<Keybind>, String>,
    index: usize,
    expected: &Keybind,
) -> Result<(), String> {
    match current?.get(index) {
        Some(b) if b == expected => Ok(()),
        _ => Err("The keybind changed in your config since it was loaded. \
                  Refresh and try again."
            .into()),
    }
}

fn load_binds(s: &mut State) {
    let attempt = s.keybinds.binds.begin();
    s.jobs.run("keybinds", hypr::get_keybinds, move |s, r, _| {
        s.keybinds.binds.land(attempt, r);
    });
}

fn load_universal(s: &mut State) {
    let attempt = s.keybinds.universal.begin();
    s.jobs.run("bindu", hypr::get_all_bindu, move |s, r, _| {
        s.keybinds.universal.land(attempt, r);
    });
}

pub fn open_form(s: &mut State, mode: FormMode) {
    let kb = &mut s.keybinds;
    let existing = match mode {
        FormMode::Edit(i) => {
            // Positions shift while a change is written; wait for the list to reload.
            if kb.binds.settling() {
                return;
            }
            let Some(b) = kb.binds.data.as_ref().and_then(|b| b.get(i)).cloned() else {
                return;
            };
            Some(b)
        }
        _ => None,
    };
    let b = existing.clone().unwrap_or(Keybind {
        modifiers: Vec::new(),
        key: String::new(),
        dispatcher: String::new(),
        params: String::new(),
    });
    let vars: Vec<(&str, &str)> = kb
        .variables
        .iter()
        .map(|v| (v.name.as_str(), v.value.as_str()))
        .collect();
    kb.original_mods = b
        .modifiers
        .iter()
        .map(|m| (m.clone(), dispatchers::modifier_id(m, &vars)))
        .collect();
    kb.mods = Vec::new();
    for (_, id) in &kb.original_mods {
        if !kb.mods.contains(id) {
            kb.mods.push(id.clone());
        }
    }
    kb.original = existing;
    kb.form = Some(mode);
    kb.key = TextEdit::new(b.key);
    kb.key.focused = true;
    kb.dispatcher = TextEdit::new(b.dispatcher);
    kb.params = TextEdit::new(b.params);
    kb.form_error = None;
    kb.form_busy = false;
}

pub fn close_form(s: &mut State) {
    if !s.keybinds.form_busy {
        s.keybinds.form = None;
    }
}

pub fn toggle_mod(s: &mut State, m: &str) {
    let mods = &mut s.keybinds.mods;
    match mods.iter().position(|x| x == m) {
        Some(i) => {
            mods.remove(i);
        }
        None => mods.push(m.to_owned()),
    }
}

pub fn submit(s: &mut State) {
    let kb = &mut s.keybinds;
    let Some(mode) = kb.form else { return };
    if kb.form_busy {
        return;
    }
    let key = kb.key.text.trim().to_owned();
    let dispatcher = kb.dispatcher.text.trim().to_owned();
    let params = kb.params.text.trim().to_owned();
    if key.is_empty() {
        kb.form_error = Some("Key is required".into());
        return;
    }
    if dispatcher.is_empty() {
        kb.form_error = Some("Dispatcher is required".into());
        return;
    }
    // Modifiers the user left alone keep their spelling, order and any the form has no
    // checkbox for.
    let mods = dispatchers::modifiers_to_save(&kb.original_mods, &kb.mods);
    let original = kb.original.clone();
    if matches!(mode, FormMode::Edit(_)) && original.is_none() {
        return;
    }
    kb.form_busy = true;
    kb.form_error = None;
    let tab = if mode == FormMode::AddUniversal {
        Tab::Universal
    } else {
        Tab::Binds
    };
    remote(s, tab).hold();
    s.jobs.write(
        "save-keybind",
        move || match (mode, original) {
            (FormMode::Edit(i), Some(expected)) => {
                check_bind(hypr::get_keybinds(), i, &expected)?;
                hypr::edit_keybind(i, mods, key, dispatcher, params)
            }
            (FormMode::AddUniversal, _) => hypr::add_bindu(mods, key, dispatcher, params),
            _ => hypr::add_keybind(mods, key, dispatcher, params),
        },
        move |s, r, _| {
            s.keybinds.form_busy = false;
            match r {
                Ok(()) => {
                    s.keybinds.form = None;
                    remote(s, tab).written();
                    reload(s, tab);
                }
                Err(e) => {
                    remote(s, tab).release();
                    s.keybinds.form_error = Some(e);
                }
            }
        },
    );
}

pub fn delete(s: &mut State, tab: Tab, index: usize) {
    let r = remote(s, tab);
    // One change at a time: positions shift until the list reloads.
    if r.settling() {
        return;
    }
    let Some(expected) = r.data.as_ref().and_then(|b| b.get(index)).cloned() else {
        return;
    };
    r.hold();
    s.jobs.write(
        "delete-keybind",
        move || match tab {
            Tab::Binds => {
                check_bind(hypr::get_keybinds(), index, &expected)?;
                hypr::delete_keybind(index)
            }
            Tab::Universal => {
                check_bind(hypr::get_all_bindu(), index, &expected)?;
                hypr::delete_bindu(index)
            }
        },
        move |s, r, tx| match r {
            Ok(()) => {
                remote(s, tab).written();
                reload(s, tab);
            }
            Err(e) => {
                remote(s, tab).release();
                app::fail(s, tx, "Failed to delete the keybind", e);
            }
        },
    );
}

fn modifier_tone(m: &str) -> Tone {
    match m.to_uppercase().as_str() {
        "SUPER" | "MOD4" | "WIN" => Tone::Accent,
        "SHIFT" => Tone::Neutral,
        "CTRL" | "CONTROL" => Tone::Danger,
        _ => Tone::Outline,
    }
}

fn command(k: &Kit, params: &str, vars: &[Variable]) -> El {
    let parts = dispatchers::parse_command(params);
    if !parts.iter().any(|p| p.variable) {
        let shown = if params.is_empty() {
            "—".to_owned()
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
                .then(|| {
                    vars.iter()
                        .find(|v| v.name == dispatchers::variable_name(&p.text))
                })
                .flatten();
            let t = text(p.text).key(format!("p{i}")).font(k.ty.body);
            match value {
                Some(v) => t
                    .font(k.ty.body_medium)
                    .color(k.c.text_accent)
                    .help(v.value.clone()),
                None => t.color(k.c.text_muted),
            }
        }))
}

const WIDTHS: [Option<f32>; 5] = [Some(170.0), Some(110.0), Some(170.0), None, Some(76.0)];

fn bind_row(s: &State, k: &Kit, tab: Tab, i: usize, b: &Keybind, settling: bool) -> El {
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
            command(k, &b.params, &s.keybinds.variables),
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
                    .map(|(i, b)| bind_row(s, k, tab, i, b, settling))
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

fn form_dialog(s: &State, k: &Kit, layers: &mut Vec<El>) {
    let kb = &s.keybinds;
    let Some(mode) = kb.form else { return };
    let lua = s.is_lua();
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
    let picks = dispatchers::suggestions(&kb.dispatcher.text, lua);
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
    let common = "Common: exec, killactive, workspace, togglefloating, fullscreen";
    let dispatcher_hint = if lua {
        format!("{common}. Use lua for any other hl.dsp.* call")
    } else {
        common.to_owned()
    };

    let lua_dispatcher = kb.dispatcher.text.trim() == "lua";
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
    } else if lua {
        "Command arguments. Can use variables like $terminal or ${terminal}"
    } else {
        "Command arguments. Can use variables like $terminal"
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
            Some(dispatcher_hint),
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
