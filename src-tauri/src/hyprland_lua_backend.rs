use crate::hyprland_backend::{
    EnvVar, Keybind, Layerrule, LayerruleProperty, Variable, Windowrule, WindowruleProperty,
};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::process::Command;

mod dispatchers;
mod lexer;

use dispatchers::*;
use lexer::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HyprlandConfigFormat {
    Hyprlang,
    Lua,
}

#[derive(Debug, Clone)]
pub struct HyprlandConfig {
    pub path: PathBuf,
    pub format: HyprlandConfigFormat,
}

/// Resolves the config the same way Hyprland does: `HYPRLAND_CONFIG` first, then
/// `hypr/hyprland.lua`. Hyprland removed `hyprland.conf` support after 0.56, so the
/// legacy file is only used when no Lua config exists yet.
pub fn get_preferred_config() -> Result<HyprlandConfig, String> {
    if let Ok(path) = std::env::var("HYPRLAND_CONFIG")
        && !path.trim().is_empty()
    {
        let path = PathBuf::from(path);
        return Ok(HyprlandConfig {
            format: format_for_path(&path),
            path,
        });
    }

    let config_dir = if let Ok(xdg_config) = std::env::var("XDG_CONFIG_HOME") {
        PathBuf::from(xdg_config)
    } else {
        let home_dir =
            std::env::var("HOME").map_err(|_| "Could not determine home directory".to_string())?;
        PathBuf::from(home_dir).join(".config")
    };

    let hypr_dir = config_dir.join("hypr");
    let lua_path = hypr_dir.join("hyprland.lua");
    if lua_path.exists() {
        return Ok(HyprlandConfig {
            path: lua_path,
            format: HyprlandConfigFormat::Lua,
        });
    }

    Ok(HyprlandConfig {
        path: hypr_dir.join("hyprland.conf"),
        format: HyprlandConfigFormat::Hyprlang,
    })
}

fn format_for_path(path: &Path) -> HyprlandConfigFormat {
    if path.extension().and_then(|e| e.to_str()) == Some("lua") {
        HyprlandConfigFormat::Lua
    } else {
        HyprlandConfigFormat::Hyprlang
    }
}

// ============================================================================
// Project model
// ============================================================================

#[derive(Debug, Clone)]
struct LuaSource {
    path: PathBuf,
    contents: String,
    /// `contents` with every comment blanked out. Byte offsets match `contents`, so
    /// spans found in `masked` can be applied to the original text.
    masked: String,
}

impl LuaSource {
    fn new(path: PathBuf, contents: String) -> Self {
        let masked = mask_lua_comments(&contents);
        Self {
            path,
            contents,
            masked,
        }
    }
}

#[derive(Debug)]
struct LuaProject {
    sources: Vec<LuaSource>,
    variables: HashMap<String, String>,
}

impl LuaProject {
    fn source(&self, path: &Path) -> Option<&LuaSource> {
        self.sources.iter().find(|source| source.path == path)
    }
}

/// A `hl.<fn>(...)` call.
#[derive(Debug, Clone)]
struct LuaStatement {
    path: PathBuf,
    /// Offset of the call expression (`hl.`).
    start: usize,
    /// Offset one past the closing parenthesis.
    end: usize,
    /// Offset of the first byte inside the parentheses.
    args_start: usize,
    /// Argument source with comments blanked out.
    args: String,
}

impl LuaStatement {
    /// Absolute spans of the top-level call arguments.
    fn arg_spans(&self) -> Vec<(usize, usize)> {
        split_top_level_spans(&self.args, b',')
            .into_iter()
            .map(|(start, end)| (self.args_start + start, self.args_start + end))
            .collect()
    }

    fn args_list(&self) -> Vec<&str> {
        split_top_level_spans(&self.args, b',')
            .into_iter()
            .map(|(start, end)| &self.args[start..end])
            .collect()
    }
}

#[derive(Debug, Clone)]
struct LuaVariable {
    name: String,
    value: String,
    path: PathBuf,
    /// Assigned without `local`, so visible from every file loaded afterwards.
    global: bool,
    value_start: usize,
    value_end: usize,
    remove_start: usize,
    remove_end: usize,
}

#[derive(Debug)]
struct ParsedBind {
    statement: LuaStatement,
    keybind: Keybind,
    submap_universal: bool,
    /// Absolute spans of the keys, dispatcher and (optional) options arguments.
    arg_spans: Vec<(usize, usize)>,
}

#[derive(Debug)]
struct Edit {
    start: usize,
    end: usize,
    text: String,
}

impl Edit {
    fn replace((start, end): (usize, usize), text: impl Into<String>) -> Self {
        Self {
            start,
            end,
            text: text.into(),
        }
    }

    fn insert(at: usize, text: impl Into<String>) -> Self {
        Self::replace((at, at), text)
    }

    fn remove((start, end): (usize, usize)) -> Self {
        Self::replace((start, end), "")
    }
}

// ============================================================================
// Keybinds
// ============================================================================

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

