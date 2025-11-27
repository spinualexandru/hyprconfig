// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod hyprland_backend;
mod hyprpaper_backend;
mod os_backend;
mod config_backend;
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            hyprland_backend::get_monitors,
            hyprland_backend::get_network_info,
            hyprland_backend::scan_wifi_networks,
            hyprland_backend::get_system_info,
            hyprland_backend::get_keybinds,
            hyprland_backend::get_variables,
            hyprland_backend::set_variable,
            hyprland_backend::add_variable,
            hyprland_backend::delete_variable,
            hyprland_backend::add_keybind,
            hyprland_backend::edit_keybind,
            hyprland_backend::delete_keybind,
            hyprpaper_backend::get_hyprpaper_config,
            hyprpaper_backend::add_preload,
            hyprpaper_backend::remove_preload,
            hyprpaper_backend::set_wallpaper,
            hyprpaper_backend::remove_wallpaper,
            hyprpaper_backend::update_wallpaper,
            hyprpaper_backend::replace_wallpaper,
            os_backend::tool_exists,
            os_backend::run_matugen,
            config_backend::get_preferences,
            config_backend::update_matugen_preferences,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
