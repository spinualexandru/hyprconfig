export interface WindowruleProperty {
  key: string;
  value: string;
  property_type: string;
}

export interface Windowrule {
  name: string;
  match_properties: WindowruleProperty[];
  effect_properties: WindowruleProperty[];
}

// Match properties for windowrule v3
export const WINDOWRULE_MATCH_PROPERTIES = [
  "class",
  "title",
  "initialclass",
  "initialtitle",
  "tag",
  "floating",
  "fullscreen",
  "pinned",
  "focus",
  "fullscreenstate",
  "workspace",
  "onworkspace",
  "xwayland",
  "pgrp",
  "pid",
  "activeworkspace",
  "activeactivegroup",
  "monitordesc",
  "monitormake",
  "monitormodel",
  "monitorserial",
  "active",
  "gpuindex",
  "mapped",
] as const;

// Effect properties for windowrule v3
export const WINDOWRULE_EFFECT_PROPERTIES = [
  "float",
  "tile",
  "fullscreen",
  "maximize",
  "move",
  "size",
  "minsize",
  "maxsize",
  "center",
  "pseudo",
  "monitor",
  "workspace",
  "animation",
  "rounding",
  "shadow",
  "decorate",
  "noblur",
  "nodim",
  "nofocus",
  "noinitialfocus",
  "forceinput",
  "windowdance",
  "pin",
  "stayfocused",
  "suppressevent",
  "bordercolor",
  "bordersize",
  "dimaround",
  "focusonactivate",
  "group",
  "idleinhibit",
  "immediate",
  "keepaspectratio",
  "xray",
  "roundingpower",
  "nearestneighbor",
  "opacity",
  "scrollmouse",
  "scrolltouchpad",
  "syncfullscreen",
  "tag",
  "content",
  "prop",
  "forcergbx",
  "fullscreenstate",
  "allowsmovement",
  "floatoffset",
  "source",
  "plugin:*",
] as const;

export type WindowruleMatchProperty = typeof WINDOWRULE_MATCH_PROPERTIES[number];
export type WindowruleEffectProperty = typeof WINDOWRULE_EFFECT_PROPERTIES[number];
