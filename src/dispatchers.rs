//! Hyprland dispatchers the keybind form suggests, and how a bind's command names variables.
//! Names are the classic dispatcher names; for Lua configs the backend translates them to the
//! matching `hl.dsp.*` call.

/// Every suggested dispatcher, with what it does.
pub const DISPATCHERS: &[(&str, &str)] = &[
    ("exec", "Execute a shell command"),
    ("execr", "Execute a raw shell command (no window rules)"),
    ("killactive", "Close the active window"),
    ("forcekillactive", "Kill the active window's process"),
    ("closewindow", "Close a window (window selector)"),
    ("killwindow", "Kill a window's process (window selector)"),
    ("signal", "Send a signal to the active window"),
    ("signalwindow", "Send a signal to a window (window,signal)"),
    (
        "togglefloating",
        "Toggle floating mode for the active window",
    ),
    ("setfloating", "Make the active window floating"),
    ("settiled", "Make the active window tiled"),
    (
        "fullscreen",
        "Toggle fullscreen (0 fullscreen, 1 maximize; optional set/unset)",
    ),
    (
        "fullscreenstate",
        "Set internal and client fullscreen state (internal client)",
    ),
    ("pseudo", "Toggle pseudo-tiling mode"),
    ("pin", "Pin the active window"),
    ("centerwindow", "Center the active window"),
    ("workspace", "Switch to a specific workspace"),
    (
        "focusworkspaceoncurrentmonitor",
        "Focus workspace on current monitor",
    ),
    ("renameworkspace", "Rename a workspace (id name)"),
    ("movetoworkspace", "Move window to workspace"),
    (
        "movetoworkspacesilent",
        "Move window to workspace without switching to it",
    ),
    (
        "togglespecialworkspace",
        "Toggle special workspace (scratchpad)",
    ),
    ("movefocus", "Move focus in a direction (l/r/u/d)"),
    (
        "movewindow",
        "Move window in a direction, or to a monitor (mon:NAME)",
    ),
    (
        "movewindoworgroup",
        "Move window in a direction, entering or leaving groups",
    ),
    (
        "swapwindow",
        "Swap with the window in a direction or matching a selector",
    ),
    ("swapnext", "Swap with the next window (prev for previous)"),
    (
        "cyclenext",
        "Cycle to the next window (prev, tiled, floating)",
    ),
    ("focuswindow", "Focus a window (window selector)"),
    ("focusmonitor", "Focus a specific monitor"),
    ("focusurgentorlast", "Focus urgent or last window"),
    ("focuscurrentorlast", "Focus the previously focused window"),
    (
        "resizeactive",
        "Resize the active window (x y, or exact x y)",
    ),
    ("moveactive", "Move the active window (x y, or exact x y)"),
    ("resizewindowpixel", "Resize a window (x y,window)"),
    ("movewindowpixel", "Move a window (x y,window)"),
    ("tagwindow", "Toggle a tag on a window (tag [window])"),
    ("toggleswallow", "Toggle window swallowing"),
    ("bringactivetotop", "Bring active window to top"),
    ("alterzorder", "Change window z-order (top/bottom[,window])"),
    ("setprop", "Set a window property (window property value)"),
    (
        "mouse",
        "Drag or resize with the mouse (movewindow/resizewindow)",
    ),
    ("togglegroup", "Toggle the current window's group"),
    (
        "changegroupactive",
        "Change the active window in a group (f, b or index)",
    ),
    (
        "movegroupwindow",
        "Move window within group (b for backwards)",
    ),
    ("lockgroups", "Lock/unlock window groups"),
    ("lockactivegroup", "Lock/unlock the active group"),
    ("moveintogroup", "Move window into a group"),
    (
        "moveintoorcreategroup",
        "Move window into a group, creating one if needed",
    ),
    ("moveoutofgroup", "Move window out of group"),
    (
        "denywindowfromgroup",
        "Prevent the active window from joining groups (on/off/toggle)",
    ),
    ("movecursortocorner", "Move cursor to a corner"),
    ("movecursor", "Move cursor to coordinates"),
    (
        "movecurrentworkspacetomonitor",
        "Move current workspace to a monitor",
    ),
    (
        "moveworkspacetomonitor",
        "Move a workspace to a monitor (workspace monitor)",
    ),
    (
        "swapactiveworkspaces",
        "Swap active workspaces of two monitors",
    ),
    ("submap", "Switch to a key submap"),
    ("pass", "Pass key to a window"),
    (
        "sendshortcut",
        "Send shortcut to a window (MODS, key[, window])",
    ),
    (
        "sendkeystate",
        "Send a key state to a window (MODS, key, down|up|repeat[, window])",
    ),
    ("layoutmsg", "Send message to layout (e.g. togglesplit)"),
    ("dpms", "Control monitor power (on/off/toggle [monitor])"),
    ("exit", "Exit Hyprland"),
    ("forcerendererreload", "Force renderer reload"),
    (
        "forceidle",
        "Pretend the session has been idle for N seconds",
    ),
    ("releaseinputcapture", "Release an active input capture"),
    ("event", "Emit a custom event on the event socket"),
    ("global", "Global shortcut (works when not focused)"),
    (
        RAW_LUA,
        "Raw Lua dispatcher, e.g. hl.dsp.window.close() or function() ... end",
    ),
];

