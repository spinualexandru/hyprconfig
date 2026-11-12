//! Parse and edit round-trips over in-memory sources and throwaway config directories.

use super::dispatchers::*;
use super::env::*;
use super::keybinds::*;
use super::lexer::*;
use super::monitors::*;
use super::project::*;
use super::rules::*;
use super::variables::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

fn layout(width: u16, height: u16, refresh_rate: f32, x: i32, y: i32, scale: f32) -> MonitorLayout {
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
        let root =
            std::env::temp_dir().join(format!("hyprconfig-lua-{}-{}", name, std::process::id()));
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

    let unnamed = find_rule_statement(&project, "window_rule", "unnamed window rule #1").unwrap();
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
    assert!(
        dir.read("hyprland.lua").contains(
            "hl.bind(mainMod .. \" + mouse:272\", hl.dsp.window.drag(), { mouse = true })"
        )
    );
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
    assert!(
        dir.read("hyprland.lua")
            .contains("hl.bind(\"SUPER + Return\", hl.dsp.exec_cmd(TERMINAL .. \" -e $private\"))")
    );
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
    assert!(
        dir.read("modules/keybinds.lua").contains(
            "hl.bind(main_mod .. \" + Return\", hl.dsp.exec_cmd(terminal .. \" --hold\"))"
        )
    );
}
