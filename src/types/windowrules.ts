export interface WindowruleProperty {
  key: string;
  value: string;
  property_type: string;
}

export interface Windowrule {
  name: string;
  enabled: boolean;
  match_properties: WindowruleProperty[];
  effect_properties: WindowruleProperty[];
}

// Match properties for windowrule v3 (synced with hyprlang-rs 0.5.0)
export const WINDOWRULE_MATCH_PROPERTIES = [
  "class",
  "title",
  "initial_class",
  "initial_title",
  "floating",
  "tag",
  "xwayland",
  "fullscreen",
  "pinned",
  "focus",
  "group",
  "modal",
  "fullscreenstate_internal",
  "fullscreenstate_client",
  "on_workspace",
  "content",
  "xdg_tag",
  "namespace",
  "exec_token",
  // Aliases
  "float",
  "pin",
  "workspace",
  "fullscreen_state_internal",
  "fullscreen_state_client",
] as const;

// Effect properties for windowrule v3 (synced with hyprlang-rs 0.5.0)
export const WINDOWRULE_EFFECT_PROPERTIES = [
  // Static effects
  "float",
  "tile",
  "fullscreen",
  "maximize",
  "fullscreenstate",
  "fullscreen_state",
  "move",
  "size",
  "center",
  "pseudo",
  "monitor",
  "workspace",
  "noinitialfocus",
  "no_initial_focus",
  "pin",
  "group",
  "suppressevent",
  "suppress_event",
  "content",
  "noclosefor",
  "no_close_for",
  // Dynamic effects
  "rounding",
  "rounding_power",
  "persistent_size",
  "animation",
  "border_color",
  "bordercolor",
  "idle_inhibit",
  "idleinhibit",
  "opacity",
  "tag",
  "max_size",
  "maxsize",
  "min_size",
  "minsize",
  "border_size",
  "bordersize",
  "allows_input",
  "dim_around",
  "decorate",
  "focus_on_activate",
  "keep_aspect_ratio",
  "keepaspectratio",
  "nearest_neighbor",
  "nearestneighbor",
  "no_anim",
  "noanim",
  "no_blur",
  "noblur",
  "no_dim",
  "nodim",
  "no_focus",
  "nofocus",
  "no_follow_mouse",
  "nofollowmouse",
  "no_max_size",
  "nomaxsize",
  "no_shadow",
  "noshadow",
  "no_shortcuts_inhibit",
  "noshortcutsinhibit",
  "opaque",
  "force_rgbx",
  "forcergbx",
  "sync_fullscreen",
  "syncfullscreen",
  "immediate",
  "xray",
  "render_unfocused",
  "renderunfocused",
  "no_screen_share",
  "noscreenshare",
  "no_vrr",
  "novrr",
  "scroll_mouse",
  "scrollmouse",
  "scroll_touchpad",
  "scrolltouchpad",
  "stay_focused",
  "stayfocused",
] as const;

export type WindowruleMatchProperty = typeof WINDOWRULE_MATCH_PROPERTIES[number];
export type WindowruleEffectProperty = typeof WINDOWRULE_EFFECT_PROPERTIES[number];