/// The dispatcher whose params are written to the config as Lua, verbatim.
pub const RAW_LUA: &str = "lua";

/// The modifiers the keybind form has a checkbox for.
pub const MODIFIERS: [&str; 4] = ["SUPER", "SHIFT", "ALT", "CTRL"];

/// The checkbox a modifier as written in a config ticks: Hyprland's aliases (`CONTROL`,
/// `MOD4`, `WIN`, ...) included, any case. `None` for modifiers without a checkbox (`CAPS`,
/// `MOD5`, a variable, `CTRL+ALT` written as one word).
pub fn known_modifier(written: &str) -> Option<&'static str> {
    match written.to_uppercase().as_str() {
        "SUPER" | "WIN" | "LOGO" | "MOD4" | "META" => Some("SUPER"),
        "SHIFT" => Some("SHIFT"),
        "ALT" | "MOD1" => Some("ALT"),
        "CTRL" | "CONTROL" => Some("CTRL"),
        _ => None,
    }
}

/// What a bind's modifier stands for in the form: a checkbox name (`SUPER`), also through a
/// `$variable` naming one modifier, or else the modifier exactly as written.
pub fn modifier_id(written: &str, variables: &[(&str, &str)]) -> String {
    if let Some(m) = known_modifier(written) {
        return m.to_owned();
    }
    if written.starts_with('$') {
        let name = variable_name(written);
        if let Some(m) = variables
            .iter()
            .find(|(n, _)| *n == name)
            .and_then(|(_, v)| known_modifier(v.trim()))
        {
            return m.to_owned();
        }
    }
    written.to_owned()
}

/// The modifiers to write for a bind that had `original` (each as written, with its
/// `modifier_id`) once the form's ticked ids are `ticked`. Kept modifiers stay as written and
/// in their order, so an untouched set saves unchanged; newly ticked ones follow in the form's
/// order.
pub fn modifiers_to_save(original: &[(String, String)], ticked: &[String]) -> Vec<String> {
    let mut out: Vec<String> = original
        .iter()
        .filter(|(_, id)| ticked.contains(id))
        .map(|(written, _)| written.clone())
        .collect();
    for m in MODIFIERS {
        let wanted = ticked.iter().any(|t| t == m);
        let kept = original.iter().any(|(_, id)| id == m);
        if wanted && !kept {
            out.push(m.to_owned());
        }
    }
    out
}

pub fn description(dispatcher: &str) -> &'static str {
    let d = dispatcher.to_lowercase();
    DISPATCHERS
        .iter()
        .find(|(n, _)| *n == d)
        .map_or("Custom dispatcher command", |(_, desc)| desc)
}

