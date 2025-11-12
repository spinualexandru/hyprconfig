use hyprland::data::Monitors;
use hyprland::shared::HyprData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
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
}

#[tauri::command]
pub fn get_monitors() -> Result<Vec<MonitorInfo>, String> {
    match Monitors::get() {
        Ok(monitors) => {
            let monitor_infos: Vec<MonitorInfo> = monitors
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
                    transform: format!("{:?}", m.transform),
                    active_workspace_id: m.active_workspace.id,
                    active_workspace_name: m.active_workspace.name,
                })
                .collect();
            Ok(monitor_infos)
        }
        Err(e) => Err(format!("Failed to get monitor info: {}", e)),
    }
}
