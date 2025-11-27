use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml_edit::{value, DocumentMut};

/// Matugen preferences structure
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MatugenPreferences {
    pub enable: bool,
    pub light_mode: bool,
    pub generator_type: String,
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

/// Tauri command to update matugen preferences
#[tauri::command]
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
const DEFAULT_THEME_CSS: &str = r#"/* Hyprconfig Theme - Generated Default
 * This file is loaded by the app and can be overwritten by matugen.
 * To use matugen, add the following to ~/.config/matugen/config.toml:
 *
 * [templates.hyprconfig]
 * input_path = "~/.config/matugen/templates/hyprconfig.css"
 * output_path = "~/.config/hyprconfig/theme.css"
 */

:root {
  /* Light theme - Sidebar */
  --sidebar: hsl(0 0% 98%);
  --sidebar-foreground: hsl(240 5.3% 26.1%);
  --sidebar-primary: hsl(240 5.9% 10%);
  --sidebar-primary-foreground: hsl(0 0% 98%);
  --sidebar-accent: hsl(240 4.8% 95.9%);
  --sidebar-accent-foreground: hsl(240 5.9% 10%);
  --sidebar-border: hsl(220 13% 91%);
  --sidebar-ring: hsl(217.2 91.2% 59.8%);

  /* Light theme - Base colors */
  --background: hsl(0 0% 100%);
  --foreground: hsl(240 10% 3.9%);
  --card: hsl(0 0% 100%);
  --card-foreground: hsl(240 10% 3.9%);
  --popover: hsl(0 0% 100%);
  --popover-foreground: hsl(240 10% 3.9%);
  --primary: hsl(240 5.9% 10%);
  --primary-foreground: hsl(0 0% 98%);
  --secondary: hsl(240 4.8% 95.9%);
  --secondary-foreground: hsl(240 5.9% 10%);
  --muted: hsl(240 4.8% 95.9%);
  --muted-foreground: hsl(240 3.8% 46.1%);
  --accent: hsl(240 4.8% 95.9%);
  --accent-foreground: hsl(240 5.9% 10%);
  --destructive: hsl(0 84.2% 60.2%);
  --destructive-foreground: hsl(0 0% 98%);
  --border: hsl(240 5.9% 90%);
  --input: hsl(240 5.9% 90%);
  --ring: hsl(240 5.9% 10%);

  /* Tailwind @theme overrides */
  --color-sidebar: hsl(0 0% 98%);
  --color-sidebar-foreground: hsl(240 5.3% 26.1%);
  --color-sidebar-primary: hsl(240 5.9% 10%);
  --color-sidebar-primary-foreground: hsl(0 0% 98%);
  --color-sidebar-accent: hsl(240 4.8% 95.9%);
  --color-sidebar-accent-foreground: hsl(240 5.9% 10%);
  --color-sidebar-border: hsl(220 13% 91%);
  --color-sidebar-ring: hsl(217.2 91.2% 59.8%);
  --color-background: hsl(0 0% 100%);
  --color-foreground: hsl(240 10% 3.9%);
  --color-card: hsl(0 0% 100%);
  --color-card-foreground: hsl(240 10% 3.9%);
  --color-popover: hsl(0 0% 100%);
  --color-popover-foreground: hsl(240 10% 3.9%);
  --color-primary: hsl(240 5.9% 10%);
  --color-primary-foreground: hsl(0 0% 98%);
  --color-secondary: hsl(240 4.8% 95.9%);
  --color-secondary-foreground: hsl(240 5.9% 10%);
  --color-muted: hsl(240 4.8% 95.9%);
  --color-muted-foreground: hsl(240 3.8% 46.1%);
  --color-accent: hsl(240 4.8% 95.9%);
  --color-accent-foreground: hsl(240 5.9% 10%);
  --color-destructive: hsl(0 84.2% 60.2%);
  --color-destructive-foreground: hsl(0 0% 98%);
  --color-border: hsl(240 5.9% 90%);
  --color-input: hsl(240 5.9% 90%);
  --color-ring: hsl(240 5.9% 10%);
}

.dark {
  /* Tokyo Night Theme - Sidebar */
  --sidebar: hsl(235 16% 13%);
  --sidebar-foreground: hsl(230 70% 85%);
  --sidebar-primary: hsl(215 90% 73%);
  --sidebar-primary-foreground: hsl(235 16% 13%);
  --sidebar-accent: hsl(226 25% 22%);
  --sidebar-accent-foreground: hsl(230 70% 85%);
  --sidebar-border: hsl(228 24% 19%);
  --sidebar-ring: hsl(189 100% 74%);

  /* Tokyo Night Theme - Base colors */
  --background: hsl(235 16% 13%);
  --foreground: hsl(230 70% 85%);
  --card: hsl(240 14% 11%);
  --card-foreground: hsl(230 70% 85%);
  --popover: hsl(240 14% 11%);
  --popover-foreground: hsl(230 70% 85%);
  --primary: hsl(215 90% 73%);
  --primary-foreground: hsl(235 16% 13%);
  --secondary: hsl(226 25% 22%);
  --secondary-foreground: hsl(230 70% 85%);
  --muted: hsl(226 25% 22%);
  --muted-foreground: hsl(225 24% 44%);
  --accent: hsl(189 100% 74%);
  --accent-foreground: hsl(235 16% 13%);
  --destructive: hsl(347 88% 72%);
  --destructive-foreground: hsl(235 16% 13%);
  --border: hsl(228 24% 19%);
  --input: hsl(228 24% 19%);
  --ring: hsl(189 100% 74%);

  /* Tailwind @theme overrides */
  --color-sidebar: hsl(235 16% 13%);
  --color-sidebar-foreground: hsl(230 70% 85%);
  --color-sidebar-primary: hsl(215 90% 73%);
  --color-sidebar-primary-foreground: hsl(235 16% 13%);
  --color-sidebar-accent: hsl(226 25% 22%);
  --color-sidebar-accent-foreground: hsl(230 70% 85%);
  --color-sidebar-border: hsl(228 24% 19%);
  --color-sidebar-ring: hsl(189 100% 74%);
  --color-background: hsl(235 16% 13%);
  --color-foreground: hsl(230 70% 85%);
  --color-card: hsl(240 14% 11%);
  --color-card-foreground: hsl(230 70% 85%);
  --color-popover: hsl(240 14% 11%);
  --color-popover-foreground: hsl(230 70% 85%);
  --color-primary: hsl(215 90% 73%);
  --color-primary-foreground: hsl(235 16% 13%);
  --color-secondary: hsl(226 25% 22%);
  --color-secondary-foreground: hsl(230 70% 85%);
  --color-muted: hsl(226 25% 22%);
  --color-muted-foreground: hsl(225 24% 44%);
  --color-accent: hsl(189 100% 74%);
  --color-accent-foreground: hsl(235 16% 13%);
  --color-destructive: hsl(347 88% 72%);
  --color-destructive-foreground: hsl(235 16% 13%);
  --color-border: hsl(228 24% 19%);
  --color-input: hsl(228 24% 19%);
  --color-ring: hsl(189 100% 74%);
}
"#;

/// Matugen template for generating theme CSS
const MATUGEN_TEMPLATE: &str = r#"/* Hyprconfig Theme - Matugen Template
 * This template generates theme.css using matugen colors.
 * Configure matugen to use this template by adding to ~/.config/matugen/config.toml:
 *
 * [templates.hyprconfig]
 * input_path = "~/.config/matugen/templates/hyprconfig.css"
 * output_path = "~/.config/hyprconfig/theme.css"
 */

<* if {{is_dark_mode}} *>
.dark {
<* else *>
:root {
<* endif *>
  /* Sidebar */
  --sidebar: {{colors.surface.default.hex}};
  --sidebar-foreground: {{colors.on_surface.default.hex}};
  --sidebar-primary: {{colors.primary.default.hex}};
  --sidebar-primary-foreground: {{colors.on_primary.default.hex}};
  --sidebar-accent: {{colors.secondary_container.default.hex}};
  --sidebar-accent-foreground: {{colors.on_secondary_container.default.hex}};
  --sidebar-border: {{colors.outline_variant.default.hex}};
  --sidebar-ring: {{colors.primary.default.hex}};

  /* Base colors */
  --background: {{colors.surface.default.hex}};
  --foreground: {{colors.on_surface.default.hex}};
  --card: {{colors.surface_container.default.hex}};
  --card-foreground: {{colors.on_surface.default.hex}};
  --popover: {{colors.surface_container_high.default.hex}};
  --popover-foreground: {{colors.on_surface.default.hex}};
  --primary: {{colors.primary.default.hex}};
  --primary-foreground: {{colors.on_primary.default.hex}};
  --secondary: {{colors.secondary_container.default.hex}};
  --secondary-foreground: {{colors.on_secondary_container.default.hex}};
  --muted: {{colors.surface_variant.default.hex}};
  --muted-foreground: {{colors.on_surface_variant.default.hex}};
  --accent: {{colors.tertiary.default.hex}};
  --accent-foreground: {{colors.on_tertiary.default.hex}};
  --destructive: {{colors.error.default.hex}};
  --destructive-foreground: {{colors.on_error.default.hex}};
  --border: {{colors.outline_variant.default.hex}};
  --input: {{colors.outline_variant.default.hex}};
  --ring: {{colors.primary.default.hex}};

  /* Tailwind @theme overrides */
  --color-sidebar: {{colors.surface.default.hex}};
  --color-sidebar-foreground: {{colors.on_surface.default.hex}};
  --color-sidebar-primary: {{colors.primary.default.hex}};
  --color-sidebar-primary-foreground: {{colors.on_primary.default.hex}};
  --color-sidebar-accent: {{colors.secondary_container.default.hex}};
  --color-sidebar-accent-foreground: {{colors.on_secondary_container.default.hex}};
  --color-sidebar-border: {{colors.outline_variant.default.hex}};
  --color-sidebar-ring: {{colors.primary.default.hex}};
  --color-background: {{colors.surface.default.hex}};
  --color-foreground: {{colors.on_surface.default.hex}};
  --color-card: {{colors.surface_container.default.hex}};
  --color-card-foreground: {{colors.on_surface.default.hex}};
  --color-popover: {{colors.surface_container_high.default.hex}};
  --color-popover-foreground: {{colors.on_surface.default.hex}};
  --color-primary: {{colors.primary.default.hex}};
  --color-primary-foreground: {{colors.on_primary.default.hex}};
  --color-secondary: {{colors.secondary_container.default.hex}};
  --color-secondary-foreground: {{colors.on_secondary_container.default.hex}};
  --color-muted: {{colors.surface_variant.default.hex}};
  --color-muted-foreground: {{colors.on_surface_variant.default.hex}};
  --color-accent: {{colors.tertiary.default.hex}};
  --color-accent-foreground: {{colors.on_tertiary.default.hex}};
  --color-destructive: {{colors.error.default.hex}};
  --color-destructive-foreground: {{colors.on_error.default.hex}};
  --color-border: {{colors.outline_variant.default.hex}};
  --color-input: {{colors.outline_variant.default.hex}};
  --color-ring: {{colors.primary.default.hex}};
}
"#;

/// Get the theme CSS file path (XDG_CONFIG_HOME/hyprconfig/theme.css)
fn get_theme_css_path() -> Result<PathBuf, String> {
    let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else {
        let home_dir = std::env::var("HOME")
            .map_err(|_| "Could not determine home directory".to_string())?;
        PathBuf::from(home_dir).join(".config")
    };

    let hyprconfig_dir = config_dir.join("hyprconfig");

    if !hyprconfig_dir.exists() {
        fs::create_dir_all(&hyprconfig_dir)
            .map_err(|e| format!("Failed to create config directory: {}", e))?;
    }

    Ok(hyprconfig_dir.join("theme.css"))
}

/// Get the matugen template file path (~/.config/matugen/templates/hyprconfig.css)
fn get_matugen_template_path() -> Result<PathBuf, String> {
    let home_dir = std::env::var("HOME")
        .map_err(|_| "Could not determine home directory".to_string())?;

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

/// Tauri command to get theme CSS content, creating default if missing
#[tauri::command]
pub fn get_theme_css() -> Result<String, String> {
    let theme_path = get_theme_css_path()?;

    if !theme_path.exists() {
        fs::write(&theme_path, DEFAULT_THEME_CSS)
            .map_err(|e| format!("Failed to create default theme CSS: {}", e))?;
    }

    fs::read_to_string(&theme_path)
        .map_err(|e| format!("Failed to read theme CSS: {}", e))
}

/// Tauri command to ensure matugen template exists
#[tauri::command]
pub fn ensure_matugen_template() -> Result<String, String> {
    let template_path = get_matugen_template_path()?;

    if !template_path.exists() {
        fs::write(&template_path, MATUGEN_TEMPLATE)
            .map_err(|e| format!("Failed to create matugen template: {}", e))?;
    }

    Ok(template_path.to_string_lossy().to_string())
}
