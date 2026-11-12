//! Text-level edits that keep the rest of the file intact, and the Hyprland reload after.

use super::lexer::{TableEntry, is_lua_identifier, lua_table_fields};
use super::project::{LuaProject, LuaSource, LuaStatement, read_lua_config};
use std::fs;
use std::path::Path;
#[cfg(not(test))]
use std::process::Command;

#[derive(Debug)]
pub(super) struct Edit {
    start: usize,
    end: usize,
    text: String,
}

impl Edit {
    pub(super) fn replace((start, end): (usize, usize), text: impl Into<String>) -> Self {
        Self {
            start,
            end,
            text: text.into(),
        }
    }

    pub(super) fn insert(at: usize, text: impl Into<String>) -> Self {
        Self::replace((at, at), text)
    }

    pub(super) fn remove((start, end): (usize, usize)) -> Self {
        Self::replace((start, end), "")
    }
}

pub(super) fn append_lua_line(path: &Path, line: &str) -> Result<(), String> {
    let mut contents = read_lua_config(path)?;
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(line);
    contents.push('\n');
    fs::write(path, contents).map_err(|e| format!("Failed to write {:?}: {}", path, e))?;
    reload_hyprland()
}

pub(super) fn apply_edits(source: &LuaSource, mut edits: Vec<Edit>) -> Result<(), String> {
    let mut contents = read_lua_config(&source.path)?;
    if contents != source.contents {
        return Err(format!(
            "{:?} changed while it was being edited, please try again",
            source.path
        ));
    }

    edits.sort_by(|a, b| b.start.cmp(&a.start).then(b.end.cmp(&a.end)));
    for edit in edits {
        contents.replace_range(edit.start..edit.end, &edit.text);
    }

    fs::write(&source.path, contents)
        .map_err(|e| format!("Failed to write {:?}: {}", source.path, e))?;
    reload_hyprland()
}

pub(super) fn delete_statement(
    project: &LuaProject,
    statement: &LuaStatement,
) -> Result<(), String> {
    let source = project
        .source(&statement.path)
        .ok_or_else(|| format!("Source {:?} not found", statement.path))?;
    let range = removal_range(&source.masked, statement.start, statement.end);
    apply_edits(source, vec![Edit::remove(range)])
}

/// The range to delete for `[start, end)`: the whole line when nothing else but
/// whitespace, a trailing comment or a `local name =` prefix shares it.
pub(super) fn removal_range(masked: &str, start: usize, end: usize) -> (usize, usize) {
    let line_start = masked[..start].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let line_end = masked[end..]
        .find('\n')
        .map(|pos| end + pos + 1)
        .unwrap_or(masked.len());
    let prefix = masked[line_start..start].trim();
    let suffix = masked[end..line_end].trim().trim_start_matches(';').trim();

    if suffix.is_empty() && (prefix.is_empty() || is_assignment_prefix(prefix)) {
        (line_start, line_end)
    } else {
        (start, end)
    }
}

fn is_assignment_prefix(prefix: &str) -> bool {
    let Some(target) = prefix.strip_suffix('=') else {
        return false;
    };
    let target = target.trim();
    let target = target.strip_prefix("local ").unwrap_or(target).trim();
    is_lua_identifier(target)
}

/// Inserts `fields` into the table spanning `[table_start, table_end)`, following the
/// table's single- or multi-line layout.
pub(super) fn table_insert_edit(
    masked: &str,
    table_start: usize,
    table_end: usize,
    entries: &[TableEntry],
    fields: &[(String, String)],
) -> Edit {
    let close = table_end - 1;
    let inner = &masked[table_start + 1..close];
    let content_end = table_start + 1 + inner.trim_end().len();
    let has_content = !inner.trim().is_empty();
    let needs_comma = has_content && !masked[..content_end].ends_with([',', ';']);

    if inner.contains('\n') {
        let indent = match entries.first() {
            Some(entry) => {
                let abs = table_start + entry.start;
                let line_start = masked[..abs].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
                masked[line_start..abs].to_string()
            }
            None => "    ".to_string(),
        };
        let mut text = if needs_comma {
            ",".to_string()
        } else {
            String::new()
        };
        for (key, value) in fields {
            text.push_str(&format!("\n{}{} = {},", indent, key, value));
        }
        return Edit::insert(content_end, text);
    }

    let joined = lua_table_fields(fields);
    if has_content {
        let separator = if needs_comma { ", " } else { " " };
        Edit::insert(content_end, format!("{}{}", separator, joined))
    } else {
        Edit::replace((table_start + 1, close), format!(" {} ", joined))
    }
}

fn reload_hyprland() -> Result<(), String> {
    #[cfg(test)]
    {
        Ok(())
    }

    #[cfg(not(test))]
    {
        let output = Command::new("hyprctl")
            .arg("reload")
            .output()
            .map_err(|e| format!("Failed to run hyprctl reload: {}", e))?;

        if output.status.success() {
            return Ok(());
        }

        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if stderr.is_empty() { stdout } else { stderr };

        Err(format!("hyprctl reload failed: {}", detail))
    }
}
