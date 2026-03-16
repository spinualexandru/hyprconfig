use std::path::Path;
use hyprlang::{Config, ConfigValue, SpecialCategoryDescriptor};

fn register_hyprpaper_config(config: &mut Config) {
    let keywords = vec!["preload", "splash", "splash_offset", "splash_opacity", "ipc"];
    for keyword in keywords {
        config.register_handler_fn(keyword, |_ctx| Ok(()));
    }

    config.register_special_category(SpecialCategoryDescriptor::anonymous("wallpaper"));
    config.register_special_category_value("wallpaper", "monitor", ConfigValue::String(String::new()));
    config.register_special_category_value("wallpaper", "path", ConfigValue::String(String::new()));
    config.register_special_category_value("wallpaper", "fit_mode", ConfigValue::String("cover".to_string()));
}

fn main() {
    let home_dir = std::env::var("HOME").expect("Could not determine home directory");
    let config_path = Path::new(&home_dir).join(".config/hypr/hyprpaper.conf");

    if !config_path.exists() {
        eprintln!("Hyprpaper config file not found at {:?}", config_path);
        return;
    }

    let mut config = Config::new();
    register_hyprpaper_config(&mut config);

    match config.parse_file(&config_path) {
        Ok(_) => {
            println!("Successfully parsed hyprpaper config!");

            let all_handlers = config.all_handler_calls();

            // Print preloads
            if let Some(preloads) = all_handlers.get("preload") {
                println!("\nPreloads ({}):", preloads.len());
                for (i, preload) in preloads.iter().enumerate() {
                    println!("  [{}] {}", i, preload);
                }
            } else {
                println!("\nNo preloads found");
            }

            // Print wallpapers (special category format)
            let wallpaper_keys = config.list_special_category_keys("wallpaper");
            println!("\nWallpapers ({}):", wallpaper_keys.len());
            for key in wallpaper_keys {
                if let Ok(instance) = config.get_special_category("wallpaper", &key) {
                    let monitor = instance
                        .get("monitor")
                        .and_then(|v| v.as_string().ok())
                        .unwrap_or("");
                    let path = instance
                        .get("path")
                        .and_then(|v| v.as_string().ok())
                        .unwrap_or("");
                    let fit_mode = instance
                        .get("fit_mode")
                        .and_then(|v| v.as_string().ok())
                        .unwrap_or("cover");
                    println!("  [{}] monitor={}, path={}, fit_mode={}", key, monitor, path, fit_mode);
                }
            }
        }
        Err(e) => {
            eprintln!("Failed to parse hyprpaper config: {:?}", e);
        }
    }
}
