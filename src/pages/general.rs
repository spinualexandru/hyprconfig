use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use rune::{El, col, icon, row, text};
use rune_kit::{IconButton, IconButtonVariant, Kit, SegmentedControl, Switch};

use crate::app::{self, State};
use crate::backend::palette::{self, Colors, Generator};
use crate::icons;
use crate::jobs::Remote;
use crate::theme::Palettes;
use crate::ui::{self, Card};

/// The `prefs.json` key holding the preferred colour generator.
pub const GENERATOR_PREF: &str = "color_generator";

#[derive(Default)]
pub struct GeneralState {
    /// The colour generator tried first.
    pub generator: Generator,
    /// What the last recolouring used, and why generators were passed over.
    pub colors: Remote<Colors>,
    /// Whether the Noctalia integration (template and hooks) is set up for this app; `None`
    /// while Noctalia isn't running.
    pub integration: Remote<Option<bool>>,
    /// Setting the integration up or taking it out.
    pub integration_busy: bool,
    /// The integration is on, so the file watcher checks Noctalia's rendered palette rather
    /// than its settings.
    pub hooked: Arc<AtomicBool>,
}

pub fn load_integration(s: &mut State) {
    let attempt = s.general.integration.begin();
    s.jobs.run(
        "noctalia-integration",
        palette::noctalia_integration,
        move |s, r, _| {
            if s.general.integration.land(attempt, r) {
                let on = s.general.integration.data == Some(Some(true));
                s.general.hooked.store(on, Ordering::Relaxed);
            }
        },
    );
}

fn set_integration(s: &mut State, on: bool) {
    s.general.integration_busy = true;
    s.jobs.write(
        "noctalia-integration-set",
        move || palette::set_noctalia_integration(on),
        move |s, r, tx| {
            s.general.integration_busy = false;
            match r {
                Ok(()) if on => s
                    .toasts
                    .success(tx, "Hyprconfig now follows Noctalia's colours"),
                Ok(()) => s
                    .toasts
                    .success(tx, "Removed Hyprconfig's template and hooks from Noctalia"),
                Err(e) => app::fail(s, tx, "Couldn't change the Noctalia integration", e),
            }
            load_integration(s);
            // Noctalia renders the template on reload, and its post hook usually reports
            // that; recolour anyway in case it was quicker than the listener.
            app::reload_theme(s);
        },
    );
}

/// Recolours the app from the generators (preferred first), or `theme.css` when neither
/// answers, and caches the result for the next launch.
pub fn refresh_colors(s: &mut State) {
    let attempt = s.general.colors.begin();
    let preferred = s.general.generator;
    s.jobs.run(
        "colors",
        move || palette::resolve(preferred),
        move |s, r, _| {
            if !s.general.colors.land(attempt, r) {
                return;
            }
            let Some(c) = &s.general.colors.data else {
                return;
            };
            s.palettes = Palettes::parse(&c.css);
            let cache = c.used.map(|_| c.css.clone());
            s.jobs.write(
                "colors-cache",
                move || palette::save_cache(cache),
                |_, r, _| {
                    if let Err(e) = r {
                        eprintln!("hyprconfig: {e}");
                    }
                },
            );
        },
    );
}

pub fn choose_generator(s: &mut State, g: Generator) {
    if s.general.generator == g {
        return;
    }
    s.general.generator = g;
    if let Some(p) = s.prefs.as_mut()
        && let Err(e) = p.set(GENERATOR_PREF, &g)
    {
        eprintln!("hyprconfig: couldn't save the colour generator: {e}");
    }
    refresh_colors(s);
}

/// What the app's colours come from right now, in a sentence.
fn status(colors: &Remote<Colors>) -> String {
    match &colors.data {
        _ if colors.loading => "Generating colours…".to_owned(),
        Some(c) => match c.used {
            Some(g) => format!("Using {}", g.name()),
            None => {
                "Neither generator is available; using ~/.config/hyprconfig/theme.css".to_owned()
            }
        },
        None => match &colors.error {
            Some(e) => format!("Couldn't load colours: {e}"),
            None => "Colours from the last run".to_owned(),
        },
    }
}

