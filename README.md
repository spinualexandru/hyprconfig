# hyprconfig

Configure Hyprland using a modern GUI built with Tauri.

## Technology Stack

- **Backend**: Rust with Tauri 2.1
- **Frontend**: HTML5, CSS3, JavaScript
- **Hyprland Integration**: `hyprland` crate for native Hyprland IPC

## System Requirements

### Linux
Before building, install the required system dependencies:

```bash
sudo apt-get update
sudo apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libayatana-appindicator3-dev \
    librsvg2-dev \
    patchelf
```

For other distributions, see [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/).

## Building

```bash
cargo build --release
```

## Running

```bash
cargo run
```

## Development

The project structure:
- `src/` - Rust backend with Tauri commands
- `ui/dist/` - Frontend HTML/CSS/JS
- `tauri.conf.json` - Tauri configuration

# Roadmap

## General

- [x] Setup basic GUI
- [x] Add navigation menu

## Appearance

## Keybinds

## Display

- [x] Add display page
- [x] Display information on the monitors
    - [x] Add resolution and refresh rate
    - [x] Add position and scale
    - [x] Add name and identifier
- [ ] Add options to change resolution and refresh rate
    - [ ] Create a list of available modes
    - [ ] Add confirmation dialog
- [ ] Add options to change position
- [ ] Add options to change scale
    - [ ] Create a slider for scale
    - [ ] Add confirmation dialog

## Network

## Plugins

## Audio

## About

- [x] Add about page