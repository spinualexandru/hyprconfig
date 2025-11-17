# hyprconfig

Configure hyprland using a GUI.

# Roadmap

## General

- [x] Setup basic GUI
- [x] Add navigation menu

## Appearance

- [ ] Border Colors
- [ ] Animations
- [ ] Blur
- [ ] Shadows

## Keybinds

- [x] Display keybinds
- [x] Tooltip to show values of variables when keybinds contain variables
- [x] Edit a variable in line ( Excel like)
- [x] Delete a variable
- [x] Add a new variable
    - [x] Modifiers checkbox ( SUPER, SHIFT, ALT, CTRL )
    - [x] Key text input with auto complete
    - [x] Dispatcher text input with auto complete
    - [x] Parameters (e.g. $variable) 

## Variables

- [x] Display defined variables
- [x] Modify variables inline ( Excel like )
- [x] Add variables 

## Display

- [x] Add display page
- [x] Display information on the monitors
    - [x] Add resolution and refresh rate
    - [x] Add position and scale
    - [x] Add name and identifier
    - [x] Display transform
    - [x] Display active workspace
- [x] Add options to change resolution and refresh rate
    - [x] Create a list of available modes
    - [ ] Add confirmation dialog
- [ ] Add options to change position
- [ ] Add options to change scale
    - [ ] Create a slider for scale
    - [ ] Add confirmation dialog
- [ ] VRR Toggle
- [ ] Tearing toggle

## Network

- [x] Add network page
- [x] Display network information
- [x] Display connected Wi-Fi Network
- [x] Display connected Ethernet network
- [x] Display link status (UP/DOWN)
- [x] Display available Wi-Fi network
- [x] Scan for new networks
    - [ ] Display SSID
    - [ ] Display security protocol (e.g. WPA2 / WPA3 etc)
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

## Plugins

- [ ] Display installed plugins
- [ ] List available plugins
- [ ] List remote plugins
- [ ] Install a plugin
- [ ] Uninstall a plugin
- [ ] Check plugin for updates

## Audio

- [ ] Integrate pipeline connector
- [ ] Display Audio Controllers
- [ ] Volume Modifier
- [ ] Mute / Unmute
- [ ] Change primary audio source
- [ ] Change bitrate
- [ ] Display per application volume modifier

## About
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
