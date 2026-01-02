use serde::{Deserialize, Serialize};
use std::process::Command;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub device_type: String, // "sink" or "source"
    pub volume: f32,         // 0.0 - 1.5 (allows boost)
    pub muted: bool,
    pub is_default: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AudioStream {
    pub id: u32,
    pub app_name: String,
    pub media_name: Option<String>,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioState {
    pub sinks: Vec<AudioDevice>,
    pub sources: Vec<AudioDevice>,
    pub streams: Vec<AudioStream>,
}

/// Check if wpctl is available on the system
fn check_wpctl_available() -> Result<(), String> {
    Command::new("which")
        .arg("wpctl")
        .output()
        .map_err(|e| format!("Failed to check for wpctl: {}", e))
        .and_then(|output| {
            if output.status.success() {
                Ok(())
            } else {
                Err("wpctl not found. Please install WirePlumber to manage audio.".to_string())
            }
        })
}

/// Get volume for a specific node ID
fn get_node_volume(node_id: u32) -> Result<(f32, bool), String> {
    let output = Command::new("wpctl")
        .args(["get-volume", &node_id.to_string()])
        .output()
        .map_err(|e| format!("Failed to get volume for node {}: {}", node_id, e))?;

    if !output.status.success() {
        return Err(format!(
            "wpctl get-volume failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Format: "Volume: 0.30" or "Volume: 0.30 [MUTED]"
    let muted = stdout.contains("[MUTED]");
    let volume = stdout
        .split_whitespace()
        .nth(1)
        .and_then(|v| v.parse::<f32>().ok())
        .unwrap_or(0.0);

    Ok((volume, muted))
}

/// Parse a device/sink/source line from wpctl status
/// Format: " *   53. Built-in Audio Analog Stereo        [vol: 0.30]"
/// or:     "     53. Built-in Audio Analog Stereo        [vol: 0.30]"
fn parse_device_line(line: &str, device_type: &str) -> Option<AudioDevice> {
    let trimmed = line.trim_start_matches(&[' ', '│', '├', '└', '─'][..]);
    let is_default = trimmed.starts_with('*');
    let content = trimmed.trim_start_matches(&['*', ' '][..]);

    // Find the ID (number before the dot)
    let dot_pos = content.find('.')?;
    let id_str = content[..dot_pos].trim();
    let id: u32 = id_str.parse().ok()?;

    // Get the rest after the dot
    let rest = content[dot_pos + 1..].trim();

    // Extract volume if present (format: [vol: 0.30])
    let (description, _inline_volume) = if let Some(vol_start) = rest.find("[vol:") {
        let desc = rest[..vol_start].trim();
        let vol_end = rest[vol_start..].find(']').unwrap_or(rest.len() - vol_start);
        let vol_str = &rest[vol_start + 5..vol_start + vol_end];
        let vol: f32 = vol_str.trim().parse().unwrap_or(0.0);
        (desc, Some(vol))
    } else {
        (rest, None)
    };

    Some(AudioDevice {
        id,
        name: description.to_string(),
        description: description.to_string(),
        device_type: device_type.to_string(),
        volume: 0.0, // Will be filled later with wpctl get-volume
        muted: false,
        is_default,
    })
}

/// Parse a stream line from wpctl status
/// Format: "     64. firefox: AudioStream [vol: 0.50]"
fn parse_stream_line(line: &str) -> Option<AudioStream> {
    let trimmed = line.trim_start_matches(&[' ', '│', '├', '└', '─'][..]).trim();

    // Find the ID (number before the dot)
    let dot_pos = trimmed.find('.')?;
    let id_str = trimmed[..dot_pos].trim().trim_start_matches('*').trim();
    let id: u32 = id_str.parse().ok()?;

    // Get the rest after the dot
    let rest = trimmed[dot_pos + 1..].trim();

    // Parse app name (before colon or the whole name)
    let (app_name, media_name) = if let Some(colon_pos) = rest.find(':') {
        let app = rest[..colon_pos].trim();
        let media = rest[colon_pos + 1..].split('[').next().unwrap_or("").trim();
        (app, if media.is_empty() { None } else { Some(media) })
    } else {
        let name = rest.split('[').next().unwrap_or(rest).trim();
        (name, None)
    };

    Some(AudioStream {
        id,
        app_name: app_name.to_string(),
        media_name: media_name.map(|s| s.to_string()),
        volume: 0.0, // Will be filled later
        muted: false,
    })
}

#[tauri::command]
pub fn get_audio_state() -> Result<AudioState, String> {
    check_wpctl_available()?;

    let output = Command::new("wpctl")
        .arg("status")
        .output()
        .map_err(|e| format!("Failed to run wpctl status: {}", e))?;

    if !output.status.success() {
        return Err(format!(
            "wpctl status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();

    let mut sinks: Vec<AudioDevice> = Vec::new();
    let mut sources: Vec<AudioDevice> = Vec::new();
    let mut streams: Vec<AudioStream> = Vec::new();

    #[derive(PartialEq)]
    enum Section {
        None,
        Sinks,
        Sources,
        Streams,
    }

    let mut current_section = Section::None;
    let mut in_audio_section = false;

    for line in lines {
        // Detect main sections
        if line.starts_with("Audio") {
            in_audio_section = true;
            continue;
        }
        if line.starts_with("Video") || line.starts_with("Settings") {
            in_audio_section = false;
            current_section = Section::None;
            continue;
        }

        if !in_audio_section {
            continue;
        }

        // Detect subsections within Audio
        if line.contains("Sinks:") {
            current_section = Section::Sinks;
            continue;
        }
        if line.contains("Sources:") {
            current_section = Section::Sources;
            continue;
        }
        if line.contains("Streams:") {
            current_section = Section::Streams;
            continue;
        }
        if line.contains("Devices:") || line.contains("Filters:") {
            current_section = Section::None;
            continue;
        }

        // Skip empty or tree-only lines
        let content = line
            .trim_start_matches(&[' ', '│', '├', '└', '─'][..])
            .trim();
        if content.is_empty() {
            continue;
        }

        // Parse based on current section
        match current_section {
            Section::Sinks => {
                if let Some(device) = parse_device_line(line, "sink") {
                    sinks.push(device);
                }
            }
            Section::Sources => {
                if let Some(device) = parse_device_line(line, "source") {
                    sources.push(device);
                }
            }
            Section::Streams => {
                if let Some(stream) = parse_stream_line(line) {
                    streams.push(stream);
                }
            }
            Section::None => {}
        }
    }

    // Fetch actual volume for each device and stream
    for sink in &mut sinks {
        if let Ok((vol, muted)) = get_node_volume(sink.id) {
            sink.volume = vol;
            sink.muted = muted;
        }
    }

    for source in &mut sources {
        if let Ok((vol, muted)) = get_node_volume(source.id) {
            source.volume = vol;
            source.muted = muted;
        }
    }

    for stream in &mut streams {
        if let Ok((vol, muted)) = get_node_volume(stream.id) {
            stream.volume = vol;
            stream.muted = muted;
        }
    }

    Ok(AudioState {
        sinks,
        sources,
        streams,
    })
}

#[tauri::command]
pub fn set_default_device(device_id: u32) -> Result<(), String> {
    check_wpctl_available()?;

    let output = Command::new("wpctl")
        .args(["set-default", &device_id.to_string()])
        .output()
        .map_err(|e| format!("Failed to set default device: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to set default device: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[tauri::command]
pub fn set_volume(node_id: u32, volume: f32) -> Result<(), String> {
    check_wpctl_available()?;

    // Clamp volume to reasonable range (0.0 - 1.5 for 150% boost)
    let vol = volume.clamp(0.0, 1.5);
    let vol_str = format!("{:.2}", vol);

    let output = Command::new("wpctl")
        .args(["set-volume", &node_id.to_string(), &vol_str])
        .output()
        .map_err(|e| format!("Failed to set volume: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to set volume: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}

#[tauri::command]
pub fn set_mute(node_id: u32, muted: bool) -> Result<(), String> {
    check_wpctl_available()?;

    let mute_val = if muted { "1" } else { "0" };

    let output = Command::new("wpctl")
        .args(["set-mute", &node_id.to_string(), mute_val])
        .output()
        .map_err(|e| format!("Failed to set mute: {}", e))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "Failed to set mute state: {}",
            String::from_utf8_lossy(&output.stderr)
        ))
    }
}
