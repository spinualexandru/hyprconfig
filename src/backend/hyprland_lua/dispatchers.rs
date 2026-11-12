//! Translation between classic dispatcher names (what the UI edits) and `hl.dsp.*`
//! expressions (what hyprland.lua contains).

use super::expr::*;
use super::lexer::*;
use super::table::*;
use crate::dispatchers::{RAW_LUA, is_name_char, resolve_command, variable_name};
use std::collections::{HashMap, HashSet};

/// Dispatchers that take nothing, and the `hl.dsp` function each one calls.
const NO_ARGS: &[(&str, &str)] = &[
    ("centerwindow", "window.center"),
    ("toggleswallow", "window.toggle_swallow"),
    ("bringactivetotop", "window.bring_to_top"),
    ("togglegroup", "group.toggle"),
    ("exit", "exit"),
    ("forcerendererreload", "force_renderer_reload"),
    ("releaseinputcapture", "release_input_capture"),
];

/// Dispatchers whose params pass through as one string argument: the dispatcher, its
/// `hl.dsp` function, and what the params are (for the error when they're missing).
const ONE_STRING: &[(&str, &str, &str)] = &[
    ("submap", "submap", "a submap name"),
    ("layoutmsg", "layout", "a layout message"),
    ("event", "event", "an event payload"),
    ("global", "global", "a shortcut name"),
];

/// How the dispatchers with an `action` field spell toggle, on and off, in that order.
const LOCK_WORDS: [&str; 3] = ["toggle", "lock", "unlock"];
const SWITCH_WORDS: [&str; 3] = ["toggle", "on", "off"];
/// The floating dispatchers are one `window.float` call with each of the three actions:
/// each dispatcher and its action, in the toggle, on, off order `DspArgs::action` reads.
const FLOAT_ACTIONS: [(&str, &str); 3] = [
    ("togglefloating", "toggle"),
    ("setfloating", "enable"),
    ("settiled", "disable"),
];

