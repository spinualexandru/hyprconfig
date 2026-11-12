use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml_edit::{DocumentMut, value};

use super::os;

/// Matugen preferences structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MatugenPreferences {
    pub enable: bool,
    pub light_mode: bool,
    pub generator_type: String,
}

/// Main preferences configuration structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PreferencesConfig {
    pub matugen: MatugenPreferences,
}

/// Get the configuration file path using XDG_CONFIG_HOME or fallback to ~/.config
fn get_config_path() -> Result<PathBuf, String> {
    let hyprconfig_dir = os::config_home()?.join("hyprconfig");

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
generator_type = "scheme-tonal-spot"
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

/// Read the current preferences, creating the file with defaults if missing
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

    let generator_type = doc
        .get("matugen")
        .and_then(|t| t.get("generator_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("scheme-tonal-spot")
        .to_string();

    Ok(PreferencesConfig {
        matugen: MatugenPreferences {
            enable,
            light_mode,
            generator_type,
        },
    })
}

/// Update the matugen preferences, preserving the rest of the file
pub fn update_matugen_preferences(
    enable: bool,
    light_mode: bool,
    generator_type: String,
) -> Result<(), String> {
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
    doc["matugen"]["generator_type"] = value(generator_type);

    // Write back to file
    fs::write(&config_path, doc.to_string())
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

// ============================================================================
// Theme CSS Management
// ============================================================================

/// Default theme CSS with light and dark mode variables
const DEFAULT_THEME_CSS: &str = include_str!("../../assets/theme/default.css");

/// Matugen template for generating theme CSS
const MATUGEN_TEMPLATE: &str = include_str!("../../assets/theme/matugen-template.css");

/// Get the theme CSS file path (XDG_CONFIG_HOME/hyprconfig/theme.css)
fn get_theme_css_path() -> Result<PathBuf, String> {
    let hyprconfig_dir = os::config_home()?.join("hyprconfig");

    if !hyprconfig_dir.exists() {
        fs::create_dir_all(&hyprconfig_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    Ok(hyprconfig_dir.join("theme.css"))
}

/// Get the matugen template file path (~/.config/matugen/templates/hyprconfig.css)
fn get_matugen_template_path() -> Result<PathBuf, String> {
    let home_dir =
        std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;

    let templates_dir = PathBuf::from(home_dir)
        .join(".config")
        .join("matugen")
        .join("templates");

    if !templates_dir.exists() {
        fs::create_dir_all(&templates_dir)
            .map_err(|e| format!("Failed to create matugen templates directory: {}", e))?;
    }

    Ok(templates_dir.join("hyprconfig.css"))
}

/// Get the theme CSS content, creating the default if missing
pub fn get_theme_css() -> Result<String, String> {
    let theme_path = get_theme_css_path()?;

    if !theme_path.exists() {
        fs::write(&theme_path, DEFAULT_THEME_CSS)
            .map_err(|e| format!("Failed to create default theme CSS: {}", e))?;
    }

    fs::read_to_string(&theme_path).map_err(|e| format!("Failed to read theme CSS: {}", e))
}

/// Ensure the matugen template exists and return its path
pub fn ensure_matugen_template() -> Result<String, String> {
    let template_path = get_matugen_template_path()?;

    if !template_path.exists() {
        fs::write(&template_path, MATUGEN_TEMPLATE)
            .map_err(|e| format!("Failed to create matugen template: {}", e))?;
    }

    Ok(template_path.to_string_lossy().to_string())
}
