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

    let mut config = Config::new();
    register_hyprpaper_handlers(&mut config);

    match config.parse_file(&config_path) {
        Ok(_) => {
            println!("✓ Successfully parsed hyprpaper config!");

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

            // Print wallpapers
            if let Some(wallpapers) = all_handlers.get("wallpaper") {
                println!("\nWallpapers ({}):", wallpapers.len());
                for (i, wallpaper) in wallpapers.iter().enumerate() {
                    if let Some((monitor, path)) = wallpaper.split_once(',') {
                        let monitor_display = if monitor.is_empty() {
                            "all monitors".to_string()
                        } else {
                            monitor.to_string()
                        };
                        println!("  [{}] {} -> {}", i, monitor_display, path);
                    } else {
                        println!("  [{}] {}", i, wallpaper);
                    }
                }
            } else {
                println!("\nNo wallpapers found");
            }
        }
        Err(e) => {
            eprintln!("✗ Failed to parse hyprpaper config: {:?}", e);
        }
    }
}