/// Translates a classic dispatcher (as shown and edited in the UI) into
/// its `hl.dsp.*` expression. The argument semantics follow Hyprland's own legacy
/// translator (src/config/legacy/DispatcherTranslator.cpp, removed after 0.56).
pub(super) fn dispatcher_to_lua_expr(
    dispatcher: &str,
    params: &str,
    scope: &HashSet<String>,
) -> Result<String, String> {
    let dispatcher = dispatcher.trim();
    let params = params.trim();
    let words = params.split_whitespace().collect::<Vec<_>>();

    if let Some((_, function)) = NO_ARGS.iter().find(|(d, _)| *d == dispatcher) {
        return Ok(format!("hl.dsp.{function}()"));
    }
    if let Some((_, function, what)) = ONE_STRING.iter().find(|(d, ..)| *d == dispatcher) {
        let arg = lua_quote(required(params, dispatcher, what)?);
        return Ok(format!("hl.dsp.{function}({arg})"));
    }
    if let Some((_, action)) = FLOAT_ACTIONS.iter().find(|(d, _)| *d == dispatcher) {
        let mut fields = vec![("action", lua_quote(action))];
        fields.extend(window_field(params));
        return Ok(dsp_call("window.float", &fields));
    }
    // A call with one required field, quoted.
    let one = |function: &str, key: &str, what: &str| {
        required(params, dispatcher, what).map(|v| dsp_call(function, &[(key, lua_quote(v))]))
    };

    let expr = match dispatcher {
        RAW_LUA => {
            if params.is_empty() {
                return Err(
                    "The lua dispatcher needs an expression, e.g. hl.dsp.window.close()"
                        .to_string(),
                );
            }
            params.to_string()
        }
        "exec" | "execr" => format!(
            "hl.dsp.{}({})",
            if dispatcher == "exec" {
                "exec_cmd"
            } else {
                "exec_raw"
            },
            lua_string_expr_with_vars(required(params, dispatcher, "a command")?, scope)
        ),
        "killactive" => "hl.dsp.window.close()".to_string(),
        "forcekillactive" => "hl.dsp.window.kill()".to_string(),
        "closewindow" => one("window.close", "window", "a window")?,
        "killwindow" => one("window.kill", "window", "a window")?,
        "signal" => dsp_call("window.signal", &[("signal", number(params, dispatcher)?)]),
        "signalwindow" => {
            let (window, signal) = params
                .split_once(',')
                .ok_or_else(|| "signalwindow expects: window,signal".to_string())?;
            dsp_call(
                "window.signal",
                &[
                    ("signal", number(signal, dispatcher)?),
                    ("window", lua_quote(window.trim())),
                ],
            )
        }
        "pseudo" => dsp_call("window.pseudo", &window_field(params)),
        "pin" => dsp_call("window.pin", &window_field(params)),
        "workspace" => dsp_call(
            "focus",
            &[(
                "workspace",
                lua_value_expr(required(params, dispatcher, "a workspace")?),
            )],
        ),
        "focusworkspaceoncurrentmonitor" => dsp_call(
            "focus",
            &[
                (
                    "workspace",
                    lua_value_expr(required(params, dispatcher, "a workspace")?),
                ),
                ("on_current_monitor", "true".to_string()),
            ],
        ),
        "renameworkspace" => {
            let (workspace, name) = params
                .split_once(char::is_whitespace)
                .map(|(workspace, name)| (workspace, name.trim()))
                .unwrap_or((params, ""));
            let mut fields = vec![(
                "workspace",
                lua_value_expr(required(workspace, dispatcher, "a workspace")?),
            )];
            if !name.is_empty() {
                fields.push(("name", lua_quote(name)));
            }
            dsp_call("workspace.rename", &fields)
        }
        "fullscreen" => {
            let mut fields = Vec::new();
            match words.first().copied() {
                None => {}
                Some("0") if words.len() == 1 => {}
                Some("1") => fields.push(("mode", lua_quote("maximized"))),
                Some("0") => fields.push(("mode", lua_quote("fullscreen"))),
                Some(other) => {
                    return Err(format!("fullscreen mode must be 0 or 1, got '{}'", other));
                }
            }
            if let Some(action) = words.get(1) {
                if !matches!(*action, "toggle" | "set" | "unset") {
                    return Err(format!(
                        "fullscreen action must be toggle, set or unset, got '{}'",
                        action
                    ));
                }
                fields.push(("action", lua_quote(action)));
            }
            dsp_call("window.fullscreen", &fields)
        }
        "fullscreenstate" => {
            if words.len() < 2 {
                return Err(
                    "fullscreenstate expects: internal client [toggle|set|unset]".to_string(),
                );
            }
            let mut fields = vec![
                ("internal", number(words[0], dispatcher)?),
                ("client", number(words[1], dispatcher)?),
            ];
            if let Some(action) = words.get(2) {
                fields.push(("action", lua_quote(action)));
            }
            dsp_call("window.fullscreen_state", &fields)
        }
        "movetoworkspace" | "movetoworkspacesilent" => {
            let (workspace, window) = match params.rsplit_once(',') {
                Some((workspace, window)) => (workspace.trim(), Some(window.trim())),
                None => (params, None),
            };
            let mut fields = vec![(
                "workspace",
                lua_value_expr(required(workspace, dispatcher, "a workspace")?),
            )];
            if dispatcher == "movetoworkspacesilent" {
                fields.push(("follow", "false".to_string()));
            }
            fields.extend(given_window(window));
            dsp_call("window.move", &fields)
        }
        "movefocus" => dsp_call("focus", &[("direction", direction(params)?)]),
        "movewindow" => {
            let (target, silent) = match params.strip_suffix(" silent") {
                Some(target) => (target.trim(), true),
                None => (params, false),
            };
            if let Some(monitor) = target.strip_prefix("mon:") {
                let mut fields = vec![("monitor", lua_quote(monitor.trim()))];
                if silent {
                    fields.push(("follow", "false".to_string()));
                }
                dsp_call("window.move", &fields)
            } else {
                dsp_call("window.move", &[("direction", direction(target)?)])
            }
        }
        "movewindoworgroup" => dsp_call(
            "window.move",
            &[
                ("direction", direction(params)?),
                ("group_aware", "true".to_string()),
            ],
        ),
        "swapwindow" => match direction(params) {
            Ok(dir) => dsp_call("window.swap", &[("direction", dir)]),
            Err(_) => dsp_call(
                "window.swap",
                &[(
                    "target",
                    lua_quote(required(params, dispatcher, "a direction or window")?),
                )],
            ),
        },
        "swapnext" => {
            let way = if matches!(params, "l" | "last" | "prev" | "b" | "back") {
                "prev"
            } else {
                "next"
            };
            dsp_call("window.swap", &[(way, "true".to_string())])
        }
        "cyclenext" => {
            let mut fields = Vec::new();
            let has = |names: &[&str]| words.iter().any(|word| names.contains(word));
            if has(&["prev", "p", "last", "l"]) && !has(&["next", "n"]) {
                fields.push(("next", "false".to_string()));
            }
            if has(&["tile", "tiled"]) {
                fields.push(("tiled", "true".to_string()));
            }
            if has(&["float", "floating"]) {
                fields.push(("floating", "true".to_string()));
            }
            dsp_call("window.cycle_next", &fields)
        }
        "focuswindow" | "focuswindowbyclass" => one("focus", "window", "a window")?,
        "focusmonitor" => one("focus", "monitor", "a monitor")?,
        "focusurgentorlast" => dsp_call("focus", &[("urgent_or_last", "true".to_string())]),
        "focuscurrentorlast" => dsp_call("focus", &[("last", "true".to_string())]),
        "resizeactive" | "moveactive" | "resizewindowpixel" | "movewindowpixel" => {
            let (vector, window) = match dispatcher {
                "resizewindowpixel" | "movewindowpixel" => {
                    let (vector, window) = params
                        .split_once(',')
                        .ok_or_else(|| format!("{} expects: x y,window", dispatcher))?;
                    (vector.trim(), Some(window.trim()))
                }
                _ => (params, None),
            };
            let (relative, vector) = match vector.strip_prefix("exact") {
                Some(rest) => (false, rest.trim()),
                None => (true, vector),
            };
            let coords = vector.split_whitespace().collect::<Vec<_>>();
            if coords.len() != 2 {
                return Err(format!(
                    "{} expects two numbers, e.g. \"10 -10\" or \"exact 800 600\"",
                    dispatcher
                ));
            }
            let mut fields = vec![
                ("x", pixel_number(coords[0], dispatcher)?),
                ("y", pixel_number(coords[1], dispatcher)?),
            ];
            if relative {
                fields.push(("relative", "true".to_string()));
            }
            fields.extend(given_window(window));
            let function = if dispatcher.starts_with("resize") {
                "window.resize"
            } else {
                "window.move"
            };
            dsp_call(function, &fields)
        }
        "tagwindow" => {
            let tag = words
                .first()
                .ok_or_else(|| "tagwindow expects: tag [window]".to_string())?;
            let mut fields = vec![("tag", lua_quote(tag))];
            if words.len() > 1 {
                fields.push(("window", lua_quote(&words[1..].join(" "))));
            }
            dsp_call("window.tag", &fields)
        }
        "alterzorder" => {
            let (mode, window) = match params.split_once(',') {
                Some((mode, window)) => (mode.trim(), Some(window.trim())),
                None => (params, None),
            };
            let mut fields = vec![(
                "mode",
                lua_quote(required(mode, dispatcher, "top or bottom")?),
            )];
            fields.extend(given_window(window));
            dsp_call("window.alter_zorder", &fields)
        }
        "setprop" => {
            if words.len() < 3 {
                return Err("setprop expects: window property value".to_string());
            }
            dsp_call(
                "window.set_prop",
                &[
                    ("prop", lua_quote(words[1])),
                    ("value", lua_quote(&words[2..].join(" "))),
                    ("window", lua_quote(words[0])),
                ],
            )
        }
        "mouse" => match words.as_slice() {
            ["movewindow"] => "hl.dsp.window.drag()".to_string(),
            ["resizewindow"] => "hl.dsp.window.resize()".to_string(),
            ["resizewindow", n @ ("1" | "2")] => dsp_call(
                "window.resize",
                &[("keep_aspect_ratio", (*n == "1").to_string())],
            ),
            _ => return Err("mouse expects movewindow or resizewindow [1|2]".to_string()),
        },
        "changegroupactive" => match params {
            "b" | "prev" => "hl.dsp.group.prev()".to_string(),
            "" | "f" | "next" => "hl.dsp.group.next()".to_string(),
            index if index.parse::<i64>().is_ok() => {
                dsp_call("group.active", &[("index", index.to_string())])
            }
            other => {
                return Err(format!(
                    "changegroupactive expects f, b or an index, got '{}'",
                    other
                ));
            }
        },
        "movegroupwindow" => {
            if matches!(params, "b" | "prev") {
                dsp_call("group.move_window", &[("forward", "false".to_string())])
            } else {
                "hl.dsp.group.move_window()".to_string()
            }
        }
        "lockgroups" => dsp_call("group.lock", &[action_field(params, LOCK_WORDS, true)?]),
        "lockactivegroup" => dsp_call(
            "group.lock_active",
            &[action_field(params, LOCK_WORDS, true)?],
        ),
        "denywindowfromgroup" => dsp_call(
            "window.deny_from_group",
            &[action_field(params, SWITCH_WORDS, false)?],
        ),
        "moveintogroup" => dsp_call("window.move", &[("into_group", direction(params)?)]),
        "moveintoorcreategroup" => dsp_call(
            "window.move",
            &[("into_or_create_group", direction(params)?)],
        ),
        "moveoutofgroup" => {
            let mut fields = vec![("out_of_group", "true".to_string())];
            fields.extend(window_field(params));
            dsp_call("window.move", &fields)
        }
        "movecursortocorner" => dsp_call(
            "cursor.move_to_corner",
            &[("corner", number(params, dispatcher)?)],
        ),
        "movecursor" => {
            if words.len() != 2 {
                return Err("movecursor expects: x y".to_string());
            }
            dsp_call(
                "cursor.move",
                &[
                    ("x", number(words[0], dispatcher)?),
                    ("y", number(words[1], dispatcher)?),
                ],
            )
        }
        "movecurrentworkspacetomonitor" => one("workspace.move", "monitor", "a monitor")?,
        "moveworkspacetomonitor" => {
            if words.len() != 2 {
                return Err("moveworkspacetomonitor expects: workspace monitor".to_string());
            }
            dsp_call(
                "workspace.move",
                &[
                    ("workspace", lua_value_expr(words[0])),
                    ("monitor", lua_quote(words[1])),
                ],
            )
        }
        "swapactiveworkspaces" => {
            if words.len() != 2 {
                return Err("swapactiveworkspaces expects: monitor1 monitor2".to_string());
            }
            dsp_call(
                "workspace.swap_monitors",
                &[
                    ("monitor1", lua_quote(words[0])),
                    ("monitor2", lua_quote(words[1])),
                ],
            )
        }
        "togglespecialworkspace" => {
            if params.is_empty() {
                "hl.dsp.workspace.toggle_special()".to_string()
            } else {
                format!("hl.dsp.workspace.toggle_special({})", lua_quote(params))
            }
        }
        "pass" => one("pass", "window", "a window")?,
        "sendshortcut" | "sendkeystate" => {
            let (needed, usage, function) = if dispatcher == "sendshortcut" {
                (2, "MODS, key[, window]", "send_shortcut")
            } else {
                (3, "MODS, key, down|up|repeat[, window]", "send_key_state")
            };
            let parts = params.split(',').map(str::trim).collect::<Vec<_>>();
            if parts.len() < needed || parts[1].is_empty() {
                return Err(format!("{dispatcher} expects: {usage}"));
            }
            let mut fields = vec![("mods", lua_quote(parts[0])), ("key", lua_quote(parts[1]))];
            if needed == 3 {
                fields.push(("state", lua_quote(parts[2])));
            }
            fields.extend(given_window(parts.get(needed).copied()));
            dsp_call(function, &fields)
        }
        // Older configs used these as dispatchers; they are layout messages now.
        "togglesplit" | "swapsplit" | "splitratio" => format!(
            "hl.dsp.layout({})",
            lua_quote(format!("{} {}", dispatcher, params).trim())
        ),
        "dpms" => {
            let action = match words.first().copied() {
                Some(word) if word.starts_with("on") => "on",
                Some(word) if word.starts_with("toggle") => "toggle",
                _ => "off",
            };
            let mut fields = vec![("action", lua_quote(action))];
            if words.len() > 1 {
                fields.push(("monitor", lua_quote(&words[1..].join(" "))));
            }
            dsp_call("dpms", &fields)
        }
        "forceidle" => format!("hl.dsp.force_idle({})", number(params, dispatcher)?),
        other => {
            return Err(format!(
                "'{}' has no Lua equivalent. Use the 'lua' dispatcher with an hl.dsp.* expression instead.",
                other
            ));
        }
    };

    Ok(expr)
}

