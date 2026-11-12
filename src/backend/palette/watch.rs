//! Which files can change the colours, and stamps to notice when they do.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::integration::{integration_paths, noctalia_config_dir};
use super::xdg_dir;
use crate::backend::hyprpaper;

/// The files whose changes can change the colours: chromasync's config, hyprpaper's config
/// (the wallpaper chromasync renders), `theme.css` (the fallback, which matugen rewrites) and
/// Noctalia's: with the integration on (`integrated`), the palette it renders; otherwise its
/// settings (scheme, mode and wallpaper), config and community palettes.
pub fn watched_paths(integrated: bool) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if integrated {
        paths.extend(integration_paths().ok().map(|p| p.rendered));
    } else {
        if let Some(state) = xdg_dir("XDG_STATE_HOME", ".local/state") {
            paths.push(state.join("noctalia/settings.toml"));
            paths.push(state.join("noctalia/community-palettes/.catalog/palettes.json"));
        }
        paths.extend(noctalia_config_dir());
    }
    if let Some(config) = xdg_dir("XDG_CONFIG_HOME", ".config") {
        paths.push(config.join("chromasync/config.toml"));
        paths.push(config.join("hyprconfig/theme.css"));
    }
    if let Ok(p) = hyprpaper::hyprpaper_config_path() {
        paths.push(p);
    }
    paths
}

/// When each file was last written and how big it is; a directory stands for the files
/// directly in it. Missing files count too, so creating one is a change.
pub type Stamps = Vec<(PathBuf, Option<(SystemTime, u64)>)>;

pub fn stamps(paths: &[PathBuf]) -> Stamps {
    let stamp = |p: &Path| {
        fs::metadata(p)
            .ok()
            .and_then(|m| Some((m.modified().ok()?, m.len())))
    };
    let mut out = Vec::new();
    for p in paths {
        if p.is_dir() {
            let mut files: Vec<PathBuf> = fs::read_dir(p)
                .into_iter()
                .flatten()
                .flatten()
                .map(|e| e.path())
                .filter(|f| f.is_file())
                .collect();
            files.sort();
            out.extend(files.into_iter().map(|f| {
                let s = stamp(&f);
                (f, s)
            }));
        } else {
            out.push((p.clone(), stamp(p)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stamps_notice_writes_new_files_and_directories() {
        let dir = std::env::temp_dir().join(format!("hyprconfig-stamps-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("noctalia")).unwrap();
        let file = dir.join("settings.toml");
        let paths = vec![file.clone(), dir.join("noctalia")];

        let empty = stamps(&paths);
        assert_eq!(
            empty,
            vec![(file.clone(), None)],
            "a missing file, an empty directory"
        );
        fs::write(&file, "a = 1").unwrap();
        let written = stamps(&paths);
        assert_ne!(written, empty);
        fs::write(&file, "a = 12").unwrap();
        assert_ne!(stamps(&paths), written, "a rewrite of another size");
        fs::write(dir.join("noctalia/config.toml"), "").unwrap();
        assert_eq!(stamps(&paths).len(), 2, "files in a watched directory");
        fs::remove_dir_all(&dir).unwrap();
    }
}
