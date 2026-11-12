//! The variables page view: the table of rows, inline editor and add dialog.

use rune::{Color, El, Key, KeyEvent, Tx, col, row, text};
use rune_kit::{Button, ButtonVariant, Dialog, IconButton, IconButtonVariant, Kit, TextInput};

use super::*;
use crate::app::State;
use crate::color;
use crate::icons;
use crate::ui::{self, Card};

fn on_escape(kind: Kind) -> impl Fn(&mut State, &KeyEvent, &mut Tx) -> bool {
    move |s, e, _| {
        if e.key == Key::Escape {
            cancel_edit(s, kind);
            true
        } else {
            false
        }
    }
}

/// A small rounded square filled with `c`, outlined so it shows on any background.
pub(super) fn swatch(k: &Kit, c: Color) -> El {
    row()
        .w(14.0)
        .h(14.0)
        .shrink()
        .radius(4.0)
        .fill(c)
        .border(1.0, k.c.border_strong)
}

const WIDTHS: [Option<f32>; 3] = [Some(240.0), None, Some(56.0)];

fn entry_row(s: &State, k: &Kit, kind: Kind, e: &Entry, layers: &mut Vec<El>) -> El {
    let v = of_ref(s, kind);
    let settling = v.list.settling();
    let id = e.id(kind);
    let editing = v.editing.as_deref() == Some(id.as_str());
    let shown_name = match kind {
        Kind::Variables => format!("${}", e.name),
        Kind::Env => e.name.clone(),
    };
    let name = ui::code(k, shown_name);
    let value = if editing {
        row()
            .key("editor")
            .w_full()
            .gap(4.0)
            .center_y()
            .children(
                color::value_color(&v.edit_value.text)
                    .map(|c| swatch(k, c).key("swatch").mr(4.0).a11y_hidden()),
            )
            .child(
                TextInput::new(&v.edit_value)
                    .label(format!("Value of {}", e.name))
                    .text_style(ui::mono(k))
                    .width(None)
                    .disabled(v.edit_busy)
                    .on_edit(edit_access(kind))
                    .on_submit(move |s: &mut State, _| save_edit(s, kind))
                    .on_key(on_escape(kind))
                    .build(k)
                    .key("input")
                    .w(0.0)
                    .grow(1.0),
            )
            .child(
                ui::icon_button(&icons::CHECK, "Save")
                    .disabled(v.edit_busy)
                    .on_press(move |s: &mut State| save_edit(s, kind))
                    .build(k)
                    .key("save"),
            )
            .child(
                ui::icon_button(&icons::X, "Cancel")
                    .disabled(v.edit_busy)
                    .on_press(move |s: &mut State| cancel_edit(s, kind))
                    .build(k)
                    .key("cancel"),
            )
    } else {
        let (start_id, start_value) = (id.clone(), e.value.clone());
        // While its colour is being picked, the row previews the value it would write.
        let picking = v.picker.as_ref().filter(|p| p.id == id);
        let shown = picking.map_or_else(|| e.value.clone(), Picker::value);
        let code = ui::code(
            k,
            if shown.is_empty() {
                " ".to_owned()
            } else {
                shown
            },
        )
        .key("value")
        .role("button")
        .label(format!("Edit {}", e.name))
        .help("Click to edit")
        .focusable(true)
        .hover(k.c.hover)
        .press(k.c.pressed)
        .on_press(move |s: &mut State| start_edit(s, kind, start_id.clone(), &start_value));
        let c = match picking {
            Some(p) => Some(p.rgba().color()),
            None => color::value_color(&e.value),
        };
        match c {
            Some(c) => row()
                .key("value-cell")
                .gap(8.0)
                .center_y()
                .child(swatch_button(s, k, kind, e, c, layers))
                .child(code),
            None => code,
        }
    };
    let actions = if editing {
        rune::el()
    } else {
        let entry = e.clone();
        IconButton::new(&icons::TRASH_2)
            .variant(IconButtonVariant::Ghost)
            .label(format!("Delete {}", e.name))
            .help("Delete")
            .disabled(settling)
            .on_press(move |s: &mut State| delete(s, kind, entry.clone()))
            .build(k)
            .key("delete")
    };
    ui::table_row(k, format!("row-{id}"), &WIDTHS, vec![name, value, actions])
}

