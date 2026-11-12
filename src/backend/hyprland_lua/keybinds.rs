//! `hl.bind` keybinds: reading, adding, editing and deleting them.

use super::discovery::normalize_lua_path;
use super::dispatchers::{dispatcher_to_lua_expr, parse_dispatcher_expr};
use super::expr::{eval_lua_string_expr, is_lua_identifier, lua_quote};
use super::project::{LuaProject, LuaSource, LuaStatement, find_calls, read_lua_project};
use super::table::{table_entries, table_field};
use super::variables::assigned_identifiers;
use super::write::{Edit, append_lua_line, apply_edits, delete_statement, table_insert_edit};
use crate::backend::hyprland::Keybind;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug)]
pub(super) struct ParsedBind {
    statement: LuaStatement,
    pub(super) keybind: Keybind,
    pub(super) submap_universal: bool,
    /// Absolute spans of the keys, dispatcher and (optional) options arguments.
    arg_spans: Vec<(usize, usize)>,
}

pub fn get_keybinds(path: &Path) -> Result<Vec<Keybind>, String> {
    Ok(parse_binds(path)?
        .into_iter()
        .filter(|bind| !bind.submap_universal)
        .map(|bind| bind.keybind)
        .collect())
}

pub fn get_all_bindu(path: &Path) -> Result<Vec<Keybind>, String> {
    Ok(parse_binds(path)?
        .into_iter()
        .filter(|bind| bind.submap_universal)
        .map(|bind| bind.keybind)
        .collect())
}

pub fn add_keybind(
    path: &Path,
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    append_bind(path, modifiers, key, dispatcher, params, false)
}

pub fn edit_keybind(
    path: &Path,
    index: usize,
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    validate_keybind(&key, &dispatcher)?;
    let project = read_lua_project(path)?;
    let bind = parse_binds_from_project(&project)
        .into_iter()
        .filter(|bind| !bind.submap_universal)
        .nth(index)
        .ok_or_else(|| format!("Keybind index {} not found", index))?;
    let source = project
        .source(&bind.statement.path)
        .ok_or_else(|| format!("Source for keybind {} not found", index))?;

    let key = key.trim();
    let dispatcher = dispatcher.trim();
    let params = params.trim();
    let mut edits = Vec::new();

    // Only rewrite the arguments that changed, so variables like `mainMod`, spacing,
    // comments and bind options survive an edit.
    if bind.keybind.modifiers != modifiers || bind.keybind.key != key {
        edits.push(Edit::replace(
            bind.arg_spans[0],
            keys_expr(source, &bind, &modifiers, key, &project.variables),
        ));
    }

    if bind.keybind.dispatcher != dispatcher || bind.keybind.params != params {
        let scope = assigned_identifiers(source, &project);
        edits.push(Edit::replace(
            bind.arg_spans[1],
            dispatcher_to_lua_expr(dispatcher, params, &scope)?,
        ));

        let was_mouse = bind.keybind.dispatcher == "mouse";
        let is_mouse = dispatcher == "mouse";
        if was_mouse != is_mouse {
            edits.extend(set_bind_option(source, &bind, "mouse", is_mouse));
        }
    }

    if edits.is_empty() {
        return Ok(());
    }
    apply_edits(source, edits)
}

pub fn delete_keybind(path: &Path, index: usize) -> Result<(), String> {
    delete_bind(path, index, false)
}

pub fn add_bindu(
    path: &Path,
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    append_bind(path, modifiers, key, dispatcher, params, true)
}

pub fn delete_bindu(path: &Path, index: usize) -> Result<(), String> {
    delete_bind(path, index, true)
}

fn append_bind(
    path: &Path,
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
    submap_universal: bool,
) -> Result<(), String> {
    validate_keybind(&key, &dispatcher)?;
    let project = read_lua_project(path)?;
    let entrypoint = normalize_lua_path(path);
    let scope = project
        .source(&entrypoint)
        .map(|source| assigned_identifiers(source, &project))
        .unwrap_or_default();
    let dispatcher = dispatcher.trim();

    let mut options = Vec::new();
    if submap_universal {
        options.push("submap_universal = true");
    }
    if dispatcher == "mouse" {
        options.push("mouse = true");
    }

    let mut call = format!(
        "hl.bind({}, {}",
        lua_quote(&key_string(&modifiers, &key)),
        dispatcher_to_lua_expr(dispatcher, &params, &scope)?
    );
    if !options.is_empty() {
        call.push_str(&format!(", {{ {} }}", options.join(", ")));
    }
    call.push(')');
    append_lua_line(path, &call)
}

fn delete_bind(path: &Path, index: usize, submap_universal: bool) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let bind = parse_binds_from_project(&project)
        .into_iter()
        .filter(|bind| bind.submap_universal == submap_universal)
        .nth(index)
        .ok_or_else(|| format!("Keybind index {} not found", index))?;
    delete_statement(&project, &bind.statement)
}