fn dsp_call(function: &str, fields: &[(&str, String)]) -> String {
    if fields.is_empty() {
        format!("hl.dsp.{}()", function)
    } else {
        format!("hl.dsp.{}({{ {} }})", function, lua_table_fields(fields))
    }
}

fn required<'a>(value: &'a str, dispatcher: &str, what: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{} expects {}", dispatcher, what))
    } else {
        Ok(value)
    }
}

fn number(value: &str, dispatcher: &str) -> Result<String, String> {
    let value = value.trim();
    is_lua_number(value)
        .then(|| value.to_string())
        .ok_or_else(|| format!("{} expects a number, got '{}'", dispatcher, value))
}

fn pixel_number(value: &str, dispatcher: &str) -> Result<String, String> {
    if value.ends_with('%') {
        return Err(format!(
            "{}: percentages are not supported by hl.dsp; use pixels",
            dispatcher
        ));
    }
    number(value, dispatcher)
}

fn direction(value: &str) -> Result<String, String> {
    let dir = match value.trim() {
        "l" | "left" => "left",
        "r" | "right" => "right",
        "u" | "t" | "up" => "up",
        "d" | "b" | "down" => "down",
        other => return Err(format!("Invalid direction '{}' (expected l/r/u/d)", other)),
    };
    Ok(lua_quote(dir))
}

