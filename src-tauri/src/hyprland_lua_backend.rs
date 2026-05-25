use crate::hyprland_backend::{
    EnvVar, Keybind, LAYERRULE_EFFECT_PROPERTIES, LAYERRULE_MATCH_PROPERTIES, Layerrule,
    LayerruleProperty, Variable, WINDOWRULE_EFFECT_PROPERTIES, WINDOWRULE_MATCH_PROPERTIES,
    Windowrule, WindowruleProperty,
};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(not(test))]
use std::process::Command;

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

pub fn get_preferred_config() -> Result<HyprlandConfig, String> {
    if let Ok(path) = std::env::var("HYPRLAND_CONFIG") {
        if !path.trim().is_empty() {
            let path = PathBuf::from(path);
            return Ok(HyprlandConfig {
                format: format_for_path(&path),
                path,
            });
        }
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

#[derive(Debug, Clone)]
struct LuaStatement {
    path: PathBuf,
    start: usize,
    end: usize,
    args: String,
}

#[derive(Debug, Clone)]
struct LuaSource {
    path: PathBuf,
    contents: String,
}

#[derive(Debug)]
struct LuaProject {
    sources: Vec<LuaSource>,
    variables: HashMap<String, String>,
}

#[derive(Debug)]
struct ParsedBind {
    statement: LuaStatement,
    keybind: Keybind,
    submap_universal: bool,
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
    validate_keybind(&key, &dispatcher)?;
    append_lua_line(
        path,
        &format!(
            "hl.bind({}, {})",
            lua_quote(&key_string(&modifiers, &key)),
            dispatcher_to_lua_expr(&dispatcher, &params)
        ),
    )
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
    let binds = parse_binds_from_project(&project)
        .into_iter()
        .filter(|bind| !bind.submap_universal)
        .collect::<Vec<_>>();
    let statement = binds
        .get(index)
        .ok_or_else(|| format!("Keybind index {} not found", index))?
        .statement
        .clone();
    let replacement = format!(
        "hl.bind({}, {})\n",
        lua_quote(&key_string(&modifiers, &key)),
        dispatcher_to_lua_expr(&dispatcher, &params)
    );
    write_replaced_statement(&statement, &replacement)
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
    validate_keybind(&key, &dispatcher)?;
    append_lua_line(
        path,
        &format!(
            "hl.bind({}, {}, {{ submap_universal = true }})",
            lua_quote(&key_string(&modifiers, &key)),
            dispatcher_to_lua_expr(&dispatcher, &params)
        ),
    )
}

pub fn delete_bindu(path: &Path, index: usize) -> Result<(), String> {
    delete_bind(path, index, true)
}

fn delete_bind(path: &Path, index: usize, submap_universal: bool) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let binds = parse_binds_from_project(&project)
        .into_iter()
        .filter(|bind| bind.submap_universal == submap_universal)
        .collect::<Vec<_>>();
    let statement = binds
        .get(index)
        .ok_or_else(|| format!("Keybind index {} not found", index))?
        .statement
        .clone();
    write_replaced_statement(&statement, "")
}

pub fn get_variables(path: &Path) -> Result<Vec<Variable>, String> {
    let project = read_lua_project(path)?;
    let mut variables = parse_project_variables(&project)
        .into_iter()
        .map(|(name, value, statement)| Variable {
            name,
            value,
            source_file: Some(
                statement
                    .path
                    .strip_prefix(path.parent().unwrap_or_else(|| Path::new("")))
                    .unwrap_or(&statement.path)
                    .to_str()
                    .or_else(|| statement.path.file_name().and_then(|n| n.to_str()))
                    .unwrap_or("hyprland.lua")
                    .to_string(),
            ),
        })
        .collect::<Vec<_>>();
    variables.sort_by(|a, b| a.name.cmp(&b.name));
    variables.dedup_by(|a, b| a.name == b.name);
    Ok(variables)
}

pub fn set_variable(path: &Path, name: String, value: String) -> Result<(), String> {
    validate_variable_name(&name)?;
    let project = read_lua_project(path)?;
    let variables = parse_project_variables(&project);
    if let Some((_, _, statement)) = variables.into_iter().find(|(n, _, _)| n == &name) {
        let prefix = if statement.args == "local_variable" {
            "local "
        } else {
            ""
        };
        let suffix = if statement.args == "table_field" {
            ","
        } else {
            ""
        };
        write_replaced_statement(
            &statement,
            &format!("{}{} = {}{}\n", prefix, name, lua_quote(&value), suffix),
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
    let variables = parse_project_variables(&project);
    let statement = variables
        .into_iter()
        .find(|(n, _, _)| n == &name)
        .map(|(_, _, statement)| statement)
        .ok_or_else(|| format!("Variable '{}' not found", name))?;
    write_replaced_statement(&statement, "")
}

pub fn get_env_vars(path: &Path) -> Result<Vec<EnvVar>, String> {
    let project = read_lua_project(path)?;
    let mut env_vars = Vec::new();

    for (index, statement) in find_project_lua_function_calls(&project, "env")
        .into_iter()
        .enumerate()
    {
        let args = split_top_level_args(&statement.args);
        if args.len() < 2 {
            continue;
        }
        let Some(name) = eval_lua_string_expr(&args[0], &project.variables) else {
            continue;
        };
        let Some(value) = eval_lua_string_expr(&args[1], &project.variables) else {
            continue;
        };
        env_vars.push(EnvVar {
            name,
            value,
            index,
            source_file: Some(
                statement
                    .path
                    .strip_prefix(path.parent().unwrap_or_else(|| Path::new("")))
                    .unwrap_or(&statement.path)
                    .to_str()
                    .or_else(|| statement.path.file_name().and_then(|n| n.to_str()))
                    .unwrap_or("hyprland.lua")
                    .to_string(),
            ),
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
    let statement = find_project_lua_function_calls(&project, "env")
        .get(index)
        .cloned()
        .ok_or_else(|| format!("Environment variable index {} not found", index))?;
    write_replaced_statement(
        &statement,
        &format!("hl.env({}, {})\n", lua_quote(&name), lua_quote(&value)),
    )
}

pub fn delete_env_var(path: &Path, index: usize) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let statement = find_project_lua_function_calls(&project, "env")
        .get(index)
        .cloned()
        .ok_or_else(|| format!("Environment variable index {} not found", index))?;
    write_replaced_statement(&statement, "")
}

pub fn save_monitor_settings(
    path: &Path,
    name: String,
    width: u16,
    height: u16,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let replacement = format!(
        "hl.monitor({{\n    output = {},\n    mode = {},\n    position = {},\n    scale = {},\n}})\n",
        lua_quote(&name),
        lua_quote(&format!("{}x{}@{:.2}Hz", width, height, refresh_rate)),
        lua_quote(&format!("{}x{}", x, y)),
        scale
    );

    for statement in find_project_lua_function_calls(&project, "monitor") {
        let fields = parse_lua_table_fields(statement.args.trim());
        let output = fields
            .iter()
            .find(|(key, _)| key == "output")
            .and_then(|(_, value)| eval_lua_string_expr(value, &project.variables));
        if output.as_deref() == Some(name.as_str()) {
            return write_replaced_statement(&statement, &replacement);
        }
    }

    append_lua_line(path, replacement.trim_end())
}

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
    let fields = parse_lua_table_fields(statement.args.trim());

    let enabled = fields
        .iter()
        .find(|(key, _)| key == "enabled")
        .map(|(_, value)| value.trim() != "false")
        .unwrap_or(true);

    let mut match_properties = Vec::new();
    if let Some((_, match_table)) = fields.iter().find(|(key, _)| key == "match") {
        for (key, value) in parse_lua_table_fields(match_table) {
            if WINDOWRULE_MATCH_PROPERTIES.contains(&key.as_str()) {
                match_properties.push(WindowruleProperty {
                    key,
                    value: lua_value_to_display(&value),
                    property_type: "match".to_string(),
                });
            }
        }
    }

    let mut effect_properties = Vec::new();
    for (key, value) in fields {
        if key == "name" || key == "enabled" || key == "match" {
            continue;
        }
        if WINDOWRULE_EFFECT_PROPERTIES.contains(&key.as_str()) {
            effect_properties.push(WindowruleProperty {
                key,
                value: lua_value_to_display(&value),
                property_type: "effect".to_string(),
            });
        }
    }

    Ok(Windowrule {
        name,
        enabled,
        match_properties,
        effect_properties,
    })
}

pub fn get_layerrule(path: &Path, name: String) -> Result<Layerrule, String> {
    let project = read_lua_project(path)?;
    let statement = find_rule_statement(&project, "layer_rule", &name)
        .ok_or_else(|| format!("Layerrule '{}' not found", name))?;
    let fields = parse_lua_table_fields(statement.args.trim());

    let enabled = fields
        .iter()
        .find(|(key, _)| key == "enabled")
        .map(|(_, value)| value.trim() != "false")
        .unwrap_or(true);

    let mut match_properties = Vec::new();
    if let Some((_, match_table)) = fields.iter().find(|(key, _)| key == "match") {
        for (key, value) in parse_lua_table_fields(match_table) {
            if LAYERRULE_MATCH_PROPERTIES.contains(&key.as_str()) {
                match_properties.push(LayerruleProperty {
                    key,
                    value: lua_value_to_display(&value),
                    property_type: "match".to_string(),
                });
            }
        }
    }

    let mut effect_properties = Vec::new();
    for (key, value) in fields {
        if key == "name" || key == "enabled" || key == "match" {
            continue;
        }
        if LAYERRULE_EFFECT_PROPERTIES.contains(&key.as_str()) {
            effect_properties.push(LayerruleProperty {
                key,
                value: lua_value_to_display(&value),
                property_type: "effect".to_string(),
            });
        }
    }

    Ok(Layerrule {
        name,
        enabled,
        match_properties,
        effect_properties,
    })
}

pub fn delete_windowrule(path: &Path, name: String) -> Result<(), String> {
    delete_rule(path, "window_rule", &name)
}

pub fn delete_layerrule(path: &Path, name: String) -> Result<(), String> {
    delete_rule(path, "layer_rule", &name)
}

fn read_lua_config(path: &Path) -> Result<String, String> {
    if !path.exists() {
        return Err(format!("Hyprland Lua config file not found at {:?}", path));
    }
    fs::read_to_string(path).map_err(|e| format!("Failed to read {:?}: {}", path, e))
}

fn read_lua_project(path: &Path) -> Result<LuaProject, String> {
    let source_paths = discover_lua_sources(path)?;
    let mut sources = Vec::new();

    for source_path in source_paths {
        let contents = read_lua_config(&source_path)?;
        sources.push(LuaSource {
            path: source_path,
            contents,
        });
    }

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

        let contents = read_lua_config(&path)?;
        let vars = string_vars_map_from_contents(&contents, &HashMap::new(), &path);

        for (name, value) in &vars {
            if name.ends_with("dir") || name.ends_with("_dir") || name.ends_with("home") {
                let dir = PathBuf::from(value);
                if dir.is_dir() && !known_dirs.contains(&dir) {
                    known_dirs.push(dir);
                }
            }
        }

        for import in find_lua_import_exprs(&contents) {
            if let Some(import_path) = eval_lua_string_expr(&import, &vars) {
                let mut candidates =
                    resolve_lua_path(&root, path.parent().unwrap_or(&root), &import_path);
                if candidates.is_empty() {
                    candidates = resolve_named_lua_module(&root, &known_dirs, &import_path);
                }
                for candidate in candidates {
                    if !paths.contains(&candidate) && !pending.contains(&candidate) {
                        pending.push(candidate);
                    }
                }
            }
        }

        for module_name in find_loader_module_names(&contents) {
            for candidate in resolve_named_lua_module(&root, &known_dirs, &module_name) {
                if !paths.contains(&candidate) && !pending.contains(&candidate) {
                    pending.push(candidate);
                }
            }
        }

        paths.push(path);
    }

    Ok(paths)
}

fn normalize_lua_path(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn resolve_lua_path(root: &Path, current_dir: &Path, value: &str) -> Vec<PathBuf> {
    let raw = expand_tilde(value);
    let path = PathBuf::from(&raw);
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
            dir.join("modules").join(&relative).with_extension("lua"),
        ] {
            if candidate.exists() {
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
    if let Some(rest) = value.strip_prefix("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}/{}", home, rest);
        }
    }
    value.to_string()
}

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

fn write_replaced_statement(statement: &LuaStatement, replacement: &str) -> Result<(), String> {
    let contents = read_lua_config(&statement.path)?;
    let mut next = contents.to_string();
    next.replace_range(statement.start..statement.end, replacement);
    fs::write(&statement.path, next)
        .map_err(|e| format!("Failed to write {:?}: {}", statement.path, e))?;
    reload_hyprland()
}

fn reload_hyprland() -> Result<(), String> {
    #[cfg(test)]
    {
        return Ok(());
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

fn parse_binds(path: &Path) -> Result<Vec<ParsedBind>, String> {
    let project = read_lua_project(path)?;
    Ok(parse_binds_from_project(&project))
}

fn parse_binds_from_project(project: &LuaProject) -> Vec<ParsedBind> {
    let mut binds = Vec::new();

    for statement in find_project_lua_function_calls(project, "bind") {
        if let Some(bind) = parse_bind_statement(statement, &project.variables) {
            binds.push(bind);
        }
    }

    binds
}

#[cfg(test)]
fn parse_binds_from_contents(contents: &str) -> Vec<ParsedBind> {
    let source = LuaSource {
        path: PathBuf::from("hyprland.lua"),
        contents: contents.to_string(),
    };
    let project = LuaProject {
        variables: string_vars_map(contents),
        sources: vec![source],
    };
    parse_binds_from_project(&project)
}

fn parse_bind_statement(
    statement: LuaStatement,
    variables: &HashMap<String, String>,
) -> Option<ParsedBind> {
    let args = split_top_level_args(&statement.args);
    if args.len() < 2 {
        return None;
    }

    let keys = eval_lua_string_expr(&args[0], variables)?;
    let (dispatcher, params) = parse_dispatcher_expr(&args[1], variables)?;

    let submap_universal = args
        .get(2)
        .map(|opts| opts.contains("submap_universal") && opts.contains("true"))
        .unwrap_or(false);

    let (modifiers, key) = parse_key_string(&keys);
    Some(ParsedBind {
        statement,
        keybind: Keybind {
            modifiers,
            key,
            dispatcher,
            params,
        },
        submap_universal,
    })
}

fn parse_project_variables(project: &LuaProject) -> Vec<(String, String, LuaStatement)> {
    let mut binds = Vec::new();

    for source in &project.sources {
        binds.extend(parse_return_table_variables(source, &project.variables));
        binds.extend(parse_string_variables_from_source(
            source,
            &project.variables,
        ));
    }

    binds
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

fn parse_string_variables_from_source(
    source: &LuaSource,
    variables: &HashMap<String, String>,
) -> Vec<(String, String, LuaStatement)> {
    let mut parsed = Vec::new();
    let mut offset = 0;

    for line in source.contents.lines() {
        let line_start = offset;
        offset += line.len() + 1;
        let trimmed = line.trim_start();
        let local = trimmed.strip_prefix("local ").unwrap_or(trimmed);
        let Some((name_raw, value_raw)) = local.split_once('=') else {
            continue;
        };
        let name = name_raw.trim();
        if !is_lua_identifier(name) {
            continue;
        }
        let value_expr = strip_lua_line_comment(value_raw).trim();
        let Some(value) = eval_lua_string_expr(value_expr, variables) else {
            continue;
        };
        parsed.push((
            name.to_string(),
            value,
            LuaStatement {
                path: source.path.clone(),
                start: line_start,
                end: offset.min(source.contents.len()),
                args: "local_variable".to_string(),
            },
        ));
    }

    parsed
}

fn parse_return_table_variables(
    source: &LuaSource,
    variables: &HashMap<String, String>,
) -> Vec<(String, String, LuaStatement)> {
    let Some(return_pos) = source.contents.find("return") else {
        return Vec::new();
    };
    let after_return = &source.contents[return_pos + "return".len()..];
    if !after_return.trim_start().starts_with('{') {
        return Vec::new();
    }
    let Some(open_relative) = source.contents[return_pos..].find('{') else {
        return Vec::new();
    };
    let open = return_pos + open_relative;
    let Some(close) = find_matching_delimiter(&source.contents, open, b'{', b'}') else {
        return Vec::new();
    };

    parse_lua_table_fields_with_offsets(&source.contents[open..=close], open)
        .into_iter()
        .filter_map(|(key, value, start, end)| {
            if !is_lua_identifier(&key) {
                return None;
            }
            let value = eval_lua_string_expr(&value, variables)?;
            Some((
                key,
                value,
                LuaStatement {
                    path: source.path.clone(),
                    start,
                    end,
                    args: "table_field".to_string(),
                },
            ))
        })
        .collect()
}

#[cfg(test)]
fn string_vars_map(contents: &str) -> HashMap<String, String> {
    let source = LuaSource {
        path: PathBuf::from("hyprland.lua"),
        contents: contents.to_string(),
    };
    string_vars_map_for_sources(&[source])
}

fn string_vars_map_for_sources(sources: &[LuaSource]) -> HashMap<String, String> {
    let mut variables = HashMap::new();

    for _ in 0..4 {
        let mut changed = false;
        for source in sources {
            for (name, value, _) in parse_return_table_variables(source, &variables)
                .into_iter()
                .chain(parse_string_variables_from_source(source, &variables))
            {
                if variables.get(&name) != Some(&value) {
                    variables.insert(name, value);
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

fn string_vars_map_from_contents(
    contents: &str,
    variables: &HashMap<String, String>,
    path: &Path,
) -> HashMap<String, String> {
    let source = LuaSource {
        path: path.to_path_buf(),
        contents: contents.to_string(),
    };
    parse_return_table_variables(&source, variables)
        .into_iter()
        .chain(parse_string_variables_from_source(&source, variables))
        .map(|(name, value, _statement)| (name, value))
        .collect()
}

fn find_project_lua_function_calls(project: &LuaProject, name: &str) -> Vec<LuaStatement> {
    project
        .sources
        .iter()
        .flat_map(|source| find_lua_function_calls_in_source(&source.path, &source.contents, name))
        .collect()
}

fn find_lua_function_calls_in_source(path: &Path, contents: &str, name: &str) -> Vec<LuaStatement> {
    let needle = format!("hl.{}", name);
    find_function_calls_in_source(path, contents, &needle)
}

fn find_function_calls_in_source(path: &Path, contents: &str, needle: &str) -> Vec<LuaStatement> {
    let mut statements = Vec::new();
    let mut search_from = 0;

    while let Some(relative) = contents[search_from..].find(&needle) {
        let call_start = search_from + relative;
        if !is_probable_code_position(contents, call_start) {
            search_from = call_start + needle.len();
            continue;
        }

        let mut open = call_start + needle.len();
        while contents
            .as_bytes()
            .get(open)
            .is_some_and(|b| b.is_ascii_whitespace())
        {
            open += 1;
        }
        if contents.as_bytes().get(open) != Some(&b'(') {
            search_from = call_start + needle.len();
            continue;
        }

        if let Some(close) = find_matching_delimiter(contents, open, b'(', b')') {
            let line_start = contents[..call_start]
                .rfind('\n')
                .map(|idx| idx + 1)
                .unwrap_or(0);
            let mut end = close + 1;
            while contents
                .as_bytes()
                .get(end)
                .is_some_and(|b| *b == b' ' || *b == b'\t')
            {
                end += 1;
            }
            if contents.as_bytes().get(end) == Some(&b'\n') {
                end += 1;
            }
            statements.push(LuaStatement {
                path: path.to_path_buf(),
                start: line_start,
                end,
                args: contents[open + 1..close].to_string(),
            });
            search_from = end;
        } else {
            search_from = call_start + needle.len();
        }
    }

    statements
}

fn find_lua_import_exprs(contents: &str) -> Vec<String> {
    ["dofile", "loadfile", "require"]
        .iter()
        .flat_map(|name| {
            find_function_calls_in_source(Path::new("hyprland.lua"), contents, name)
                .into_iter()
                .filter_map(|statement| split_top_level_args(&statement.args).first().cloned())
                .collect::<Vec<_>>()
        })
        .collect()
}

fn find_loader_module_names(contents: &str) -> Vec<String> {
    let mut names = Vec::new();
    for method in [".load", ".apply"] {
        let mut search_from = 0;
        while let Some(relative) = contents[search_from..].find(method) {
            let call_start = search_from + relative;
            if !is_probable_code_position(contents, call_start) {
                search_from = call_start + method.len();
                continue;
            }

            let mut open = call_start + method.len();
            while contents
                .as_bytes()
                .get(open)
                .is_some_and(|b| b.is_ascii_whitespace())
            {
                open += 1;
            }

            if contents.as_bytes().get(open) != Some(&b'(') {
                search_from = call_start + method.len();
                continue;
            }

            if let Some(close) = find_matching_delimiter(contents, open, b'(', b')') {
                if let Some(first_arg) = split_top_level_args(&contents[open + 1..close]).first() {
                    if let Some(name) = parse_lua_string_literal(first_arg) {
                        names.push(name);
                    }
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

fn is_probable_code_position(contents: &str, idx: usize) -> bool {
    let line_start = contents[..idx].rfind('\n').map(|pos| pos + 1).unwrap_or(0);
    let before = &contents[line_start..idx];
    !before.contains("--")
}

fn find_matching_delimiter(contents: &str, open: usize, left: u8, right: u8) -> Option<usize> {
    let bytes = contents.as_bytes();
    let mut depth = 0usize;
    let mut i = open;
    let mut quote: Option<u8> = None;
    let mut in_comment = false;

    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'\n' {
                in_comment = false;
            }
            i += 1;
            continue;
        }
        if let Some(q) = quote {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if b == b'-' && bytes.get(i + 1) == Some(&b'-') {
            in_comment = true;
            i += 2;
            continue;
        }
        if b == b'\'' || b == b'"' {
            quote = Some(b);
            i += 1;
            continue;
        }
        if b == left {
            depth += 1;
        } else if b == right {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(i);
            }
        }
        i += 1;
    }

    None
}

fn split_top_level_args(args: &str) -> Vec<String> {
    split_top_level(args, b',')
}

fn split_top_level(source: &str, separator: u8) -> Vec<String> {
    split_top_level_spans(source, separator)
        .into_iter()
        .map(|(part, _, _)| part)
        .collect()
}

fn split_top_level_spans(source: &str, separator: u8) -> Vec<(String, usize, usize)> {
    let bytes = source.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut paren = 0usize;
    let mut brace = 0usize;
    let mut bracket = 0usize;
    let mut quote: Option<u8> = None;
    let mut i = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' => paren += 1,
            b')' => paren = paren.saturating_sub(1),
            b'{' => brace += 1,
            b'}' => brace = brace.saturating_sub(1),
            b'[' => bracket += 1,
            b']' => bracket = bracket.saturating_sub(1),
            _ if b == separator && paren == 0 && brace == 0 && bracket == 0 => {
                let raw = &source[start..=i];
                let trim_start = raw.len().saturating_sub(raw.trim_start().len());
                let trimmed_without_separator =
                    raw.trim().trim_end_matches(separator as char).trim_end();
                if !trimmed_without_separator.is_empty() {
                    parts.push((
                        trimmed_without_separator.to_string(),
                        start + trim_start,
                        i + 1,
                    ));
                }
                start = i + 1;
            }
            _ => {}
        }
        i += 1;
    }

    if start <= source.len() {
        let raw = &source[start..];
        let trim_start = raw.len().saturating_sub(raw.trim_start().len());
        let tail = raw.trim();
        if !tail.is_empty() {
            parts.push((tail.to_string(), start + trim_start, source.len()));
        }
    }

    parts
}

fn eval_lua_string_expr(expr: &str, variables: &HashMap<String, String>) -> Option<String> {
    let expr = strip_wrapping_parens(strip_lua_line_comment(expr).trim());
    if expr.is_empty() {
        return Some(String::new());
    }

    if let Some((left, right)) = split_lua_or(expr) {
        if let Some(value) = eval_lua_string_expr(left, variables) {
            if !value.is_empty() {
                return Some(value);
            }
        }
        return eval_lua_string_expr(right, variables);
    }

    if let Some(env_name) = expr
        .strip_prefix("os.getenv(")
        .and_then(|rest| rest.strip_suffix(')'))
        .and_then(parse_lua_string_literal)
    {
        return std::env::var(env_name).ok();
    }

    let parts = split_lua_concat(expr);
    let mut output = String::new();
    for part in parts {
        let part = strip_wrapping_parens(part.trim());
        if let Some(value) = parse_lua_string_literal(part) {
            output.push_str(&value);
        } else if let Some(value) = variables.get(part) {
            output.push_str(value);
        } else if let Some(key) = part
            .strip_prefix("vars.")
            .or_else(|| part.strip_prefix("context.vars."))
        {
            output.push_str(variables.get(key)?);
        } else if let Some((_prefix, key)) = part.rsplit_once('.') {
            output.push_str(variables.get(key)?);
        } else if part.chars().all(|c| c.is_ascii_digit()) {
            output.push_str(part);
        } else {
            return None;
        }
    }
    Some(output)
}

fn strip_wrapping_parens(value: &str) -> &str {
    let mut value = value.trim();
    loop {
        if value.starts_with('(')
            && value.ends_with(')')
            && find_matching_delimiter(value, 0, b'(', b')') == Some(value.len() - 1)
        {
            value = value[1..value.len() - 1].trim();
        } else {
            return value;
        }
    }
}

fn split_lua_or(expr: &str) -> Option<(&str, &str)> {
    let bytes = expr.as_bytes();
    let mut paren = 0usize;
    let mut brace = 0usize;
    let mut quote: Option<u8> = None;
    let mut i = 0usize;

    while i + 4 <= bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'\'' | b'"' => quote = Some(b),
            b'(' => paren += 1,
            b')' => paren = paren.saturating_sub(1),
            b'{' => brace += 1,
            b'}' => brace = brace.saturating_sub(1),
            b' ' if paren == 0 && brace == 0 && bytes.get(i..i + 4) == Some(b" or ") && i > 0 => {
                return Some((&expr[..i], &expr[i + 4..]));
            }
            _ => {}
        }
        i += 1;
    }

    None
}

fn split_lua_concat(expr: &str) -> Vec<String> {
    let bytes = expr.as_bytes();
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut quote: Option<u8> = None;
    let mut i = 0usize;
    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }
        if b == b'\'' || b == b'"' {
            quote = Some(b);
            i += 1;
            continue;
        }
        if b == b'.' && bytes.get(i + 1) == Some(&b'.') {
            parts.push(expr[start..i].trim().to_string());
            start = i + 2;
            i += 2;
            continue;
        }
        i += 1;
    }
    parts.push(expr[start..].trim().to_string());
    parts
}

fn parse_lua_string_literal(value: &str) -> Option<String> {
    let value = value.trim();
    let quote = value.as_bytes().first().copied()?;
    if quote != b'\'' && quote != b'"' {
        return None;
    }
    if value.as_bytes().last().copied() != Some(quote) {
        return None;
    }

    let inner = &value[1..value.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('\\') => out.push('\\'),
                Some('"') => out.push('"'),
                Some('\'') => out.push('\''),
                Some(other) => out.push(other),
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    Some(out)
}

fn strip_lua_line_comment(value: &str) -> &str {
    let bytes = value.as_bytes();
    let mut quote: Option<u8> = None;
    let mut i = 0usize;

    while i < bytes.len() {
        let b = bytes[i];
        if let Some(q) = quote {
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == q {
                quote = None;
            }
            i += 1;
            continue;
        }

        match b {
            b'\'' | b'"' => {
                quote = Some(b);
                i += 1;
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => return &value[..i],
            _ => i += 1,
        }
    }

    value
}

fn parse_dispatcher_expr(
    expr: &str,
    variables: &HashMap<String, String>,
) -> Option<(String, String)> {
    let expr = expr.trim();
    let dsp = expr.strip_prefix("hl.dsp.")?;

    if let Some(args) = call_args(dsp, "exec_cmd") {
        return Some((
            "exec".to_string(),
            split_top_level_args(args)
                .first()
                .and_then(|arg| eval_lua_string_expr(arg, variables))
                .unwrap_or_default(),
        ));
    }
    if let Some(args) = call_args(dsp, "exec_raw") {
        return Some((
            "exec".to_string(),
            split_top_level_args(args)
                .first()
                .and_then(|arg| eval_lua_string_expr(arg, variables))
                .unwrap_or_default(),
        ));
    }
    if call_args(dsp, "window.close").is_some() {
        return Some(("killactive".to_string(), String::new()));
    }
    if call_args(dsp, "window.float").is_some() {
        return Some(("togglefloating".to_string(), String::new()));
    }
    if call_args(dsp, "window.fullscreen").is_some() {
        return Some(("fullscreen".to_string(), String::new()));
    }
    if let Some(args) = call_args(dsp, "window.move") {
        let fields = parse_lua_table_fields(args);
        if let Some(value) = field_string(&fields, "workspace", variables) {
            let follow = field_raw(&fields, "follow").unwrap_or("true");
            let dispatcher = if follow.trim() == "false" {
                "movetoworkspacesilent"
            } else {
                "movetoworkspace"
            };
            return Some((dispatcher.to_string(), value));
        }
        if let Some(value) = field_string(&fields, "direction", variables) {
            return Some(("movewindow".to_string(), value));
        }
    }
    if let Some(args) = call_args(dsp, "focus") {
        let fields = parse_lua_table_fields(args);
        if let Some(value) = field_string(&fields, "workspace", variables) {
            if field_raw(&fields, "on_current_monitor") == Some("true") {
                return Some(("focusworkspaceoncurrentmonitor".to_string(), value));
            }
            return Some(("workspace".to_string(), value));
        }
        if let Some(value) = field_string(&fields, "direction", variables) {
            return Some(("movefocus".to_string(), value));
        }
        if let Some(value) = field_string(&fields, "monitor", variables) {
            return Some(("focusmonitor".to_string(), value));
        }
        if field_raw(&fields, "urgent_or_last") == Some("true") {
            return Some(("focusurgentorlast".to_string(), String::new()));
        }
    }
    if let Some(args) = call_args(dsp, "layout") {
        return Some((
            "layoutmsg".to_string(),
            split_top_level_args(args)
                .first()
                .and_then(|arg| eval_lua_string_expr(arg, variables))
                .unwrap_or_default(),
        ));
    }
    if call_args(dsp, "exit").is_some() {
        return Some(("exit".to_string(), String::new()));
    }
    if call_args(dsp, "force_renderer_reload").is_some() {
        return Some(("forcerendererreload".to_string(), String::new()));
    }
    if let Some(args) = call_args(dsp, "workspace.toggle_special") {
        return Some((
            "togglespecialworkspace".to_string(),
            split_top_level_args(args)
                .first()
                .and_then(|arg| eval_lua_string_expr(arg, variables))
                .unwrap_or_default(),
        ));
    }
    if call_args(dsp, "window.pin").is_some() {
        return Some(("pin".to_string(), String::new()));
    }
    if call_args(dsp, "window.center").is_some() {
        return Some(("centerwindow".to_string(), String::new()));
    }
    if call_args(dsp, "window.cycle_next").is_some() {
        return Some(("cyclenext".to_string(), String::new()));
    }
    if call_args(dsp, "window.bring_to_top").is_some() {
        return Some(("bringactivetotop".to_string(), String::new()));
    }
    if call_args(dsp, "window.pseudo").is_some() {
        return Some(("pseudo".to_string(), String::new()));
    }
    if call_args(dsp, "group.toggle").is_some() {
        return Some(("togglegroup".to_string(), String::new()));
    }
    if call_args(dsp, "global").is_some() {
        return Some(("global".to_string(), String::new()));
    }
    if let Some(args) = call_args(dsp, "submap") {
        return Some((
            "submap".to_string(),
            split_top_level_args(args)
                .first()
                .and_then(|arg| eval_lua_string_expr(arg, variables))
                .unwrap_or_default(),
        ));
    }

    Some((
        dsp.split('(').next().unwrap_or(dsp).replace('.', "."),
        String::new(),
    ))
}

fn call_args<'a>(expr: &'a str, name: &str) -> Option<&'a str> {
    let rest = expr.strip_prefix(name)?.trim_start();
    if !rest.starts_with('(') {
        return None;
    }
    let close = find_matching_delimiter(rest, 0, b'(', b')')?;
    Some(&rest[1..close])
}

fn dispatcher_to_lua_expr(dispatcher: &str, params: &str) -> String {
    let params_expr = lua_string_expr_with_vars(params);
    let params_lit = lua_quote(params);
    match dispatcher.trim() {
        "exec" => format!("hl.dsp.exec_cmd({})", params_expr),
        "killactive" => "hl.dsp.window.close()".to_string(),
        "togglefloating" => "hl.dsp.window.float({ action = \"toggle\" })".to_string(),
        "fullscreen" => "hl.dsp.window.fullscreen()".to_string(),
        "workspace" => format!("hl.dsp.focus({{ workspace = {} }})", lua_value_expr(params)),
        "movetoworkspace" => {
            format!(
                "hl.dsp.window.move({{ workspace = {}, follow = true }})",
                lua_value_expr(params)
            )
        }
        "movetoworkspacesilent" => {
            format!(
                "hl.dsp.window.move({{ workspace = {}, follow = false }})",
                lua_value_expr(params)
            )
        }
        "togglesplit" | "swapsplit" | "splitratio" | "layoutmsg" => {
            let msg = if dispatcher == "layoutmsg" {
                params.to_string()
            } else if params.trim().is_empty() {
                dispatcher.to_string()
            } else {
                format!("{} {}", dispatcher, params)
            };
            format!("hl.dsp.layout({})", lua_quote(&msg))
        }
        "movefocus" => format!(
            "hl.dsp.focus({{ direction = {} }})",
            lua_quote(&direction(params))
        ),
        "movewindow" => {
            format!(
                "hl.dsp.window.move({{ direction = {} }})",
                lua_quote(&direction(params))
            )
        }
        "resizeactive" => {
            let parts = params.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                format!(
                    "hl.dsp.window.resize({{ x = {}, y = {}, relative = true }})",
                    parts[0], parts[1]
                )
            } else {
                hyprctl_dispatcher_expr(dispatcher, params)
            }
        }
        "centerwindow" => "hl.dsp.window.center()".to_string(),
        "cyclenext" => "hl.dsp.window.cycle_next()".to_string(),
        "swapnext" => "hl.dsp.window.swap({ next = true })".to_string(),
        "focusmonitor" => format!("hl.dsp.focus({{ monitor = {} }})", params_lit),
        "movecursortocorner" => format!("hl.dsp.cursor.move_to_corner({{ corner = {} }})", params),
        "movecursor" => {
            let parts = params.split_whitespace().collect::<Vec<_>>();
            if parts.len() >= 2 {
                format!(
                    "hl.dsp.cursor.move({{ x = {}, y = {} }})",
                    parts[0], parts[1]
                )
            } else {
                hyprctl_dispatcher_expr(dispatcher, params)
            }
        }
        "exit" => "hl.dsp.exit()".to_string(),
        "forcerendererreload" => "hl.dsp.force_renderer_reload()".to_string(),
        "movecurrentworkspacetomonitor" => {
            format!("hl.dsp.workspace.move({{ monitor = {} }})", params_lit)
        }
        "focusworkspaceoncurrentmonitor" => {
            format!(
                "hl.dsp.focus({{ workspace = {}, on_current_monitor = true }})",
                lua_value_expr(params)
            )
        }
        "focusurgentorlast" => "hl.dsp.focus({ urgent_or_last = true })".to_string(),
        "togglespecialworkspace" => format!("hl.dsp.workspace.toggle_special({})", params_lit),
        "pin" => "hl.dsp.window.pin()".to_string(),
        "mouse" if params.contains("resize") => "hl.dsp.window.resize()".to_string(),
        "mouse" => "hl.dsp.window.drag()".to_string(),
        "bringactivetotop" => "hl.dsp.window.bring_to_top()".to_string(),
        "alterzorder" => format!("hl.dsp.window.alter_zorder({{ mode = {} }})", params_lit),
        "togglegroup" => "hl.dsp.group.toggle()".to_string(),
        "changegroupactive" if params.trim().eq_ignore_ascii_case("b") => {
            "hl.dsp.group.prev()".to_string()
        }
        "changegroupactive" => "hl.dsp.group.next()".to_string(),
        "lockgroups" => "hl.dsp.group.lock()".to_string(),
        "lockactivegroup" => "hl.dsp.group.lock_active()".to_string(),
        "moveintogroup" => {
            format!(
                "hl.dsp.window.move({{ into_group = {} }})",
                lua_quote(&direction(params))
            )
        }
        "moveoutofgroup" => "hl.dsp.window.move({ out_of_group = true })".to_string(),
        "movegroupwindow" => "hl.dsp.group.move_window()".to_string(),
        "global" => format!("hl.dsp.global({})", params_lit),
        "submap" => format!("hl.dsp.submap({})", params_lit),
        "dpms" => hyprctl_dispatcher_expr(dispatcher, params),
        "pseudo" => "hl.dsp.window.pseudo()".to_string(),
        _ => hyprctl_dispatcher_expr(dispatcher, params),
    }
}

fn hyprctl_dispatcher_expr(dispatcher: &str, params: &str) -> String {
    let command = if params.trim().is_empty() {
        format!("hyprctl dispatch {}", dispatcher.trim())
    } else {
        format!("hyprctl dispatch {} {}", dispatcher.trim(), params.trim())
    };
    format!("hl.dsp.exec_cmd({})", lua_quote(&command))
}

fn direction(value: &str) -> String {
    match value.trim() {
        "l" => "left",
        "r" => "right",
        "u" => "up",
        "d" => "down",
        other => other,
    }
    .to_string()
}

fn lua_value_expr(value: &str) -> String {
    if value.trim().parse::<i64>().is_ok() {
        value.trim().to_string()
    } else {
        lua_quote(value)
    }
}

fn lua_string_expr_with_vars(value: &str) -> String {
    let mut parts = Vec::new();
    let mut literal = String::new();
    let chars = value.chars().collect::<Vec<_>>();
    let mut i = 0usize;
    while i < chars.len() {
        if chars[i] == '$' {
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_alphanumeric() || chars[j] == '_') {
                j += 1;
            }
            if j > i + 1 {
                if !literal.is_empty() {
                    parts.push(lua_quote(&literal));
                    literal.clear();
                }
                parts.push(chars[i + 1..j].iter().collect::<String>());
                i = j;
                continue;
            }
        }
        literal.push(chars[i]);
        i += 1;
    }
    if !literal.is_empty() || parts.is_empty() {
        parts.push(lua_quote(&literal));
    }
    parts.join(" .. ")
}

fn parse_lua_table_fields(source: &str) -> Vec<(String, String)> {
    let mut table = source.trim();
    if table.starts_with('{') && table.ends_with('}') {
        table = &table[1..table.len() - 1];
    }

    split_top_level(table, b',')
        .into_iter()
        .filter_map(|part| {
            let (key, value) = part.split_once('=')?;
            let key = key.trim().trim_matches(['[', ']', '"', '\'']).to_string();
            Some((key, value.trim().to_string()))
        })
        .collect()
}

fn parse_lua_table_fields_with_offsets(
    source: &str,
    source_offset: usize,
) -> Vec<(String, String, usize, usize)> {
    let mut table = source.trim();
    let leading_ws = source.len().saturating_sub(source.trim_start().len());
    let mut table_offset = source_offset + leading_ws;

    if table.starts_with('{') && table.ends_with('}') {
        table = &table[1..table.len() - 1];
        table_offset += 1;
    }

    split_top_level_spans(table, b',')
        .into_iter()
        .filter_map(|(part, start, end)| {
            let (key, value) = part.split_once('=')?;
            let key = key.trim().trim_matches(['[', ']', '"', '\'']).to_string();
            Some((
                key,
                value.trim().to_string(),
                table_offset + start,
                table_offset + end,
            ))
        })
        .collect()
}

fn field_string(
    fields: &[(String, String)],
    name: &str,
    variables: &HashMap<String, String>,
) -> Option<String> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .and_then(|(_, value)| eval_lua_string_expr(value, variables))
}

fn field_raw<'a>(fields: &'a [(String, String)], name: &str) -> Option<&'a str> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.trim())
}

fn get_rule_names(path: &Path, function: &str) -> Result<Vec<String>, String> {
    let project = read_lua_project(path)?;
    let mut names = Vec::new();

    for statement in find_project_lua_function_calls(&project, function) {
        let fields = parse_lua_table_fields(statement.args.trim());
        if let Some(name) = field_string(&fields, "name", &project.variables) {
            names.push(name);
        }
    }

    Ok(names)
}

fn find_rule_statement(project: &LuaProject, function: &str, name: &str) -> Option<LuaStatement> {
    find_project_lua_function_calls(project, function)
        .into_iter()
        .find(|statement| {
            let fields = parse_lua_table_fields(statement.args.trim());
            field_string(&fields, "name", &project.variables).as_deref() == Some(name)
        })
}

fn delete_rule(path: &Path, function: &str, name: &str) -> Result<(), String> {
    let project = read_lua_project(path)?;
    let statement = find_rule_statement(&project, function, name)
        .ok_or_else(|| format!("Rule '{}' not found", name))?;
    write_replaced_statement(&statement, "")
}

fn lua_value_to_display(value: &str) -> String {
    parse_lua_string_literal(value.trim()).unwrap_or_else(|| value.trim().to_string())
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

fn validate_variable_name(name: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err("Variable name cannot be empty".to_string());
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(
            "Variable name must contain only letters, numbers, and underscores".to_string(),
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

fn is_lua_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn lua_quote(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(binds[0].keybind.params, "kitty");

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
        assert_eq!(binds[1].keybind.params, "wofi --show drun");
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
    fn parses_lua_rules_with_nested_match_tables() {
        let contents = r#"
hl.window_rule({
    name = "suppress-maximize-events",
    match = { class = ".*", float = false },
    suppress_event = "maximize",
})
"#;

        let source = LuaSource {
            path: PathBuf::from("hyprland.lua"),
            contents: contents.to_string(),
        };
        let project = LuaProject {
            variables: string_vars_map(contents),
            sources: vec![source],
        };
        let statement =
            find_rule_statement(&project, "window_rule", "suppress-maximize-events").unwrap();
        let fields = parse_lua_table_fields(statement.args.trim());
        let match_table = fields.iter().find(|(key, _)| key == "match").unwrap();
        let matches = parse_lua_table_fields(&match_table.1);

        assert!(
            matches
                .iter()
                .any(|(key, value)| key == "class" && value == "\".*\"")
        );
        assert!(
            matches
                .iter()
                .any(|(key, value)| key == "float" && value == "false")
        );
        assert!(fields.iter().any(|(key, value)| {
            key == "suppress_event" && lua_value_to_display(value) == "maximize"
        }));
    }

    #[test]
    fn writes_exec_dispatchers_with_lua_variable_expansion() {
        assert_eq!(
            dispatcher_to_lua_expr("exec", "$terminal --hold"),
            "hl.dsp.exec_cmd(terminal .. \" --hold\")"
        );
    }

    #[test]
    fn reads_modular_lua_config_from_entrypoint_imports() {
        let root = std::env::temp_dir().join(format!("hyprconfig-lua-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("lib")).unwrap();
        fs::create_dir_all(root.join("modules")).unwrap();

        fs::write(
            root.join("hyprland.lua"),
            r#"
local hypr_dir = "./"
local module = dofile("lib/config_loader.lua")
local context = { vars = module.apply("variables") }
module.load("display", context)
module.load("keybinds", context)
module.load("rules", context)
"#,
        )
        .unwrap();
        fs::write(
            root.join("lib/config_loader.lua"),
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
        )
        .unwrap();
        fs::write(
            root.join("modules/variables.lua"),
            r#"
return {
    terminal = "ghostty",
    main_mod = "SUPER",
}
"#,
        )
        .unwrap();
        fs::write(
            root.join("modules/display.lua"),
            r#"
return function()
    hl.env("XCURSOR_SIZE", "24")
end
"#,
        )
        .unwrap();
        fs::write(
            root.join("modules/keybinds.lua"),
            r#"
return function(context)
    local vars = context.vars
    local terminal = vars.terminal
    local main_mod = vars.main_mod
    hl.bind(main_mod .. " + Return", hl.dsp.exec_cmd(terminal))
end
"#,
        )
        .unwrap();
        fs::write(
            root.join("modules/rules.lua"),
            r#"
return function()
    hl.window_rule({ name = "terminal", match = { class = "ghostty" }, float = true })
    hl.layer_rule({ name = "shell", match = { namespace = "shell" }, blur = true })
end
"#,
        )
        .unwrap();

        let entrypoint = root.join("hyprland.lua");
        let keybinds = get_keybinds(&entrypoint).unwrap();
        let variables = get_variables(&entrypoint).unwrap();
        let env_vars = get_env_vars(&entrypoint).unwrap();
        let windowrules = get_windowrule_names(&entrypoint).unwrap();
        let layerrules = get_layerrule_names(&entrypoint).unwrap();

        assert_eq!(keybinds.len(), 1);
        assert_eq!(keybinds[0].modifiers, vec!["SUPER"]);
        assert_eq!(keybinds[0].key, "Return");
        assert_eq!(keybinds[0].dispatcher, "exec");
        assert_eq!(keybinds[0].params, "ghostty");
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

        let _ = fs::remove_dir_all(&root);
    }
}
