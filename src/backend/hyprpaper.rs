use hyprlang::{Config, ConfigValue, SpecialCategoryDescriptor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Wallpaper {
    pub monitor: String,
    pub path: String,
    pub fit_mode: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HyprpaperConfig {
    pub wallpapers: Vec<Wallpaper>,
    /// What draws the wallpaper; `None` when nothing is running or configured.
    pub source: Option<Daemon>,
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{}/{}", home, rest);
    }
    path.to_string()
}

pub(crate) fn hyprpaper_config_path() -> Result<PathBuf, String> {
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    Ok(Path::new(&home_dir).join(".config/hypr/hyprpaper.conf"))
}

fn register_hyprpaper_config(config: &mut Config) {
    // Register hyprpaper-specific keywords as handlers
    let keywords = vec![
        "preload",
        "splash",
        "splash_offset",
        "splash_opacity",
        "ipc",
    ];
    for keyword in keywords {
        config.register_handler_fn(keyword, |_ctx| Ok(()));
    }

    // Register wallpaper as anonymous special category with its properties
    config.register_special_category(SpecialCategoryDescriptor::anonymous("wallpaper"));
    config.register_special_category_value(
        "wallpaper",
        "monitor",
        ConfigValue::String(String::new()),
    );
    config.register_special_category_value("wallpaper", "path", ConfigValue::String(String::new()));
    config.register_special_category_value(
        "wallpaper",
        "fit_mode",
        ConfigValue::String("cover".to_string()),
    );
}

fn read_hyprpaper_wallpapers() -> Result<Vec<Wallpaper>, String> {
    let config_path = hyprpaper_config_path()?;
    if !config_path.exists() {
        return Ok(Vec::new());
    }

    let mut config = Config::new();
    register_hyprpaper_config(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Get wallpaper anonymous category instances
    let wallpaper_keys = config.list_special_category_keys("wallpaper");
    let mut wallpapers = Vec::new();

    for key in wallpaper_keys {
        if let Ok(instance) = config.get_special_category("wallpaper", &key) {
            let monitor = instance
                .get("monitor")
                .and_then(|v| v.as_string().ok())
                .unwrap_or("")
                .to_string();
            let path = expand_tilde(
                instance
                    .get("path")
                    .and_then(|v| v.as_string().ok())
                    .unwrap_or(""),
            );
            let fit_mode = instance
                .get("fit_mode")
                .and_then(|v| v.as_string().ok())
                .unwrap_or("cover")
                .to_string();

            if !path.is_empty() {
                wallpapers.push(Wallpaper {
                    monitor,
                    path,
                    fit_mode,
                });
            }
        }
    }

    Ok(wallpapers)
}

pub(crate) fn parse_noctalia_wallpaper_output(output: &str) -> Option<String> {
    let trimmed = output.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("null")
        || trimmed.eq_ignore_ascii_case("nil")
        || trimmed.eq_ignore_ascii_case("none")
    {
        return None;
    }

    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return wallpaper_path_from_json(&value).map(|path| expand_tilde(&path));
    }

    let first_line = trimmed.lines().next()?.trim();
    let unquoted = first_line
        .strip_prefix('"')
        .and_then(|line| line.strip_suffix('"'))
        .or_else(|| {
            first_line
                .strip_prefix('\'')
                .and_then(|line| line.strip_suffix('\''))
        })
        .unwrap_or(first_line)
        .trim();

    if unquoted.is_empty() {
        None
    } else {
        Some(expand_tilde(unquoted))
    }
}

fn wallpaper_path_from_json(value: &Value) -> Option<String> {
    match value {
        Value::String(path) if !path.trim().is_empty() => Some(path.trim().to_string()),
        Value::Object(map) => ["path", "wallpaper", "image", "current"]
            .iter()
            .find_map(|key| map.get(*key).and_then(wallpaper_path_from_json)),
        _ => None,
    }
}

/// What draws the wallpaper.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Daemon {
    Hyprpaper,
    /// Noctalia 5 and later, a native shell driven by `noctalia msg`.
    Noctalia,
    /// Noctalia before 5, a Quickshell config driven by `qs -c noctalia-shell ipc`.
    NoctaliaLegacy,
}

impl Daemon {
    pub fn name(self) -> &'static str {
        match self {
            Daemon::Hyprpaper => "hyprpaper",
            Daemon::Noctalia | Daemon::NoctaliaLegacy => "Noctalia",
        }
    }
}

