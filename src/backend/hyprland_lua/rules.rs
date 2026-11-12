//! `hl.window_rule` and `hl.layer_rule`: listing, reading and deleting rules.

use super::expr::{
    collapse_whitespace, eval_lua_string_expr, lua_value_to_display, parse_lua_string_literal,
};
use super::project::{LuaProject, LuaStatement, project_calls, read_lua_project};
use super::table::{parse_lua_table_fields, table_value};
use super::write::delete_statement;
use crate::backend::hyprland::{Layerrule, LayerruleProperty, Windowrule, WindowruleProperty};
use std::collections::HashMap;
use std::path::Path;

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

pub(super) struct RuleProperties {
    pub(super) enabled: bool,
    pub(super) matches: Vec<(String, String)>,
    pub(super) effects: Vec<(String, String)>,
}

/// Every non-meta key is reported. Hyprland also accepts effects registered by plugins,
/// so filtering against a fixed list would hide valid rules.
pub(super) fn rule_properties(
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
pub(super) fn rule_labels(project: &LuaProject, function: &str) -> Vec<(String, LuaStatement)> {
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

pub(super) fn find_rule_statement(
    project: &LuaProject,
    function: &str,
    name: &str,
) -> Option<LuaStatement> {
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
