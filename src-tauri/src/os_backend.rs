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
pub fn run_matugen(image_path: String) -> Result<(), String> {
    let output = Command::new("matugen")
        .args(&["image", &image_path])
        .output()
        .map_err(|e| format!("Failed to execute matugen: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Matugen failed: {}", stderr))
    }
}