/// What's running, as far as wallpapers go.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Running {
    hyprpaper: bool,
    /// Noctalia 5 is running and draws the wallpaper (`wallpaper.enabled`).
    noctalia: bool,
    /// An older Noctalia answers its Quickshell IPC.
    legacy: bool,
}

impl Running {
    fn detect() -> Running {
        let hyprpaper = process_running("hyprpaper");
        let noctalia = process_running("noctalia") && noctalia_draws_wallpaper();
        // Noctalia 4 runs as `qs`/`quickshell`; only ask it when nothing newer answers.
        let legacy = !hyprpaper && !noctalia && get_legacy_noctalia_wallpaper().is_some();
        Running {
            hyprpaper,
            noctalia,
            legacy,
        }
    }

    /// Where a new wallpaper goes: every running daemon, or hyprpaper's config (shown from its
    /// next start) when none runs.
    fn targets(self) -> Vec<Daemon> {
        let mut out = Vec::new();
        if self.hyprpaper {
            out.push(Daemon::Hyprpaper);
        }
        if self.noctalia {
            out.push(Daemon::Noctalia);
        }
        if self.legacy {
            out.push(Daemon::NoctaliaLegacy);
        }
        if out.is_empty() {
            out.push(Daemon::Hyprpaper);
        }
        out
    }

    /// Where the shown wallpaper is read from: a running daemon first (Noctalia draws over
    /// hyprpaper when both run), then hyprpaper's config.
    fn source(self, config_has_wallpapers: bool) -> Option<Daemon> {
        if self.noctalia {
            Some(Daemon::Noctalia)
        } else if self.legacy {
            Some(Daemon::NoctaliaLegacy)
        } else if self.hyprpaper || config_has_wallpapers {
            Some(Daemon::Hyprpaper)
        } else {
            None
        }
    }
}

/// A process whose name (`/proc/<pid>/comm`) is `name`.
fn process_running(name: &str) -> bool {
    let Ok(entries) = fs::read_dir("/proc") else {
        return false;
    };
    entries.flatten().any(|e| {
        e.file_name()
            .to_str()
            .is_some_and(|pid| pid.bytes().all(|b| b.is_ascii_digit()))
            && fs::read_to_string(e.path().join("comm")).is_ok_and(|comm| comm.trim() == name)
    })
}

/// `noctalia msg <args>`: its output, or why it failed.
pub(crate) fn noctalia_msg(args: &[&str]) -> Result<String, String> {
    let output = Command::new("noctalia")
        .arg("msg")
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run noctalia: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    noctalia_reply(output.status.success(), stdout, &stderr)
}

/// Noctalia prints `error: ...` and exits 1 when it refuses a request.
fn noctalia_reply(success: bool, stdout: String, stderr: &str) -> Result<String, String> {
    if success && !stdout.starts_with("error:") {
        return Ok(stdout);
    }
    let detail = [stdout.as_str(), stderr]
        .into_iter()
        .find(|d| !d.is_empty())
        .unwrap_or("noctalia msg failed");
    Err(detail.trim_start_matches("error:").trim().to_string())
}

/// `config-get` prints TOML values: `true`, `'crop'`.
pub(crate) fn toml_scalar(value: &str) -> String {
    value.trim().trim_matches(['\'', '"']).to_string()
}

/// Noctalia can run with its wallpaper turned off, leaving it to another daemon.
fn noctalia_draws_wallpaper() -> bool {
    noctalia_msg(&["config-get", "wallpaper.enabled"]).map_or(true, |v| toml_scalar(&v) != "false")
}

fn get_noctalia_wallpaper() -> Result<Option<Wallpaper>, String> {
    let path = parse_noctalia_wallpaper_output(&noctalia_msg(&["wallpaper-get"])?);
    let fit_mode = noctalia_msg(&["config-get", "wallpaper.fill_mode"])
        .map(|v| toml_scalar(&v))
        .unwrap_or_default();
    Ok(path.map(|path| Wallpaper {
        monitor: String::new(),
        path,
        fit_mode,
    }))
}

fn set_noctalia_wallpaper(path: &str) -> Result<(), String> {
    noctalia_msg(&["wallpaper-set", path]).map(|_| ())
}

fn legacy_noctalia_ipc(args: &[&str]) -> Result<std::process::Output, String> {
    Command::new("qs")
        .args(["-c", "noctalia-shell", "ipc", "call", "wallpaper"])
        .args(args)
        .output()
        .map_err(|e| format!("Failed to run Noctalia wallpaper IPC: {}", e))
}

