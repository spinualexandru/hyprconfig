use hyprland::dispatch::DispatchType;
use hyprlang::{Config, ConfigValue, SpecialCategoryDescriptor};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Wallpaper {
    pub monitor: String,
    pub path: String,
    pub fit_mode: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HyprpaperConfig {
    pub wallpapers: Vec<Wallpaper>,
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    path.to_string()
}

fn hyprpaper_config_path() -> Result<PathBuf, String> {
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

fn parse_noctalia_wallpaper_output(output: &str) -> Option<String> {
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

fn get_noctalia_wallpaper() -> Option<Wallpaper> {
    let output = match Command::new("qs")
        .args(["-c", "noctalia-shell", "ipc", "call", "wallpaper", "get"])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("Noctalia wallpaper IPC unavailable: {}", error);
            return None;
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        eprintln!("Noctalia wallpaper IPC get failed: {}", stderr.trim());
        return None;
    }

    parse_noctalia_wallpaper_output(&String::from_utf8_lossy(&output.stdout)).map(|path| {
        Wallpaper {
            monitor: String::new(),
            path,
            fit_mode: "cover".to_string(),
        }
    })
}

fn set_noctalia_wallpaper(path: &str) -> Result<(), String> {
    let output = Command::new("qs")
        .args([
            "-c",
            "noctalia-shell",
            "ipc",
            "call",
            "wallpaper",
            "set",
            path,
        ])
        .output()
        .map_err(|e| format!("Failed to run Noctalia wallpaper IPC: {}", e))?;

    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if stderr.is_empty() { stdout } else { stderr };

    Err(format!("Failed to set Noctalia wallpaper: {}", detail))
}

fn should_write_to_noctalia(config_path: &Path, existing_wallpapers: &[Wallpaper]) -> bool {
    if config_path.exists() || !existing_wallpapers.is_empty() {
        return false;
    }

    get_noctalia_wallpaper().is_some()
}

#[cfg(test)]
fn should_write_to_noctalia_state(
    hyprpaper_config_exists: bool,
    has_hyprpaper_wallpapers: bool,
    noctalia_wallpaper_available: bool,
) -> bool {
    !hyprpaper_config_exists && !has_hyprpaper_wallpapers && noctalia_wallpaper_available
}

fn write_hyprpaper_config(config_path: &Path, content: impl AsRef<[u8]>) -> Result<(), String> {
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create Hyprpaper config directory: {}", e))?;
    }

    fs::write(config_path, content).map_err(|e| format!("Failed to write config file: {}", e))
}

#[tauri::command]
pub fn get_hyprpaper_config() -> Result<HyprpaperConfig, String> {
    let mut wallpapers = read_hyprpaper_wallpapers()?;

    if wallpapers.is_empty() {
        if let Some(wallpaper) = get_noctalia_wallpaper() {
            wallpapers.push(wallpaper);
        }
    }

    Ok(HyprpaperConfig { wallpapers })
}

#[tauri::command]
pub fn set_wallpaper(
    monitor: String,
    path: String,
    fit_mode: Option<String>,
) -> Result<(), String> {
    // Validate path
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let fit = fit_mode.unwrap_or_else(|| "cover".to_string());
    let config_path = hyprpaper_config_path()?;
    let existing_wallpapers = read_hyprpaper_wallpapers()?;

    if should_write_to_noctalia(&config_path, &existing_wallpapers) {
        return set_noctalia_wallpaper(path.trim());
    }

    // Build wallpaper category content
    let wallpaper_block = format!(
        "\nwallpaper {{\n    monitor = {}\n    path = {}\n    fit_mode = {}\n}}\n",
        monitor.trim(),
        path.trim(),
        fit
    );

    // Append to config file (or create if doesn't exist)
    let mut content = fs::read_to_string(&config_path).unwrap_or_default();
    content.push_str(&wallpaper_block);
    write_hyprpaper_config(&config_path, content)?;

    Ok(())
}

#[tauri::command]
pub fn remove_wallpaper(name: String) -> Result<(), String> {
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        return Err("Hyprpaper config file not found".to_string());
    }

    let mut config = Config::new();
    register_hyprpaper_config(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Remove wallpaper special category instance by name
    config
        .remove_special_category_instance("wallpaper", &name)
        .map_err(|e| format!("Failed to remove wallpaper: {:?}", e))?;

    config
        .save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn update_wallpaper(
    name: String,
    monitor: String,
    path: String,
    fit_mode: Option<String>,
) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // Remove old and add new (simpler than trying to update in place)
    remove_wallpaper(name)?;
    set_wallpaper(monitor, path, fit_mode)?;

    Ok(())
}

#[tauri::command]
pub fn replace_wallpaper(
    monitor: String,
    path: String,
    fit_mode: Option<String>,
) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let config_path = hyprpaper_config_path()?;

    let monitor_str = monitor.trim();
    let path_str = path.trim();
    let fit = fit_mode.unwrap_or_else(|| "cover".to_string());

    let existing_wallpapers = read_hyprpaper_wallpapers()?;
    if should_write_to_noctalia(&config_path, &existing_wallpapers) {
        return set_noctalia_wallpaper(path_str);
    }

    // Write new config file with single wallpaper in new format
    let content = format!(
        "wallpaper {{\n    monitor = {}\n    path = {}\n    fit_mode = {}\n}}\n",
        monitor_str, path_str, fit
    );

    write_hyprpaper_config(&config_path, content)?;

    // Use new IPC format: hyprctl hyprpaper wallpaper '[mon], [path], [fit_mode]'
    let command = format!(
        "hyprctl hyprpaper wallpaper '{}, {}, {}'",
        monitor_str, path_str, fit
    );

    let hyprpaper_result = hyprland::dispatch::Dispatch::call(DispatchType::Exec(&command));

    match hyprpaper_result {
        Ok(_) => println!("Hyprpaper wallpaper command sent successfully."),
        Err(e) => println!("Failed to send Hyprpaper wallpaper command: {:?}", e),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_noctalia_wallpaper_output, should_write_to_noctalia_state};

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

    #[test]
    fn fresh_hyprpaper_install_uses_hyprpaper_when_noctalia_is_unavailable() {
        assert!(!should_write_to_noctalia_state(false, false, false));
    }

    #[test]
    fn empty_existing_hyprpaper_config_stays_on_hyprpaper() {
        assert!(!should_write_to_noctalia_state(true, false, true));
    }

    #[test]
    fn missing_hyprpaper_config_can_use_available_noctalia_backend() {
        assert!(should_write_to_noctalia_state(false, false, true));
    }
}
