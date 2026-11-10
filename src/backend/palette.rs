//! The app's colours from a colour scheme generator: chromasync (the default) or Noctalia 5.
//! The preferred one is tried first, then the other, then `theme.css` (written by matugen or
//! left at its default). Either generator's answer becomes a small CSS document in the
//! `theme.css` format, cached so the next launch draws its first frame in the same colours.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::backend::{config, hyprpaper, os};
use crate::ipc::Event;
use crate::theme;

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Generator {
    #[default]
    Chromasync,
    Noctalia,
}

impl Generator {
    pub const ALL: [Generator; 2] = [Generator::Chromasync, Generator::Noctalia];

    pub fn name(self) -> &'static str {
        match self {
            Generator::Chromasync => "Chromasync",
            Generator::Noctalia => "Noctalia",
        }
    }

    /// This one, then the other.
    fn order(self) -> [Generator; 2] {
        match self {
            Generator::Chromasync => [Generator::Chromasync, Generator::Noctalia],
            Generator::Noctalia => [Generator::Noctalia, Generator::Chromasync],
        }
    }

    fn generate(self) -> Result<String, String> {
        match self {
            Generator::Chromasync => chromasync(),
            Generator::Noctalia => noctalia(),
        }
    }
}

/// Where the colours came from, and why the generators before it were passed over.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Colors {
    /// `None` when neither generator answered and `theme.css` was used.
    pub used: Option<Generator>,
    pub css: String,
    pub skipped: Vec<(Generator, String)>,
}

/// Asks the generators in order of preference, falling back to `theme.css`.
pub fn resolve(preferred: Generator) -> Result<Colors, String> {
    let mut skipped = Vec::new();
    for g in preferred.order() {
        match g.generate() {
            Ok(css) => {
                return Ok(Colors {
                    used: Some(g),
                    css,
                    skipped,
                });
            }
            Err(e) => skipped.push((g, e)),
        }
    }
    Ok(Colors {
        used: None,
        css: config::get_theme_css()?,
        skipped,
    })
}

// ============================================================================
// Watching
// ============================================================================

/// The files whose changes can change the colours: chromasync's config, hyprpaper's config
/// (the wallpaper chromasync renders), `theme.css` (the fallback, which matugen rewrites) and
/// Noctalia's: with the integration on (`integrated`), the palette it renders; otherwise its
/// settings (scheme, mode and wallpaper), config and community palettes.
pub fn watched_paths(integrated: bool) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if integrated {
        paths.extend(integration_paths().ok().map(|p| p.rendered));
    } else {
        if let Some(state) = xdg_dir("XDG_STATE_HOME", ".local/state") {
            paths.push(state.join("noctalia/settings.toml"));
            paths.push(state.join("noctalia/community-palettes/.catalog/palettes.json"));
        }
        paths.extend(noctalia_config_dir());
    }
    if let Some(config) = xdg_dir("XDG_CONFIG_HOME", ".config") {
        paths.push(config.join("chromasync/config.toml"));
        paths.push(config.join("hyprconfig/theme.css"));
    }
    if let Ok(p) = hyprpaper::hyprpaper_config_path() {
        paths.push(p);
    }
    paths
}

/// When each file was last written and how big it is; a directory stands for the files
/// directly in it. Missing files count too, so creating one is a change.
pub type Stamps = Vec<(PathBuf, Option<(SystemTime, u64)>)>;

pub fn stamps(paths: &[PathBuf]) -> Stamps {
    let stamp = |p: &Path| {
        fs::metadata(p)
            .ok()
            .and_then(|m| Some((m.modified().ok()?, m.len())))
    };
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            let mut files: Vec<PathBuf> = fs::read_dir(p)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|f| f.is_file())
                .collect();
            files.sort();
            out.extend(files.into_iter().map(|f| {
                let s = stamp(&f);
                (f, s)
            }));
        } else {
            out.push((p.clone(), stamp(p)));
        }
    }
    out
}

// ============================================================================
// Cache
// ============================================================================

fn xdg_dir(var: &str, fallback: &str) -> Option<PathBuf> {
    match std::env::var_os(var) {
        Some(d) if !d.is_empty() => Some(PathBuf::from(d)),
        _ => std::env::var_os("HOME").map(|h| PathBuf::from(h).join(fallback)),
    }
}

