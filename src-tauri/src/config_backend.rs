use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, DocumentMut};

/// Matugen preferences structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatugenPreferences {
    pub enable: bool,
    pub light_mode: bool,
}

/// Main preferences configuration structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PreferencesConfig {
    pub matugen: MatugenPreferences,
}

/// Get the configuration file path using XDG_CONFIG_HOME or fallback to ~/.config
fn get_config_path() -> Result<PathBuf, String> {
    // Try XDG_CONFIG_HOME first
    let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else {
        // Fallback to ~/.config
        let home_dir = std::env::var("HOME")
            .map_err(|_| "Could not determine home directory".to_string())?;
        PathBuf::from(home_dir).join(".config")
    };

    let hyprconfig_dir = config_dir.join("hyprconfig");

    // Create the hyprconfig directory if it doesn't exist
    if !hyprconfig_dir.exists() {
        fs::create_dir_all(&hyprconfig_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    Ok(hyprconfig_dir.join("preferences.toml"))
}

/// Ensure the config file exists, creating it with defaults if it doesn't
fn ensure_config_exists(config_path: &PathBuf) -> Result<(), String> {
    if !config_path.exists() {
        let default_config = r#"[matugen]
enable = false
light_mode = false
"#;
        fs::write(config_path, default_config)
            .map_err(|e| format!("Failed to create default config: {}", e))?;
    }
    Ok(())
}

/// Load the configuration file as a mutable TOML document
fn load_config_document(config_path: &PathBuf) -> Result<DocumentMut, String> {
    let contents = fs::read_to_string(config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    contents
        .parse::<DocumentMut>()
        .map_err(|e| format!("Failed to parse TOML: {}", e))
}

/// Tauri command to get current preferences
#[tauri::command]
pub fn get_preferences() -> Result<PreferencesConfig, String> {
    let config_path = get_config_path()?;
    ensure_config_exists(&config_path)?;

    let doc = load_config_document(&config_path)?;

    // Extract values from the [matugen] table with defaults
    let enable = doc
        .get("matugen")
        .and_then(|t| t.get("enable"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let light_mode = doc
        .get("matugen")
        .and_then(|t| t.get("light_mode"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    Ok(PreferencesConfig {
        matugen: MatugenPreferences { enable, light_mode },
    })
}

/// Tauri command to update matugen preferences
#[tauri::command]
pub fn update_matugen_preferences(enable: bool, light_mode: bool) -> Result<(), String> {
    let config_path = get_config_path()?;
    ensure_config_exists(&config_path)?;

    let mut doc = load_config_document(&config_path)?;

    // Ensure [matugen] table exists
    if !doc.contains_table("matugen") {
        doc["matugen"] = toml_edit::table();
    }

    // Update the values - toml_edit preserves all other content
    doc["matugen"]["enable"] = value(enable);
    doc["matugen"]["light_mode"] = value(light_mode);

    // Write back to file
    fs::write(&config_path, doc.to_string())
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}