/// The `window` field for an optional selector, when one was given.
fn given_window(window: Option<&str>) -> Option<(&'static str, String)> {
    window
        .filter(|window| !window.is_empty())
        .map(|window| ("window", lua_quote(window)))
}

fn window_field(params: &str) -> Vec<(&'static str, String)> {
    let window = params.trim();
    if window.is_empty() || window == "active" {
        Vec::new()
    } else {
        vec![("window", lua_quote(window))]
    }
}

/// The `action` field for params that are one of `words` (toggle, on, off; empty means
/// toggle). `hl.dsp` spells on and off as `enable`/`disable` when `enable` is set, else as-is.
fn action_field(
    params: &str,
    [toggle, on, off]: [&str; 3],
    enable: bool,
) -> Result<(&'static str, String), String> {
    let action = match params.trim() {
        "" => "toggle",
        p if p == toggle => "toggle",
        p if p == on => {
            if enable {
                "enable"
            } else {
                "on"
            }
        }
        p if p == off => {
            if enable {
                "disable"
            } else {
                "off"
            }
        }
        other => return Err(format!("Expected {on}, {off} or {toggle}, got '{other}'")),
    };
    Ok(("action", lua_quote(action)))
}

/// Maps an `hl.dsp.*` expression back to a legacy dispatcher and params for the UI.
/// Anything that can't be expressed that way (Lua functions, extra fields, unknown
/// dispatchers) is reported as the `lua` dispatcher with the raw expression.
pub(super) fn parse_dispatcher_expr(
    expr: &str,
    variables: &HashMap<String, String>,
) -> (String, String) {
    parse_known_dispatcher(expr.trim(), variables)
        .unwrap_or_else(|| (RAW_LUA.to_string(), collapse_whitespace(expr.trim())))
}

