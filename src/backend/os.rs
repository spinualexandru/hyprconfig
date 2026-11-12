use std::path::PathBuf;
use std::process::Command;

/// `$XDG_CONFIG_HOME`, or `$HOME/.config` when it is unset (or not valid UTF-8). A set but
/// empty value is used as is.
pub fn config_home() -> Result<PathBuf, String> {
    match std::env::var("XDG_CONFIG_HOME") {
        Ok(xdg_config) => Ok(PathBuf::from(xdg_config)),
        Err(_) => std::env::var("HOME")
            .map(|home| PathBuf::from(home).join(".config"))
            .map_err(|_| "Could not determine home directory".to_string()),
    }
}

/// Whether `name` is an executable on `PATH`, as `which` would find it, without starting a
/// process to ask.
pub fn tool_exists(name: &str) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|dir| {
            std::fs::metadata(dir.join(name))
                .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        })
    })
}

pub fn run_matugen(
    image_path: String,
    light_mode: bool,
    generator_type: String,
) -> Result<(), String> {
    let mut args = vec!["image", &image_path];
    if light_mode {
        args.push("--mode");
        args.push("light");
    }
    args.push("--type");
    args.push(&generator_type);
    let output = Command::new("matugen")
        .args(&args)
        .output()
        .map_err(|e| format!("Failed to execute matugen: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Matugen failed: {}", stderr))
    }
}
