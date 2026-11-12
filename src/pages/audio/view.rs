//! The audio page view: device and stream cards with mute buttons and volume sliders.

use rune::{El, col, el, row, text};
use rune_kit::{Button, ButtonVariant, IconButton, IconButtonVariant, Kit};

use super::*;
use crate::backend::audio::{AudioDevice, AudioStream};
use crate::icons;
use crate::ui::{self, Tone};

fn volume_row(
    k: &Kit,
    id: u32,
    volume: f32,
    muted: bool,
    glyphs: (&'static rune::Icon, &'static rune::Icon),
    what: &str,
) -> El {
    let percent = (volume * 100.0).round() as i32;
    let (on, off) = glyphs;
    let mute = IconButton::new(if muted { off } else { on })
        .variant(if muted {
            IconButtonVariant::Neutral
        } else {
            IconButtonVariant::Ghost
        })
        .label(if muted {
            format!("Unmute {what}")
        } else {
            format!("Mute {what}")
        })
        .on_press(move |s: &mut State| toggle_mute(s, id))
        .build(k)
        .key("mute");
    row()
        .key("volume")
        .w_full()
        .gap(12.0)
        .center_y()
        .child(mute)
        .child(ui::slider(
            k,
            "slider",
            &format!("{what} volume"),
            volume,
            MAX_VOLUME,
            0.01,
            muted,
            move |s, v, tx| set_volume(s, id, v, tx),
        ))
        .child(
            text(format!("{percent}%"))
                .key("percent")
                .w(48.0)
                .align(rune::Align::End)
                .font(ui::mono(k))
                .color(if percent > 100 {
                    k.c.warning.icon
                } else {
                    k.c.text_muted
                }),
        )
}

fn device_card(k: &Kit, d: &AudioDevice) -> El {
    let sink = d.device_type == "sink";
    let glyphs = if sink {
        (&icons::VOLUME_2, &icons::VOLUME_OFF)
    } else {
        (&icons::MIC, &icons::MIC_OFF)
    };
    let mut title = row().key("title").w_full().gap(8.0).center_y().child(
        text(d.description.clone())
            .font(ui::card_title_style())
            .color(k.c.text_strong)
            .truncate()
            .shrink(),
    );
    if d.is_default {
        title = title.child(ui::pill(k, "Default", Tone::Accent));
    }
    let mut card = ui::surface(k)
        .key(format!("device-{}", d.id))
        .gap(12.0)
        .child(
            row()
                .w_full()
                .gap(12.0)
                .items_start()
                .child(if d.muted {
                    ui::icon_tile_colored(k, glyphs.1, k.c.icon_muted)
                } else {
                    ui::icon_tile(k, glyphs.0)
                })
                .child(
                    col()
                        .grow(1.0)
                        .shrink()
                        .min_w(0.0)
                        .gap(4.0)
                        .child(title)
                        .child(
                            text(if sink {
                                "Output Device"
                            } else {
                                "Input Device"
                            })
                            .font(k.ty.body)
                            .color(k.c.text_muted),
                        )
                        .child(el().h(4.0))
                        .child(volume_row(
                            k,
                            d.id,
                            d.volume,
                            d.muted,
                            glyphs,
                            &d.description,
                        )),
                ),
        );
    if d.is_default {
        card = card.border(1.0, k.c.bg_accent);
    } else {
        let id = d.id;
        card = card
            .child(el().w_full().h(1.0).fill(k.c.border_weak).a11y_hidden())
            .child(
                Button::new("Set as Default")
                    .icon(&icons::SPEAKER)
                    .variant(ButtonVariant::Outline)
                    .on_press(move |s: &mut State| set_default(s, id))
                    .build(k)
                    .key("default")
                    .w_full(),
            );
    }
    card
}