fn add_dialog(s: &State, k: &Kit, kind: Kind, layers: &mut Vec<El>) {
    let v = of_ref(s, kind);
    if !v.add_open {
        return;
    }
    let c = texts(kind);
    let label = |t: &str| {
        text(t.to_owned())
            .font(k.ty.body_medium)
            .color(k.c.text_strong)
    };
    let mut fields = col()
        .key("fields")
        .w_full()
        .gap(16.0)
        .child(
            col()
                .key("name-field")
                .w_full()
                .gap(6.0)
                .child(label("Variable Name"))
                .child(
                    TextInput::new(&v.add_name)
                        .label("Variable Name")
                        .placeholder(c.name_placeholder)
                        .width(None)
                        .disabled(v.add_busy)
                        .invalid(v.add_error.is_some())
                        .on_edit(name_access(kind))
                        .on_change(move |s: &mut State, _| of(s, kind).add_error = None)
                        .on_submit(move |s: &mut State, _| submit_add(s, kind))
                        .build(k)
                        .key("name"),
                )
                .child(text(c.name_hint).font(k.ty.small).color(k.c.text_muted)),
        )
        .child(
            col()
                .key("value-field")
                .w_full()
                .gap(6.0)
                .child(label("Value"))
                .child(
                    TextInput::new(&v.add_value)
                        .label("Value")
                        .placeholder(c.value_placeholder)
                        .width(None)
                        .disabled(v.add_busy)
                        .on_edit(value_access(kind))
                        .on_submit(move |s: &mut State, _| submit_add(s, kind))
                        .build(k)
                        .key("value"),
                ),
        );
    if let Some(e) = &v.add_error {
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
            .disabled(v.add_busy)
            .on_press(move |s: &mut State| close_add(s, kind))
            .build(k)
            .key("cancel"),
        Button::new(if v.add_busy { "Adding…" } else { c.submit })
            .variant(ButtonVariant::Contrast)
            .disabled(v.add_busy)
            .on_press(move |s: &mut State| submit_add(s, kind))
            .build(k)
            .key("submit"),
    ];
    Dialog::new(format!("add-{}", c.key), c.dialog)
        .description(c.dialog_description)
        .open(true)
        .fit()
        .on_close(move |s: &mut State| close_add(s, kind))
        .child(ui::dialog_body(fields, buttons))
        .build(k, layers);
}

pub fn view(s: &State, k: &Kit, layers: &mut Vec<El>, kind: Kind) -> El {
    let v = of_ref(s, kind);
    let c = texts(kind);
    let add = Button::new(c.add)
        .icon(&icons::PLUS)
        .variant(ButtonVariant::Contrast)
        .on_press(move |s: &mut State| open_add(s, kind))
        .build(k)
        .key("add");
    let refresh = ui::refresh_button(v.list.loading, move |s, _| load(s, kind))
        .build(k)
        .key("refresh");
    let mut body = Vec::new();
    // A failed reload shows above the rows from before.
    if let Some(e) = &v.list.error {
        body.push(ui::error_card(k, "error", c.error, e));
    }
    match v.list.data.as_deref() {
        None if v.list.first_load() => body.push(
            Card::new()
                .icon(c.icon)
                .title(c.card)
                .description(c.card_description)
                .child(ui::skeleton_rows(k, 6, &[Some(128.0), None]))
                .build(k)
                .key("loading"),
        ),
        None => {}
        Some([]) => body.push(ui::note_card(k, "empty", &[c.empty])),
        Some(list) => body.push(
            Card::new()
                .icon(c.icon)
                .title(format!("{} ({})", c.card, list.len()))
                .description(c.card_description)
                .child(ui::table(
                    k,
                    "table",
                    &[("Name", WIDTHS[0]), ("Value", WIDTHS[1]), ("", WIDTHS[2])],
                    list.iter()
                        .map(|e| entry_row(s, k, kind, e, layers))
                        .collect(),
                ))
                .build(k)
                .key("list"),
        ),
    }
    add_dialog(s, k, kind, layers);
    ui::page(k, c.key, c.title, c.subtitle, vec![add, refresh], body)
}
