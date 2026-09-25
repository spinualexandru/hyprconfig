export interface LayerruleProperty {
  key: string;
  value: string;
  property_type: string;
}

export interface Layerrule {
  name: string;
  enabled: boolean;
  match_properties: LayerruleProperty[];
  effect_properties: LayerruleProperty[];
}

// Layer rules only match on the surface namespace
export const LAYERRULE_MATCH_PROPERTIES = ["namespace"] as const;

// Effect properties (synced with Hyprland LAYER_RULE_EFFECT_DESCS)
export const LAYERRULE_EFFECT_PROPERTIES = [
  "no_anim",
  "blur",
  "blur_popups",
  "ignore_alpha",
  "dim_around",
  "xray",
  "animation",
  "order",
  "above_lock",
  "no_screen_share",
] as const;

export type LayerruleMatchProperty = typeof LAYERRULE_MATCH_PROPERTIES[number];
export type LayerruleEffectProperty = typeof LAYERRULE_EFFECT_PROPERTIES[number];
