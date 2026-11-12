use serde::{Deserialize, Serialize};
use std::process::Command;

use super::os::tool_exists;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioDevice {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub device_type: String, // "sink" or "source"
    pub volume: f32,         // 0.0 - 1.5 (allows boost)
    pub muted: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioStream {
    pub id: u32,
    pub app_name: String,
    pub media_name: Option<String>,
    pub volume: f32,
    pub muted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AudioState {
    pub sinks: Vec<AudioDevice>,
    pub sources: Vec<AudioDevice>,
    pub streams: Vec<AudioStream>,
}

/// Check if wpctl is available on the system
fn check_wpctl_available() -> Result<(), String> {
    if tool_exists("wpctl") {
        Ok(())
    } else {
        Err("wpctl not found. Please install WirePlumber to manage audio.".to_string())
    }
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

    // Drop the inline volume if present (format: [vol: 0.30]); the real one is fetched later
    let description = match rest.find("[vol:") {
        Some(vol_start) => rest[..vol_start].trim(),
        None => rest,
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
    let trimmed = line
        .trim_start_matches(&[' ', '│', '├', '└', '─'][..])
        .trim();

    // Find the ID (number before the dot)
    let dot_pos = trimmed.find('.')?;
    let id_str = trimmed[..dot_pos].trim().trim_start_matches('*').trim();
    let id: u32 = id_str.parse().ok()?;

    // Get the rest after the dot, without the inline volume (its colon is not a separator)
    let rest = trimmed[dot_pos + 1..]
        .split('[')
        .next()
        .unwrap_or("")
        .trim();

    // Parse app name (before colon or the whole name)
    let (app_name, media_name) = match rest.split_once(':') {
        Some((app, media)) => {
            let media = media.trim();
            (app.trim(), (!media.is_empty()).then_some(media))
        }
        None => (rest, None),
    };

    Some(AudioStream {
        id,
        app_name: app_name.to_string(),
        media_name: media_name.map(|s| s.to_string()),
        volume: 0.0, // Will be filled later
        muted: false,
    })
}

/// Get streams using pactl (PulseAudio compatibility layer)
fn get_pactl_streams() -> Vec<AudioStream> {
    let output = match Command::new("pactl").args(["list", "sink-inputs"]).output() {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut streams = Vec::new();
    let mut current_stream: Option<AudioStream> = None;

    for line in stdout.lines() {
        let trimmed = line.trim();

        // New sink input starts
        if trimmed.starts_with("Sink Input #") {
            // Save previous stream if exists
            if let Some(stream) = current_stream.take()
                && !stream.app_name.is_empty()
            {
                streams.push(stream);
            }
            // Parse ID from "Sink Input #123"
            if let Some(id_str) = trimmed.strip_prefix("Sink Input #")
                && let Ok(id) = id_str.parse::<u32>()
            {
                current_stream = Some(AudioStream {
                    id,
                    app_name: String::new(),
                    media_name: None,
                    volume: 1.0,
                    muted: false,
                });
            }
        } else if let Some(ref mut stream) = current_stream {
            // Parse properties
            if trimmed.starts_with("Volume:") {
                // Format: "Volume: front-left: 65536 / 100% / 0.00 dB, ..."
                if let Some(pct_start) = trimmed.find('/') {
                    let after_slash = &trimmed[pct_start + 1..];
                    if let Some(pct_end) = after_slash.find('%')
                        && let Ok(pct) = after_slash[..pct_end].trim().parse::<f32>()
                    {
                        stream.volume = pct / 100.0;
                    }
                }
            } else if trimmed.starts_with("Mute:") {
                stream.muted = trimmed.contains("yes");
            } else if trimmed.starts_with("application.name = ") {
                stream.app_name = trimmed
                    .strip_prefix("application.name = ")
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_string();
            } else if trimmed.starts_with("media.name = ") {
                let media = trimmed
                    .strip_prefix("media.name = ")
                    .unwrap_or("")
                    .trim_matches('"')
                    .to_string();
                if !media.is_empty() {
                    stream.media_name = Some(media);
                }
            }
        }
    }

    // Don't forget the last stream
    if let Some(stream) = current_stream
        && !stream.app_name.is_empty()
    {
        streams.push(stream);
    }

    streams
}

/// Get audio streams using pw-dump (works for JACK clients and all PipeWire nodes)
fn get_pw_dump_streams(sink_ids: &[u32], source_ids: &[u32]) -> Vec<AudioStream> {
    let output = match Command::new("pw-dump").output() {
        Ok(o) if o.status.success() => o,
        _ => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = match serde_json::from_str(&stdout) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };

    let mut streams = Vec::new();

    if let Some(array) = json.as_array() {
        for obj in array {
            // Check if it's a Node type
            let node_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if node_type != "PipeWire:Interface:Node" {
                continue;
            }

            let id = obj.get("id").and_then(|i| i.as_u64()).unwrap_or(0) as u32;

            // Skip if this is a sink or source device (we already show those)
            if sink_ids.contains(&id) || source_ids.contains(&id) {
                continue;
            }

            let info = match obj.get("info") {
                Some(i) => i,
                None => continue,
            };

            let props = match info.get("props") {
                Some(p) => p,
                None => continue,
            };

            // Check if this is an audio node
            let media_type = props
                .get("media.type")
                .and_then(|m| m.as_str())
                .unwrap_or("");
            let media_class = props
                .get("media.class")
                .and_then(|m| m.as_str())
                .unwrap_or("");

            // Skip non-audio nodes
            if media_type != "Audio" && !media_class.contains("Audio") {
                continue;
            }

            // Skip device sinks/sources (we show those separately)
            if media_class == "Audio/Sink" || media_class == "Audio/Source" {
                continue;
            }

            // Get application/client name
            let app_name = props
                .get("application.name")
                .or_else(|| props.get("client.name"))
                .or_else(|| props.get("node.description"))
                .or_else(|| props.get("node.name"))
                .and_then(|n| n.as_str())
                .unwrap_or("")
                .to_string();

            // Skip if no name or if it's a system node
            if app_name.is_empty()
                || app_name.contains("Dummy")
                || app_name.contains("Freewheel")
                || app_name.contains("Midi")
            {
                continue;
            }

            // Check if running/idle (active)
            let state = info.get("state").and_then(|s| s.as_str()).unwrap_or("");
            if state != "running" && state != "idle" {
                continue;
            }

            // Get media name if available
            let media_name = props
                .get("media.name")
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());

            // Get volume from wpctl
            let (volume, muted) = get_node_volume(id).unwrap_or((1.0, false));

            streams.push(AudioStream {
                id,
                app_name,
                media_name,
                volume,
                muted,
            });
        }
    }

    streams
}

/// The column where a line's id field starts. wpctl right-aligns ids in a 4-wide
/// field, so " 99." and "100." under the same parent start at the same column.
fn id_field_start(line: &str) -> usize {
    let pre = line.chars().take_while(|c| !c.is_ascii_digit()).count();
    let digits = line
        .chars()
        .skip(pre)
        .take_while(char::is_ascii_digit)
        .count();
    (pre + digits).saturating_sub(digits.max(4))
}

/// Walk `wpctl status` output for the Audio sinks, sources and streams.
/// Volumes are left at 0.0 for `wpctl get-volume` to fill.
fn parse_status(stdout: &str) -> AudioState {
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
    // wpctl nests each stream's ports one level deeper than the stream itself
    let mut stream_indent: Option<usize> = None;

    for line in stdout.lines() {
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
                let indent = id_field_start(line);
                if indent > *stream_indent.get_or_insert(indent) {
                    continue;
                }
                if let Some(stream) = parse_stream_line(line) {
                    streams.push(stream);
                }
            }
            Section::None => {}
        }
    }

    AudioState {
        sinks,
        sources,
        streams,
    }
}

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

    let AudioState {
        mut sinks,
        mut sources,
        mut streams,
    } = parse_status(&String::from_utf8_lossy(&output.stdout));

    // Fetch actual volume for each device
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

    // Collect device IDs to exclude from streams
    let sink_ids: Vec<u32> = sinks.iter().map(|s| s.id).collect();
    let source_ids: Vec<u32> = sources.iter().map(|s| s.id).collect();

    // If wpctl didn't find streams, try other methods
    if streams.is_empty() {
        // Try pactl first (PulseAudio compatibility)
        streams = get_pactl_streams();
    }

    // If still no streams, try pw-dump (works for JACK clients like Firefox)
    if streams.is_empty() {
        streams = get_pw_dump_streams(&sink_ids, &source_ids);
    }

    // Update wpctl streams with volume info if we got them from wpctl
    if !streams.is_empty() && streams.iter().all(|s| s.volume == 0.0) {
        for stream in &mut streams {
            if let Ok((vol, muted)) = get_node_volume(stream.id) {
                stream.volume = vol;
                stream.muted = muted;
            }
        }
    }

    Ok(AudioState {
        sinks,
        sources,
        streams,
    })
}

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

