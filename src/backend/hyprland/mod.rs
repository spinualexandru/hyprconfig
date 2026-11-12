//! Hyprland-adjacent system data: monitors, network, system info, and the config types and
//! wrappers over `hyprland_lua`.
//!
//! Each concern lives in its own file; everything is re-exported here so callers keep using
//! `hyprland::get_monitors` and friends.

mod lua_config;
mod monitors;
mod network;
mod system;

pub use lua_config::*;
pub use monitors::*;
pub use network::*;
pub use system::*;
