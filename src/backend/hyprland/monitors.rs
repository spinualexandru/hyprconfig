//! Connected monitors as `hyprctl monitors -j` reports them, and applying a layout live.

use crate::backend::hyprland_lua as lua;
use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DisplayMode {
    pub width: u16,
    pub height: u16,
    pub refresh_rate: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorInfo {
    pub id: i128,
    pub name: String,
    pub description: String,
    pub width: u16,
    pub height: u16,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
    pub transform: String,
    pub active_workspace_id: i32,
    pub active_workspace_name: String,
    pub available_modes: Vec<DisplayMode>,
}

impl MonitorInfo {
    /// Width over height as the screen shows it: a monitor turned 90 or 270 degrees swaps its
    /// mode's sides.
    pub fn aspect(&self) -> f32 {
        let (w, h) = (f32::from(self.width), f32::from(self.height));
        if self.transform.ends_with("90") || self.transform.ends_with("270") {
            h / w
        } else {
            w / h
        }
    }
}

pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    let output = Command::new("hyprctl")
        .args(["monitors", "-j"])
        .output()
        .map_err(|e| format!("Failed to run hyprctl: {e}. Make sure Hyprland is running."))?;
    if !output.status.success() {
        return Err(format!(
            "Failed to get monitor info: {}. Make sure Hyprland is running.",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    parse_monitors_json(&output.stdout)
}

/// `hyprctl monitors -j`, read leniently: Hyprland adds and drops fields between releases
/// (0.57 leaves `specialWorkspace.id` out when none is shown), so only what the app shows is
/// required.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct HyprctlMonitor {
    id: i128,
    name: String,
    #[serde(default)]
    description: String,
    width: u16,
    height: u16,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
    #[serde(default)]
    transform: i64,
    #[serde(default)]
    active_workspace: HyprctlWorkspace,
    #[serde(default)]
    available_modes: Vec<String>,
}

#[derive(Deserialize, Default)]
struct HyprctlWorkspace {
    #[serde(default)]
    id: i32,
    #[serde(default)]
    name: String,
}

/// Hyprland's `wl_output_transform` numbers, named as the `hyprland` crate named them.
fn transform_name(t: i64) -> String {
    match t {
        0 => "Normal",
        1 => "Normal90",
        2 => "Normal180",
        3 => "Normal270",
        4 => "Flipped",
        5 => "Flipped90",
        6 => "Flipped180",
        7 => "Flipped270",
        _ => return format!("Unknown ({t})"),
    }
    .to_string()
}

fn parse_monitors_json(json: &[u8]) -> Result<Vec<MonitorInfo>, String> {
    let monitors: Vec<HyprctlMonitor> = serde_json::from_slice(json)
        .map_err(|e| format!("Failed to read hyprctl's monitor list: {e}"))?;
    Ok(monitors
        .into_iter()
        .map(|m| MonitorInfo {
            id: m.id,
            name: m.name,
            description: m.description,
            width: m.width,
            height: m.height,
            refresh_rate: m.refresh_rate,
            x: m.x,
            y: m.y,
            scale: m.scale,
            transform: transform_name(m.transform),
            active_workspace_id: m.active_workspace.id,
            active_workspace_name: m.active_workspace.name,
            available_modes: parse_available_modes(&m.available_modes.join(" ")),
        })
        .collect())
}

fn parse_available_modes(modes_str: &str) -> Vec<DisplayMode> {
    let mut modes = Vec::new();

    // Parse modes like "2560x1600@240.00Hz 2560x1600@60.00Hz"
    for mode_str in modes_str.split_whitespace() {
        if let Some((resolution, refresh)) = mode_str.split_once('@')
            && let Some((width_str, height_str)) = resolution.split_once('x')
            && let (Ok(width), Ok(height)) = (width_str.parse::<u16>(), height_str.parse::<u16>())
        {
            // Parse refresh rate (remove "Hz" suffix)
            let refresh_str = refresh.trim_end_matches("Hz");
            if let Ok(refresh_rate) = refresh_str.parse::<f32>() {
                modes.push(DisplayMode {
                    width,
                    height,
                    refresh_rate,
                });
            }
        }
    }

    modes
}

/// Applies a monitor layout to the running session (`hyprctl eval`), without saving it.
pub fn apply_monitor_settings(
    name: String,
    width: u16,
    height: u16,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
) -> Result<(), String> {
    let layout = lua::MonitorLayout {
        width,
        height,
        refresh_rate,
        x,
        y,
        scale,
    };
    let output = Command::new("hyprctl")
        .args(["eval", &lua::monitor_eval_code(&name, &layout)])
        .output()
        .map_err(|e| format!("Failed to run hyprctl: {}", e))?;

    // hyprctl exits 0 even when Hyprland rejects the request, so check the reply too.
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if !output.status.success() || stdout != "ok" {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let detail = if stdout.is_empty() { stderr } else { stdout };
        return Err(format!("Failed to apply monitor settings: {}", detail));
    }

    Ok(())
}

#[cfg(test)]
mod monitor_tests {
    use super::parse_monitors_json;

    /// Hyprland 0.57.2's output for a laptop panel with no special workspace shown: that
    /// object has no `id`, which the `hyprland` crate rejected.
    const HYPRLAND_0_57: &str = r#"[{
        "id": 0, "name": "eDP-1", "description": "BOE NE160QDM-NM7", "make": "BOE",
        "width": 2560, "height": 1600, "physicalWidth": 340, "physicalHeight": 220,
        "refreshRate": 240.00000, "x": 0, "y": 0,
        "activeWorkspace": { "address": "2", "id": 2, "type": "normal", "name": "2" },
        "specialWorkspace": { "address": "", "type": "", "name": "" },
        "reserved": [0, 34, 0, 0], "scale": 1, "transform": 0, "focused": true,
        "solitaryBlockedBy": ["WINDOWED","CANDIDATE"], "disabled": false,
        "availableModes": ["2560x1600@240.00Hz","2560x1600@60.00Hz"],
        "sdrBrightness": 1
    }]"#;

    #[test]
    fn reads_hyprland_0_57_monitors() {
        let m = parse_monitors_json(HYPRLAND_0_57.as_bytes()).unwrap();
        assert_eq!(m.len(), 1);
        let m = &m[0];
        assert_eq!(
            (m.id, m.name.as_str(), m.width, m.height),
            (0, "eDP-1", 2560, 1600)
        );
        assert_eq!(m.refresh_rate, 240.0);
        assert_eq!(m.scale, 1.0);
        assert_eq!(m.transform, "Normal");
        assert_eq!(
            (m.active_workspace_id, m.active_workspace_name.as_str()),
            (2, "2")
        );
        assert_eq!(m.available_modes.len(), 2);
        assert_eq!(m.available_modes[1].refresh_rate, 60.0);
    }

    #[test]
    fn a_broken_list_is_an_error_not_a_panic() {
        assert!(parse_monitors_json(b"not json").is_err());
        assert!(parse_monitors_json(b"[]").unwrap().is_empty());
    }
}