/// Up to eight dispatchers containing `typed`, case-insensitively; none for an empty field or
/// an exact match.
pub fn suggestions(typed: &str) -> Vec<&'static str> {
    let q = typed.trim().to_lowercase();
    if q.is_empty() {
        return Vec::new();
    }
    let hits: Vec<&'static str> = DISPATCHERS
        .iter()
        .map(|(n, _)| *n)
        .filter(|n| n.contains(&q))
        .take(8)
        .collect();
    if hits.len() == 1 && hits[0] == q {
        Vec::new()
    } else {
        hits
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub text: String,
    pub variable: bool,
}

/// Whether `c` can be part of a variable name.
pub fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Splits a command into plain text and `$name` / `${name}` variable references.
pub fn parse_command(command: &str) -> Vec<Part> {
    let word = is_name_char;
    let mut parts = Vec::new();
    let mut plain = String::new();
    let mut rest = command;
    while let Some(i) = rest.find('$') {
        plain.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let braced = after.strip_prefix('{').and_then(|b| {
            let end = b.find('}')?;
            (end > 0 && b[..end].chars().all(word)).then_some(end + 3)
        });
        let len =
            braced.unwrap_or_else(|| 1 + after.find(|c: char| !word(c)).unwrap_or(after.len()));
        if len > 1 {
            if !plain.is_empty() {
                parts.push(Part {
                    text: std::mem::take(&mut plain),
                    variable: false,
                });
            }
            parts.push(Part {
                text: rest[i..i + len].to_owned(),
                variable: true,
            });
        } else {
            plain.push('$');
        }
        rest = &rest[i + len..];
    }
    plain.push_str(rest);
    if !plain.is_empty() {
        parts.push(Part {
            text: plain,
            variable: false,
        });
    }
    parts
}

/// Splits a command like `parse_command`, keeping only references to names `known` answers
/// for as variables. `${name}` must name one whole; an unbraced `$name` takes the longest known
/// prefix, so with `terminal` known, `$terminalx` is `$terminal` followed by `x`.
pub fn resolve_command(command: &str, known: impl Fn(&str) -> bool) -> Vec<Part> {
    let mut out: Vec<Part> = Vec::new();
    let mut push = |text: &str, variable: bool| match out.last_mut() {
        Some(last) if !variable && !last.variable => last.text.push_str(text),
        _ if !text.is_empty() => out.push(Part {
            text: text.to_owned(),
            variable,
        }),
        _ => {}
    };
    for part in parse_command(command) {
        if !part.variable {
            push(&part.text, false);
            continue;
        }
        let name = variable_name(&part.text);
        let braced = part.text.starts_with("${");
        let prefix = if braced {
            known(name).then_some(name.len())
        } else {
            (1..=name.len()).rev().find(|&n| known(&name[..n]))
        };
        match prefix {
            Some(_) if braced => push(&part.text, true),
            Some(n) => {
                push(&part.text[..1 + n], true);
                push(&name[n..], false);
            }
            None => push(&part.text, false),
        }
    }
    out
}