pub fn set_volume(node_id: u32, volume: f32) -> Result<(), String> {
    // Clamp volume to reasonable range (0.0 - 1.5 for 150% boost)
    let vol = volume.clamp(0.0, 1.5);

    // Try wpctl first
    let wpctl_result = Command::new("wpctl")
        .args(["set-volume", &node_id.to_string(), &format!("{:.2}", vol)])
        .output();

    if let Ok(output) = wpctl_result
        && output.status.success()
    {
        return Ok(());
    }

    // If wpctl fails, try pactl (for sink-input streams)
    let pct = (vol * 100.0).round() as u32;
    let pactl_result = Command::new("pactl")
        .args([
            "set-sink-input-volume",
            &node_id.to_string(),
            &format!("{}%", pct),
        ])
        .output();

    match pactl_result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(format!(
            "Failed to set volume: {}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(e) => Err(format!("Failed to set volume: {}", e)),
    }
}

pub fn set_mute(node_id: u32, muted: bool) -> Result<(), String> {
    let mute_val = if muted { "1" } else { "0" };

    // Try wpctl first
    let wpctl_result = Command::new("wpctl")
        .args(["set-mute", &node_id.to_string(), mute_val])
        .output();

    if let Ok(output) = wpctl_result
        && output.status.success()
    {
        return Ok(());
    }

    // If wpctl fails, try pactl (for sink-input streams)
    let pactl_result = Command::new("pactl")
        .args(["set-sink-input-mute", &node_id.to_string(), mute_val])
        .output();

    match pactl_result {
        Ok(output) if output.status.success() => Ok(()),
        Ok(output) => Err(format!(
            "Failed to set mute state: {}",
            String::from_utf8_lossy(&output.stderr)
        )),
        Err(e) => Err(format!("Failed to set mute: {}", e)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn device(line: &str) -> AudioDevice {
        parse_device_line(line, "sink").expect("line should parse")
    }

    fn stream(line: &str) -> AudioStream {
        parse_stream_line(line).expect("line should parse")
    }

    #[test]
    fn a_device_line_gives_its_id_name_and_default_marker() {
        let d = device(" │  *   53. Built-in Audio Analog Stereo        [vol: 0.30]");
        assert_eq!(d.id, 53);
        assert_eq!(d.name, "Built-in Audio Analog Stereo");
        assert_eq!(d.description, d.name);
        assert_eq!(d.device_type, "sink");
        assert!(d.is_default);

        let d = device(" │      54. HDMI / DisplayPort 2 Output         [vol: 1.00]");
        assert_eq!(d.id, 54);
        assert_eq!(d.name, "HDMI / DisplayPort 2 Output");
        assert!(!d.is_default);
    }

    #[test]
    fn a_device_line_leaves_volume_and_mute_for_get_volume_to_fill() {
        let d = device(" │      55. USB Headset [vol: 0.40 MUTED]");
        assert_eq!(d.name, "USB Headset");
        assert_eq!(d.volume, 0.0);
        assert!(!d.muted);
    }

    #[test]
    fn a_device_name_may_contain_dots_and_need_not_have_a_volume() {
        let d = device(" ├─ *   61. USB Audio 2.0 Analog Stereo [vol: 0.75]");
        assert_eq!(d.id, 61);
        assert_eq!(d.name, "USB Audio 2.0 Analog Stereo");
        assert!(d.is_default);

        assert_eq!(device(" │      62. Dummy Output").name, "Dummy Output");
    }

    #[test]
    fn a_device_line_without_a_numeric_id_is_skipped() {
        assert_eq!(parse_device_line(" │  Built-in Audio", "sink"), None);
        assert_eq!(parse_device_line(" │  v1.2 thing", "source"), None);
    }

    #[test]
    fn a_stream_line_splits_app_and_media_name_at_the_colon() {
        let s = stream(" │      64. firefox: AudioStream [vol: 0.50]");
        assert_eq!(s.id, 64);
        assert_eq!(s.app_name, "firefox");
        assert_eq!(s.media_name.as_deref(), Some("AudioStream"));
        assert_eq!(s.volume, 0.0);
        assert!(!s.muted);
    }

    #[test]
    fn a_stream_line_without_a_media_name_has_none() {
        let s = stream(" │      70. Firefox");
        assert_eq!(s.app_name, "Firefox");
        assert_eq!(s.media_name, None);

        assert_eq!(stream("     72. mpv: [vol: 0.80]").media_name, None);
    }

    #[test]
    fn a_stream_without_a_colon_keeps_its_whole_name_before_the_inline_volume() {
        let s = stream(" │      71. Brave Browser [vol: 1.00 MUTED]");
        assert_eq!(s.app_name, "Brave Browser");
        assert_eq!(s.media_name, None);
        assert!(!s.muted);
    }

    #[test]
    fn a_media_name_may_contain_a_colon() {
        let s = stream("     74. mpv: Artist - Song: Remix [vol: 0.80]");
        assert_eq!(s.app_name, "mpv");
        assert_eq!(s.media_name.as_deref(), Some("Artist - Song: Remix"));
    }

    #[test]
    fn a_stream_line_tolerates_a_default_marker() {
        assert_eq!(stream(" │  *   73. spotify").id, 73);
    }

    #[test]
    fn status_lists_streams_without_their_nested_ports() {
        // Shape of real `wpctl status` output; ids are right-aligned in a 4-wide field.
        let status = "\
PipeWire 'pipewire-0' [1.6.11, user@host, cookie:1]
 └─ Clients:
        33. xdg-desktop-portal                  [1.6.11, user@host, pid:2060]

Audio
 ├─ Devices:
 │      55. Built-in Audio                      [alsa]
 │  
 ├─ Sinks:
 │  *   64. Built-in Audio Analog Stereo        [vol: 0.70]
 │      66. USB Audio 2.0: Line Out             [vol: 0.40 MUTED]
 │  
 ├─ Sources:
 │      65. Built-in Audio Analog Stereo        [vol: 0.31]
 │  
 ├─ Filters:
 │  *   37. rnnoise_source                                               [Audio/Source]
 │  
 └─ Streams:
       100. Brave Browser                                               
            105. output_FR       > ALC1220 Analog:playback_FR\t[active]
             89. output_FL       > ALC1220 Analog:playback_FL\t[active]
        47. voice-recorder                                              
             73. input_FL        < Noise Canceling source:capture_MONO\t[active]
             74. input_FR       
       103. game: Main Menu                                             
             87. output_FL       > ALC1220 Analog:playback_FL\t[active]

Video
 └─ Streams:
        80. webcam-viewer
";
        let state = parse_status(status);

        let names = |ds: &[AudioDevice]| -> Vec<(u32, String, bool)> {
            ds.iter()
                .map(|d| (d.id, d.name.clone(), d.is_default))
                .collect()
        };
        assert_eq!(
            names(&state.sinks),
            [
                (64, "Built-in Audio Analog Stereo".into(), true),
                (66, "USB Audio 2.0: Line Out".into(), false),
            ]
        );
        assert_eq!(
            names(&state.sources),
            [(65, "Built-in Audio Analog Stereo".into(), false)]
        );

        let streams: Vec<(u32, &str, Option<&str>)> = state
            .streams
            .iter()
            .map(|s| (s.id, s.app_name.as_str(), s.media_name.as_deref()))
            .collect();
        assert_eq!(
            streams,
            [
                (100, "Brave Browser", None),
                (47, "voice-recorder", None),
                (103, "game", Some("Main Menu")),
            ]
        );
    }

    #[test]
    fn a_stream_line_without_a_numeric_id_is_skipped() {
        assert_eq!(parse_stream_line(" │  Firefox"), None);
        assert_eq!(parse_stream_line(" │  firefox.bin: Playback"), None);
    }
}
