//! Colours from chromasync, rendered from the hyprpaper wallpaper with the user's profile.

use std::fs;
use std::path::Path;
use std::process::Command;

use super::cache::cache_dir;
use super::{Generator, Scheme, failure, installed, to_css, xdg_dir};
use crate::backend::hyprpaper;

/// The chromasync template used when the user's chromasync config doesn't name one.
const CHROMASYNC_TEMPLATE: &str = "materialish";

/// A declarative chromasync target: one file with the four colours the app reads.
const CHROMASYNC_TARGET: &str = r#"name = "hyprconfig"
description = "Hyprconfig's window colours."

[[artifacts]]
file_name = "colors.css"
template = """
--background: {{tokens.bg}};
--foreground: {{tokens.text}};
--primary: {{tokens.accent}};
--destructive: {{tokens.error}};
"""
"#;

/// Template, chroma and contrast from the user's default chromasync sync profile, so the app
/// matches the rest of their desktop.
#[derive(Clone, Debug, PartialEq)]
struct ChromasyncProfile {
    template: String,
    chroma: Option<String>,
    contrast: Option<String>,
}

fn chromasync_profile(config: Option<&str>) -> ChromasyncProfile {
    let profile = config
        .and_then(|c| c.parse::<toml_edit::DocumentMut>().ok())
        .and_then(|doc| {
            let configs = doc.get("configs")?.as_array_of_tables()?;
            configs
                .iter()
                .find(|t| t.get("name").and_then(|n| n.as_str()) == Some("default"))
                .cloned()
        });
    let get = |key: &str| {
        profile
            .as_ref()
            .and_then(|t| t.get(key))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
    };
    ChromasyncProfile {
        template: get("template").unwrap_or_else(|| CHROMASYNC_TEMPLATE.into()),
        chroma: get("chroma"),
        contrast: get("contrast"),
    }
}

/// `--name: value;` lines, as chromasync renders the target above.
fn read_declarations(text: &str) -> Result<Scheme, String> {
    let get = |name: &str| {
        text.lines()
            .filter_map(|l| l.trim().trim_end_matches(';').split_once(':'))
            .find(|(k, _)| k.trim() == name)
            .map(|(_, v)| v.trim().to_owned())
            .ok_or_else(|| format!("its output has no {name}"))
    };
    Scheme {
        bg: get("--background")?,
        fg: get("--foreground")?,
        accent: get("--primary")?,
        danger: get("--destructive")?,
    }
    .check()
}

pub(super) fn chromasync() -> Result<String, String> {
    installed("chromasync")?;
    let wallpaper = hyprpaper::get_hyprpaper_config()
        .ok()
        .and_then(|c| c.wallpapers.last().map(|w| w.path.clone()))
        .filter(|p| !p.is_empty())
        .ok_or("no wallpaper to generate from")?;
    let dir = cache_dir()?.join("chromasync");
    fs::create_dir_all(&dir).map_err(|e| format!("Failed to create cache directory: {e}"))?;
    let target = dir.join("hyprconfig.toml");
    fs::write(&target, CHROMASYNC_TARGET)
        .map_err(|e| format!("Failed to write chromasync target: {e}"))?;
    let config = xdg_dir("XDG_CONFIG_HOME", ".config")
        .and_then(|d| fs::read_to_string(d.join("chromasync/config.toml")).ok());
    let profile = chromasync_profile(config.as_deref());

    // Each mode decodes the wallpaper on its own; run both at once.
    let run = |mode: &'static str| {
        let (wallpaper, target, dir, profile) = (
            wallpaper.clone(),
            target.clone(),
            dir.clone(),
            profile.clone(),
        );
        std::thread::spawn(move || {
            chromasync_mode(&wallpaper, &target, &dir.join(mode), mode, &profile)
        })
    };
    let (dark, light) = (run("dark"), run("light"));
    let join = |h: std::thread::JoinHandle<Result<Scheme, String>>| {
        h.join().map_err(|_| "chromasync crashed".to_string())?
    };
    let (dark, light) = (join(dark)?, join(light)?);
    Ok(to_css(Generator::Chromasync, &dark, &light))
}

fn chromasync_mode(
    wallpaper: &str,
    target: &Path,
    out: &Path,
    mode: &str,
    profile: &ChromasyncProfile,
) -> Result<Scheme, String> {
    let mut cmd = Command::new("chromasync");
    cmd.arg("wallpaper")
        .args(["--image", wallpaper])
        .args(["--template", &profile.template])
        .args(["--mode", mode])
        .arg("--targets")
        .arg(target)
        .arg("--output")
        .arg(out)
        .arg("--force");
    if let Some(c) = &profile.chroma {
        cmd.args(["--chroma", c]);
    }
    if let Some(c) = &profile.contrast {
        cmd.args(["--contrast", c]);
    }
    let output = cmd
        .output()
        .map_err(|e| format!("couldn't run chromasync: {e}"))?;
    if !output.status.success() {
        return Err(failure(&output));
    }
    let text = fs::read_to_string(out.join("colors.css"))
        .map_err(|e| format!("couldn't read its output: {e}"))?;
    read_declarations(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Palettes;

    #[test]
    fn chromasync_output_becomes_both_appearances() {
        let dark = read_declarations(
            "--background: #030101;\n--foreground: #F1E9E5;\n--primary: #B4876B;\n--destructive: #F08B7C;\n",
        )
        .unwrap();
        let light = read_declarations(
            "--background: #FBF7F5;\n--foreground: #1A1311;\n--primary: #8C5A3C;\n--destructive: #B3261E;\n",
        )
        .unwrap();
        let p = Palettes::parse(&to_css(Generator::Chromasync, &dark, &light));
        assert!(p.dark.unwrap().bg.luminance() < 0.05);
        assert!(p.light.unwrap().bg.luminance() > 0.8);
        assert!(read_declarations("--background: #000;").is_err());
        assert!(
            read_declarations(
                "--background: nope;\n--foreground: #fff;\n--primary: #fff;\n--destructive: #fff;"
            )
            .is_err()
        );
    }

    #[test]
    fn chromasync_profile_comes_from_the_default_config() {
        let config = r#"
[[configs]]
name = "work"
template = "brutalist"

[[configs]]
name = "default"
template = "terminal"
chroma = "industrial"
"#;
        let p = chromasync_profile(Some(config));
        assert_eq!(p.template, "terminal");
        assert_eq!(p.chroma.as_deref(), Some("industrial"));
        assert_eq!(p.contrast, None);
        assert_eq!(chromasync_profile(None).template, CHROMASYNC_TEMPLATE);
        assert_eq!(
            chromasync_profile(Some("not toml [")).template,
            CHROMASYNC_TEMPLATE
        );
    }
}