struct DspArgs<'a> {
    positional: Vec<&'a str>,
    fields: Vec<(String, String)>,
    variables: &'a HashMap<String, String>,
}

impl DspArgs<'_> {
    fn raw(&self, key: &str) -> Option<&str> {
        table_value(&self.fields, key).map(str::trim)
    }

    fn string(&self, key: &str) -> Option<String> {
        eval_lua_string_expr(self.raw(key)?, self.variables)
    }

    fn flag(&self, key: &str) -> Option<bool> {
        match self.raw(key)? {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    }

    /// True when no field outside `allowed` is present.
    fn only(&self, allowed: &[&str]) -> bool {
        self.fields
            .iter()
            .all(|(key, _)| allowed.contains(&key.as_str()))
    }

    /// No arguments, or an empty table.
    fn is_empty(&self) -> bool {
        match self.positional.as_slice() {
            [] => true,
            [table] => table.starts_with('{') && table_entries(table).is_empty(),
            _ => false,
        }
    }

    /// `key`'s value after `sep`, or nothing when the field is absent.
    fn suffix(&self, key: &str, sep: &str) -> String {
        self.string(key)
            .map(|value| format!("{sep}{value}"))
            .unwrap_or_default()
    }

    /// The `action` field mapped to the dispatcher's own words for toggle, on and off.
    fn action(&self, [toggle, on, off]: [&'static str; 3]) -> Option<&'static str> {
        match self.string("action").as_deref() {
            None | Some("toggle") => Some(toggle),
            Some("enable" | "on") => Some(on),
            Some("disable" | "off") => Some(off),
            Some(_) => None,
        }
    }

    fn positional_string(&self) -> Option<String> {
        match self.positional.as_slice() {
            [value] => eval_lua_string_expr(value, self.variables),
            _ => None,
        }
    }
}

fn ok(dispatcher: &str, params: String) -> Option<(String, String)> {
    Some((dispatcher.to_string(), params))
}

fn parse_known_dispatcher(
    expr: &str,
    variables: &HashMap<String, String>,
) -> Option<(String, String)> {
    let rest = expr.strip_prefix("hl.dsp.")?;
    let open = rest.find('(')?;
    let name = rest[..open].trim();
    let close = find_matching_delimiter(rest, open, b'(', b')')?;
    if !rest[close + 1..].trim().is_empty() {
        return None;
    }

    let positional = split_top_level(&rest[open + 1..close], b',');
    let is_table = matches!(positional.as_slice(), [table] if table.starts_with('{'));
    let fields = if is_table {
        parse_lua_table_fields(positional[0])
    } else {
        Vec::new()
    };
    let args = DspArgs {
        positional,
        fields,
        variables,
    };

    if let Some((dispatcher, _)) = NO_ARGS.iter().find(|(_, f)| *f == name) {
        return args
            .is_empty()
            .then(|| (dispatcher.to_string(), String::new()));
    }
    if let Some((dispatcher, ..)) = ONE_STRING.iter().find(|(_, f, _)| *f == name) {
        return ok(dispatcher, args.positional_string()?);
    }

    match name {
        "exec_cmd" if args.positional.len() == 1 => {
            ok("exec", lua_expr_to_template(args.positional[0], variables)?)
        }
        "exec_raw" if args.positional.len() == 1 => ok(
            "execr",
            lua_expr_to_template(args.positional[0], variables)?,
        ),
        "force_idle" => ok("forceidle", args.positional_string()?),
        "pass" if is_table && args.only(&["window"]) => ok("pass", args.string("window")?),
        "send_shortcut" if is_table && args.only(&["mods", "key", "window"]) => ok(
            "sendshortcut",
            format!(
                "{}, {}{}",
                args.string("mods")?,
                args.string("key")?,
                args.suffix("window", ", ")
            ),
        ),
        "send_key_state" if is_table && args.only(&["mods", "key", "state", "window"]) => ok(
            "sendkeystate",
            format!(
                "{}, {}, {}{}",
                args.string("mods")?,
                args.string("key")?,
                args.string("state")?,
                args.suffix("window", ", ")
            ),
        ),
        "dpms" if args.only(&["action", "monitor"]) => {
            let action = args.action(SWITCH_WORDS)?;
            ok("dpms", format!("{}{}", action, args.suffix("monitor", " ")))
        }
        "focus" if is_table => {
            if let Some(dir) = args.string("direction") {
                return args
                    .only(&["direction"])
                    .then(|| ("movefocus".to_string(), dir));
            }
            if let Some(monitor) = args.string("monitor") {
                return args
                    .only(&["monitor"])
                    .then(|| ("focusmonitor".to_string(), monitor));
            }
            if let Some(workspace) = args.string("workspace") {
                if !args.only(&["workspace", "on_current_monitor"]) {
                    return None;
                }
                return match args.flag("on_current_monitor") {
                    Some(true) => ok("focusworkspaceoncurrentmonitor", workspace),
                    None | Some(false) => ok("workspace", workspace),
                };
            }
            if let Some(window) = args.string("window") {
                return args
                    .only(&["window"])
                    .then(|| ("focuswindow".to_string(), window));
            }
            if args.flag("urgent_or_last") == Some(true) && args.only(&["urgent_or_last"]) {
                return ok("focusurgentorlast", String::new());
            }
            if args.flag("last") == Some(true) && args.only(&["last"]) {
                return ok("focuscurrentorlast", String::new());
            }
            None
        }
        "cursor.move_to_corner" if is_table && args.only(&["corner"]) => {
            ok("movecursortocorner", args.string("corner")?)
        }
        "cursor.move" if is_table && args.only(&["x", "y"]) => ok(
            "movecursor",
            format!("{} {}", args.string("x")?, args.string("y")?),
        ),
        "group.next" if args.is_empty() => ok("changegroupactive", "f".to_string()),
        "group.prev" if args.is_empty() => ok("changegroupactive", "b".to_string()),
        "group.active" if is_table && args.only(&["index"]) => {
            ok("changegroupactive", args.string("index")?)
        }
        "group.move_window" if args.only(&["forward"]) => match args.flag("forward") {
            Some(false) => ok("movegroupwindow", "b".to_string()),
            _ => ok("movegroupwindow", String::new()),
        },
        "group.lock" | "group.lock_active" if args.only(&["action"]) => {
            let action = args.action(LOCK_WORDS)?;
            let dispatcher = if name == "group.lock" {
                "lockgroups"
            } else {
                "lockactivegroup"
            };
            ok(dispatcher, action.to_string())
        }
        "window.close" if args.only(&["window"]) => match args.string("window") {
            Some(window) => ok("closewindow", window),
            None => ok("killactive", String::new()),
        },
        "window.kill" if args.only(&["window"]) => match args.string("window") {
            Some(window) => ok("killwindow", window),
            None => ok("forcekillactive", String::new()),
        },
        "window.signal" if is_table && args.only(&["signal", "window"]) => {
            let signal = args.string("signal")?;
            match args.string("window") {
                Some(window) => ok("signalwindow", format!("{},{}", window, signal)),
                None => ok("signal", signal),
            }
        }
        "window.float" if args.only(&["action", "window"]) => {
            let dispatcher = args.action(FLOAT_ACTIONS.map(|(d, _)| d))?;
            ok(dispatcher, args.string("window").unwrap_or_default())
        }
        "window.pseudo" | "window.pin"
            if args.only(&["action", "window"])
                && args
                    .string("action")
                    .is_none_or(|action| action == "toggle") =>
        {
            let dispatcher = if name == "window.pseudo" {
                "pseudo"
            } else {
                "pin"
            };
            ok(dispatcher, args.string("window").unwrap_or_default())
        }
        "window.fullscreen" if args.only(&["mode", "action"]) => {
            let mode = match args.string("mode").as_deref() {
                None | Some("fullscreen" | "0") => "0",
                Some("maximized" | "1") => "1",
                Some(_) => return None,
            };
            match args.string("action") {
                Some(action) => ok("fullscreen", format!("{} {}", mode, action)),
                None if mode == "0" => ok("fullscreen", String::new()),
                None => ok("fullscreen", mode.to_string()),
            }
        }
        "window.fullscreen_state" if is_table && args.only(&["internal", "client", "action"]) => {
            ok(
                "fullscreenstate",
                format!(
                    "{} {}{}",
                    args.string("internal")?,
                    args.string("client")?,
                    args.suffix("action", " ")
                ),
            )
        }
        "window.move" | "window.resize" if is_table => parse_window_move(name, &args),
        "window.drag" if args.is_empty() => ok("mouse", "movewindow".to_string()),
        "window.resize" if args.is_empty() => ok("mouse", "resizewindow".to_string()),
        "window.swap" if is_table => {
            if let Some(dir) = args.string("direction") {
                return args
                    .only(&["direction"])
                    .then(|| ("swapwindow".to_string(), dir));
            }
            if args.flag("next") == Some(true) && args.only(&["next"]) {
                return ok("swapnext", String::new());
            }
            if args.flag("prev") == Some(true) && args.only(&["prev"]) {
                return ok("swapnext", "prev".to_string());
            }
            for key in ["target", "with", "other"] {
                if let Some(target) = args.string(key) {
                    return args
                        .only(&[key])
                        .then(|| ("swapwindow".to_string(), target));
                }
            }
            None
        }
        "window.cycle_next" if args.only(&["next", "tiled", "floating"]) => {
            let mut words = Vec::new();
            if args.flag("next") == Some(false) {
                words.push("prev");
            }
            match args.flag("tiled") {
                Some(true) => words.push("tiled"),
                Some(false) => return None,
                None => {}
            }
            match args.flag("floating") {
                Some(true) => words.push("floating"),
                Some(false) => return None,
                None => {}
            }
            ok("cyclenext", words.join(" "))
        }
        "window.tag" if is_table && args.only(&["tag", "window"]) => ok(
            "tagwindow",
            format!("{}{}", args.string("tag")?, args.suffix("window", " ")),
        ),
        "window.alter_zorder" if is_table && args.only(&["mode", "window"]) => ok(
            "alterzorder",
            format!("{}{}", args.string("mode")?, args.suffix("window", ",")),
        ),
        "window.set_prop" if is_table && args.only(&["prop", "value", "window"]) => ok(
            "setprop",
            format!(
                "{} {} {}",
                args.string("window")?,
                args.string("prop")?,
                args.string("value")?
            ),
        ),
        "window.deny_from_group" if args.only(&["action"]) => {
            let action = args.action(SWITCH_WORDS)?;
            ok("denywindowfromgroup", action.to_string())
        }
        "workspace.toggle_special" => {
            let workspace = if args.positional.is_empty() {
                String::new()
            } else {
                args.positional_string()?
            };
            ok("togglespecialworkspace", workspace)
        }
        "workspace.rename" if is_table && args.only(&["workspace", "name"]) => ok(
            "renameworkspace",
            format!("{}{}", args.string("workspace")?, args.suffix("name", " ")),
        ),
        "workspace.move" if is_table && args.only(&["workspace", "monitor"]) => {
            let monitor = args.string("monitor")?;
            match args.string("workspace") {
                Some(workspace) => ok(
                    "moveworkspacetomonitor",
                    format!("{} {}", workspace, monitor),
                ),
                None => ok("movecurrentworkspacetomonitor", monitor),
            }
        }
        "workspace.swap_monitors" if is_table && args.only(&["monitor1", "monitor2"]) => ok(
            "swapactiveworkspaces",
            format!("{} {}", args.string("monitor1")?, args.string("monitor2")?),
        ),
        _ => None,
    }
}

fn parse_window_move(name: &str, args: &DspArgs) -> Option<(String, String)> {
    if let (Some(x), Some(y)) = (args.string("x"), args.string("y")) {
        if !args.only(&["x", "y", "relative", "window"]) {
            return None;
        }
        let vector = match args.flag("relative") {
            Some(true) => format!("{} {}", x, y),
            _ => format!("exact {} {}", x, y),
        };
        let resize = name == "window.resize";
        return match args.string("window") {
            Some(window) => ok(
                if resize {
                    "resizewindowpixel"
                } else {
                    "movewindowpixel"
                },
                format!("{},{}", vector, window),
            ),
            None => ok(if resize { "resizeactive" } else { "moveactive" }, vector),
        };
    }

    if name == "window.resize" {
        let keep = args
            .flag("keep_aspect_ratio")
            .filter(|_| args.only(&["keep_aspect_ratio"]))?;
        return ok(
            "mouse",
            format!("resizewindow {}", if keep { 1 } else { 2 }),
        );
    }

    if let Some(dir) = args.string("direction") {
        if !args.only(&["direction", "group_aware"]) {
            return None;
        }
        return match args.flag("group_aware") {
            Some(true) => ok("movewindoworgroup", dir),
            _ => ok("movewindow", dir),
        };
    }

    if let Some(workspace) = args.string("workspace") {
        if !args.only(&["workspace", "follow", "window"]) {
            return None;
        }
        let dispatcher = match args.flag("follow") {
            Some(false) => "movetoworkspacesilent",
            _ => "movetoworkspace",
        };
        return ok(
            dispatcher,
            format!("{}{}", workspace, args.suffix("window", ",")),
        );
    }

    if let Some(monitor) = args.string("monitor") {
        if !args.only(&["monitor", "follow"]) {
            return None;
        }
        let silent = if args.flag("follow") == Some(false) {
            " silent"
        } else {
            ""
        };
        return ok("movewindow", format!("mon:{}{}", monitor, silent));
    }

    for (key, dispatcher) in [
        ("into_group", "moveintogroup"),
        ("into_or_create_group", "moveintoorcreategroup"),
    ] {
        if let Some(dir) = args.string(key) {
            return args.only(&[key]).then(|| (dispatcher.to_string(), dir));
        }
    }

    if args.flag("out_of_group") == Some(true) && args.only(&["out_of_group", "window"]) {
        return ok("moveoutofgroup", args.string("window").unwrap_or_default());
    }

    None
}

/// Converts a Lua string expression into the `$var` template the UI edits, e.g.
/// `terminal .. " --hold"` becomes `$terminal --hold`.
pub(super) fn lua_expr_to_template(
    expr: &str,
    variables: &HashMap<String, String>,
) -> Option<String> {
    let expr = strip_wrapping_parens(expr.trim());
    if split_lua_or(expr).is_some() {
        return eval_lua_string_expr(expr, variables);
    }

    let mut out = String::new();
    // A variable directly followed by text is written as `${name}` so the name stays
    // readable, e.g. `noctCall .. "panel-toggle"` becomes `${noctCall}panel-toggle`.
    let mut open_variable: Option<String> = None;
    for part in split_lua_concat(expr) {
        let part = strip_wrapping_parens(part.trim());
        let text = if is_lua_identifier(part) && variables.contains_key(part) {
            if let Some(name) = open_variable.replace(part.to_string()) {
                out.push_str(&format!("${}", name));
            }
            continue;
        } else if let Some(literal) = parse_lua_string_literal(part) {
            literal
        } else {
            eval_lua_string_expr(part, variables)?
        };
        if let Some(name) = open_variable.take() {
            let glued = text.starts_with(is_name_char);
            out.push_str(&if glued {
                format!("${{{}}}", name)
            } else {
                format!("${}", name)
            });
        }
        out.push_str(&text);
    }
    if let Some(name) = open_variable {
        out.push_str(&format!("${}", name));
    }
    Some(out)
}

/// Converts a `$var` template into a Lua string expression. Only names assigned in the
/// target file become identifiers, so shell variables like `$HOME` stay literal.
pub(super) fn lua_string_expr_with_vars(value: &str, scope: &HashSet<String>) -> String {
    let parts = resolve_command(value, |name| scope.contains(name));
    if parts.is_empty() {
        return lua_quote("");
    }
    parts
        .iter()
        .map(|part| {
            if part.variable {
                variable_name(&part.text).to_string()
            } else {
                lua_quote(&part.text)
            }
        })
        .collect::<Vec<_>>()
        .join(" .. ")
}

#[cfg(test)]
mod tests {
    use super::{dispatcher_to_lua_expr, parse_dispatcher_expr};
    use std::collections::{HashMap, HashSet};

    #[test]
    fn every_suggested_dispatcher_has_a_lua_form() {
        let scope = HashSet::new();
        for (name, _) in crate::dispatchers::DISPATCHERS {
            if let Err(e) = dispatcher_to_lua_expr(name, "", &scope) {
                assert!(!e.contains("no Lua equivalent"), "{name}: {e}");
            }
        }
    }

    #[test]
    fn floating_dispatchers_map_to_their_actions() {
        let scope = HashSet::new();
        let vars = HashMap::new();
        for (dispatcher, action) in [
            ("togglefloating", "toggle"),
            ("setfloating", "enable"),
            ("settiled", "disable"),
        ] {
            let expr = dispatcher_to_lua_expr(dispatcher, "class:mpv", &scope).unwrap();
            assert_eq!(
                expr,
                format!("hl.dsp.window.float({{ action = \"{action}\", window = \"class:mpv\" }})")
            );
            assert_eq!(
                parse_dispatcher_expr(&expr, &vars),
                (dispatcher.to_string(), "class:mpv".to_string())
            );
        }
    }

    #[test]
    fn rejects_params_hl_dsp_cant_take() {
        let scope = HashSet::new();
        assert!(dispatcher_to_lua_expr("forceidle", "inf", &scope).is_err());
        assert!(dispatcher_to_lua_expr("mouse", "dragwindow", &scope).is_err());
        assert_eq!(
            dispatcher_to_lua_expr("mouse", "resizewindow 1", &scope).unwrap(),
            "hl.dsp.window.resize({ keep_aspect_ratio = true })"
        );
    }
}