fn cache_dir() -> Result<PathBuf, String> {
    xdg_dir("XDG_CACHE_HOME", ".cache")
        .map(|d| d.join("hyprconfig"))
        .ok_or_else(|| "Could not determine home directory".to_string())
}

fn cache_path() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("colors.css"))
}

/// The colours a generator made last time, if any.
pub fn cached_css() -> Option<String> {
    fs::read_to_string(cache_path().ok()?).ok()
}

/// Keeps a generator's colours for the next launch; `None` (theme.css is in use) forgets them.
pub fn save_cache(css: Option<String>) -> Result<(), String> {
    let path = cache_path()?;
    match css {
        Some(css) => {
            fs::create_dir_all(cache_dir()?)
                .map_err(|e| format!("Failed to create cache directory: {e}"))?;
            fs::write(&path, css).map_err(|e| format!("Failed to cache colours: {e}"))
        }
        None => match fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("Failed to remove cached colours: {e}"))
            }
            _ => Ok(()),
        },
    }
}

// ============================================================================
// Shared
// ============================================================================

/// One appearance's colours, as CSS colour strings.
#[derive(Clone, Debug, PartialEq)]
struct Scheme {
    bg: String,
    fg: String,
    accent: String,
    danger: String,
}

impl Scheme {
    fn check(self) -> Result<Scheme, String> {
        for v in [&self.bg, &self.fg, &self.accent, &self.danger] {
            if theme::color(v).is_none() {
                return Err(format!("unreadable colour \"{v}\""));
            }
        }
        Ok(self)
    }

    fn declarations(&self) -> String {
        format!(
            "  --background: {};\n  --foreground: {};\n  --primary: {};\n  --destructive: {};\n",
            self.bg, self.fg, self.accent, self.danger
        )
    }
}

fn to_css(by: Generator, dark: &Scheme, light: &Scheme) -> String {
    format!(
        "/* Generated by {} for Hyprconfig. */\n:root {{\n{}}}\n\n.dark {{\n{}}}\n",
        by.name(),
        light.declarations(),
        dark.declarations()
    )
}

fn installed(tool: &str) -> Result<(), String> {
    if os::tool_exists(tool.into()).unwrap_or(false) {
        Ok(())
    } else {
        Err("not installed".into())
    }
}

/// The first line a failed command printed, for a short reason.
fn failure(output: &std::process::Output) -> String {
    [&output.stderr, &output.stdout]
        .into_iter()
        .map(|b| String::from_utf8_lossy(b).trim().to_string())
        .find(|s| !s.is_empty())
        .and_then(|s| s.lines().next().map(str::to_owned))
        .unwrap_or_else(|| format!("exited with {}", output.status))
}

// ============================================================================
// Chromasync
// ============================================================================

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

