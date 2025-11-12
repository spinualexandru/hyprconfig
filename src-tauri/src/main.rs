// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod hyprland_backend;
fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            hyprland_backend::get_monitors,
            hyprland_backend::get_network_info,
            hyprland_backend::scan_wifi_networks,
            hyprland_backend::get_system_info,
            hyprland_backend::get_keybinds,
            hyprland_backend::get_variables,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