fn parse_binds_from_project(project: &LuaProject) -> Vec<ParsedBind> {
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

// ============================================================================
// Variables
// ============================================================================

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

fn parse_source_variables(
    source: &LuaSource,
    variables: &HashMap<String, String>,
) -> Vec<LuaVariable> {
    let mut parsed = parse_return_table_variables(source, variables);
    parsed.extend(parse_assignment_variables(source, variables));
    parsed
}

/// Identifiers a new expression in `source` can reference: names assigned in that file
/// plus globals assigned anywhere in the project.
fn assigned_identifiers(source: &LuaSource, project: &LuaProject) -> HashSet<String> {
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

fn string_vars_map_for_sources(sources: &[LuaSource]) -> HashMap<String, String> {
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

// ============================================================================
// Environment variables
// ============================================================================

pub fn get_env_vars(path: &Path) -> Result<Vec<EnvVar>, String> {
    let project = read_lua_project(path)?;
    let root = path.parent().unwrap_or_else(|| Path::new(""));
    let mut env_vars = Vec::new();

    for (index, statement) in project_calls(&project, "hl.env").into_iter().enumerate() {
        let args = statement.args_list();
        if args.len() < 2 {
            continue;
        }
        let Some(name) = eval_lua_string_expr(args[0], &project.variables) else {
            continue;
        };
        let Some(value) = eval_lua_string_expr(args[1], &project.variables) else {
            continue;
        };
        env_vars.push(EnvVar {
            name,
            value,
            index,
            source_file: Some(display_source_file(root, &statement.path)),
        });
    }

    env_vars.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(env_vars)
}

pub fn add_env_var(path: &Path, name: String, value: String) -> Result<(), String> {
    validate_env_name(&name)?;
    append_lua_line(
        path,
        &format!("hl.env({}, {})", lua_quote(&name), lua_quote(&value)),
    )
}

pub fn edit_env_var(path: &Path, index: usize, name: String, value: String) -> Result<(), String> {
    validate_env_name(&name)?;
    let project = read_lua_project(path)?;
    let statement = project_calls(&project, "hl.env")
        .into_iter()
        .nth(index)
        .ok_or_else(|| format!("Environment variable index {} not found", index))?;
    let source = project
        .source(&statement.path)
        .ok_or_else(|| format!("Source for environment variable {} not found", index))?;
    apply_edits(
        source,
        vec![Edit::replace(
            (statement.start, statement.end),
            format!("hl.env({}, {})", lua_quote(&name), lua_quote(&value)),
        )],
    )
}

pub fn delete_env_var(path: &Path, index: usize) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let statement = project_calls(&project, "hl.env")
        .into_iter()
        .nth(index)
        .ok_or_else(|| format!("Environment variable index {} not found", index))?;
    delete_statement(&project, &statement)
}

// ============================================================================
// Monitors
// ============================================================================

/// Mode, position and scale for a single output.
pub struct MonitorLayout {
    pub width: u16,
    pub height: u16,
    pub refresh_rate: f32,
    pub x: i32,
    pub y: i32,
    pub scale: f32,
}

fn monitor_fields(layout: &MonitorLayout) -> Vec<(String, String)> {
    vec![
        (
            "mode".to_string(),
            lua_quote(&format!(
                "{}x{}@{:.2}Hz",
                layout.width, layout.height, layout.refresh_rate
            )),
        ),
        (
            "position".to_string(),
            lua_quote(&format!("{}x{}", layout.x, layout.y)),
        ),
        ("scale".to_string(), format!("{}", layout.scale)),
    ]
}

/// Lua for `hyprctl eval`: `hl.monitor` merges into the existing rule for the output
/// and schedules a monitor refresh, replacing the removed `hyprctl keyword monitor`.
pub fn monitor_eval_code(name: &str, layout: &MonitorLayout) -> String {
    let fields = monitor_fields(layout)
        .into_iter()
        .map(|(key, value)| format!("{} = {}", key, value))
        .collect::<Vec<_>>();
    format!(
        "hl.monitor({{ output = {}, {} }})",
        lua_quote(name),
        fields.join(", ")
    )
}

/// Updates `mode`, `position` and `scale` of the matching `hl.monitor` rule in place,
/// keeping every other field (transform, vrr, bitdepth, cm, ...).
pub fn save_monitor_settings(
    path: &Path,
    name: String,
    layout: &MonitorLayout,
) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let fields = monitor_fields(layout);

    for source in &project.sources {
        for statement in find_calls(source, "hl.monitor") {
            let Some(&(table_start, table_end)) = statement.arg_spans().first() else {
                continue;
            };
            let table = &source.masked[table_start..table_end];
            if !table.starts_with('{') {
                continue;
            }
            let entries = table_entries(table);
            let output = entries
                .iter()
                .find(|entry| entry.key.as_deref() == Some("output"))
                .and_then(|entry| {
                    eval_lua_string_expr(
                        &table[entry.value_start..entry.value_end],
                        &project.variables,
                    )
                });
            if output.as_deref() != Some(name.as_str()) {
                continue;
            }

            let mut edits = Vec::new();
            let mut missing = Vec::new();
            for (key, value) in &fields {
                match entries
                    .iter()
                    .find(|entry| entry.key.as_deref() == Some(key.as_str()))
                {
                    Some(entry) => edits.push(Edit::replace(
                        (
                            table_start + entry.value_start,
                            table_start + entry.value_end,
                        ),
                        value.clone(),
                    )),
                    None => missing.push((key.clone(), value.clone())),
                }
            }
            if !missing.is_empty() {
                edits.push(table_insert_edit(
                    &source.masked,
                    table_start,
                    table_end,
                    &entries,
                    &missing,
                ));
            }
            return apply_edits(source, edits);
        }
    }

    let mut block = format!("hl.monitor({{\n    output = {},\n", lua_quote(&name));
    for (key, value) in fields {
        block.push_str(&format!("    {} = {},\n", key, value));
    }
    block.push_str("})");
    append_lua_line(path, &block)
}

// ============================================================================
// Window and layer rules
// ============================================================================

pub fn get_windowrule_names(path: &Path) -> Result<Vec<String>, String> {
    get_rule_names(path, "window_rule")
}

pub fn get_layerrule_names(path: &Path) -> Result<Vec<String>, String> {
    get_rule_names(path, "layer_rule")
}

pub fn get_windowrule(path: &Path, name: String) -> Result<Windowrule, String> {
    let project = read_lua_project(path)?;
    let statement = find_rule_statement(&project, "window_rule", &name)
        .ok_or_else(|| format!("Windowrule '{}' not found", name))?;
    let RuleProperties {
        enabled,
        matches,
        effects,
    } = rule_properties(&statement, &project.variables);

    Ok(Windowrule {
        name,
        enabled,
        match_properties: matches
            .into_iter()
            .map(|(key, value)| WindowruleProperty {
                key,
                value,
                property_type: "match".to_string(),
            })
            .collect(),
        effect_properties: effects
            .into_iter()
            .map(|(key, value)| WindowruleProperty {
                key,
                value,
                property_type: "effect".to_string(),
            })
            .collect(),
    })
}

pub fn get_layerrule(path: &Path, name: String) -> Result<Layerrule, String> {
    let project = read_lua_project(path)?;
    let statement = find_rule_statement(&project, "layer_rule", &name)
        .ok_or_else(|| format!("Layerrule '{}' not found", name))?;
    let RuleProperties {
        enabled,
        matches,
        effects,
    } = rule_properties(&statement, &project.variables);

    Ok(Layerrule {
        name,
        enabled,
        match_properties: matches
            .into_iter()
            .map(|(key, value)| LayerruleProperty {
                key,
                value,
                property_type: "match".to_string(),
            })
            .collect(),
        effect_properties: effects
            .into_iter()
            .map(|(key, value)| LayerruleProperty {
                key,
                value,
                property_type: "effect".to_string(),
            })
            .collect(),
    })
}

pub fn delete_windowrule(path: &Path, name: String) -> Result<(), String> {
    delete_rule(path, "window_rule", &name)
}

pub fn delete_layerrule(path: &Path, name: String) -> Result<(), String> {
    delete_rule(path, "layer_rule", &name)
}

struct RuleProperties {
    enabled: bool,
    matches: Vec<(String, String)>,
    effects: Vec<(String, String)>,
}

/// Every non-meta key is reported. Hyprland also accepts effects registered by plugins,
/// so filtering against a fixed list would hide valid rules.
fn rule_properties(
    statement: &LuaStatement,
    variables: &HashMap<String, String>,
) -> RuleProperties {
    let table = statement.args_list().first().copied().unwrap_or("");
    let fields = parse_lua_table_fields(table);
    let enabled = table_value(&fields, "enabled").is_none_or(|value| value.trim() != "false");
    let matches = match table_value(&fields, "match").map(str::trim) {
        Some(table) if table.starts_with('{') => parse_lua_table_fields(table)
            .into_iter()
            .map(|(key, value)| (key, rule_value_to_display(&value, variables)))
            .collect(),
        // `match = m` inside a loop: show the expression rather than nothing.
        Some(expr) => vec![("match".to_string(), collapse_whitespace(expr))],
        None => Vec::new(),
    };
    let effects = fields
        .into_iter()
        .filter(|(key, _)| !matches!(key.as_str(), "name" | "enabled" | "match"))
        .map(|(key, value)| (key, rule_value_to_display(&value, variables)))
        .collect();
    RuleProperties {
        enabled,
        matches,
        effects,
    }
}

/// Resolves variable references (`workspace = gamingWorkspace`) to their value.
fn rule_value_to_display(value: &str, variables: &HashMap<String, String>) -> String {
    let value = value.trim();
    if parse_lua_string_literal(value).is_none()
        && !value.starts_with('{')
        && let Some(resolved) = eval_lua_string_expr(value, variables).filter(|v| !v.is_empty())
    {
        return resolved;
    }
    lua_value_to_display(value)
}

fn unnamed_rule_label(function: &str, position: usize) -> String {
    format!("unnamed {} #{}", function.replace('_', " "), position)
}

/// Rule names, with a positional label for rules without a `name` field.
fn rule_labels(project: &LuaProject, function: &str) -> Vec<(String, LuaStatement)> {
    let mut unnamed = 0;
    project_calls(project, &format!("hl.{}", function))
        .into_iter()
        .map(|statement| {
            let fields = statement
                .args_list()
                .first()
                .map(|table| parse_lua_table_fields(table))
                .unwrap_or_default();
            let name = table_value(&fields, "name")
                .and_then(|value| eval_lua_string_expr(value, &project.variables))
                .unwrap_or_else(|| {
                    unnamed += 1;
                    unnamed_rule_label(function, unnamed)
                });
            (name, statement)
        })
        .collect()
}

fn get_rule_names(path: &Path, function: &str) -> Result<Vec<String>, String> {
    let project = read_lua_project(path)?;
    Ok(rule_labels(&project, function)
        .into_iter()
        .map(|(name, _)| name)
        .collect())
}

fn find_rule_statement(project: &LuaProject, function: &str, name: &str) -> Option<LuaStatement> {
    rule_labels(project, function)
        .into_iter()
        .find(|(label, _)| label == name)
        .map(|(_, statement)| statement)
}

fn delete_rule(path: &Path, function: &str, name: &str) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let statement = find_rule_statement(&project, function, name)
        .ok_or_else(|| format!("Rule '{}' not found", name))?;
    delete_statement(&project, &statement)
}

