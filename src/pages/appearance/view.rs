//! The appearance page view: the wallpaper card with its preview and the Matugen card.

use rune::{Cx, El, SurfaceFit, col, el, icon, row, text};
use rune_kit::{
    Button, ButtonVariant, IconButton, IconButtonVariant, Kit, Select, SelectAppearance, Switch,
};

use super::*;
use crate::app::State;
use crate::icons;
use crate::ui::{self, Card};

/// The wallpaper at the shape of its screen, filled as the desktop fills it, so resizing the
/// window scales the preview instead of changing its crop.
fn preview(s: &State, k: &Kit, w: &Wallpaper) -> El {
    let fit = match w.fit_mode.as_str() {
        "contain" | "fit" => SurfaceFit::Fit,
        _ => SurfaceFit::Fill,
    };
    let frame = el()
        .key("preview")
        .aspect_fit(screen_aspect(&s.appearance, w))
        .max_h(PREVIEW_MAX_H)
        .self_center()
        .radius(10.0)
        .fill(k.c.layer_02)
        .label("Current wallpaper");
    match (&s.appearance.image, s.appearance.preview) {
        (Some(_), Preview::Failed) | (None, _) => frame.center().child(
            col()
                .items_center()
                .gap(8.0)
                .child(icon(&icons::IMAGE, 48.0).color(k.c.icon_muted))
                .child(
                    text("Failed to load image")
                        .font(k.ty.body)
                        .color(k.c.text_muted),
                ),
        ),
        (Some((_, img)), _) => frame.pixels_fit(img.pixels(), fit),
    }
}

/// Tallest the preview grows in a wide window, so the details under it stay in view.
const PREVIEW_MAX_H: f32 = 600.0;

fn wallpaper_card(s: &State, k: &Kit) -> El {
    let a = &s.appearance;
    let refresh = IconButton::new(&icons::REFRESH_CW)
        .variant(IconButtonVariant::Neutral)
        .label("Reload wallpaper configuration")
        .help("Refresh")
        .disabled(a.config.loading)
        .on_press(load_config)
        .build(k)
        .key("refresh");
    let change = Button::new(if a.updating {
        "Updating…"
    } else {
        "Change Wallpaper"
    })
    .icon(&icons::UPLOAD)
    .variant(ButtonVariant::Contrast)
    .disabled(a.updating || a.config.loading)
    .on_press_tx(choose_wallpaper)
    .build(k)
    .key("change");
    let mut card = Card::new()
        .title("Wallpaper")
        .description(match a.config.data.as_ref().and_then(|c| c.source) {
            Some(daemon) => format!("Current wallpaper, shown by {}", daemon.name()),
            None => "Current wallpaper from hyprpaper or Noctalia".to_owned(),
        })
        .action(refresh)
        .action(change);
    if let Some(e) = &a.config.error {
        card = card.child(
            text(format!("Failed to load wallpaper configuration: {e}"))
                .key("error")
                .font(k.ty.body)
                .color(k.c.danger.icon)
                .wrap()
                .w_full(),
        );
    }
    let body =
        match a.config.data.as_ref().and_then(current) {
            _ if a.config.first_load() => col()
                .key("loading")
                .w_full()
                .gap(16.0)
                .child(ui::skeleton(k, None, 400.0).w_full())
                .child(ui::skeleton(k, Some(280.0), 16.0)),
            Some(w) => col()
                .key("current")
                .w_full()
                .gap(16.0)
                .child(preview(s, k, w))
                .child(ui::field(
                    k,
                    "path",
                    "Path",
                    text(w.path.clone())
                        .key("value")
                        .font(ui::mono(k))
                        .color(k.c.text_muted)
                        .wrap_anywhere()
                        .w_full()
                        .selectable(true),
                ))
                .child(ui::field(
                    k,
                    "monitor",
                    "Monitor",
                    ui::value_text(
                        k,
                        if w.monitor.is_empty() {
                            "All monitors".to_owned()
                        } else {
                            w.monitor.clone()
                        },
                    ),
                ))
                .children((!w.fit_mode.is_empty()).then(|| {
                    ui::field(k, "fit", "Fit Mode", ui::value_text(k, w.fit_mode.clone()))
                })),
            None => col()
                .key("none")
                .w_full()
                .h(400.0)
                .center()
                .gap(8.0)
                .child(icon(&icons::IMAGE, 56.0).color(k.c.icon_muted))
                .child(
                    text("No wallpaper configured")
                        .font(ui::card_title_style())
                        .color(k.c.text_strong),
                )
                .child(
                    text("Click \"Change Wallpaper\" to set one")
                        .font(k.ty.body)
                        .color(k.c.text_muted),
                ),
        };
    card.child(body).build(k).key("wallpaper")
}

