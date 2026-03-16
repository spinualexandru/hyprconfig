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

// Match properties for layerrule v2
export const LAYERRULE_MATCH_PROPERTIES = [
  "namespace",
  "address",
  "class",
  "title",
  "monitor",
  "layer",
] as const;

// Effect properties for layerrule v2 (synced with hyprlang-rs 0.5.0)
export const LAYERRULE_EFFECT_PROPERTIES = [
  "blur",
  "blur_popups",
  "ignorealpha",
  "ignore_alpha",
  "ignorezero",
  "animation",
  "noanim",
  "no_anim",
  "xray",
  "dim_around",
  "order",
  "above_lock",
  "no_screen_share",
  "noscreenshare",
] as const;

export type LayerruleMatchProperty = typeof LAYERRULE_MATCH_PROPERTIES[number];
export type LayerruleEffectProperty = typeof LAYERRULE_EFFECT_PROPERTIES[number];