fn colors_card(s: &State, k: &Kit) -> El {
    let g = &s.general;
    let selected = Generator::ALL.iter().position(|x| *x == g.generator);
    let names: Vec<&str> = Generator::ALL.iter().map(|x| x.name()).collect();
    let picker = SegmentedControl::new(&names, selected)
        .label("Preferred colour generator")
        .fit()
        .on_pick(|s: &mut State, i, _| {
            if let Some(&g) = Generator::ALL.get(i) {
                choose_generator(s, g);
            }
        })
        .build(k)
        .key("generator");
    let refresh = IconButton::new(&icons::REFRESH_CW)
        .variant(IconButtonVariant::Neutral)
        .label("Regenerate colours")
        .help("Regenerate")
        .disabled(g.colors.loading)
        .on_press(refresh_colors)
        .build(k)
        .key("regenerate");

    let used = g.colors.data.as_ref().and_then(|c| c.used);
    let ok = used.is_some() && !g.colors.loading;
    let mut notes = vec![
        row()
            .key("status")
            .w_full()
            .gap(8.0)
            .center_y()
            .child(
                icon(if ok { &icons::CHECK } else { &icons::INFO }, 16.0).color(if ok {
                    k.c.success.icon
                } else {
                    k.c.icon_muted
                }),
            )
            .child(
                text(status(&g.colors))
                    .font(k.ty.body_medium)
                    .color(k.c.text_strong)
                    .wrap()
                    .shrink()
                    .min_w(0.0),
            ),
    ];
    if let Some(c) = g.colors.data.as_ref().filter(|_| !g.colors.loading) {
        notes.extend(c.skipped.iter().map(|(by, why)| {
            text(format!("{}: {why}", by.name()))
                .key(format!("skipped-{}", by.name().to_lowercase()))
                .font(k.ty.body)
                .color(k.c.text_muted)
                .wrap()
                .w_full()
        }));
    }

    Card::new()
        .title("Colours")
        .description("Where Hyprconfig takes its colours from")
        .action(refresh)
        .gap(16.0)
        .child(row().key("generator-row").w_full().child(ui::setting(
            k,
            "generator-setting",
            "Preferred generator",
            "Chromasync renders your wallpaper with your chromasync profile; Noctalia uses its \
                 current colour scheme. When the preferred one isn't available the other is used, \
                 then ~/.config/hyprconfig/theme.css.",
            true,
            picker,
        )))
        .child(col().key("notes").w_full().gap(6.0).children(notes))
        .children(integration_setting(s, k))
        .build(k)
        .key("colors")
}

/// The switch for the Noctalia integration, while Noctalia runs.
fn integration_setting(s: &State, k: &Kit) -> Option<El> {
    let g = &s.general;
    let on = (*g.integration.data.as_ref()?)?;
    let busy = g.integration_busy || g.integration.loading;
    let switch = Switch::new(on)
        .label("Noctalia integration")
        .hide_label()
        .disabled(busy)
        .on_toggle(move |s: &mut State| set_integration(s, !on))
        .build(k)
        .key("switch");
    Some(row().key("noctalia-row").w_full().child(ui::setting(
        k,
        "noctalia",
        "Noctalia integration",
        "Reads every Noctalia colour scheme, builtin and custom ones included, and recolours \
         the moment its colours, light or dark mode or wallpaper change. Adds a template \
         (~/.config/noctalia/hyprconfig.toml) and `hyprconfig notify` to its On Theme Mode \
         Changed and On Wallpaper Changed hooks; your other hooks stay.",
        !busy,
        switch,
    )))
}

pub fn view(s: &State, k: &Kit) -> El {
    ui::page(
        k,
        "general",
        "General",
        "General settings and preferences for Hyprland configuration.",
        Vec::new(),
        vec![colors_card(s, k)],
    )
}