fn matugen_card(s: &State, k: &Kit, layers: &mut Vec<El>) -> El {
    let p = &s.appearance.prefs;
    let on = p.enable;
    let enable = Switch::new(on)
        .label("Enable Matugen")
        .hide_label()
        .on_toggle(|s: &mut State| {
            s.appearance.prefs.enable = !s.appearance.prefs.enable;
            save_prefs(s);
        })
        .build(k)
        .key("switch");
    let light = Switch::new(p.light_mode)
        .label("Light Mode")
        .hide_label()
        .disabled(!on)
        .on_toggle(|s: &mut State| {
            s.appearance.prefs.light_mode = !s.appearance.prefs.light_mode;
            save_prefs(s);
        })
        .build(k)
        .key("switch");
    let labels: Vec<&str> = GENERATORS.iter().map(|(_, l)| *l).collect();
    let chosen = GENERATORS
        .iter()
        .find(|(id, _)| *id == p.generator_type)
        .map(|(_, l)| *l);
    let generator = Select::new("generator", &labels)
        .label("Generator Type")
        .placeholder("Select generator type")
        .current(chosen)
        .open(s.appearance.generator_open && on)
        .disabled(!on)
        .appearance(SelectAppearance::Field)
        .width(280.0)
        .on_toggle(|s: &mut State| s.appearance.generator_open = !s.appearance.generator_open)
        .on_close(|s: &mut State| s.appearance.generator_open = false)
        .on_choose(|s: &mut State, label: &str, _| {
            s.appearance.generator_open = false;
            if let Some((id, _)) = GENERATORS.iter().find(|(_, l)| *l == label) {
                s.appearance.prefs.generator_type = (*id).to_owned();
                save_prefs(s);
            }
        })
        .build(k, layers);
    let has_wallpaper = s
        .appearance
        .config
        .data
        .as_ref()
        .and_then(current)
        .is_some();
    Card::new()
        .title("General")
        .description("Appearance settings and theme synchronization")
        .gap(20.0)
        .child(text("Matugen").key("matugen-title").role("heading").font(ui::card_title_style()).color(k.c.text_strong))
        .child(
            row()
                .key("switches")
                .w_full()
                .gap(32.0)
                .child(ui::setting(k, "enable", "Enable", "Automatically generate color scheme from wallpaper", true, enable))
                .child(ui::setting(k, "light", "Light Mode", "Use light color scheme instead of dark", on, light)),
        )
        .child(
            col()
                .key("generator-field")
                .gap(6.0)
                .opacity(if on { 1.0 } else { 0.5 })
                .child(text("Generator Type").font(k.ty.body_medium).color(k.c.text_strong))
                .child(generator)
                .child(text("Color scheme generation algorithm").font(k.ty.body).color(k.c.text_muted)),
        )
        .child(text("Theme").key("theme-title").role("heading").font(ui::card_title_style()).color(k.c.text_strong))
        .child(
            row()
                .key("theme-buttons")
                .gap(12.0)
                .child(Button::new("Setup Template").icon(&icons::FILE_CODE).variant(ButtonVariant::Outline).on_press(setup_template).build(k).key("template"))
                .child(
                    Button::new("Reload Theme")
                        .icon(&icons::PALETTE)
                        .variant(ButtonVariant::Outline)
                        .disabled(!has_wallpaper)
                        .on_press(regenerate_theme)
                        .build(k)
                        .key("reload-theme"),
                ),
        )
        .child(
            text("Setup Template creates the matugen template at ~/.config/matugen/templates/hyprconfig.css. Reload Theme runs matugen on the current wallpaper and reloads the colors.")
                .key("theme-help")
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .w_full(),
        )
        .build(k)
        .key("matugen")
}

pub fn view(s: &State, k: &Kit, _cx: &mut Cx<State>, layers: &mut Vec<El>) -> El {
    let mut body = vec![wallpaper_card(s, k)];
    if s.appearance.matugen {
        body.push(matugen_card(s, k, layers));
    }
    ui::page(
        k,
        "appearance",
        "Appearance",
        "Customize your wallpaper and appearance settings",
        Vec::new(),
        body,
    )
}
