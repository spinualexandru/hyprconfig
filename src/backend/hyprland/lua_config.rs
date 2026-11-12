//! Config types (keybinds, variables, env vars, rules) and wrappers that read or edit the
//! user's `hyprland.lua` through `hyprland_lua`, after resolving which file that is.

use crate::backend::hyprland_lua as lua;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keybind {
    pub modifiers: Vec<String>,
    pub key: String,
    pub dispatcher: String,
    pub params: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Variable {
    pub name: String,
    pub value: String,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EnvVar {
    pub name: String,
    pub value: String,
    pub index: usize,
    pub source_file: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowruleProperty {
    pub key: String,
    pub value: String,
    pub property_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Windowrule {
    pub name: String,
    pub enabled: bool,
    pub match_properties: Vec<WindowruleProperty>,
    pub effect_properties: Vec<WindowruleProperty>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerruleProperty {
    pub key: String,
    pub value: String,
    pub property_type: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Layerrule {
    pub name: String,
    pub enabled: bool,
    pub match_properties: Vec<LayerruleProperty>,
    pub effect_properties: Vec<LayerruleProperty>,
}

pub fn get_keybinds() -> Result<Vec<Keybind>, String> {
    lua::get_keybinds(&lua::config_path()?)
}

pub fn add_keybind(
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    lua::add_keybind(&lua::config_path()?, modifiers, key, dispatcher, params)
}

pub fn edit_keybind(
    index: usize,
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    lua::edit_keybind(
        &lua::config_path()?,
        index,
        modifiers,
        key,
        dispatcher,
        params,
    )
}

pub fn delete_keybind(index: usize) -> Result<(), String> {
    lua::delete_keybind(&lua::config_path()?, index)
}

/// Universal binds: the ones that work in every submap.
pub fn get_all_bindu() -> Result<Vec<Keybind>, String> {
    lua::get_all_bindu(&lua::config_path()?)
}

pub fn add_bindu(
    modifiers: Vec<String>,
    key: String,
    dispatcher: String,
    params: String,
) -> Result<(), String> {
    lua::add_bindu(&lua::config_path()?, modifiers, key, dispatcher, params)
}

pub fn delete_bindu(index: usize) -> Result<(), String> {
    lua::delete_bindu(&lua::config_path()?, index)
}

pub fn get_variables() -> Result<Vec<Variable>, String> {
    lua::get_variables(&lua::config_path()?)
}

pub fn set_variable(name: String, value: String) -> Result<(), String> {
    lua::set_variable(&lua::config_path()?, name, value)
}

pub fn add_variable(name: String, value: String) -> Result<(), String> {
    lua::add_variable(&lua::config_path()?, name, value)
}

pub fn delete_variable(name: String) -> Result<(), String> {
    lua::delete_variable(&lua::config_path()?, name)
}

pub fn get_env_vars() -> Result<Vec<EnvVar>, String> {
    lua::get_env_vars(&lua::config_path()?)
}

pub fn add_env_var(name: String, value: String) -> Result<(), String> {
    lua::add_env_var(&lua::config_path()?, name, value)
}

pub fn edit_env_var(index: usize, name: String, value: String) -> Result<(), String> {
    lua::edit_env_var(&lua::config_path()?, index, name, value)
}

pub fn delete_env_var(index: usize) -> Result<(), String> {
    lua::delete_env_var(&lua::config_path()?, index)
}

pub fn save_monitor_settings(
    name: String,
    width: u16,
    height: u16,
    refresh_rate: f32,
    x: i32,
    y: i32,
    scale: f32,
) -> Result<(), String> {
    lua::save_monitor_settings(
        &lua::config_path()?,
        name,
        &lua::MonitorLayout {
            width,
            height,
            refresh_rate,
            x,
            y,
            scale,
        },
    )
}

pub fn get_windowrule_names() -> Result<Vec<String>, String> {
    lua::get_windowrule_names(&lua::config_path()?)
}

pub fn get_windowrule(name: String) -> Result<Windowrule, String> {
    lua::get_windowrule(&lua::config_path()?, name)
}

pub fn delete_windowrule(name: String) -> Result<(), String> {
    lua::delete_windowrule(&lua::config_path()?, name)
}

pub fn get_layerrule_names() -> Result<Vec<String>, String> {
    lua::get_layerrule_names(&lua::config_path()?)
}

pub fn get_layerrule(name: String) -> Result<Layerrule, String> {
    lua::get_layerrule(&lua::config_path()?, name)
}

pub fn delete_layerrule(name: String) -> Result<(), String> {
    lua::delete_layerrule(&lua::config_path()?, name)
}