fn chromasync() -> Result<String, String> {
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

// ============================================================================
// Noctalia
// ============================================================================

/// `noctalia msg color-scheme-get`: `<source> <name>`, the name possibly with spaces.
fn parse_scheme(reply: &str) -> Option<(String, String)> {
    let (source, name) = reply.trim().split_once(char::is_whitespace)?;
    Some((source.to_owned(), name.trim().to_owned()))
}

fn noctalia() -> Result<String, String> {
    noctalia_running()?;
    // With the integration on, Noctalia renders whatever palette it shows.
    if let Some(css) = noctalia_rendered() {
        return Ok(css);
    }
    let reply = hyprpaper::noctalia_msg(&["color-scheme-get"])
        .map_err(|e| format!("isn't answering ({e})"))?;
    let (source, name) =
        parse_scheme(&reply).ok_or_else(|| format!("unexpected colour scheme \"{reply}\""))?;
    match source.as_str() {
        "wallpaper" => noctalia_wallpaper(&name),
        "community" => noctalia_community(&name),
        _ => Err(format!(
            "its {source} scheme \"{name}\" can only be read with Noctalia integration on"
        )),
    }
}

/// Runs Noctalia's own generator on its wallpaper, with its scheme.
fn noctalia_wallpaper(scheme: &str) -> Result<String, String> {
    let wallpaper = hyprpaper::noctalia_msg(&["wallpaper-get"])
        .ok()
        .and_then(|w| hyprpaper::parse_noctalia_wallpaper_output(&w))
        .ok_or("has no wallpaper to generate from")?;
    let pure_black = hyprpaper::noctalia_msg(&["config-get", "theme.pure_black_dark"])
        .is_ok_and(|v| hyprpaper::toml_scalar(&v) == "true");
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

// ============================================================================
// Noctalia integration and mode
// ============================================================================

/// A Noctalia user template: Noctalia renders it with its active palette (whatever the
/// scheme's source, builtin schemes included) on every palette change, then runs the
/// template's post hook, which tells the running app.
const NOCTALIA_TEMPLATE: &str =
    "/* Rendered by Noctalia from Hyprconfig's template. Hyprconfig reads this file. */
:root {
  --background: {{colors.surface.light.hex}};
  --foreground: {{colors.on_surface.light.hex}};
  --primary: {{colors.primary.light.hex}};
  --destructive: {{colors.error.light.hex}};
}

.dark {
  --background: {{colors.surface.dark.hex}};
  --foreground: {{colors.on_surface.dark.hex}};
  --primary: {{colors.primary.dark.hex}};
  --destructive: {{colors.error.dark.hex}};
}
";

/// Noctalia's hook lists Hyprconfig joins, and the event each sends. Palette changes come
/// through the template's post hook instead, so `colors_changed` only loses entries an older
/// Hyprconfig added.
const HOOKS: [(&str, Option<Event>); 3] = [
    ("colors_changed", None),
    ("theme_mode_changed", Some(Event::Mode)),
    ("wallpaper_changed", Some(Event::Wallpaper)),
];

/// Noctalia's config directory, which loads every `*.toml` in it.
fn noctalia_config_dir() -> Option<PathBuf> {
    match std::env::var_os("NOCTALIA_CONFIG_HOME") {
        Some(d) if !d.is_empty() => Some(PathBuf::from(d)),
        _ => xdg_dir("XDG_CONFIG_HOME", ".config").map(|d| d.join("noctalia")),
    }
}

/// The paths the integration uses: Noctalia's config file for it, the template, and what
/// Noctalia renders from it.
struct Integration {
    config: PathBuf,
    template: PathBuf,
    rendered: PathBuf,
}

fn integration_paths() -> Result<Integration, String> {
    let config = noctalia_config_dir().ok_or("no home directory")?;
    let ours = xdg_dir("XDG_CONFIG_HOME", ".config").ok_or("no home directory")?;
    Ok(Integration {
        config: config.join("hyprconfig.toml"),
        template: ours.join("hyprconfig/noctalia-template.css"),
        rendered: cache_dir()?.join("noctalia.css"),
    })
}

/// The command a hook runs: this executable, by its full path, since Noctalia's environment
/// may not have `~/.local/bin` on its PATH.
fn hook_command(exe: &Path, event: Event) -> String {
    let path = exe.to_string_lossy().replace('\'', r"'\''");
    format!("'{path}' notify {}", event.name())
}

/// Hyprconfig's own entries, from this build or an older one.
fn is_ours(command: &str) -> bool {
    command.contains("hyprconfig") && command.contains("' notify ")
}

/// Noctalia's config for the template, as Hyprconfig writes it.
fn integration_config(paths: &Integration, exe: &Path) -> String {
    let q = |s: &str| toml_edit::Value::from(s).to_string();
    format!(
        "# Written by Hyprconfig (General > Noctalia integration); turn it off there to remove it.\n\
         [theme.templates.user.hyprconfig]\n\
         input_path = {}\n\
         output_path = {}\n\
         post_hook = {}\n",
        q(&paths.template.to_string_lossy()),
        q(&paths.rendered.to_string_lossy()),
        q(&hook_command(exe, Event::Colors)),
    )
}

/// `config-get` prints a TOML array: `[ 'a', 'b' ]`.
fn parse_list(toml: &str) -> Result<Vec<String>, String> {
    let doc = format!("v = {toml}")
        .parse::<toml_edit::DocumentMut>()
        .map_err(|e| format!("unexpected hook list {toml}: {e}"))?;
    let list = doc["v"]
        .as_array()
        .ok_or_else(|| format!("unexpected hook list {toml}"))?;
    Ok(list
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect())
}

fn format_list(commands: &[String]) -> String {
    let mut a = toml_edit::Array::new();
    for c in commands {
        a.push(c.as_str());
    }
    a.to_string()
}

/// A hook list with Hyprconfig's entries replaced by `ours` (or just removed).
fn rewire(list: Vec<String>, ours: Option<String>) -> Vec<String> {
    let mut out: Vec<String> = list.into_iter().filter(|c| !is_ours(c)).collect();
    out.extend(ours);
    out
}

fn noctalia_running() -> Result<(), String> {
    installed("noctalia")?;
    hyprpaper::noctalia_msg(&["color-scheme-get"])
        .map(|_| ())
        .map_err(|e| format!("isn't answering ({e})"))
}

fn hook_list(key: &str) -> Result<Vec<String>, String> {
    parse_list(&hyprpaper::noctalia_msg(&[
        "config-get",
        &format!("hooks.{key}"),
    ])?)
}

/// Whether the template and hooks are in place for this executable. `None` when Noctalia
/// isn't running.
pub fn noctalia_integration() -> Result<Option<bool>, String> {
    if noctalia_running().is_err() {
        return Ok(None);
    }
    let exe = std::env::current_exe().map_err(|e| format!("couldn't find Hyprconfig: {e}"))?;
    let paths = integration_paths()?;
    if fs::read_to_string(&paths.config).ok() != Some(integration_config(&paths, &exe))
        || !paths.template.exists()
    {
        return Ok(Some(false));
    }
    for (key, event) in HOOKS {
        let Some(event) = event else { continue };
        if !hook_list(key)?.contains(&hook_command(&exe, event)) {
            return Ok(Some(false));
        }
    }
    Ok(Some(true))
}

/// Sets up the template (Noctalia renders it straight away, on `config-reload`) and the mode
/// and wallpaper hooks, or takes them all out. The user's other hook commands stay.
pub fn set_noctalia_integration(on: bool) -> Result<(), String> {
    noctalia_running().map_err(|e| format!("Noctalia {e}"))?;
    let exe = std::env::current_exe().map_err(|e| format!("couldn't find Hyprconfig: {e}"))?;
    let paths = integration_paths()?;
    let write = |path: &Path, text: &str| {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("couldn't create {}: {e}", dir.display()))?;
        }
        fs::write(path, text).map_err(|e| format!("couldn't write {}: {e}", path.display()))
    };
    let remove = |path: &Path| match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
            Err(format!("couldn't remove {}: {e}", path.display()))
        }
        _ => Ok(()),
    };
    if on {
        write(&paths.template, NOCTALIA_TEMPLATE)?;
        write(&paths.config, &integration_config(&paths, &exe))?;
    } else {
        remove(&paths.config)?;
    }
    for (key, event) in HOOKS {
        let list = hook_list(key)?;
        let ours = event.filter(|_| on).map(|e| hook_command(&exe, e));
        let next = rewire(list.clone(), ours);
        if next != list {
            hyprpaper::noctalia_msg(&["config-set", &format!("hooks.{key}"), &format_list(&next)])?;
        }
    }
    hyprpaper::noctalia_msg(&["config-reload"])?;
    if !on {
        remove(&paths.template)?;
        remove(&paths.rendered)?;
    }
    Ok(())
}

