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

    println!("=== BEFORE REPLACEMENT ===");
    let mut config = Config::new();
    register_hyprpaper_config(&mut config);
    config.parse_file(&config_path).expect("Failed to parse");

    let all_handlers = config.all_handler_calls();
    println!("Preloads: {:?}", all_handlers.get("preload"));

    let wallpaper_keys = config.list_special_category_keys("wallpaper");
    println!("Wallpapers: {} instances", wallpaper_keys.len());
    for key in &wallpaper_keys {
        if let Ok(instance) = config.get_special_category("wallpaper", key) {
            let path = instance.get("path").and_then(|v| v.as_string().ok()).unwrap_or("");
            println!("  [{}] path={}", key, path);
        }
    }

    // Remove all preloads
    let preload_count = all_handlers.get("preload").map(|v| v.len()).unwrap_or(0);
    println!("\nRemoving {} preloads...", preload_count);
    for i in (0..preload_count).rev() {
        config.remove_handler_call("preload", i).expect("Failed to remove preload");
    }

    // Remove all wallpaper instances
    println!("Removing {} wallpaper instances...", wallpaper_keys.len());
    for key in &wallpaper_keys {
        config.remove_special_category_instance("wallpaper", key).expect("Failed to remove wallpaper");
    }

    println!("\nAdding new wallpaper...");
    config.add_handler_call("preload", "/tmp/test.jpg".to_string()).expect("Failed to add preload");

    // Save to a temp file for testing
    let test_path = Path::new("/tmp/hyprpaper_test.conf");
    config.save_as(test_path).expect("Failed to save");

    println!("\nSaved to /tmp/hyprpaper_test.conf");
    println!("Contents:");
    let contents = std::fs::read_to_string(test_path).expect("Failed to read test file");
    println!("{}", contents);
}