fn get_legacy_noctalia_wallpaper() -> Option<Wallpaper> {
    let output = legacy_noctalia_ipc(&["get"]).ok()?;
    if !output.status.success() {
        return None;
    }
    parse_noctalia_wallpaper_output(&String::from_utf8_lossy(&output.stdout)).map(|path| {
        Wallpaper {
            monitor: String::new(),
            path,
            fit_mode: String::new(),
        }
    })
}

fn set_legacy_noctalia_wallpaper(path: &str) -> Result<(), String> {
    let output = legacy_noctalia_ipc(&["set", path])?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Err(if stderr.is_empty() { stdout } else { stderr })
}

fn write_hyprpaper_config(config_path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Hyprpaper config directory: {}", e))?;
    }

    fs::write(config_path, content).map_err(|e| format!("Failed to write config file: {}", e))
}

/// The wallpaper shown and what shows it: Noctalia's when it draws the wallpaper, else
/// hyprpaper's config.
pub fn get_hyprpaper_config() -> Result<HyprpaperConfig, String> {
    let from_config = read_hyprpaper_wallpapers();
    let running = Running::detect();
    let source = running.source(from_config.as_ref().is_ok_and(|w| !w.is_empty()));
    let wallpapers = match source {
        Some(Daemon::Noctalia) => get_noctalia_wallpaper()?.into_iter().collect(),
        Some(Daemon::NoctaliaLegacy) => get_legacy_noctalia_wallpaper().into_iter().collect(),
        Some(Daemon::Hyprpaper) | None => from_config?,
    };
    Ok(HyprpaperConfig { wallpapers, source })
}

/// Shows `path` everywhere a daemon draws wallpapers: hyprpaper (its config is rewritten with
/// `path` as the only wallpaper, then the running hyprpaper is asked to show it) and Noctalia.
/// With none running, hyprpaper's config is written so it shows from hyprpaper's next start.
///
/// `Ok(Some(problems))` when the wallpaper was saved somewhere but a daemon didn't take it;
/// `Err` when it went nowhere.
pub fn replace_wallpaper(
    monitor: String,
    path: String,
    fit_mode: Option<String>,
) -> Result<Option<String>, String> {
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }
    let monitor = monitor.trim();
    let path = path.trim();
    let fit = fit_mode.unwrap_or_else(|| "cover".to_string());

    let mut saved = false;
    let mut problems = Vec::new();
    for daemon in Running::detect().targets() {
        let result = match daemon {
            Daemon::Hyprpaper => hyprpaper_config_path()
                .and_then(|config| {
                    write_hyprpaper_config(&config, wallpaper_block(monitor, path, &fit))
                })
                .and_then(|()| {
                    saved = true;
                    show_in_hyprpaper(monitor, path, &fit)
                }),
            Daemon::Noctalia => set_noctalia_wallpaper(path).map(|()| saved = true),
            Daemon::NoctaliaLegacy => set_legacy_noctalia_wallpaper(path).map(|()| saved = true),
        };
        if let Err(e) = result {
            problems.push(format!("{} didn't show it: {}", daemon.name(), e));
        }
    }
    match (saved, problems.is_empty()) {
        (_, true) => Ok(None),
        (true, false) => Ok(Some(problems.join("; "))),
        (false, false) => Err(problems.join("; ")),
    }
}

/// hyprpaper's config for one wallpaper. A `#` starts a comment in hyprlang, so a literal one
/// is written `##`.
fn wallpaper_block(monitor: &str, path: &str, fit: &str) -> String {
    let esc = |s: &str| s.replace('#', "##");
    format!(
        "wallpaper {{\n    monitor = {}\n    path = {}\n    fit_mode = {}\n}}\n",
        esc(monitor),
        esc(path),
        esc(fit)
    )
}

