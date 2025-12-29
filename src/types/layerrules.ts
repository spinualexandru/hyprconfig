export interface LayerruleProperty {
  key: string;
  value: string;
  property_type: string;
}

export interface Layerrule {
  name: string;
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

// Effect properties for layerrule v2
export const LAYERRULE_EFFECT_PROPERTIES = [
  "blur",
  "blurpopups",
  "ignorealpha",
  "ignorezero",
  "dimaround",
  "xray",
  "animation",
  "order",
  "abovelock",
  "noscreenshot",
  "noscreenshare",
  "noshadow",
] as const;

export type LayerruleMatchProperty = typeof LAYERRULE_MATCH_PROPERTIES[number];
export type LayerruleEffectProperty = typeof LAYERRULE_EFFECT_PROPERTIES[number];
