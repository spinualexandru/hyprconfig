//! The last generated colours, kept so the next launch starts in them.

use std::fs;
use std::path::PathBuf;

use super::xdg_dir;

pub(super) fn cache_dir() -> Result<PathBuf, String> {
    xdg_dir("XDG_CACHE_HOME", ".cache")
        .map(|d| d.join("hyprconfig"))
        .ok_or_else(|| "Could not determine home directory".to_string())
}

fn cache_path() -> Result<PathBuf, String> {
    Ok(cache_dir()?.join("colors.css"))
}

/// The colours a generator made last time, if any.
pub fn cached_css() -> Option<String> {
    fs::read_to_string(cache_path().ok()?).ok()
}

/// Keeps a generator's colours for the next launch; `None` (theme.css is in use) forgets them.
pub fn save_cache(css: Option<String>) -> Result<(), String> {
    let path = cache_path()?;
    match css {
        Some(css) => {
            fs::create_dir_all(cache_dir()?)
                .map_err(|e| format!("Failed to create cache directory: {e}"))?;
            fs::write(&path, css).map_err(|e| format!("Failed to cache colours: {e}"))
        }
        None => match fs::remove_file(&path) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                Err(format!("Failed to remove cached colours: {e}"))
            }
            _ => Ok(()),
        },
    }
}