fn stream_card(k: &Kit, st: &AudioStream) -> El {
    let mut titles = col().grow(1.0).shrink().min_w(0.0).gap(4.0).child(
        text(st.app_name.clone())
            .key("app")
            .font(ui::card_title_style())
            .color(k.c.text_strong)
            .truncate(),
    );
    if let Some(m) = &st.media_name {
        titles = titles.child(
            text(m.clone())
                .key("media")
                .font(k.ty.body)
                .color(k.c.text_muted)
                .truncate(),
        );
    }
    titles = titles.child(el().h(4.0)).child(volume_row(
        k,
        st.id,
        st.volume,
        st.muted,
        (&icons::VOLUME_2, &icons::VOLUME_OFF),
        &st.app_name,
    ));
    ui::surface(k).key(format!("stream-{}", st.id)).child(
        row()
            .w_full()
            .gap(12.0)
            .items_start()
            .child(ui::icon_tile(k, &icons::MUSIC))
            .child(titles),
    )
}

fn skeletons(k: &Kit, n: usize) -> Vec<El> {
    (0..n)
        .map(|i| {
            ui::surface(k).key(format!("sk{i}")).child(
                row()
                    .w_full()
                    .gap(12.0)
                    .child(ui::skeleton(k, Some(40.0), 40.0))
                    .child(
                        col()
                            .grow(1.0)
                            .gap(10.0)
                            .child(ui::skeleton(k, Some(200.0), 16.0))
                            .child(ui::skeleton(k, Some(110.0), 14.0))
                            .child(ui::skeleton(k, None, 18.0)),
                    ),
            )
        })
        .collect()
}

pub fn view(s: &State, k: &Kit) -> El {
    let r = &s.audio.mixer;
    let refresh = ui::refresh_button(r.loading, |s, _| load(s))
        .build(k)
        .key("refresh");
    let mut body = Vec::new();
    if let Some(e) = &r.error {
        let mut card = ui::Card::new()
            .title("Error Loading Audio Information")
            .border(k.c.danger.icon)
            .gap(8.0)
            .child(ui::value_text(k, e.clone()));
        if e.contains("wpctl not found") {
            card = card.child(
                ui::value_text(
                    k,
                    "Please install WirePlumber to manage PipeWire audio devices.",
                )
                .key("hint"),
            );
        }
        body.push(card.build(k).key("error"));
    }
    let skeleton = r.first_load() && r.error.is_none();
    let quiet = !r.loading && r.error.is_none();
    let devices = |list: &[AudioDevice], empty: &str| -> Vec<El> {
        if skeleton {
            skeletons(k, 1)
        } else if !list.is_empty() {
            list.iter().map(|d| device_card(k, d)).collect()
        } else if quiet {
            vec![ui::note_card(k, "empty", &[empty])]
        } else {
            Vec::new()
        }
    };
    let m = r.data.as_ref();
    body.push(ui::section(
        k,
        "outputs",
        Some(&icons::VOLUME_2),
        "Output Devices",
        None,
        devices(m.map_or(&[], |m| &m.sinks), "No output devices detected."),
    ));
    body.push(ui::section(
        k,
        "inputs",
        Some(&icons::MIC),
        "Input Devices",
        None,
        devices(m.map_or(&[], |m| &m.sources), "No input devices detected."),
    ));
    if skeleton {
        body.push(ui::section(
            k,
            "streams",
            None,
            "Application Audio",
            None,
            skeletons(k, 2),
        ));
    } else if let Some(m) = m {
        if !m.streams.is_empty() {
            body.push(ui::section(
                k,
                "streams",
                None,
                "Application Audio",
                None,
                m.streams.iter().map(|st| stream_card(k, st)).collect(),
            ));
        } else if r.error.is_none() && (!m.sinks.is_empty() || !m.sources.is_empty()) {
            body.push(ui::section(
                k,
                "streams",
                None,
                "Application Audio",
                None,
                vec![ui::note_card(
                    k,
                    "none",
                    &["No applications are currently playing audio."],
                )],
            ));
        }
    }
    ui::page(
        k,
        "audio",
        "Audio",
        "Manage audio devices and application volumes",
        vec![refresh],
        body,
    )
}
