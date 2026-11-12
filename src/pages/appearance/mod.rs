//! The appearance page: the hyprpaper/Noctalia wallpaper, its decoded preview, and the
//! Matugen preferences and theme jobs.

mod view;

pub use view::view;

use std::path::{Path, PathBuf};

use rune::{FileDialogOptions, Image, Tx};

use crate::app::{self, State};
use crate::backend::config::{self, MatugenPreferences};
use crate::backend::hyprpaper::{self, HyprpaperConfig, Wallpaper};
use crate::backend::os;
use crate::jobs::{self, Remote};

pub const GENERATORS: &[(&str, &str)] = &[
    ("scheme-content", "Scheme Content"),
    ("scheme-expressive", "Scheme Expressive"),
    ("scheme-fidelity", "Scheme Fidelity"),
    ("scheme-fruit-salad", "Scheme Fruit Salad"),
    ("scheme-monochrome", "Scheme Monochrome"),
    ("scheme-neutral", "Scheme Neutral"),
    ("scheme-rainbow", "Scheme Rainbow"),
    ("scheme-tonal-spot", "Scheme Tonal Spot"),
    ("scheme-vibrant", "Scheme Vibrant"),
];

const IMAGE_TYPES: &[&str] = &["jpg", "jpeg", "png", "gif", "webp", "bmp"];

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Preview {
    #[default]
    Pending,
    Ready,
    Failed,
}

#[derive(Default)]
pub struct AppearanceState {
    pub config: Remote<HyprpaperConfig>,
    /// Whether `matugen` is installed; its settings show only then.
    pub matugen: bool,
    pub prefs: MatugenPreferences,
    pub updating: bool,
    pub generator_open: bool,
    /// The current wallpaper's path and decoded image.
    pub image: Option<(String, Image)>,
    pub preview: Preview,
}

/// The wallpaper shown: the last one listed (the most recent).
pub fn current(config: &HyprpaperConfig) -> Option<&Wallpaper> {
    config.wallpapers.last()
}

pub fn load(s: &mut State) {
    load_config(s);
    s.jobs.run(
        "matugen-exists",
        || Ok(os::tool_exists("matugen")),
        |s, exists: Result<bool, String>, _| s.appearance.matugen = exists.unwrap_or(false),
    );
    s.jobs
        .run("preferences", config::get_preferences, |s, r, _| {
            if let Ok(p) = r {
                s.appearance.prefs = p.matugen;
            }
        });
}

fn load_config(s: &mut State) {
    jobs::load_then(
        s,
        "hyprpaper",
        |s| &mut s.appearance.config,
        hyprpaper::get_hyprpaper_config,
        show_current,
    );
}

/// Starts decoding the current wallpaper unless it already is.
fn show_current(s: &mut State) {
    let a = &mut s.appearance;
    let path = a
        .config
        .data
        .as_ref()
        .and_then(current)
        .map(|w| w.path.clone());
    match (path, &s.images) {
        (Some(p), _) if a.image.as_ref().is_some_and(|(q, _)| *q == p) => {}
        (Some(p), Some(cache)) => {
            // Raster formats decode at their own size; the size only applies to SVG.
            let img = cache.file(&p, 1200.0, 2.0);
            a.image = Some((p, img));
            a.preview = Preview::Pending;
        }
        _ => {
            a.image = None;
            a.preview = Preview::Pending;
        }
    }
}

/// True when the preview finished decoding (or failed) since the last frame.
pub fn watch_image(s: &mut State) -> bool {
    let Some((_, img)) = &s.appearance.image else {
        return false;
    };
    let now = if img.error().is_some() {
        Preview::Failed
    } else if img.size().is_some() {
        Preview::Ready
    } else {
        Preview::Pending
    };
    let changed = now != s.appearance.preview;
    s.appearance.preview = now;
    changed
}

