//! `hl.monitor` rules: saving a layout and building the live `hyprctl eval` code.

use super::expr::{eval_lua_string_expr, lua_quote};
use super::project::{find_calls, read_lua_project};
use super::table::{lua_table_fields, table_entries};
use super::write::{Edit, append_lua_line, apply_edits, table_insert_edit};
use std::path::Path;

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
    format!(
        "hl.monitor({{ output = {}, {} }})",
        lua_quote(name),
        lua_table_fields(&monitor_fields(layout))
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
