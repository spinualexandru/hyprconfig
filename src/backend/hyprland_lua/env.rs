//! `hl.env` environment variables.

use super::discovery::display_source_file;
use super::expr::{eval_lua_string_expr, lua_quote};
use super::project::{project_calls, read_lua_project};
use super::write::{Edit, append_lua_line, apply_edits, delete_statement};
use crate::backend::hyprland::EnvVar;
use std::path::Path;

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