fn save_prefs(s: &mut State) {
    let p = s.appearance.prefs.clone();
    s.jobs.write(
        "save-preferences",
        move || config::update_matugen_preferences(p.enable, p.light_mode, p.generator_type),
        |s, r, tx| {
            if let Err(e) = r {
                app::fail(s, tx, "Failed to save preference", e);
            }
        },
    );
}

fn choose_wallpaper(s: &mut State, tx: &mut Tx) {
    let directory = s
        .appearance
        .config
        .data
        .as_ref()
        .and_then(current)
        .and_then(|w| Path::new(&w.path).parent().map(Path::to_path_buf));
    let options = FileDialogOptions {
        title: "Choose a wallpaper".into(),
        directory,
        ..Default::default()
    }
    .filter("Images", IMAGE_TYPES);
    tx.open_file(options, |s: &mut State, result, tx| match result {
        Ok(Some(path)) => set_wallpaper(s, path),
        Ok(None) => {}
        Err(e) => app::fail(s, tx, "Couldn't open the file chooser", e.to_string()),
    });
}

/// Which daemon didn't show a saved wallpaper (and why), and matugen's answer when it ran.
type WallpaperSet = (Option<String>, Option<Result<(), String>>);

/// Hyprpaper and/or Noctalia show `path` on every monitor; matugen recolours when enabled.
pub fn set_wallpaper(s: &mut State, path: PathBuf) {
    let path = path.to_string_lossy().into_owned();
    let p = s.appearance.prefs.clone();
    let matugen = (s.appearance.matugen && p.enable).then_some((p.light_mode, p.generator_type));
    s.appearance.updating = true;
    s.jobs.write(
        "set-wallpaper",
        move || -> Result<WallpaperSet, String> {
            let not_shown = hyprpaper::replace_wallpaper(String::new(), path.clone(), None)?;
            let synced = matugen.map(|(light, generator)| os::run_matugen(path, light, generator));
            Ok((not_shown, synced))
        },
        |s, r, tx| {
            s.appearance.updating = false;
            match r {
                Err(e) => app::fail(s, tx, "Failed to update wallpaper", e),
                Ok((not_shown, synced)) => {
                    // The generators work from the wallpaper, and matugen may have rewritten
                    // theme.css: recolour either way.
                    crate::pages::general::refresh_colors(s);
                    let mut problems = Vec::new();
                    if let Some(e) = not_shown {
                        problems.push(e);
                    }
                    if let Some(Err(e)) = &synced {
                        problems.push(format!("Matugen failed: {e}"));
                    }
                    if !problems.is_empty() {
                        s.toasts
                            .warning(tx, format!("Wallpaper saved, but {}", problems.join("; ")));
                    } else if synced.is_some() {
                        s.toasts.success(tx, "Wallpaper updated and Matugen synced");
                    } else {
                        s.toasts.success(tx, "Wallpaper updated successfully");
                    }
                }
            }
            load_config(s);
        },
    );
}

fn setup_template(s: &mut State) {
    s.jobs.write(
        "matugen-template",
        config::ensure_matugen_template,
        |s, r, tx| match r {
            Ok(path) => s.toasts.success(tx, format!("Template created at {path}")),
            Err(e) => app::fail(s, tx, "Failed to create template", e),
        },
    );
}

fn regenerate_theme(s: &mut State) {
    let wallpaper = s
        .appearance
        .config
        .data
        .as_ref()
        .and_then(current)
        .map(|w| w.path.clone());
    let p = s.appearance.prefs.clone();
    s.jobs.write(
        "regenerate-theme",
        move || match wallpaper {
            Some(path) => os::run_matugen(path, p.light_mode, p.generator_type),
            None => Ok(()),
        },
        |s, r, tx| match r {
            Ok(()) => {
                crate::pages::general::refresh_colors(s);
                s.toasts.success(tx, "Theme regenerated and reloaded");
            }
            Err(e) => app::fail(s, tx, "Failed to reload theme", e),
        },
    );
}
