//! Reads and edits `hyprland.lua` (and the files it pulls in) at the text level, so
//! everything Hyprconfig doesn't touch is preserved.
//!
//! Each concern lives in its own file; the public API is re-exported here so callers keep
//! using `hyprland_lua::get_keybinds` and friends.

use std::path::PathBuf;

mod discovery;
mod dispatchers;
mod env;
mod expr;
mod keybinds;
mod lexer;
mod monitors;
mod project;
mod rules;
mod table;
#[cfg(test)]
mod tests;
mod variables;
mod write;

pub use env::*;
pub use keybinds::*;
pub use monitors::*;
pub use rules::*;
pub use variables::*;

/// The config Hyprland reads: `HYPRLAND_CONFIG` first, then `hypr/hyprland.lua`. Only Lua
/// configs are supported; Hyprland itself stopped reading `hyprland.conf` after 0.56.
pub fn config_path() -> Result<PathBuf, String> {
    if let Ok(path) = std::env::var("HYPRLAND_CONFIG")
        && !path.trim().is_empty()
    {
        let path = PathBuf::from(path);
        if path.extension().and_then(|e| e.to_str()) != Some("lua") {
            return Err(format!(
                "HYPRLAND_CONFIG points to {}, but Hyprconfig only edits hyprland.lua configs",
                path.display()
            ));
        }
        return Ok(path);
    }

    let hypr_dir = super::os::config_home()?.join("hypr");
    let lua_path = hypr_dir.join("hyprland.lua");
    if !lua_path.exists() && hypr_dir.join("hyprland.conf").exists() {
        return Err(format!(
            "Found hyprland.conf but no hyprland.lua in {}. Hyprland reads only hyprland.lua \
             since 0.56; migrate your config to edit it here.",
            hypr_dir.display()
        ));
    }
    Ok(lua_path)
}
