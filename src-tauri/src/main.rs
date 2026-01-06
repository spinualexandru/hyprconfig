// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod audio_backend;
mod config_backend;
mod hyprland_backend;
mod hyprpaper_backend;
mod os_backend;
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
            // env var handlers
            hyprland_backend::get_env_vars,
            hyprland_backend::add_env_var,
            hyprland_backend::edit_env_var,
            hyprland_backend::delete_env_var,
            hyprland_backend::add_keybind,
            hyprland_backend::edit_keybind,
            hyprland_backend::delete_keybind,
            hyprland_backend::apply_monitor_settings,
            hyprland_backend::save_monitor_settings,
            // bindu handlers (universal submap bindings)
            hyprland_backend::get_all_bindu,
            hyprland_backend::add_bindu,
            hyprland_backend::delete_bindu,
            // windowrule v3 handlers
            hyprland_backend::get_windowrule_names,
            hyprland_backend::get_windowrule,
            hyprland_backend::delete_windowrule,
            // layerrule v2 handlers
            hyprland_backend::get_layerrule_names,
            hyprland_backend::get_layerrule,
            hyprland_backend::delete_layerrule,
            hyprpaper_backend::get_hyprpaper_config,
            hyprpaper_backend::set_wallpaper,
            hyprpaper_backend::remove_wallpaper,
            hyprpaper_backend::update_wallpaper,
            hyprpaper_backend::replace_wallpaper,
            os_backend::tool_exists,
            os_backend::run_matugen,
            config_backend::get_preferences,
            config_backend::update_matugen_preferences,
            config_backend::get_theme_css,
            config_backend::ensure_matugen_template,
            audio_backend::get_audio_state,
            audio_backend::set_default_device,
            audio_backend::set_volume,
            audio_backend::set_mute,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