/// The palette Noctalia last rendered from the template, when the integration is on.
fn noctalia_rendered() -> Option<String> {
    let paths = integration_paths().ok()?;
    if !paths.config.exists() {
        return None;
    }
    let css = fs::read_to_string(&paths.rendered).ok()?;
    let p = theme::Palettes::parse(&css);
    (p.light.is_some() && p.dark.is_some()).then_some(css)
}

/// Noctalia's resolved light or dark mode; `None` when Noctalia isn't running.
pub fn noctalia_dark() -> Result<Option<bool>, String> {
    if noctalia_running().is_err() {
        return Ok(None);
    }
    match hyprpaper::noctalia_msg(&["theme-mode-get"])?.trim() {
        "dark" => Ok(Some(true)),
        "light" => Ok(Some(false)),
        other => Err(format!("unexpected theme mode \"{other}\"")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::Palettes;

    #[test]
    fn stamps_notice_writes_new_files_and_directories() {
        let dir = std::env::temp_dir().join(format!("hyprconfig-stamps-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("noctalia")).unwrap();
        let file = dir.join("settings.toml");
        let paths = vec![file.clone(), dir.join("noctalia")];

        let empty = stamps(&paths);
        assert_eq!(
            empty,
            vec![(file.clone(), None)],
            "a missing file, an empty directory"
        );
        fs::write(&file, "a = 1").unwrap();
        let written = stamps(&paths);
        assert_ne!(written, empty);
        fs::write(&file, "a = 12").unwrap();
        assert_ne!(stamps(&paths), written, "a rewrite of another size");
        fs::write(dir.join("noctalia/config.toml"), "").unwrap();
        assert_eq!(stamps(&paths).len(), 2, "files in a watched directory");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn hook_lists_keep_the_users_commands() {
        let exe = Path::new("/home/a/.local/bin/hyprconfig");
        let ours = hook_command(exe, Event::Mode);
        assert_eq!(ours, "'/home/a/.local/bin/hyprconfig' notify mode");
        assert!(is_ours(&ours));
        let old = "'/old/target/release/hyprconfig' notify mode".to_owned();
        let theirs = "notify-send 'mode changed'".to_owned();
        let list = parse_list(&format_list(&[theirs.clone(), old])).unwrap();
        assert_eq!(
            rewire(list.clone(), Some(ours.clone())),
            vec![theirs.clone(), ours],
            "an older Hyprconfig's entry is replaced"
        );
        assert_eq!(rewire(list, None), vec![theirs]);
        assert_eq!(parse_list("[ 'true' ]").unwrap(), vec!["true"]);
        assert_eq!(parse_list("[]").unwrap(), Vec::<String>::new());
        assert!(parse_list("nope").is_err());
        let quoted = hook_command(Path::new("/it's/hyprconfig"), Event::Mode);
        assert_eq!(quoted, r"'/it'\''s/hyprconfig' notify mode");
    }

    #[test]
    fn the_integration_config_is_valid_toml() {
        let paths = Integration {
            config: "/c/noctalia/hyprconfig.toml".into(),
            template: "/c/hyprconfig/noctalia \"template\".css".into(),
            rendered: "/cache/hyprconfig/noctalia.css".into(),
        };
        let text = integration_config(&paths, Path::new("/bin/hyprconfig"));
        let doc = text.parse::<toml_edit::DocumentMut>().unwrap();
        let t = &doc["theme"]["templates"]["user"]["hyprconfig"];
        assert_eq!(
            t["input_path"].as_str(),
            Some("/c/hyprconfig/noctalia \"template\".css")
        );
        assert_eq!(
            t["output_path"].as_str(),
            Some("/cache/hyprconfig/noctalia.css")
        );
        assert_eq!(
            t["post_hook"].as_str(),
            Some("'/bin/hyprconfig' notify colors")
        );
    }

    #[test]
    fn the_template_renders_to_both_appearances() {
        // What Noctalia makes of the template, with made-up colours.
        let rendered = NOCTALIA_TEMPLATE
            .replace("{{colors.surface.light.hex}}", "#fcf8ff")
            .replace("{{colors.on_surface.light.hex}}", "#1b1b21")
            .replace("{{colors.primary.light.hex}}", "#575992")
            .replace("{{colors.error.light.hex}}", "#ba1a1a")
            .replace("{{colors.surface.dark.hex}}", "#131318")
            .replace("{{colors.on_surface.dark.hex}}", "#e4e1e9")
            .replace("{{colors.primary.dark.hex}}", "#c0c1ff")
            .replace("{{colors.error.dark.hex}}", "#ffb4ab");
        let p = Palettes::parse(&rendered);
        assert_eq!(p.dark.unwrap().accent, theme::color("#c0c1ff").unwrap());
        assert_eq!(p.light.unwrap().danger, theme::color("#ba1a1a"));
    }

    #[test]
    fn preference_order() {
        assert_eq!(
            Generator::Noctalia.order(),
            [Generator::Noctalia, Generator::Chromasync]
        );
        assert_eq!(Generator::default(), Generator::Chromasync);
        assert_eq!(
            serde_json::to_string(&Generator::Noctalia).unwrap(),
            "\"noctalia\""
        );
    }

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
