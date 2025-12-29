use hyprland::dispatch::DispatchType;
use hyprlang::{Config, SpecialCategoryDescriptor};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

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

fn register_hyprpaper_config(config: &mut Config) {
    // Register hyprpaper-specific keywords as handlers
    let keywords = vec!["splash", "splash_offset", "splash_opacity", "ipc"];
    for keyword in keywords {
        config.register_handler_fn(keyword, |_ctx| Ok(()));
    }

    // Register wallpaper as anonymous special category
    config.register_special_category(SpecialCategoryDescriptor::anonymous("wallpaper"));
}

#[tauri::command]
pub fn get_hyprpaper_config() -> Result<HyprpaperConfig, String> {
    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        // Return empty config if file doesn't exist (config is optional)
        return Ok(HyprpaperConfig {
            wallpapers: Vec::new(),
        });
    }

    // Parse the config file using hyprlang
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
            let path = instance
                .get("path")
                .and_then(|v| v.as_string().ok())
                .unwrap_or("")
                .to_string();
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

    Ok(HyprpaperConfig { wallpapers })
}

#[tauri::command]
pub fn set_wallpaper(monitor: String, path: String, fit_mode: Option<String>) -> Result<(), String> {
    // Validate path
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let fit = fit_mode.unwrap_or_else(|| "cover".to_string());
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

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
    fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

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
pub fn replace_wallpaper(monitor: String, path: String, fit_mode: Option<String>) -> Result<(), String> {
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    let monitor_str = monitor.trim();
    let path_str = path.trim();
    let fit = fit_mode.unwrap_or_else(|| "cover".to_string());

    // Write new config file with single wallpaper in new format
    let content = format!(
        "wallpaper {{\n    monitor = {}\n    path = {}\n    fit_mode = {}\n}}\n",
        monitor_str, path_str, fit
    );

    fs::write(&config_path, content).map_err(|e| format!("Failed to write config file: {}", e))?;

    // Use new IPC format: hyprctl hyprpaper wallpaper '[mon], [path], [fit_mode]'
    let command = format!("hyprctl hyprpaper wallpaper '{}, {}, {}'", monitor_str, path_str, fit);

    let hyprpaper_result = hyprland::dispatch::Dispatch::call(DispatchType::Exec(&command));

    match hyprpaper_result {
        Ok(_) => println!("Hyprpaper wallpaper command sent successfully."),
        Err(e) => println!("Failed to send Hyprpaper wallpaper command: {:?}", e),
    }

    Ok(())
}
