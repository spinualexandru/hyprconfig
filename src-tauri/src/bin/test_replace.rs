use std::path::Path;
use hyprlang::Config;

fn register_hyprpaper_handlers(config: &mut Config) {
    let keywords = vec!["preload", "wallpaper", "splash", "ipc"];
    for keyword in keywords {
        config.register_handler_fn(keyword, |_ctx| Ok(()));
    }
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
    register_hyprpaper_handlers(&mut config);
    config.parse_file(&config_path).expect("Failed to parse");

    let all_handlers = config.all_handler_calls();
    println!("Preloads: {:?}", all_handlers.get("preload"));
    println!("Wallpapers: {:?}", all_handlers.get("wallpaper"));

    // Get counts
    let preload_count = all_handlers.get("preload").map(|v| v.len()).unwrap_or(0);
    let wallpaper_count = all_handlers.get("wallpaper").map(|v| v.len()).unwrap_or(0);

    println!("\nRemoving {} preloads and {} wallpapers...", preload_count, wallpaper_count);

    // Remove all preloads
    for i in (0..preload_count).rev() {
        println!("Removing preload at index {}", i);
        config.remove_handler_call("preload", i).expect("Failed to remove preload");
    }

    // Remove all wallpapers
    for i in (0..wallpaper_count).rev() {
        println!("Removing wallpaper at index {}", i);
        config.remove_handler_call("wallpaper", i).expect("Failed to remove wallpaper");
    }

    println!("\nAdding new wallpaper...");
    config.add_handler_call("preload", "/tmp/test.jpg".to_string()).expect("Failed to add preload");
    config.add_handler_call("wallpaper", ",/tmp/test.jpg".to_string()).expect("Failed to add wallpaper");

    println!("\n=== AFTER REPLACEMENT (before save) ===");
    let all_handlers = config.all_handler_calls();
    println!("Preloads: {:?}", all_handlers.get("preload"));
    println!("Wallpapers: {:?}", all_handlers.get("wallpaper"));

    // Save to a temp file for testing
    let test_path = Path::new("/tmp/hyprpaper_test.conf");
    config.save_as(test_path).expect("Failed to save");

    println!("\nSaved to /tmp/hyprpaper_test.conf");
    println!("Contents:");
    let contents = std::fs::read_to_string(test_path).expect("Failed to read test file");
    println!("{}", contents);
}
