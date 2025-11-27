use std::process::Command;

#[tauri::command]
pub fn tool_exists(name: String) -> Result<bool, String> {
    let output = Command::new("which")
        .arg(&name)
        .output()
        .map_err(|e| format!("Failed to execute which: {}", e))?;

    Ok(output.status.success())
}

#[tauri::command]
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