/// `hyprctl hyprpaper wallpaper '[mon], [path], [fit_mode]'` (hyprpaper's IPC format).
fn show_in_hyprpaper(monitor: &str, path: &str, fit: &str) -> Result<(), String> {
    let output = Command::new("hyprctl")
        .args([
            "hyprpaper",
            "wallpaper",
            &format!("{}, {}, {}", monitor, path, fit),
        ])
        .output()
        .map_err(|e| format!("Failed to run hyprctl: {}", e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    hyprpaper_reply(output.status.success(), &stdout, &stderr)
}

/// hyprctl answers `ok` (older hyprpaper) or nothing on success, and `error: ...` (exiting 1)
/// when hyprpaper isn't running or refuses the request.
fn hyprpaper_reply(success: bool, stdout: &str, stderr: &str) -> Result<(), String> {
    let refused =
        stdout.to_lowercase().starts_with("error") || stdout.to_lowercase().starts_with("couldn't");
    if success && !refused {
        return Ok(());
    }
    let detail = [stdout, stderr]
        .into_iter()
        .find(|d| !d.is_empty())
        .unwrap_or("hyprctl failed");
    Err(detail.trim_start_matches("error:").trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_wallpaper_path_keeps_its_hash() {
        let mut config = Config::new();
        register_hyprpaper_config(&mut config);
        config
            .parse(&wallpaper_block("", "/home/u/walls/#1 sunset.png", "cover"))
            .unwrap();
        let key = config.list_special_category_keys("wallpaper").remove(0);
        let wallpaper = config.get_special_category("wallpaper", &key).unwrap();
        let path = wallpaper.get("path").and_then(|v| v.as_string().ok());
        assert_eq!(path, Some("/home/u/walls/#1 sunset.png"));
    }

    #[test]
    fn reads_hyprpaper_replies() {
        assert_eq!(hyprpaper_reply(true, "ok", ""), Ok(()));
        assert_eq!(hyprpaper_reply(true, "", ""), Ok(()));
        assert_eq!(
            hyprpaper_reply(
                false,
                "error: can't send: failed to connect to hyprpaper (is it running?)",
                ""
            ),
            Err("can't send: failed to connect to hyprpaper (is it running?)".into())
        );
        assert_eq!(
            hyprpaper_reply(true, "error: invalid hyprpaper request", ""),
            Err("invalid hyprpaper request".into())
        );
        assert_eq!(
            hyprpaper_reply(false, "", "no socket"),
            Err("no socket".into())
        );
    }

    #[test]
    fn reads_noctalia_replies() {
        assert_eq!(
            noctalia_reply(true, "/home/u/a.jpg".into(), ""),
            Ok("/home/u/a.jpg".into())
        );
        assert_eq!(
            noctalia_reply(
                false,
                "error: unknown output \"X\"; known: eDP-1".into(),
                ""
            ),
            Err("unknown output \"X\"; known: eDP-1".into())
        );
        assert_eq!(
            noctalia_reply(false, String::new(), "no instance running"),
            Err("no instance running".into())
        );
        assert_eq!(toml_scalar("'crop'\n"), "crop");
        assert_eq!(toml_scalar("true"), "true");
    }

    #[test]
    fn parses_noctalia_raw_path_output() {
        assert_eq!(
            parse_noctalia_wallpaper_output("/tmp/wallpaper.jpg\n").as_deref(),
            Some("/tmp/wallpaper.jpg")
        );
    }

    #[test]
    fn parses_noctalia_json_string_output() {
        assert_eq!(
            parse_noctalia_wallpaper_output("\"/tmp/wallpaper.jpg\"").as_deref(),
            Some("/tmp/wallpaper.jpg")
        );
    }

    #[test]
    fn parses_noctalia_json_object_output() {
        assert_eq!(
            parse_noctalia_wallpaper_output(r#"{"wallpaper":"/tmp/wallpaper.jpg"}"#).as_deref(),
            Some("/tmp/wallpaper.jpg")
        );
    }

    #[test]
    fn ignores_noctalia_empty_output() {
        assert_eq!(parse_noctalia_wallpaper_output("null"), None);
    }

    fn running(hyprpaper: bool, noctalia: bool, legacy: bool) -> Running {
        Running {
            hyprpaper,
            noctalia,
            legacy,
        }
    }

    #[test]
    fn sets_every_running_daemon() {
        use Daemon::*;
        assert_eq!(running(true, false, false).targets(), [Hyprpaper]);
        assert_eq!(running(false, true, false).targets(), [Noctalia]);
        assert_eq!(running(true, true, false).targets(), [Hyprpaper, Noctalia]);
        assert_eq!(running(false, false, true).targets(), [NoctaliaLegacy]);
    }

    #[test]
    fn nothing_running_writes_hyprpapers_config() {
        assert_eq!(Running::default().targets(), [Daemon::Hyprpaper]);
    }

    #[test]
    fn reads_from_whatever_draws_on_top() {
        use Daemon::*;
        assert_eq!(running(true, true, false).source(true), Some(Noctalia));
        assert_eq!(running(true, false, false).source(false), Some(Hyprpaper));
        assert_eq!(running(false, false, false).source(true), Some(Hyprpaper));
        assert_eq!(running(false, false, false).source(false), None);
        assert_eq!(
            running(false, false, true).source(true),
            Some(NoctaliaLegacy)
        );
    }
}