/// `$name` or `${name}` to `name`.
pub fn variable_name(reference: &str) -> &str {
    reference
        .trim_start_matches('$')
        .trim_start_matches('{')
        .trim_end_matches('}')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(text: &str, variable: bool) -> Part {
        Part {
            text: text.into(),
            variable,
        }
    }

    #[test]
    fn splits_variables_from_text() {
        assert_eq!(
            parse_command("$terminal -e htop"),
            vec![p("$terminal", true), p(" -e htop", false)]
        );
        assert_eq!(
            parse_command("${menu}run --x"),
            vec![p("${menu}", true), p("run --x", false)]
        );
        assert_eq!(
            parse_command("echo $ $5"),
            vec![p("echo $ ", false), p("$5", true)]
        );
        assert_eq!(parse_command("plain"), vec![p("plain", false)]);
        assert!(parse_command("").is_empty());
    }

    #[test]
    fn names_from_references() {
        assert_eq!(variable_name("$terminal"), "terminal");
        assert_eq!(variable_name("${menu}"), "menu");
    }

    #[test]
    fn resolves_only_known_variables() {
        let known = |n: &str| matches!(n, "terminal" | "menu");
        assert_eq!(
            resolve_command("$terminalx --hold $HOME ${menu}!", known),
            vec![
                p("$terminal", true),
                p("x --hold $HOME ", false),
                p("${menu}", true),
                p("!", false),
            ]
        );
        assert_eq!(
            resolve_command("${terminalx}", known),
            vec![p("${terminalx}", false)]
        );
    }

    #[test]
    fn suggestions_filter_and_cap() {
        assert!(suggestions("").is_empty());
        assert_eq!(suggestions("exec"), vec!["exec", "execr"]);
        assert!(suggestions("lua").is_empty());
        assert!(suggestions("lu").contains(&"lua"));
        assert!(suggestions("move").len() <= 8);
        assert!(suggestions("forcekillactive").is_empty());
    }

    fn written(mods: &[&str], vars: &[(&str, &str)]) -> Vec<(String, String)> {
        mods.iter()
            .map(|m| ((*m).to_owned(), modifier_id(m, vars)))
            .collect()
    }

    fn ids(original: &[(String, String)]) -> Vec<String> {
        original.iter().map(|(_, id)| id.clone()).collect()
    }

    #[test]
    fn modifier_aliases_tick_their_checkbox() {
        assert_eq!(known_modifier("control"), Some("CTRL"));
        assert_eq!(known_modifier("MOD4"), Some("SUPER"));
        assert_eq!(known_modifier("Win"), Some("SUPER"));
        assert_eq!(known_modifier("MOD5"), None);
        assert_eq!(known_modifier("CONTROL+MOD4"), None);
        let vars = [("mainMod", "SUPER")];
        assert_eq!(modifier_id("$mainMod", &vars), "SUPER");
        assert_eq!(modifier_id("${mainMod}", &vars), "SUPER");
        assert_eq!(modifier_id("$other", &vars), "$other");
        assert_eq!(modifier_id("CAPS", &vars), "CAPS");
    }

    #[test]
    fn untouched_modifiers_save_as_written() {
        let vars = [("mainMod", "SUPER")];
        for mods in [
            &["$mainMod"][..],
            &["CONTROL", "MOD4"],
            &["CTRL", "SUPER"],
            &["CONTROL+MOD4"],
            &["$unknown", "SHIFT"],
            &["super", "shift"],
            &[],
        ] {
            let original = written(mods, &vars);
            assert_eq!(modifiers_to_save(&original, &ids(&original)), mods);
        }
    }

    #[test]
    fn changed_modifiers_keep_the_rest_as_written() {
        let vars = [("mainMod", "SUPER")];
        let original = written(&["$mainMod", "CONTROL", "MOD5"], &vars);
        // Untick CTRL, tick SHIFT.
        let ticked = vec!["SUPER".to_owned(), "MOD5".to_owned(), "SHIFT".to_owned()];
        assert_eq!(
            modifiers_to_save(&original, &ticked),
            vec!["$mainMod", "MOD5", "SHIFT"]
        );
        // Untick SUPER: the variable standing for it goes.
        let ticked = vec!["CTRL".to_owned(), "MOD5".to_owned()];
        assert_eq!(
            modifiers_to_save(&original, &ticked),
            vec!["CONTROL", "MOD5"]
        );
        // A new bind gets the form's order.
        let ticked = vec!["CTRL".to_owned(), "SUPER".to_owned()];
        assert_eq!(modifiers_to_save(&[], &ticked), vec!["SUPER", "CTRL"]);
    }

    #[test]
    fn descriptions_fall_back() {
        assert_eq!(description("EXEC"), "Execute a shell command");
        assert_eq!(description("myplugin:thing"), "Custom dispatcher command");
    }
}
