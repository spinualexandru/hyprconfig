use hyprland::dispatch::DispatchType;
use hyprlang::Config;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Wallpaper {
    pub monitor: String,
    pub path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HyprpaperConfig {
    pub preloads: Vec<String>,
    pub wallpapers: Vec<Wallpaper>,
}

fn register_hyprpaper_handlers(config: &mut Config) {
    // Register hyprpaper-specific keywords as handlers
    let keywords = vec!["preload", "wallpaper", "splash", "ipc"];

    for keyword in keywords {
        config.register_handler_fn(keyword, |_ctx| Ok(()));
    }
}

#[tauri::command]
pub fn get_hyprpaper_config() -> Result<HyprpaperConfig, String> {
    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        return Err(format!(
            "Hyprpaper config file not found at {:?}",
            config_path
        ));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprpaper_handlers(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Get all handler calls
    let all_handlers = config.all_handler_calls();

    // Get preload calls
    let preloads = all_handlers
        .get("preload")
        .cloned()
        .unwrap_or_else(Vec::new);

    // Get wallpaper calls
    let wallpaper_calls = all_handlers
        .get("wallpaper")
        .cloned()
        .unwrap_or_else(Vec::new);

    let mut wallpapers = Vec::new();

    for wallpaper_str in wallpaper_calls {
        // Parse wallpaper format: "monitor,path"
        // Example: ",/path/to/wallpaper.jpg" (empty monitor means all monitors)
        // Example: "eDP-1,/path/to/wallpaper.jpg"
        if let Some((monitor, path)) = wallpaper_str.split_once(',') {
            wallpapers.push(Wallpaper {
                monitor: monitor.trim().to_string(),
                path: path.trim().to_string(),
            });
        }
    }

    Ok(HyprpaperConfig {
        preloads,
        wallpapers,
    })
}

#[tauri::command]
pub fn add_preload(path: String) -> Result<(), String> {
    // Validate path
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        // Create the file if it doesn't exist
        fs::write(&config_path, "")
            .map_err(|e| format!("Failed to create hyprpaper config file: {}", e))?;
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprpaper_handlers(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Add preload handler call
    config
        .add_handler_call("preload", path.trim().to_string())
        .map_err(|e| format!("Failed to add preload: {:?}", e))?;

    // Save the config file
    config
        .save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn remove_preload(index: usize) -> Result<(), String> {
    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        return Err(format!(
            "Hyprpaper config file not found at {:?}",
            config_path
        ));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprpaper_handlers(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Remove preload handler call at index
    config
        .remove_handler_call("preload", index)
        .map_err(|e| format!("Failed to remove preload at index {}: {:?}", index, e))?;

    // Save the config file
    config
        .save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn set_wallpaper(monitor: String, path: String) -> Result<(), String> {
    // Validate path
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        // Create the file if it doesn't exist
        fs::write(&config_path, "")
            .map_err(|e| format!("Failed to create hyprpaper config file: {}", e))?;
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprpaper_handlers(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Format wallpaper args: "monitor,path"
    let wallpaper_args = format!("{},{}", monitor.trim(), path.trim());

    // Add wallpaper handler call
    config
        .add_handler_call("wallpaper", wallpaper_args)
        .map_err(|e| format!("Failed to set wallpaper: {:?}", e))?;

    // Save the config file
    config
        .save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn remove_wallpaper(index: usize) -> Result<(), String> {
    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        return Err(format!(
            "Hyprpaper config file not found at {:?}",
            config_path
        ));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprpaper_handlers(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Remove wallpaper handler call at index
    config
        .remove_handler_call("wallpaper", index)
        .map_err(|e| format!("Failed to remove wallpaper at index {}: {:?}", index, e))?;

    // Save the config file
    config
        .save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn update_wallpaper(index: usize, monitor: String, path: String) -> Result<(), String> {
    // Validate path
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        return Err(format!(
            "Hyprpaper config file not found at {:?}",
            config_path
        ));
    }

    // Parse the config file using hyprlang
    let mut config = Config::new();
    register_hyprpaper_handlers(&mut config);
    config
        .parse_file(&config_path)
        .map_err(|e| format!("Failed to parse hyprpaper config: {:?}", e))?;

    // Remove old wallpaper at index
    config
        .remove_handler_call("wallpaper", index)
        .map_err(|e| format!("Failed to remove wallpaper at index {}: {:?}", index, e))?;

    // Format new wallpaper args: "monitor,path"
    let wallpaper_args = format!("{},{}", monitor.trim(), path.trim());

    // Add new wallpaper handler call
    config
        .add_handler_call("wallpaper", wallpaper_args)
        .map_err(|e| format!("Failed to add wallpaper: {:?}", e))?;

    // Save the config file
    config
        .save_as(&config_path)
        .map_err(|e| format!("Failed to save config file: {:?}", e))?;

    Ok(())
}

#[tauri::command]
pub fn replace_wallpaper(monitor: String, path: String) -> Result<(), String> {
    // Validate path
    if path.trim().is_empty() {
        return Err("Path cannot be empty".to_string());
    }

    // Get Hyprpaper config path
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    // Simply write a new config file with just the new wallpaper
    // This is more reliable than trying to use the mutation API which has issues with remove
    let monitor_str = monitor.trim();
    let path_str = path.trim();

    let content = format!(
        "preload = {}\nwallpaper = {},{}\n",
        path_str, monitor_str, path_str
    );

    println!("Path: {}", path_str);
    let command = format!("hyprctl hyprpaper reload ,{}", path_str);

    fs::write(&config_path, content).map_err(|e| format!("Failed to write config file: {}", e))?;

    let hyprpaper_result = hyprland::dispatch::Dispatch::call(DispatchType::Exec(command.as_str()));

    match hyprpaper_result {
        Ok(_) => println!("Hyprpaper reload command sent successfully."),
        Err(e) => println!("Failed to send Hyprpaper reload command: {:?}", e),
    }

    Ok(())
}