// ============================================================================
// File discovery
// ============================================================================

fn read_lua_config(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("Hyprland Lua config file not found at {:?}", path));
    }
    fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))
}

fn read_lua_source(path: &Path) -> Result<LuaSource, String> {
    Ok(LuaSource::new(path.to_path_buf(), read_lua_config(path)?))
}

fn read_lua_project(path: &Path) -> Result<LuaProject, String> {
    let sources = discover_lua_sources(path)?
        .iter()
        .map(|path| read_lua_source(path))
        .collect::<Result<Vec<_>, _>>()?;
    let variables = string_vars_map_for_sources(&sources);
    Ok(LuaProject { sources, variables })
}

fn discover_lua_sources(entrypoint: &Path) -> Result<Vec<PathBuf>, String> {
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

fn normalize_lua_path(path: &Path) -> PathBuf {
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

fn display_source_file(root: &Path, path: &Path) -> String {
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

// ============================================================================
// Writing
// ============================================================================

fn append_lua_line(path: &Path, line: &str) -> Result<(), String> {
    let mut contents = read_lua_config(path)?;
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    contents.push_str(line);
    contents.push('\n');
    fs::write(path, contents).map_err(|e| format!("Failed to write {:?}: {}", path, e))?;
    reload_hyprland()
}

fn apply_edits(source: &LuaSource, mut edits: Vec<Edit>) -> Result<(), String> {
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

fn delete_statement(project: &LuaProject, statement: &LuaStatement) -> Result<(), String> {
    let source = project
        .source(&statement.path)
        .ok_or_else(|| format!("Source {:?} not found", statement.path))?;
    let range = removal_range(&source.masked, statement.start, statement.end);
    apply_edits(source, vec![Edit::remove(range)])
}

/// The range to delete for `[start, end)`: the whole line when nothing else but
/// whitespace, a trailing comment or a `local name =` prefix shares it.
fn removal_range(masked: &str, start: usize, end: usize) -> (usize, usize) {
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
fn table_insert_edit(
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

    let joined = fields
        .iter()
        .map(|(key, value)| format!("{} = {}", key, value))
        .collect::<Vec<_>>()
        .join(", ");
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

// ============================================================================
// Finding calls
// ============================================================================

/// Calls to `needle` (for example `hl.bind`) in a source.
fn find_calls(source: &LuaSource, needle: &str) -> Vec<LuaStatement> {
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

fn project_calls(project: &LuaProject, needle: &str) -> Vec<LuaStatement> {
    project
        .sources
        .iter()
        .flat_map(|source| find_calls(source, needle))
        .collect()
}

// ============================================================================
// Validation
// ============================================================================

fn validate_keybind(key: &str, dispatcher: &str) -> Result<(), String> {
    if key.trim().is_empty() {
        return Err("Key is required".to_string());
    }
    if dispatcher.trim().is_empty() {
        return Err("Dispatcher is required".to_string());
    }
    Ok(())
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

fn validate_env_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Environment variable name cannot be empty".to_string());
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(
            "Environment variable name must contain only letters, numbers, and underscores"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn layout(
        width: u16,
        height: u16,
        refresh_rate: f32,
        x: i32,
        y: i32,
        scale: f32,
    ) -> MonitorLayout {
        MonitorLayout {
            width,
            height,
            refresh_rate,
            x,
            y,
            scale,
        }
    }

    fn source(contents: &str) -> LuaSource {
        LuaSource::new(PathBuf::from("hyprland.lua"), contents.to_string())
    }

    fn project(contents: &str) -> LuaProject {
        let sources = vec![source(contents)];
        let variables = string_vars_map_for_sources(&sources);
        LuaProject { sources, variables }
    }

    fn parse_binds_from_contents(contents: &str) -> Vec<ParsedBind> {
        parse_binds_from_project(&project(contents))
    }

    /// A throwaway config directory, removed on drop.
    struct TempConfig(PathBuf);

    impl TempConfig {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "hyprconfig-lua-{}-{}",
                name,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn write(&self, relative: &str, contents: &str) -> PathBuf {
            let path = self.0.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, contents).unwrap();
            path
        }

        fn read(&self, relative: &str) -> String {
            fs::read_to_string(self.0.join(relative)).unwrap()
        }
    }

    impl Drop for TempConfig {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn parses_lua_keybinds_with_locals_and_universal_option() {
        let contents = r#"
local mainMod = "SUPER"
local terminal = "kitty"

hl.bind(mainMod .. " + Q", hl.dsp.exec_cmd(terminal))
hl.bind("SUPER + SHIFT + 1", hl.dsp.window.move({ workspace = 1 }), { submap_universal = true })
"#;

        let binds = parse_binds_from_contents(contents);

        assert_eq!(binds.len(), 2);
        assert!(!binds[0].submap_universal);
        assert_eq!(binds[0].keybind.modifiers, vec!["SUPER"]);
        assert_eq!(binds[0].keybind.key, "Q");
        assert_eq!(binds[0].keybind.dispatcher, "exec");
        assert_eq!(binds[0].keybind.params, "$terminal");

        assert!(binds[1].submap_universal);
        assert_eq!(binds[1].keybind.modifiers, vec!["SUPER", "SHIFT"]);
        assert_eq!(binds[1].keybind.dispatcher, "movetoworkspace");
        assert_eq!(binds[1].keybind.params, "1");
    }

    #[test]
    fn preserves_double_dash_inside_lua_string_literals() {
        let contents = r#"
local launcher = "wofi --show drun"

hl.bind("SUPER + D", hl.dsp.exec_cmd("wofi --show drun")) -- application launcher
hl.bind("SUPER + R", hl.dsp.exec_cmd(launcher))
"#;

        let binds = parse_binds_from_contents(contents);

        assert_eq!(binds.len(), 2);
        assert_eq!(binds[0].keybind.dispatcher, "exec");
        assert_eq!(binds[0].keybind.params, "wofi --show drun");
        assert_eq!(binds[1].keybind.dispatcher, "exec");
        assert_eq!(binds[1].keybind.params, "$launcher");
    }

    #[test]
    fn strips_lua_line_comments_outside_string_literals() {
        assert_eq!(
            strip_lua_line_comment(r#""wofi --show drun" -- launcher"#).trim(),
            r#""wofi --show drun""#
        );
        assert_eq!(
            strip_lua_line_comment(r#""quoted \"--\" value" -- comment"#).trim(),
            r#""quoted \"--\" value""#
        );
    }

    #[test]
    fn masks_line_and_block_comments_but_keeps_offsets() {
        let contents = "a = 1 -- it's a comment\n--[[ block, with \"quotes\"\n]] b = '--not'\n";
        let masked = mask_lua_comments(contents);
        assert_eq!(masked.len(), contents.len());
        assert_eq!(masked.lines().next().unwrap().trim_end(), "a = 1");
        assert!(masked.contains("b = '--not'"));
        assert!(!masked.contains("block"));
    }

    #[test]
    fn parses_lua_rules_with_nested_match_tables() {
        let contents = r#"
hl.window_rule({
    name = "suppress-maximize-events",
    match = { class = ".*", float = false },
    suppress_event = "maximize",
})
"#;

        let statement = find_rule_statement(
            &project(contents),
            "window_rule",
            "suppress-maximize-events",
        )
        .unwrap();
        let RuleProperties {
            enabled,
            matches,
            effects,
        } = rule_properties(&statement, &HashMap::new());

        assert!(enabled);
        assert!(matches.contains(&("class".to_string(), ".*".to_string())));
        assert!(matches.contains(&("float".to_string(), "false".to_string())));
        assert_eq!(
            effects,
            vec![("suppress_event".to_string(), "maximize".to_string())]
        );
    }

    #[test]
    fn reads_rules_from_the_default_example_config() {
        // Comments with apostrophes used to swallow the rest of the table.
        let contents = r#"
local suppressMaximizeRule = hl.window_rule({
    -- Ignore maximize requests from all apps. You'll probably like this.
    name  = "suppress-maximize-events",
    match = { class = ".*" },

    suppress_event = "maximize",
})

hl.window_rule({
    -- Fix some dragging issues with XWayland
    name  = "fix-xwayland-drags",
    match = {
        class      = "^$",
        xwayland   = true,
    },

    no_focus = true,
})

hl.window_rule({
    match = { class = "hyprland-run" },
    move  = "20 monitor_h-120",
    float = true,
    no_glow = true,
})

hl.layer_rule({ match = { namespace = "^my-overlay$" }, no_anim = true })
"#;
        let project = project(contents);
        let names = rule_labels(&project, "window_rule")
            .into_iter()
            .map(|(name, _)| name)
            .collect::<Vec<_>>();
        assert_eq!(
            names,
            vec![
                "suppress-maximize-events",
                "fix-xwayland-drags",
                "unnamed window rule #1"
            ]
        );

        let unnamed =
            find_rule_statement(&project, "window_rule", "unnamed window rule #1").unwrap();
        let RuleProperties {
            matches, effects, ..
        } = rule_properties(&unnamed, &project.variables);
        assert_eq!(
            matches,
            vec![("class".to_string(), "hyprland-run".to_string())]
        );
        // Effects Hyprland added after the last sync (no_glow) are reported as well.
        assert!(effects.contains(&("no_glow".to_string(), "true".to_string())));

        let layer = rule_labels(&project, "layer_rule");
        assert_eq!(layer[0].0, "unnamed layer rule #1");
    }

    #[test]
    fn resolves_variables_in_rule_values() {
        let project = project(
            r#"local gamingWorkspace = "name:gaming"
local floatApps = { { class = "^(pavucontrol)$" } }
hl.window_rule({ match = { class = "^(steam)$" }, workspace = gamingWorkspace })
for _, m in ipairs(floatApps) do hl.window_rule({ match = m, float = true }) end
"#,
        );
        let rules = rule_labels(&project, "window_rule");
        let effects = rule_properties(&rules[0].1, &project.variables).effects;
        assert_eq!(
            effects,
            vec![("workspace".to_string(), "name:gaming".to_string())]
        );
        let matches = rule_properties(&rules[1].1, &project.variables).matches;
        assert_eq!(matches, vec![("match".to_string(), "m".to_string())]);
    }

    #[test]
    fn templates_glued_variables_with_braces() {
        let variables = HashMap::from([
            ("noctCall".to_string(), "noctalia msg ".to_string()),
            ("launchPrefix".to_string(), "uwsm app -- ".to_string()),
            ("TERMINAL".to_string(), "ghostty".to_string()),
        ]);
        let scope = variables.keys().cloned().collect::<HashSet<_>>();

        for (expr, template) in [
            (
                "noctCall .. \"panel-toggle launcher\"",
                "${noctCall}panel-toggle launcher",
            ),
            (
                "launchPrefix .. TERMINAL .. \" -e btop\"",
                "$launchPrefix$TERMINAL -e btop",
            ),
            ("TERMINAL", "$TERMINAL"),
        ] {
            assert_eq!(lua_expr_to_template(expr, &variables).unwrap(), template);
            assert_eq!(lua_string_expr_with_vars(template, &scope), expr);
        }
    }

    #[test]
    fn writes_exec_dispatchers_with_lua_variable_expansion() {
        let scope = HashSet::from(["terminal".to_string()]);
        assert_eq!(
            dispatcher_to_lua_expr("exec", "$terminal --hold", &scope).unwrap(),
            "hl.dsp.exec_cmd(terminal .. \" --hold\")"
        );
        // Unknown names stay literal so shell variables keep working.
        assert_eq!(
            dispatcher_to_lua_expr("exec", "echo $HOME", &scope).unwrap(),
            "hl.dsp.exec_cmd(\"echo $HOME\")"
        );
        assert_eq!(
            dispatcher_to_lua_expr("exec", "$terminalx", &scope).unwrap(),
            "hl.dsp.exec_cmd(terminal .. \"x\")"
        );
    }

    #[test]
    fn legacy_dispatchers_round_trip_through_lua() {
        let cases = [
            ("exec", "kitty --single-instance"),
            ("execr", "notify-send hi"),
            ("killactive", ""),
            ("forcekillactive", ""),
            ("closewindow", "class:kitty"),
            ("killwindow", "class:kitty"),
            ("signal", "9"),
            ("signalwindow", "class:kitty,15"),
            ("togglefloating", ""),
            ("togglefloating", "class:mpv"),
            ("setfloating", ""),
            ("settiled", ""),
            ("pseudo", ""),
            ("pin", ""),
            ("workspace", "3"),
            ("workspace", "e+1"),
            ("focusworkspaceoncurrentmonitor", "2"),
            ("renameworkspace", "3 web"),
            ("fullscreen", ""),
            ("fullscreen", "1"),
            ("fullscreen", "0 set"),
            ("fullscreenstate", "2 0"),
            ("movetoworkspace", "4"),
            ("movetoworkspace", "special:magic"),
            ("movetoworkspacesilent", "5"),
            ("movetoworkspacesilent", "5,class:kitty"),
            ("movefocus", "left"),
            ("movewindow", "right"),
            ("movewindow", "mon:DP-1"),
            ("movewindow", "mon:DP-1 silent"),
            ("movewindoworgroup", "up"),
            ("swapwindow", "down"),
            ("swapnext", ""),
            ("swapnext", "prev"),
            ("centerwindow", ""),
            ("cyclenext", ""),
            ("cyclenext", "prev tiled"),
            ("focuswindow", "class:firefox"),
            ("focusmonitor", "DP-2"),
            ("focusurgentorlast", ""),
            ("focuscurrentorlast", ""),
            ("resizeactive", "10 -10"),
            ("resizeactive", "exact 800 600"),
            ("moveactive", "-20 0"),
            ("resizewindowpixel", "exact 640 480,class:kitty"),
            ("movewindowpixel", "10 10,class:kitty"),
            ("tagwindow", "work"),
            ("tagwindow", "work class:kitty"),
            ("toggleswallow", ""),
            ("bringactivetotop", ""),
            ("alterzorder", "top"),
            ("alterzorder", "bottom,class:kitty"),
            ("setprop", "class:kitty opaque 1"),
            ("mouse", "movewindow"),
            ("mouse", "resizewindow"),
            ("mouse", "resizewindow 1"),
            ("togglegroup", ""),
            ("changegroupactive", "f"),
            ("changegroupactive", "b"),
            ("changegroupactive", "2"),
            ("movegroupwindow", ""),
            ("movegroupwindow", "b"),
            ("lockgroups", "toggle"),
            ("lockgroups", "lock"),
            ("lockactivegroup", "unlock"),
            ("denywindowfromgroup", "on"),
            ("moveintogroup", "left"),
            ("moveintoorcreategroup", "right"),
            ("moveoutofgroup", ""),
            ("movecursortocorner", "2"),
            ("movecursor", "100 200"),
            ("movecurrentworkspacetomonitor", "DP-1"),
            ("moveworkspacetomonitor", "3 HDMI-A-1"),
            ("swapactiveworkspaces", "DP-1 DP-2"),
            ("togglespecialworkspace", ""),
            ("togglespecialworkspace", "magic"),
            ("submap", "resize"),
            ("pass", "class:obs"),
            ("sendshortcut", "SUPER, Q, class:kitty"),
            ("sendshortcut", "CTRL, C"),
            ("sendkeystate", "SUPER, Q, down, class:kitty"),
            ("layoutmsg", "togglesplit"),
            ("dpms", "off"),
            ("dpms", "toggle DP-1"),
            ("exit", ""),
            ("forcerendererreload", ""),
            ("forceidle", "10"),
            ("releaseinputcapture", ""),
            ("event", "hello"),
            ("global", "app:toggle"),
        ];

        let scope = HashSet::new();
        let variables = HashMap::new();
        for (dispatcher, params) in cases {
            let expr = dispatcher_to_lua_expr(dispatcher, params, &scope)
                .unwrap_or_else(|e| panic!("{} {:?}: {}", dispatcher, params, e));
            assert!(expr.starts_with("hl.dsp."), "{} -> {}", dispatcher, expr);
            assert_eq!(
                parse_dispatcher_expr(&expr, &variables),
                (dispatcher.to_string(), params.to_string()),
                "round trip of {} {:?} via {}",
                dispatcher,
                params,
                expr
            );
        }
    }

    #[test]
    fn translates_legacy_dispatchers_to_current_lua_api() {
        let scope = HashSet::new();
        let lua = |dispatcher: &str, params: &str| {
            dispatcher_to_lua_expr(dispatcher, params, &scope).unwrap()
        };

        assert_eq!(
            lua("movefocus", "l"),
            "hl.dsp.focus({ direction = \"left\" })"
        );
        assert_eq!(
            lua("togglefloating", ""),
            "hl.dsp.window.float({ action = \"toggle\" })"
        );
        assert_eq!(lua("workspace", "1"), "hl.dsp.focus({ workspace = 1 })");
        assert_eq!(
            lua("movetoworkspacesilent", "special:magic"),
            "hl.dsp.window.move({ workspace = \"special:magic\", follow = false })"
        );
        assert_eq!(lua("dpms", "off"), "hl.dsp.dpms({ action = \"off\" })");
        assert_eq!(lua("mouse", "movewindow"), "hl.dsp.window.drag()");
        assert_eq!(
            lua("resizeactive", "exact 800 600"),
            "hl.dsp.window.resize({ x = 800, y = 600 })"
        );
        assert_eq!(lua("lua", "hl.dsp.no_op()"), "hl.dsp.no_op()");

        // hyprctl dispatch no longer takes legacy syntax, so unknown dispatchers are an
        // error instead of a broken `hyprctl dispatch` exec.
        assert!(dispatcher_to_lua_expr("toggleopaque", "", &scope).is_err());
        assert!(dispatcher_to_lua_expr("resizeactive", "50% 50%", &scope).is_err());
        assert!(dispatcher_to_lua_expr("lua", "", &scope).is_err());
    }

    #[test]
    fn unrepresentable_dispatchers_fall_back_to_raw_lua() {
        let variables = HashMap::new();
        for expr in [
            "hl.dsp.workspace.change_id({ workspace = 3, id = 7 })",
            "hl.dsp.window.fullscreen({ mode = \"maximized\", layout_aware = false })",
            "hl.dsp.no_op()",
            "hl.dsp.window.clear_tags()",
            "function() hl.dispatch(hl.dsp.exit()) end",
        ] {
            assert_eq!(
                parse_dispatcher_expr(expr, &variables),
                ("lua".to_string(), expr.to_string())
            );
        }

        let binds = parse_binds_from_contents(
            "hl.bind(\"SUPER + X\", function()\n    -- say hi\n    hl.notification.create({ text = \"hi, there\", timeout = 1 })\nend)\n",
        );
        assert_eq!(binds.len(), 1);
        assert_eq!(binds[0].keybind.dispatcher, "lua");
        assert_eq!(
            binds[0].keybind.params,
            "function() hl.notification.create({ text = \"hi, there\", timeout = 1 }) end"
        );
    }

    #[test]
    fn edits_only_the_changed_bind_arguments() {
        let dir = TempConfig::new("edit-bind");
        let config = dir.write(
            "hyprland.lua",
            r#"local mainMod = "SUPER"
local terminal = "kitty"

hl.bind(mainMod .. " + Q",  hl.dsp.exec_cmd(terminal))   -- terminal
local closeBind = hl.bind(mainMod .. " + C", hl.dsp.window.close())
hl.bind("XF86AudioMute", hl.dsp.exec_cmd("wpctl set-mute @DEFAULT_AUDIO_SINK@ toggle"), { locked = true, repeating = true })
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.drag(), { mouse = true })
"#,
        );

        // Changing only the key keeps mainMod, the dispatcher and the trailing comment.
        edit_keybind(
            &config,
            0,
            vec!["SUPER".into()],
            "T".into(),
            "exec".into(),
            "$terminal".into(),
        )
        .unwrap();
        // Changing the dispatcher keeps the `local closeBind =` handle.
        edit_keybind(
            &config,
            1,
            vec!["SUPER".into()],
            "C".into(),
            "forcekillactive".into(),
            "".into(),
        )
        .unwrap();
        // Bind options survive a dispatcher change.
        edit_keybind(
            &config,
            2,
            vec![],
            "XF86AudioMute".into(),
            "exec".into(),
            "pamixer -t".into(),
        )
        .unwrap();
        // Leaving the mouse dispatcher drops `mouse = true`.
        edit_keybind(
            &config,
            3,
            vec!["SUPER".into()],
            "mouse:272".into(),
            "togglefloating".into(),
            "".into(),
        )
        .unwrap();

        assert_eq!(
            dir.read("hyprland.lua"),
            r#"local mainMod = "SUPER"
local terminal = "kitty"

hl.bind(mainMod .. " + T",  hl.dsp.exec_cmd(terminal))   -- terminal
local closeBind = hl.bind(mainMod .. " + C", hl.dsp.window.kill())
hl.bind("XF86AudioMute", hl.dsp.exec_cmd("pamixer -t"), { locked = true, repeating = true })
hl.bind(mainMod .. " + mouse:272", hl.dsp.window.float({ action = "toggle" }))
"#
        );

        // And switching to the mouse dispatcher adds it back.
        edit_keybind(
            &config,
            3,
            vec!["SUPER".into()],
            "mouse:272".into(),
            "mouse".into(),
            "movewindow".into(),
        )
        .unwrap();
        assert!(dir.read("hyprland.lua").contains(
            "hl.bind(mainMod .. \" + mouse:272\", hl.dsp.window.drag(), { mouse = true })"
        ));
    }

    #[test]
    fn deletes_whole_lines_and_adds_binds_with_options() {
        let dir = TempConfig::new("delete-bind");
        let config = dir.write(
            "hyprland.lua",
            "hl.bind(\"SUPER + Q\", hl.dsp.exec_cmd(\"kitty\")) -- terminal\nlocal b = hl.bind(\"SUPER + C\", hl.dsp.window.close())\nhl.env(\"A\", \"1\")\n",
        );

        delete_keybind(&config, 1).unwrap();
        delete_keybind(&config, 0).unwrap();
        assert_eq!(dir.read("hyprland.lua"), "hl.env(\"A\", \"1\")\n");

        add_keybind(
            &config,
            vec!["SUPER".into()],
            "mouse:273".into(),
            "mouse".into(),
            "resizewindow".into(),
        )
        .unwrap();
        add_bindu(
            &config,
            vec!["SUPER".into()],
            "Escape".into(),
            "submap".into(),
            "reset".into(),
        )
        .unwrap();
        assert_eq!(
            dir.read("hyprland.lua"),
            "hl.env(\"A\", \"1\")\nhl.bind(\"SUPER + mouse:273\", hl.dsp.window.resize(), { mouse = true })\nhl.bind(\"SUPER + Escape\", hl.dsp.submap(\"reset\"), { submap_universal = true })\n"
        );
        assert_eq!(get_all_bindu(&config).unwrap().len(), 1);
        assert_eq!(get_keybinds(&config).unwrap()[0].dispatcher, "mouse");

        assert!(
            add_keybind(
                &config,
                vec![],
                "F1".into(),
                "toggleopaque".into(),
                "".into()
            )
            .is_err()
        );
    }

    #[test]
    fn saves_monitor_settings_without_dropping_other_fields() {
        let dir = TempConfig::new("monitor");
        let config = dir.write(
            "hyprland.lua",
            r#"hl.monitor({
    output   = "DP-1",
    mode     = "preferred", -- native
    transform = 1,
    vrr      = 2
})
hl.monitor({ output = "HDMI-A-1", bitdepth = 10 })
"#,
        );

        save_monitor_settings(
            &config,
            "DP-1".into(),
            &layout(2560, 1440, 164.96, 0, 0, 1.25),
        )
        .unwrap();
        save_monitor_settings(
            &config,
            "HDMI-A-1".into(),
            &layout(1920, 1080, 60.0, 2560, 0, 1.0),
        )
        .unwrap();
        save_monitor_settings(
            &config,
            "eDP-1".into(),
            &layout(1920, 1200, 60.0, 0, 1440, 1.5),
        )
        .unwrap();

        assert_eq!(
            dir.read("hyprland.lua"),
            r#"hl.monitor({
    output   = "DP-1",
    mode     = "2560x1440@164.96Hz", -- native
    transform = 1,
    vrr      = 2,
    position = "0x0",
    scale = 1.25,
})
hl.monitor({ output = "HDMI-A-1", bitdepth = 10, mode = "1920x1080@60.00Hz", position = "2560x0", scale = 1 })
hl.monitor({
    output = "eDP-1",
    mode = "1920x1200@60.00Hz",
    position = "0x1440",
    scale = 1.5,
})
"#
        );

        assert_eq!(
            monitor_eval_code("DP-1", &layout(1920, 1080, 60.0, 0, 0, 1.0)),
            "hl.monitor({ output = \"DP-1\", mode = \"1920x1080@60.00Hz\", position = \"0x0\", scale = 1 })"
        );
    }

    #[test]
    fn edits_variables_and_env_in_place() {
        let dir = TempConfig::new("variables");
        let config = dir.write(
            "hyprland.lua",
            r#"local terminal = "kitty" -- my terminal
hl.config({
    general = {
        layout = "dwindle"
    },
})
hl.env("XCURSOR_SIZE", "24") -- cursor
"#,
        );

        let names = get_variables(&config)
            .unwrap()
            .into_iter()
            .map(|var| var.name)
            .collect::<Vec<_>>();
        // `layout` sits inside a table, it is not a variable.
        assert_eq!(names, vec!["terminal"]);

        set_variable(&config, "terminal".into(), "foot".into()).unwrap();
        edit_env_var(&config, 0, "XCURSOR_SIZE".into(), "32".into()).unwrap();
        assert_eq!(
            dir.read("hyprland.lua"),
            r#"local terminal = "foot" -- my terminal
hl.config({
    general = {
        layout = "dwindle"
    },
})
hl.env("XCURSOR_SIZE", "32") -- cursor
"#
        );

        delete_variable(&config, "terminal".into()).unwrap();
        delete_env_var(&config, 0).unwrap();
        assert!(!dir.read("hyprland.lua").contains("terminal"));
        assert!(!dir.read("hyprland.lua").contains("XCURSOR_SIZE"));
    }

    #[test]
    fn new_binds_can_reference_globals_from_other_files() {
        let dir = TempConfig::new("globals");
        let config = dir.write("hyprland.lua", "require(\"config.variables\")\n");
        dir.write(
            "config/variables.lua",
            "TERMINAL = \"ghostty\"\nlocal private = \"x\"\n",
        );

        add_keybind(
            &config,
            vec!["SUPER".into()],
            "Return".into(),
            "exec".into(),
            "$TERMINAL -e $private".into(),
        )
        .unwrap();
        assert!(dir.read("hyprland.lua").contains(
            "hl.bind(\"SUPER + Return\", hl.dsp.exec_cmd(TERMINAL .. \" -e $private\"))"
        ));
    }

    #[test]
    fn resolves_require_like_hyprland() {
        let dir = TempConfig::new("require");
        let config = dir.write(
            "hyprland.lua",
            r#"require("./binds")
require "rules"
require("conf.d/*.lua")
require("~/does-not-exist")
"#,
        );
        dir.write(
            "binds.lua",
            "hl.bind(\"SUPER + Q\", hl.dsp.exec_cmd(\"kitty\"))\n",
        );
        dir.write(
            "rules/init.lua",
            "hl.window_rule({ name = \"from-init\", float = true })\n",
        );
        dir.write("conf.d/a.lua", "hl.env(\"A\", \"1\")\n");
        dir.write("conf.d/b.lua", "hl.env(\"B\", \"2\")\n");

        assert_eq!(get_keybinds(&config).unwrap().len(), 1);
        assert_eq!(get_windowrule_names(&config).unwrap(), vec!["from-init"]);
        let mut env = get_env_vars(&config)
            .unwrap()
            .into_iter()
            .map(|env| env.name)
            .collect::<Vec<_>>();
        env.sort();
        assert_eq!(env, vec!["A", "B"]);
    }

    #[test]
    fn reads_modular_lua_config_from_entrypoint_imports() {
        let dir = TempConfig::new("modular");
        dir.write(
            "hyprland.lua",
            r#"
local hypr_dir = "./"
local module = dofile("lib/config_loader.lua")
local context = { vars = module.apply("variables") }
module.load("display", context)
module.load("keybinds", context)
module.load("rules", context)
"#,
        );
        dir.write(
            "lib/config_loader.lua",
            r#"
local modules_dir = "modules"
local M = {}
function M.apply(name)
    return dofile(modules_dir .. "/" .. name .. ".lua")
end
function M.load(name, context)
    local configure = M.apply(name)
    if configure then configure(context) end
end
return M
"#,
        );
        dir.write(
            "modules/variables.lua",
            r#"
return {
    terminal = "ghostty",
    main_mod = "SUPER",
}
"#,
        );
        dir.write(
            "modules/display.lua",
            r#"
return function()
    hl.env("XCURSOR_SIZE", "24")
end
"#,
        );
        dir.write(
            "modules/keybinds.lua",
            r#"
return function(context)
    local vars = context.vars
    local terminal = vars.terminal
    local main_mod = vars.main_mod
    hl.bind(main_mod .. " + Return", hl.dsp.exec_cmd(terminal))
end
"#,
        );
        dir.write(
            "modules/rules.lua",
            r#"
return function()
    hl.window_rule({ name = "terminal", match = { class = "ghostty" }, float = true })
    hl.layer_rule({ name = "shell", match = { namespace = "shell" }, blur = true })
end
"#,
        );

        let entrypoint = dir.0.join("hyprland.lua");
        let keybinds = get_keybinds(&entrypoint).unwrap();
        let variables = get_variables(&entrypoint).unwrap();
        let env_vars = get_env_vars(&entrypoint).unwrap();
        let windowrules = get_windowrule_names(&entrypoint).unwrap();
        let layerrules = get_layerrule_names(&entrypoint).unwrap();

        assert_eq!(keybinds.len(), 1);
        assert_eq!(keybinds[0].modifiers, vec!["SUPER"]);
        assert_eq!(keybinds[0].key, "Return");
        assert_eq!(keybinds[0].dispatcher, "exec");
        assert_eq!(keybinds[0].params, "$terminal");
        assert!(
            variables
                .iter()
                .any(|var| var.name == "terminal" && var.value == "ghostty")
        );
        assert!(
            env_vars
                .iter()
                .any(|env| env.name == "XCURSOR_SIZE" && env.value == "24")
        );
        assert_eq!(windowrules, vec!["terminal"]);
        assert_eq!(layerrules, vec!["shell"]);

        // Editing the bind in the module keeps `terminal` as a variable reference.
        edit_keybind(
            &entrypoint,
            0,
            vec!["SUPER".into()],
            "Return".into(),
            "exec".into(),
            "$terminal --hold".into(),
        )
        .unwrap();
        assert!(dir.read("modules/keybinds.lua").contains(
            "hl.bind(main_mod .. \" + Return\", hl.dsp.exec_cmd(terminal .. \" --hold\"))"
        ));
    }
}
