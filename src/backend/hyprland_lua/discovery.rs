//! Finding the files a config pulls in through `require`, `dofile` and `loadfile`.

use super::lexer::{
    eval_lua_string_expr, find_keyword_positions, find_matching_delimiter,
    parse_lua_string_literal, split_top_level_spans, string_literal_end,
};
use super::project::{read_lua_source, string_vars_map_for_sources};
use std::fs;
use std::path::{Path, PathBuf};

pub(super) fn discover_lua_sources(entrypoint: &Path) -> Result<Vec<PathBuf>, String> {
    let root = entrypoint
        .parent()
        .ok_or_else(|| format!("Config file has no parent directory: {:?}", entrypoint))?
        .to_path_buf();
    let mut paths = Vec::new();
    let mut pending = vec![entrypoint.to_path_buf()];
    let mut known_dirs = vec![root.clone()];

    while let Some(path) = pending.pop() {
        let path = normalize_lua_path(&path);
        if paths.contains(&path) {
            continue;
        }

        let source = read_lua_source(&path)?;
        let vars = string_vars_map_for_sources(std::slice::from_ref(&source));

        for (name, value) in &vars {
            if name.ends_with("dir") || name.ends_with("_dir") || name.ends_with("home") {
                let dir = PathBuf::from(value);
                if dir.is_dir() && !known_dirs.contains(&dir) {
                    known_dirs.push(dir);
                }
            }
        }

        let mut candidates = Vec::new();
        for (loader, arg) in find_lua_import_exprs(&source.masked) {
            let Some(import_path) = eval_lua_string_expr(&arg, &vars) else {
                continue;
            };
            if loader == "require" {
                candidates.extend(resolve_require(&root, &known_dirs, &import_path));
                continue;
            }
            let found = resolve_lua_path(&root, path.parent().unwrap_or(&root), &import_path);
            if found.is_empty() {
                candidates.extend(resolve_named_lua_module(&root, &known_dirs, &import_path));
            } else {
                candidates.extend(found);
            }
        }
        for module_name in find_loader_module_names(&source.masked) {
            candidates.extend(resolve_named_lua_module(&root, &known_dirs, &module_name));
        }

        for candidate in candidates {
            if !paths.contains(&candidate) && !pending.contains(&candidate) {
                pending.push(candidate);
            }
        }

        paths.push(path);
    }

    Ok(paths)
}