/// Builds the key argument, keeping a leading `mainMod ..` style variable when the
/// first modifier still matches its value.
fn keys_expr(
    source: &LuaSource,
    bind: &ParsedBind,
    modifiers: &[String],
    key: &str,
    variables: &HashMap<String, String>,
) -> String {
    let (start, end) = bind.arg_spans[0];
    if let Some((head, _)) = source.masked[start..end].split_once("..") {
        let ident = head.trim();
        let matches_first_modifier = variables.get(ident).is_some_and(|value| {
            modifiers
                .first()
                .is_some_and(|modifier| modifier.eq_ignore_ascii_case(value))
        });
        if is_lua_identifier(ident) && matches_first_modifier {
            let tail = modifiers[1..]
                .iter()
                .map(String::as_str)
                .chain(std::iter::once(key))
                .collect::<Vec<_>>()
                .join(" + ");
            return format!("{} .. {}", ident, lua_quote(&format!(" + {}", tail)));
        }
    }
    lua_quote(&key_string(modifiers, key))
}

/// Sets or clears a boolean in the bind's option table (the third `hl.bind` argument).
fn set_bind_option(source: &LuaSource, bind: &ParsedBind, key: &str, enabled: bool) -> Vec<Edit> {
    let Some(&(opts_start, opts_end)) = bind.arg_spans.get(2) else {
        return if enabled {
            vec![Edit::insert(
                bind.arg_spans[1].1,
                format!(", {{ {} = true }}", key),
            )]
        } else {
            Vec::new()
        };
    };

    let opts = &source.masked[opts_start..opts_end];
    if !opts.starts_with('{') {
        // Options come from a variable; leave them alone.
        return Vec::new();
    }

    let entries = table_entries(opts);
    let existing = entries
        .iter()
        .position(|entry| entry.key.as_deref() == Some(key));

    match (existing, enabled) {
        (Some(idx), true) => vec![Edit::replace(
            (
                opts_start + entries[idx].value_start,
                opts_start + entries[idx].value_end,
            ),
            "true",
        )],
        (Some(_), false) if entries.len() == 1 => {
            // Dropping the only option removes the whole argument.
            vec![Edit::remove((bind.arg_spans[1].1, opts_end))]
        }
        (Some(idx), false) => {
            let range = if idx + 1 == entries.len() {
                (entries[idx - 1].end, entries[idx].end)
            } else {
                (entries[idx].start, entries[idx + 1].start)
            };
            vec![Edit::remove((opts_start + range.0, opts_start + range.1))]
        }
        (None, true) => vec![table_insert_edit(
            &source.masked,
            opts_start,
            opts_end,
            &entries,
            &[(key.to_string(), "true".to_string())],
        )],
        (None, false) => Vec::new(),
    }
}

fn parse_binds(path: &Path) -> Result<Vec<ParsedBind>, String> {
    let project = read_lua_project(path)?;
    Ok(parse_binds_from_project(&project))
}

pub(super) fn parse_binds_from_project(project: &LuaProject) -> Vec<ParsedBind> {
    project
        .sources
        .iter()
        .flat_map(|source| find_calls(source, "hl.bind"))
        .filter_map(|statement| parse_bind_statement(statement, &project.variables))
        .collect()
}

fn parse_bind_statement(
    statement: LuaStatement,
    variables: &HashMap<String, String>,
) -> Option<ParsedBind> {
    let args = statement.args_list();
    if args.len() < 2 {
        return None;
    }

    let keys = eval_lua_string_expr(args[0], variables)?;
    let (dispatcher, params) = parse_dispatcher_expr(args[1], variables);
    let submap_universal = args
        .get(2)
        .and_then(|opts| table_field(opts, "submap_universal"))
        .is_some_and(|value| value.trim() == "true");

    let (modifiers, key) = parse_key_string(&keys);
    let arg_spans = statement.arg_spans();
    Some(ParsedBind {
        statement,
        keybind: Keybind {
            modifiers,
            key,
            dispatcher,
            params,
        },
        submap_universal,
        arg_spans,
    })
}

fn parse_key_string(keys: &str) -> (Vec<String>, String) {
    let parts = keys
        .split('+')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return (Vec::new(), String::new());
    }
    let key = parts.last().unwrap_or(&"").to_string();
    let modifiers = parts[..parts.len().saturating_sub(1)]
        .iter()
        .map(|part| part.to_string())
        .collect();
    (modifiers, key)
}

fn key_string(modifiers: &[String], key: &str) -> String {
    if modifiers.is_empty() {
        key.trim().to_string()
    } else {
        format!("{} + {}", modifiers.join(" + "), key.trim())
    }
}

fn validate_keybind(key: &str, dispatcher: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Key is required".to_string());
    }
    if dispatcher.trim().is_empty() {
        return Err("Dispatcher is required".to_string());
    }
    Ok(())
}
