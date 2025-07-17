
pub fn list_hyprland_folder() -> Vec<String> {
    fn visit_dirs(dir: &std::path::Path, files: &mut Vec<String>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    visit_dirs(&path, files);
                } else if let Some(path_str) = path.to_str() {
                    files.push(path_str.to_string());
                }
            }
        }
    }

    let config_dir = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| String::from("/home/user"));
        format!("{}/.config", home)
    });
    let hypr_path = format!("{}/hypr", config_dir);
    let mut config_files = Vec::new();
    visit_dirs(std::path::Path::new(&hypr_path), &mut config_files);
    config_files
}

pub fn list_config_files() -> Vec<String> {
    let hyprland_files = list_hyprland_folder();
    // Filter by .conf extension
    let config_files: Vec<String> = hyprland_files
        .into_iter()
        .filter(|file| file.ends_with(".conf"))
        .collect();
    config_files
}

pub fn get_config_file_content(file_path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(file_path)
}