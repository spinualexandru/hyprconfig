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

// Match properties (synced with Hyprland src/desktop/rule/Rule.cpp)
export const WINDOWRULE_MATCH_PROPERTIES = [
  "class",
  "title",
  "initial_class",
  "initial_title",
  "float",
  "tag",
  "xwayland",
  "fullscreen",
  "pin",
  "focus",
  "group",
  "modal",
  "fullscreen_state_internal",
  "fullscreen_state_client",
  "workspace",
  "content",
  "xdg_tag",
] as const;

// Effect properties (synced with Hyprland WINDOW_RULE_EFFECT_DESCS). Plugins can
// register more, so rules are displayed with every field they contain.
export const WINDOWRULE_EFFECT_PROPERTIES = [
  // Static effects
  "float",
  "tile",
  "fullscreen",
  "maximize",
  "center",
  "pseudo",
  "no_initial_focus",
  "pin",
  "fullscreen_state",
  "move",
  "size",
  "monitor",
  "workspace",
  "group",
  "suppress_event",
  "content",
  "no_close_for",
  "scrolling_width",
  // Dynamic effects
  "rounding",
  "border_size",
  "rounding_power",
  "scroll_mouse",
  "scroll_touchpad",
  "animation",
  "idle_inhibit",
  "opacity",
  "tag",
  "max_size",
  "min_size",
  "border_color",
  "persistent_size",
  "allows_input",
  "dim_around",
  "decorate",
  "focus_on_activate",
  "keep_aspect_ratio",
  "nearest_neighbor",
  "no_anim",
  "no_blur",
  "no_dim",
  "no_focus",
  "no_follow_mouse",
  "no_max_size",
  "no_shadow",
  "no_glow",
  "no_wobble",
  "no_shortcuts_inhibit",
  "opaque",
  "force_rgbx",
  "sync_fullscreen",
  "immediate",
  "xray",
  "render_unfocused",
  "no_screen_share",
  "no_vrr",
  "no_auto_hdr",
  "stay_focused",
  "confine_pointer",
  "no_xdg_drags",
  "tonemap",
] as const;

export type WindowruleMatchProperty = typeof WINDOWRULE_MATCH_PROPERTIES[number];
export type WindowruleEffectProperty = typeof WINDOWRULE_EFFECT_PROPERTIES[number];
