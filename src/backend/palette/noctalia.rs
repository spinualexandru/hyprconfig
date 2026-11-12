//! Colours from Noctalia's current scheme: its wallpaper generator or a community palette.

use std::fs;
use std::process::Command;

use serde_json::Value;

use super::integration::{noctalia_rendered, noctalia_running};
use super::{Generator, Scheme, failure, to_css, xdg_dir};
use crate::backend::{self, hyprpaper};
use crate::theme;

/// `noctalia msg color-scheme-get`: `<source> <name>`, the name possibly with spaces.
fn parse_scheme(reply: &str) -> Option<(String, String)> {
    let (source, name) = reply.trim().split_once(char::is_whitespace)?;
    Some((source.to_owned(), name.trim().to_owned()))
}

pub(super) fn noctalia() -> Result<String, String> {
    let reply = noctalia_running()?;
    // With the integration on, Noctalia renders whatever palette it shows.
    if let Some(css) = noctalia_rendered() {
        return Ok(css);
    }
    let (source, name) =
        parse_scheme(&reply).ok_or_else(|| format!("unexpected colour scheme \"{reply}\""))?;
    match source.as_str() {
        "wallpaper" => noctalia_wallpaper(&name),
        "community" => noctalia_community(&name),
        _ => Err(format!("{source} scheme \"{name}\" needs the integration")),
    }
}

/// Runs Noctalia's own generator on its wallpaper, with its scheme.
fn noctalia_wallpaper(scheme: &str) -> Result<String, String> {
    let wallpaper = backend::noctalia::noctalia_msg(&["wallpaper-get"])
        .ok()
        .and_then(|w| hyprpaper::parse_noctalia_wallpaper_output(&w))
        .ok_or("has no wallpaper to generate from")?;
    let pure_black = backend::noctalia::noctalia_msg(&["config-get", "theme.pure_black_dark"])
        .is_ok_and(|v| backend::noctalia::toml_scalar(&v) == "true");
    let mut cmd = Command::new("noctalia");
    cmd.arg("theme")
        .arg(&wallpaper)
        .args(["--both", "--scheme", scheme]);
    if pure_black {
        cmd.arg("--pure-black");
    }
    let output = cmd
        .output()
        .map_err(|e| format!("couldn't run noctalia theme: {e}"))?;
    if !output.status.success() {
        return Err(failure(&output));
    }
    read_noctalia_theme(&String::from_utf8_lossy(&output.stdout))
}

/// `noctalia theme --both`: Material token maps under `dark` and `light`.
fn read_noctalia_theme(json: &str) -> Result<String, String> {
    let v: Value = serde_json::from_str(json).map_err(|e| format!("unreadable palette: {e}"))?;
    let mode = |m: &str| -> Result<Scheme, String> {
        let get = |k: &str| {
            v[m][k]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("its {m} palette has no {k}"))
        };
        Scheme {
            bg: get("surface")?,
            fg: get("on_surface")?,
            accent: get("primary")?,
            danger: get("error")?,
        }
        .check()
    };
    Ok(to_css(Generator::Noctalia, &mode("dark")?, &mode("light")?))
}

fn noctalia_community(name: &str) -> Result<String, String> {
    let path = xdg_dir("XDG_STATE_HOME", ".local/state")
        .ok_or("no home directory")?
        .join("noctalia/community-palettes/.catalog/palettes.json");
    let json = fs::read_to_string(&path)
        .map_err(|e| format!("couldn't read its community palettes: {e}"))?;
    read_community_palette(&json, name)
}

/// A palette from Noctalia's community catalog. Those list no text colour, so it's made from
/// the surface: near-white on dark, near-black on light.
fn read_community_palette(json: &str, name: &str) -> Result<String, String> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| format!("unreadable palette catalog: {e}"))?;
    let palette = v
        .as_array()
        .and_then(|all| all.iter().find(|p| p["name"].as_str() == Some(name)))
        .ok_or_else(|| format!("community palette \"{name}\" isn't downloaded"))?;
    let mode = |m: &str| -> Result<Scheme, String> {
        let get = |k: &str| {
            palette[m][k]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("its {m} palette has no {k}"))
        };
        let bg = get("surface")?;
        let surface = theme::color(&bg).ok_or_else(|| format!("unreadable colour \"{bg}\""))?;
        // Near-white or near-black in OKLab, keeping a little of the surface's hue.
        let [_, a, b, alpha] = surface.to_oklab();
        let lightness = if m == "dark" { 0.93 } else { 0.22 };
        let fg = rune::Color::from_oklab([lightness, a * 0.3, b * 0.3, alpha]);
        Scheme {
            bg,
            fg: fg.hex(),
            accent: get("primary")?,
            danger: get("error")?,
        }
        .check()
    };
    Ok(to_css(Generator::Noctalia, &mode("dark")?, &mode("light")?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Palettes;

    #[test]
    fn noctalia_scheme_reply() {
        assert_eq!(
            parse_scheme("wallpaper m3-tonal-spot\n"),
            Some(("wallpaper".into(), "m3-tonal-spot".into()))
        );
        assert_eq!(
            parse_scheme("community Catppuccin Mocha Blue"),
            Some(("community".into(), "Catppuccin Mocha Blue".into()))
        );
        assert_eq!(parse_scheme("builtin"), None);
    }

    #[test]
    fn noctalia_theme_json() {
        let json = r##"{
  "dark": {"surface": "#1a1110", "on_surface": "#f1dfdc", "primary": "#ffb4a8", "error": "#ffb4ab"},
  "light": {"surface": "#fff8f6", "on_surface": "#231918", "primary": "#904b40", "error": "#ba1a1a"}
}"##;
        let p = Palettes::parse(&read_noctalia_theme(json).unwrap());
        assert_eq!(p.dark.unwrap().accent, theme::color("#ffb4a8").unwrap());
        assert_eq!(p.light.unwrap().fg, theme::color("#231918").unwrap());
        assert!(read_noctalia_theme(r#"{"dark": {}}"#).is_err());
    }

    #[test]
    fn noctalia_community_palette() {
        let json = r##"[{"name": "ADW",
  "dark": {"primary": "#3584e4", "error": "#c01c28", "surface": "#242424"},
  "light": {"primary": "#3584e4", "error": "#e01b24", "surface": "#fafafa"}}]"##;
        let p = Palettes::parse(&read_community_palette(json, "ADW").unwrap());
        let (dark, light) = (p.dark.unwrap(), p.light.unwrap());
        assert!(dark.fg.luminance() > 0.7, "light text on the dark surface");
        assert!(light.fg.luminance() < 0.1, "dark text on the light surface");
        assert!(read_community_palette(json, "Nord").is_err());
    }
}
