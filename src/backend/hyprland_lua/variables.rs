//! String variables: `local` and global assignments and module `return` tables.

use super::discovery::display_source_file;
use super::expr::{eval_lua_string_expr, is_lua_identifier, is_lua_keyword, lua_quote};
use super::lexer::{find_keyword_positions, find_matching_delimiter, line_start_depths};
use super::project::{LuaProject, LuaSource, LuaVariable, read_lua_project};
use super::table::{find_assignment_eq, table_entries};
use super::write::{Edit, append_lua_line, apply_edits, removal_range};
use crate::backend::hyprland::Variable;
use std::collections::{HashMap, HashSet};
use std::path::Path;

pub fn get_variables(path: &Path) -> Result<Vec<Variable>, String> {
    let project = read_lua_project(path)?;
    let root = path.parent().unwrap_or_else(|| Path::new(""));
    let mut variables = project_variables(&project)
        .into_iter()
        .map(|var| Variable {
            source_file: Some(display_source_file(root, &var.path)),
            name: var.name,
            value: var.value,
        })
        .collect::<Vec<_>>();
    variables.sort_by(|a, b| a.name.cmp(&b.name));
    variables.dedup_by(|a, b| a.name == b.name);
    Ok(variables)
}

pub fn set_variable(path: &Path, name: String, value: String) -> Result<(), String> {
    validate_variable_name(&name)?;
    let project = read_lua_project(path)?;
    if let Some(var) = project_variables(&project)
        .into_iter()
        .find(|var| var.name == name)
    {
        let source = project
            .source(&var.path)
            .ok_or_else(|| format!("Source for variable '{}' not found", name))?;
        apply_edits(
            source,
            vec![Edit::replace(
                (var.value_start, var.value_end),
                lua_quote(&value),
            )],
        )
    } else {
        append_lua_line(path, &format!("local {} = {}", name, lua_quote(&value)))
    }
}

pub fn add_variable(path: &Path, name: String, value: String) -> Result<(), String> {
    set_variable(path, name, value)
}

pub fn delete_variable(path: &Path, name: String) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let var = project_variables(&project)
        .into_iter()
        .find(|var| var.name == name)
        .ok_or_else(|| format!("Variable '{}' not found", name))?;
    let source = project
        .source(&var.path)
        .ok_or_else(|| format!("Source for variable '{}' not found", name))?;
    apply_edits(
        source,
        vec![Edit::remove((var.remove_start, var.remove_end))],
    )
}

fn project_variables(project: &LuaProject) -> Vec<LuaVariable> {
    project
        .sources
        .iter()
        .flat_map(|source| parse_source_variables(source, &project.variables))
        .collect()
}

pub(super) fn parse_source_variables(
    source: &LuaSource,
    variables: &HashMap<String, String>,
) -> Vec<LuaVariable> {
    let mut parsed = parse_return_table_variables(source, variables);
    parsed.extend(parse_assignment_variables(source, variables));
    parsed
}

/// Identifiers a new expression in `source` can reference: names assigned in that file
/// plus globals assigned anywhere in the project.
pub(super) fn assigned_identifiers(source: &LuaSource, project: &LuaProject) -> HashSet<String> {
    project
        .sources
        .iter()
        .flat_map(|other| parse_assignment_variables(other, &project.variables))
        .filter(|var| var.global || var.path == source.path)
        .map(|var| var.name)
        .collect()
}

/// `local name = <string expr>` / `name = <string expr>` statements. Lines nested in a
/// table or call are skipped, so fields like `layout = "dwindle"` inside `hl.config`
/// are not mistaken for variables.
fn parse_assignment_variables(
    source: &LuaSource,
    variables: &HashMap<String, String>,
) -> Vec<LuaVariable> {
    let masked = &source.masked;
    let mut parsed = Vec::new();

    for (line_start, depth) in line_start_depths(masked) {
        if depth != 0 {
            continue;
        }
        let line_end = masked[line_start..]
            .find('\n')
            .map(|pos| line_start + pos)
            .unwrap_or(masked.len());
        let line = &masked[line_start..line_end];
        let body = line.trim_start();
        let (global, rest) = match body.strip_prefix("local") {
            Some(rest) if rest.starts_with(char::is_whitespace) => (false, rest.trim_start()),
            _ => (true, body),
        };
        let Some(eq) = find_assignment_eq(rest) else {
            continue;
        };
        let name = rest[..eq].trim();
        if !is_lua_identifier(name) || is_lua_keyword(name) {
            continue;
        }

        let after = &rest[eq + 1..];
        let value_text = after.trim().trim_end_matches(';').trim_end();
        if value_text.is_empty() {
            continue;
        }
        let Some(value) = eval_lua_string_expr(value_text, variables) else {
            continue;
        };

        let value_start = line_end - after.len() + (after.len() - after.trim_start().len());
        parsed.push(LuaVariable {
            name: name.to_string(),
            value,
            path: source.path.clone(),
            global,
            value_start,
            value_end: value_start + value_text.len(),
            remove_start: line_start,
            remove_end: (line_end + 1).min(masked.len()),
        });
    }

    parsed
}

/// Fields of a module's `return { ... }` table.
fn parse_return_table_variables(
    source: &LuaSource,
    variables: &HashMap<String, String>,
) -> Vec<LuaVariable> {
    let masked = &source.masked;
    let Some(open) = find_keyword_positions(masked, "return")
        .into_iter()
        .find_map(|pos| {
            let after = pos + "return".len();
            let skipped = masked[after..].len() - masked[after..].trim_start().len();
            (masked.as_bytes().get(after + skipped) == Some(&b'{')).then_some(after + skipped)
        })
    else {
        return Vec::new();
    };
    let Some(close) = find_matching_delimiter(masked, open, b'{', b'}') else {
        return Vec::new();
    };

    let table = &masked[open..=close];
    table_entries(table)
        .into_iter()
        .filter_map(|entry| {
            let key = entry.key.filter(|key| is_lua_identifier(key))?;
            let value =
                eval_lua_string_expr(&table[entry.value_start..entry.value_end], variables)?;
            let (remove_start, remove_end) =
                removal_range(masked, open + entry.start, open + entry.sep_end);
            Some(LuaVariable {
                name: key,
                value,
                path: source.path.clone(),
                global: false,
                value_start: open + entry.value_start,
                value_end: open + entry.value_end,
                remove_start,
                remove_end,
            })
        })
        .collect()
}

fn validate_variable_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Variable name cannot be empty".to_string());
    }
    if !is_lua_identifier(name) || is_lua_keyword(name) {
        return Err(
            "Variable name must be a Lua identifier (letters, numbers and underscores, not starting with a number)"
                .to_string(),
        );
    }
    Ok(())
}
