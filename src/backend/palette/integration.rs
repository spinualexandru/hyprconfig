//! The Noctalia integration (a template Noctalia renders, plus hooks that notify the app) and
//! Noctalia's light or dark mode.

use std::fs;
use std::path::{Path, PathBuf};

use super::cache::cache_dir;
use super::{installed, xdg_dir};
use crate::backend::noctalia;
use crate::ipc::Event;
use crate::theme;

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
pub(super) fn noctalia_config_dir() -> Option<PathBuf> {
    match std::env::var_os("NOCTALIA_CONFIG_HOME") {
        Some(d) if !d.is_empty() => Some(PathBuf::from(d)),
        _ => xdg_dir("XDG_CONFIG_HOME", ".config").map(|d| d.join("noctalia")),
    }
}

/// The paths the integration uses: Noctalia's config file for it, the template, and what
/// Noctalia renders from it.
pub(super) struct Integration {
    config: PathBuf,
    template: PathBuf,
    pub(super) rendered: PathBuf,
}

pub(super) fn integration_paths() -> Result<Integration, String> {
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

/// Whether Noctalia is installed and answering: its `color-scheme-get` reply, or why not.
pub(super) fn noctalia_running() -> Result<String, String> {
    installed("noctalia")?;
    noctalia::noctalia_msg(&["color-scheme-get"]).map_err(|e| format!("isn't answering ({e})"))
}

fn hook_list(key: &str) -> Result<Vec<String>, String> {
    parse_list(&noctalia::noctalia_msg(&[
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
            noctalia::noctalia_msg(&["config-set", &format!("hooks.{key}"), &format_list(&next)])?;
        }
    }
    noctalia::noctalia_msg(&["config-reload"])?;
    if !on {
        remove(&paths.template)?;
        remove(&paths.rendered)?;
    }
    Ok(())
}

/// `hyprconfig noctalia off`, which `just uninstall` runs while the binary the hooks name is
/// still there: the integration comes out, the user's own hooks stay. Without Noctalia
/// running its hooks can't be changed, which matters only if the integration is on.
pub fn remove_noctalia_integration() -> Result<(), String> {
    if noctalia_running().is_ok() {
        return set_noctalia_integration(false);
    }
    if integration_paths()?.config.exists() {
        return Err(
            "Noctalia isn't running, so Hyprconfig's hooks can't be taken out of it; \
                    start Noctalia and run `hyprconfig noctalia off` again"
                .into(),
        );
    }
    Ok(())
}

/// The palette Noctalia last rendered from the template, when the integration is on.
pub(super) fn noctalia_rendered() -> Option<String> {
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
    // Noctalia is running, so a refusal here is worth reporting.
    match noctalia::noctalia_msg(&["theme-mode-get"])?.trim() {
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
}
