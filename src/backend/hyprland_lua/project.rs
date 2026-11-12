//! The parsed config: sources (with comments masked), the project of files they form,
//! `hl.*` call statements and string variables, and how they are read from disk.

use super::discovery::discover_lua_sources;
use super::lexer::{
    find_matching_delimiter, is_ident_byte, mask_lua_comments, split_top_level,
    split_top_level_spans, string_literal_end,
};
use super::variables::parse_source_variables;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub(super) struct LuaSource {
    pub(super) path: PathBuf,
    pub(super) contents: String,
    /// `contents` with every comment blanked out. Byte offsets match `contents`, so
    /// spans found in `masked` can be applied to the original text.
    pub(super) masked: String,
}

impl LuaSource {
    pub(super) fn new(path: PathBuf, contents: String) -> Self {
        let masked = mask_lua_comments(&contents);
        Self {
            path,
            contents,
            masked,
        }
    }
}

#[derive(Debug)]
pub(super) struct LuaProject {
    pub(super) sources: Vec<LuaSource>,
    pub(super) variables: HashMap<String, String>,
}

impl LuaProject {
    pub(super) fn source(&self, path: &Path) -> Option<&LuaSource> {
        self.sources.iter().find(|source| source.path == path)
    }
}

/// A `hl.<fn>(...)` call.
#[derive(Debug, Clone)]
pub(super) struct LuaStatement {
    pub(super) path: PathBuf,
    /// Offset of the call expression (`hl.`).
    pub(super) start: usize,
    /// Offset one past the closing parenthesis.
    pub(super) end: usize,
    /// Offset of the first byte inside the parentheses.
    args_start: usize,
    /// Argument source with comments blanked out.
    args: String,
}

impl LuaStatement {
    /// Absolute spans of the top-level call arguments.
    pub(super) fn arg_spans(&self) -> Vec<(usize, usize)> {
        split_top_level_spans(&self.args, b',')
            .into_iter()
            .map(|(start, end)| (self.args_start + start, self.args_start + end))
            .collect()
    }

    pub(super) fn args_list(&self) -> Vec<&str> {
        split_top_level(&self.args, b',')
    }
}

#[derive(Debug, Clone)]
pub(super) struct LuaVariable {
    pub(super) name: String,
    pub(super) value: String,
    pub(super) path: PathBuf,
    /// Assigned without `local`, so visible from every file loaded afterwards.
    pub(super) global: bool,
    pub(super) value_start: usize,
    pub(super) value_end: usize,
    pub(super) remove_start: usize,
    pub(super) remove_end: usize,
}

pub(super) fn read_lua_config(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("Hyprland Lua config file not found at {:?}", path));
    }
    fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))
}

pub(super) fn read_lua_source(path: &Path) -> Result<LuaSource, String> {
    Ok(LuaSource::new(path.to_path_buf(), read_lua_config(path)?))
}

pub(super) fn read_lua_project(path: &Path) -> Result<LuaProject, String> {
    let sources = discover_lua_sources(path)?
        .iter()
        .map(|path| read_lua_source(path))
        .collect::<Result<Vec<_>, _>>()?;
    let variables = string_vars_map_for_sources(&sources);
    Ok(LuaProject { sources, variables })
}

pub(super) fn string_vars_map_for_sources(sources: &[LuaSource]) -> HashMap<String, String> {
    let mut variables = HashMap::new();

    // Iterate so values that reference other variables settle.
    for _ in 0..4 {
        let mut changed = false;
        for source in sources {
            for var in parse_source_variables(source, &variables) {
                if variables.get(&var.name) != Some(&var.value) {
                    variables.insert(var.name, var.value);
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }

    variables
}

/// Calls to `needle` (for example `hl.bind`) in a source.
pub(super) fn find_calls(source: &LuaSource, needle: &str) -> Vec<LuaStatement> {
    let masked = &source.masked;
    let bytes = masked.as_bytes();
    let mut statements = Vec::new();
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(end) = string_literal_end(bytes, i) {
            i = end.max(i + 1);
            continue;
        }
        let boundary = i == 0 || !(is_ident_byte(bytes[i - 1]) || bytes[i - 1] == b'.');
        if !boundary || !bytes[i..].starts_with(needle.as_bytes()) {
            i += 1;
            continue;
        }

        let after = i + needle.len();
        let rest = masked[after..].trim_start();
        let open = masked.len() - rest.len();
        if !rest.starts_with('(') || (after < bytes.len() && is_ident_byte(bytes[after])) {
            i = after;
            continue;
        }

        match find_matching_delimiter(masked, open, b'(', b')') {
            Some(close) => {
                statements.push(LuaStatement {
                    path: source.path.clone(),
                    start: i,
                    end: close + 1,
                    args_start: open + 1,
                    args: masked[open + 1..close].to_string(),
                });
                i = close + 1;
            }
            None => i = after,
        }
    }

    statements
}

pub(super) fn project_calls(project: &LuaProject, needle: &str) -> Vec<LuaStatement> {
    project
        .sources
        .iter()
        .flat_map(|source| find_calls(source, needle))
        .collect()
}
