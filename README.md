# hyprconfig

Configure Hyprland using a modern GUI built with Tauri.

## Technology Stack

- **Backend**: Rust with Tauri 2.1
- **Frontend**: React 19 with Vite 7
- **UI Components**: Radix UI (accessible, unstyled components)
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

You'll also need Node.js 18+ and npm to build the frontend.

For other distributions, see [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/).

## Building

First, install frontend dependencies:
```bash
cd ui
npm install
cd ..
```

Build the frontend:
```bash
cd ui && npm run build && cd ..
```

Then build the application:
```bash
cargo build --release
```

## Running

### Quick Start (Production Mode)
```bash
cargo run
```

This runs the app with the pre-built static frontend. Make sure to build the frontend first (see Building section).

### Development Mode (with Hot Reload)
For development with Vite hot reload:

1. Start the Vite dev server in one terminal:
```bash
cd ui && npm run dev
```

2. In another terminal, run Tauri with the dev config:
```bash
cargo tauri dev --config tauri.dev.conf.json
```

This will connect to the dev server and give you hot reload for frontend changes.

## Development

The project structure:
- `src/` - Rust backend with Tauri commands
- `ui/src/` - React frontend source code
- `ui/dist/` - Built frontend assets (generated)
- `tauri.conf.json` - Tauri configuration

### Frontend Development
The frontend uses:
- **React** for UI components
- **Radix UI** for accessible, unstyled primitives (Tabs, ScrollArea, etc.)
- **Vite** for fast development and optimized builds

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