pub(super) fn normalize_lua_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// Mirrors Hyprland's `require`: explicit paths (`/`, `./`, `../`, `~/`) and globs
/// resolve against the config directory, anything else is a module name looked up via
/// `<config dir>/?.lua` and `<config dir>/?/init.lua`.
fn resolve_require(root: &Path, known_dirs: &[PathBuf], name: &str) -> Vec<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return Vec::new();
    }

    if name.contains(['*', '?', '[']) {
        let pattern = config_relative_path(root, name);
        let mut paths = glob::glob(&pattern.to_string_lossy())
            .map(|entries| {
                entries
                    .flatten()
                    .filter(|path| path.is_file())
                    .map(|path| normalize_lua_path(&path))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        paths.sort();
        paths.dedup();
        return paths;
    }

    let explicit = ["/", "./", "../", "~/"]
        .iter()
        .any(|prefix| name.starts_with(prefix));
    if explicit {
        let base = config_relative_path(root, name);
        let mut candidates = vec![base.clone()];
        if !name.ends_with(".lua") {
            candidates.push(PathBuf::from(format!("{}.lua", base.display())));
        }
        candidates.push(base.join("init.lua"));
        return candidates
            .into_iter()
            .find(|candidate| candidate.is_file())
            .map(|candidate| vec![normalize_lua_path(&candidate)])
            .unwrap_or_default();
    }

    resolve_named_lua_module(root, known_dirs, name)
}

fn config_relative_path(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(expand_tilde(value));
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

fn resolve_lua_path(root: &Path, current_dir: &Path, value: &str) -> Vec<PathBuf> {
    let path = PathBuf::from(expand_tilde(value));
    let mut candidates = Vec::new();

    if path.extension().and_then(|ext| ext.to_str()) != Some("lua") {
        return candidates;
    }

    for candidate in [path.clone(), current_dir.join(&path), root.join(&path)] {
        if candidate.exists() {
            candidates.push(normalize_lua_path(&candidate));
        }
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn resolve_named_lua_module(root: &Path, known_dirs: &[PathBuf], name: &str) -> Vec<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let relative = PathBuf::from(name.replace('.', "/"));
    let mut dirs = known_dirs.to_vec();
    dirs.push(root.to_path_buf());
    dirs.sort();
    dirs.dedup();
    for dir in &dirs {
        for candidate in [
            dir.join(&relative).with_extension("lua"),
            dir.join(&relative).join("init.lua"),
            dir.join("modules").join(&relative).with_extension("lua"),
        ] {
            if candidate.is_file() {
                candidates.push(normalize_lua_path(&candidate));
            }
        }
    }

    if candidates.is_empty() {
        collect_lua_files_by_stem(root, name, &mut candidates);
    }

    candidates.sort();
    candidates.dedup();
    candidates
}

fn collect_lua_files_by_stem(dir: &Path, stem: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_lua_files_by_stem(&path, stem, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("lua")
            && path.file_stem().and_then(|file_stem| file_stem.to_str()) == Some(stem)
        {
            out.push(normalize_lua_path(&path));
        }
    }
}

fn expand_tilde(value: &str) -> String {
    if let Some(rest) = value.strip_prefix("~/")
        && let Ok(home) = std::env::var("HOME")
    {
        return format!("{}/{}", home, rest);
    }
    value.to_string()
}

pub(super) fn display_source_file(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_str()
        .or_else(|| path.file_name().and_then(|n| n.to_str()))
        .unwrap_or("hyprland.lua")
        .to_string()
}

/// `(loader, first argument)` for `dofile`, `loadfile` and `require` calls, including
/// the paren-less `require "module"` form.
fn find_lua_import_exprs(masked: &str) -> Vec<(&'static str, String)> {
    let mut imports = Vec::new();
    for loader in ["dofile", "loadfile", "require"] {
        for pos in find_keyword_positions(masked, loader) {
            let after = pos + loader.len();
            let rest = masked[after..].trim_start();
            let rest_start = masked.len() - rest.len();
            if rest.starts_with('(') {
                if let Some(close) = find_matching_delimiter(masked, rest_start, b'(', b')') {
                    let inner = &masked[rest_start + 1..close];
                    if let Some(&(start, end)) = split_top_level_spans(inner, b',').first() {
                        imports.push((loader, inner[start..end].to_string()));
                    }
                }
            } else if loader == "require"
                && let Some(end) = string_literal_end(masked.as_bytes(), rest_start)
            {
                imports.push((loader, masked[rest_start..end].to_string()));
            }
        }
    }
    imports
}

fn find_loader_module_names(masked: &str) -> Vec<String> {
    let mut names = Vec::new();
    for method in [".load", ".apply"] {
        let mut search_from = 0;
        while let Some(relative) = masked[search_from..].find(method) {
            let call_start = search_from + relative;
            let open = call_start + method.len();
            let rest = masked[open..].trim_start();
            let open = masked.len() - rest.len();

            if !rest.starts_with('(') {
                search_from = call_start + method.len();
                continue;
            }

            if let Some(close) = find_matching_delimiter(masked, open, b'(', b')') {
                let inner = &masked[open + 1..close];
                if let Some(&(start, end)) = split_top_level_spans(inner, b',').first()
                    && let Some(name) = parse_lua_string_literal(&inner[start..end])
                {
                    names.push(name);
                }
                search_from = close + 1;
            } else {
                search_from = call_start + method.len();
            }
        }
    }

    names.sort();
    names.dedup();
    names
}
