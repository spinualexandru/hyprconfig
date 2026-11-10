# hyprconfig


<details>
    <summary>Demo</summary>

https://github.com/user-attachments/assets/cd3f8ad1-40b3-4547-8da7-3544daaa0815

https://github.com/user-attachments/assets/4fa72335-f117-4fed-a3fb-e0b2b6437abb

</details>


Configure hyprland using a GUI.

Works with `hyprland.lua` (the only format Hyprland reads after 0.56) and with the legacy `hyprland.conf`.

Hyprconfig is a native Rust app drawn with [Rune](https://github.com/anomalyco/rune), a GPU UI
framework (wgpu, winit). There is no web view or JavaScript.

## How to run

You need a recent stable Rust (built and tested with 1.99) and the Wayland and xkbcommon libraries
(`wayland libxkbcommon` on Arch).

```
cargo run --release
```

Changing the wallpaper opens the desktop's file chooser through the XDG desktop portal, so an
`xdg-desktop-portal` backend with a FileChooser (`xdg-desktop-portal-gtk`, for example, next to
`xdg-desktop-portal-hyprland`) must be running.

## Installation (current user)

```
just install     # ~/.local/bin/hyprconfig, its icon and a .desktop entry
just uninstall   # also takes the Noctalia integration out (`hyprconfig noctalia off`)
```

The window's app id is `hyprconfig`, for window rules such as
`windowrule = float, class:^(hyprconfig)$`.

## Development

```
cargo test                 # unit and UI tests on Rune's headless runtime (no window, no system calls)
just check                 # fmt, clippy and tests
just snapshots             # render every scene to artifacts/*.png (`just snapshots light` for light)
cargo run -- --list-scenes
cargo run -- --scene keybinds --snap artifacts/keybinds.png
```

Scenes (`src/fixtures.rs`) fill each page with made-up data, so headless runs never read your
config. In a normal run the app reads the system only after its window is up (`App::on_ready`),
and every backend call runs on a worker thread.

The colours come from a colour scheme generator, chosen on the General page:

- [chromasync](https://github.com/spinualexandru/chromasync) (the default) renders the current
  wallpaper with the template, chroma and contrast of your `default` sync profile in
  `~/.config/chromasync/config.toml` (`materialish` when there is none).
- Noctalia 5 uses its current colour scheme. Wallpaper schemes (`m3-tonal-spot` and so on) and
  downloaded community palettes work as they are. Builtin and custom schemes need the Noctalia
  integration below, which has Noctalia render whichever palette it shows.

When the preferred generator isn't installed or can't answer, the other one is tried, then
`~/.config/hyprconfig/theme.css` (`--background`, `--foreground` and `--primary` for `:root` and
`.dark`), which matugen can write; see the Appearance page. The colours regenerate at start-up and
whenever their inputs change:

- With Noctalia running, the General page's Noctalia integration switch writes
  `~/.config/noctalia/hyprconfig.toml`, which registers a user template
  (`~/.config/hyprconfig/noctalia-template.css`). Noctalia renders it to
  `~/.cache/hyprconfig/noctalia.css` on every palette change, whatever the scheme, then runs its
  post hook. The switch also adds Hyprconfig to the On Theme Mode Changed and On Wallpaper
  Changed hooks, and leaves your other hook commands alone. The post hook and the hooks run
  `hyprconfig notify <event>` (`colors`, `mode` or `wallpaper`), which tells the running app
  through `$XDG_RUNTIME_DIR/hyprconfig.sock` and exits quietly when it isn't running. They use the
  full path of the Hyprconfig that set them up, so turn the switch on from the installed copy.
  Noctalia's mode also becomes the app's light or dark mode. A choice made in the sidebar
  overrides it until Noctalia's mode next changes. Turning the switch off, or running
  `hyprconfig noctalia off` (which `just uninstall` does), removes all of it.
- Files are checked every 0.75 s: chromasync's `config.toml`, `hyprpaper.conf`, `theme.css` and
  Noctalia's `settings.toml` and `~/.config/noctalia/`, or with the integration on, the palette
  Noctalia renders. A wallpaper set only through hyprpaper's IPC, with no file written, isn't
  noticed; press the refresh button on the General page then.

The last generated set is cached in
`~/.cache/hyprconfig/colors.css` so the window opens in them. The generator and the light or dark
choice are saved in `~/.config/hyprconfig/prefs.json`.

## Roadmap/Features

### Appearance

- [ ] Border Colors
- [ ] Animations
- [ ] Blur
- [ ] Shadows
- [x] Change Wallpaper
- [x] Sync with hyprpaper
- [x] Sync with matugen
- [x] Matugen sync type
- [x] App colours from chromasync or Noctalia 5
- [ ] Add undo

### System
- [ ] Startup Appplications
- [ ] Startup Services
- [ ] System level environment variables
- [ ] Userspace environment variables
- [x] Hyprland level environment variables
- [ ] User details
- [ ] User groups details
- [ ] Group creation
- [ ] Add user to groups
- [ ] Remove user from groups
- [ ] Change Password

### Environment Variables (Hyprland)
- [x] Display environment variables
- [x] Edit environment variables inline ( Excel like )
- [x] Add environment variables
- [x] Delete environment variables

### Keybinds

- [x] Display keybinds
- [x] Tooltip to show values of variables when keybinds contain variables
- [x] Edit a variable in line ( Excel like)
- [x] Delete a variable
- [x] Add a new variable
    - [x] Modifiers checkbox ( SUPER, SHIFT, ALT, CTRL )
    - [x] Key text input with auto complete
    - [x] Dispatcher text input with auto complete
    - [x] Parameters (e.g. $variable) 
- [ ] Add undo

### Variables

- [x] Display defined variables
- [x] Modify variables inline ( Excel like )
- [x] Add variables 

### Display

- [x] Add display page
- [x] Display information on the monitors
    - [x] Add resolution and refresh rate
    - [x] Add position and scale
    - [x] Add name and identifier
    - [x] Display transform
    - [x] Display active workspace
- [x] Add options to change resolution and refresh rate
    - [x] Create a list of available modes
    - [x] Add confirmation dialog
- [ ] Add options to change position
- [ ] Add options to change scale
    - [ ] Create a slider for scale
    - [ ] Add confirmation dialog
- [ ] VRR Toggle
- [ ] Tearing toggle

### Network

- [x] Add network page
- [x] Display network information
- [x] Display connected Wi-Fi Network
- [x] Display connected Ethernet network
- [x] Display link status (UP/DOWN)
- [x] Display available Wi-Fi network
- [x] Scan for new networks
    - [x] Display SSID
    - [x] Display security protocol (e.g. WPA2 / WPA3 etc)
    - [ ] Display if it's not protected
- [x] Display signal strength
- [x] Detailed view
    - [x] Interface name
    - [x] Interface identifier (e.g. wlna0 )
    - [x] SSID Name
    - [x] Interface Type
    - [x] Mac Address
    - [x] IPv4 / IPv6 addresses
    - [x] MTU ( Maximum Transmision Unit )
    - [x] Data Received / Data transmitted
    - [ ] Data Received / Data transmitted chart
- [ ] Connect to network
    - [ ] Password Dialog
    - [ ] Connection Status
    - [ ] Notification for "Connected" state
    - [ ] Re-prompt for password if it's invalid

### Plugins

- [ ] Display installed plugins
- [ ] List available plugins
- [ ] List remote plugins
- [ ] Install a plugin
- [ ] Uninstall a plugin
- [ ] Check plugin for updates

### Audio

- [x] Integrate pipeline connector
- [x] Display Audio Controllers
- [x] Volume Modifier
- [x] Mute / Unmute
- [ ] Change primary audio source
- [ ] Change bitrate
- [x] Display per application volume modifier

### About
- [x] Add about page
- [x] Display available GPUs
- [x] Display used and available RAM
- [x] Display CPU Info
- [x] Display Kernel version
- [ ] Display hostname
- [x] Display OS Name
- [x] Display used / available disk space
- [x] Display uptime
- [x] Display uptime version ( and commit if available )
- [x] Display shell

### Configuration
- [ ] Add option to enable .backup folder for the configs
- [ ] Add undo

### Misc
- [ ] Export/Backup config
- [ ] Rice bundler
    - [ ] Detect common tools
    - [ ] Detect hyprpaper
    - [ ] Detect executed shell scripts
- [ ] Display deprecated config keys/values
- [ ] Locate config if not found
- [ ] Internationalization
