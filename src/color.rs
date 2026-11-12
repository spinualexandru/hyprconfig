//! Colours as they appear in config values: finding one in a value (`rgb(e2e2e9)`,
//! `rgba(226, 226, 233, 0.8)`, `0xcce2e2e9`, `#e2e2e9`, the first stop of a gradient …),
//! editing it in HSV and writing it back in the form it was found in.

use std::ops::Range;

use rune::Color;
use rune::color::{srgb_decode, srgb_encode};

/// A colour in sRGB, each channel and alpha in `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rgba {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

/// How a colour was written, so an edit writes it back the same way.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Hyprland's `rgb(rrggbb)` / `rgba(rrggbbaa)`.
    Hex { alpha: bool },
    /// `rgb(r, g, b)` / `rgba(r, g, b, a)`.
    Dec { alpha: bool },
    /// Hyprland's legacy `0xAARRGGBB`.
    Argb,
    /// `#rrggbb` / `#rrggbbaa`.
    Hash { alpha: bool },
    /// Anything else readable (`hsl(...)`, `#rgb`): written back as Hyprland's `rgb(...)`.
    Other,
}

/// A colour inside a value: what it is, how it's written and where.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub rgba: Rgba,
    pub format: Format,
    /// Byte range of the colour in the value.
    pub span: Range<usize>,
}

/// The colour a value spells, if any. A gradient (`rgba(…) rgba(…) 45deg`) gives its first.
pub fn find(v: &str) -> Option<Found> {
    let b = v.as_bytes();
    let (mut s, mut e) = (v.len() - v.trim_start().len(), v.trim_end().len());
    while s < e && matches!(b[s], b'"' | b'\'') {
        s += 1;
    }
    while e > s && matches!(b[e - 1], b'"' | b'\'') {
        e -= 1;
    }
    let body = &v[s..e];
    let found = |len: usize| {
        parse(&body[..len]).map(|(rgba, format)| Found {
            rgba,
            format,
            span: s..s + len,
        })
    };
    if let Some(f) = found(body.len()) {
        return Some(f);
    }
    // A gradient's first stop: up to its `)`, or up to the first space.
    let first_space = body.find(char::is_whitespace)?;
    let paren = body[..first_space]
        .rfind(')')
        .map(|i| i + 1)
        .unwrap_or(first_space);
    found(paren)
}

/// The colour to draw for a value, if it holds one.
pub fn value_color(v: &str) -> Option<Color> {
    find(v).map(|f| f.rgba.color())
}

/// `v` with the colour `found` in it replaced by `c`, written the same way.
pub fn replace(v: &str, found: &Found, c: Rgba) -> String {
    let mut out = String::with_capacity(v.len() + 8);
    out.push_str(&v[..found.span.start]);
    out.push_str(&c.format(found.format));
    out.push_str(&v[found.span.end..]);
    out
}

/// `rrggbb` or `rrggbbaa`, with or without a leading `#`.
pub fn parse_hex(s: &str) -> Option<Rgba> {
    let h = s.trim();
    let h = h.strip_prefix('#').unwrap_or(h);
    if !matches!(h.len(), 6 | 8) || !h.bytes().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).ok().map(unit);
    Some(Rgba {
        r: byte(0)?,
        g: byte(2)?,
        b: byte(4)?,
        a: if h.len() == 8 { byte(6)? } else { 1.0 },
    })
}

fn parse(v: &str) -> Option<(Rgba, Format)> {
    if let Some(h) = v.strip_prefix("0x").or_else(|| v.strip_prefix("0X")) {
        if h.len() != 8 || !h.is_ascii() {
            return None;
        }
        return parse_hex(&format!("{}{}", &h[2..], &h[..2])).map(|c| (c, Format::Argb));
    }
    if let Some(h) = v.strip_prefix('#')
        && let Some(c) = parse_hex(h)
    {
        return Some((
            c,
            Format::Hash {
                alpha: h.len() == 8,
            },
        ));
    }
    if let Some(args) = v
        .strip_prefix("rgba(")
        .or_else(|| v.strip_prefix("rgb("))
        .and_then(|a| a.strip_suffix(')'))
    {
        let args = args.trim();
        if !args.starts_with('#')
            && let Some(c) = parse_hex(args)
        {
            return Some((
                c,
                Format::Hex {
                    alpha: args.len() == 8,
                },
            ));
        }
        if let Ok(n) = args
            .split(',')
            .map(|a| a.trim().parse::<f32>())
            .collect::<Result<Vec<f32>, _>>()
            && matches!(n.len(), 3 | 4)
        {
            let c = Rgba {
                r: n[0].clamp(0.0, 255.0) / 255.0,
                g: n[1].clamp(0.0, 255.0) / 255.0,
                b: n[2].clamp(0.0, 255.0) / 255.0,
                a: n.get(3).map_or(1.0, |a| a.clamp(0.0, 1.0)),
            };
            return Some((
                c,
                Format::Dec {
                    alpha: n.len() == 4,
                },
            ));
        }
    }
    let c = parse_css(v)?;
    Some((
        Rgba {
            r: srgb_encode(c.r),
            g: srgb_encode(c.g),
            b: srgb_encode(c.b),
            a: c.a,
        },
        Format::Other,
    ))
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa`, `hsl(h s% l%)` (commas optional) or `rgb(r g b)`.
pub fn parse_css(v: &str) -> Option<Color> {
    if v.starts_with('#') {
        return rune::color::hex(v);
    }
    let (func, args) = v.split_once('(')?;
    let args: Vec<f32> = args
        .trim_end_matches(')')
        .split(|c: char| c == ',' || c == '/' || c.is_whitespace())
        .filter(|a| !a.is_empty())
        .map(|a| {
            a.trim_end_matches(['%'])
                .trim_end_matches("deg")
                .parse::<f32>()
        })
        .collect::<Result<_, _>>()
        .ok()?;
    let [a, b, c] = [*args.first()?, *args.get(1)?, *args.get(2)?];
    let (r, g, bl) = match func.trim() {
        "hsl" | "hsla" => hsl_to_rgb(a, b / 100.0, c / 100.0),
        "rgb" | "rgba" => (a / 255.0, b / 255.0, c / 255.0),
        _ => return None,
    };
    let byte = |x: f32| (x.clamp(0.0, 1.0) * 255.0).round() as u8;
    rune::color::hex(&format!("#{:02x}{:02x}{:02x}", byte(r), byte(g), byte(bl)))
}

fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (f32, f32, f32) {
    let k = |n: f32| (n + h / 30.0).rem_euclid(12.0);
    let a = s * l.min(1.0 - l);
    let f = |n: f32| l - a * (k(n) - 3.0).min(9.0 - k(n)).clamp(-1.0, 1.0);
    (f(0.0), f(8.0), f(4.0))
}

fn unit(b: u8) -> f32 {
    b as f32 / 255.0
}

fn byte(x: f32) -> u8 {
    (x.clamp(0.0, 1.0) * 255.0).round() as u8
}

impl Rgba {
    /// For drawing (linear light).
    pub fn color(self) -> Color {
        Color {
            r: srgb_decode(self.r),
            g: srgb_decode(self.g),
            b: srgb_decode(self.b),
            a: self.a,
        }
    }

    pub fn opaque(self) -> Rgba {
        Rgba { a: 1.0, ..self }
    }

    /// `rrggbb`, or `rrggbbaa` when `alpha`.
    pub fn hex(self, alpha: bool) -> String {
        let mut s = format!(
            "{:02x}{:02x}{:02x}",
            byte(self.r),
            byte(self.g),
            byte(self.b)
        );
        if alpha {
            s.push_str(&format!("{:02x}", byte(self.a)));
        }
        s
    }

    fn translucent(self) -> bool {
        byte(self.a) < 255
    }

    pub fn format(self, f: Format) -> String {
        let alpha = |src: bool| src || self.translucent();
        match f {
            Format::Hex { alpha: a } | Format::Dec { alpha: a } | Format::Hash { alpha: a }
                if alpha(a) =>
            {
                match f {
                    Format::Hex { .. } => format!("rgba({})", self.hex(true)),
                    Format::Hash { .. } => format!("#{}", self.hex(true)),
                    _ => {
                        let a = format!("{:.2}", self.a);
                        let a = a.trim_end_matches('0').trim_end_matches('.');
                        let a = if a.is_empty() { "0" } else { a };
                        format!(
                            "rgba({}, {}, {}, {a})",
                            byte(self.r),
                            byte(self.g),
                            byte(self.b)
                        )
                    }
                }
            }
            Format::Dec { .. } => {
                format!("rgb({}, {}, {})", byte(self.r), byte(self.g), byte(self.b))
            }
            Format::Hash { .. } => format!("#{}", self.hex(false)),
            Format::Argb => format!("0x{:02x}{}", byte(self.a), self.hex(false)),
            Format::Hex { .. } | Format::Other => {
                if self.translucent() {
                    format!("rgba({})", self.hex(true))
                } else {
                    format!("rgb({})", self.hex(false))
                }
            }
        }
    }

    /// Hue in degrees, saturation, value and alpha, each but hue in `0..=1`.
    pub fn hsva(self) -> [f32; 4] {
        let (r, g, b) = (self.r, self.g, self.b);
        let max = r.max(g).max(b);
        let d = max - r.min(g).min(b);
        let h = if d == 0.0 {
            0.0
        } else if max == r {
            60.0 * ((g - b) / d).rem_euclid(6.0)
        } else if max == g {
            60.0 * ((b - r) / d + 2.0)
        } else {
            60.0 * ((r - g) / d + 4.0)
        };
        let s = if max == 0.0 { 0.0 } else { d / max };
        [h, s, max, self.a]
    }

    pub fn from_hsva([h, s, v, a]: [f32; 4]) -> Rgba {
        let f = |n: f32| {
            let k = (n + h / 60.0).rem_euclid(6.0);
            v - v * s * k.min(4.0 - k).clamp(0.0, 1.0)
        };
        Rgba {
            r: f(5.0),
            g: f(3.0),
            b: f(1.0),
            a,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn color(v: &str) -> Option<String> {
        find(v).map(|f| f.rgba.hex(true))
    }

    #[test]
    fn finds_colours() {
        assert_eq!(color("rgb(e2e2e9)").as_deref(), Some("e2e2e9ff"));
        assert_eq!(color("rgba(e2e2e9cc)").as_deref(), Some("e2e2e9cc"));
        assert_eq!(color("\"rgb(E2E2E9)\"").as_deref(), Some("e2e2e9ff"));
        assert_eq!(color("0xcce2e2e9").as_deref(), Some("e2e2e9cc"));
        assert_eq!(color("rgb(226, 226, 233)").as_deref(), Some("e2e2e9ff"));
        assert_eq!(
            color("rgba(226, 226, 233, 0.8)").as_deref(),
            Some("e2e2e9cc")
        );
        assert_eq!(color("#e2e2e9").as_deref(), Some("e2e2e9ff"));
        assert_eq!(color("hsl(0, 0%, 100%)").as_deref(), Some("ffffffff"));
        assert_eq!(
            color("rgba(33ccffee) rgba(00ff99ee) 45deg").as_deref(),
            Some("33ccffee")
        );
        assert_eq!(
            color("0xee33ccff 0xee00ff99 45deg").as_deref(),
            Some("33ccffee")
        );
        assert_eq!(value_color("rgb(e2e2e9)"), rune::color::hex("#e2e2e9"));
    }

    #[test]
    fn ignores_non_colours() {
        for v in [
            "kitty",
            "5",
            "SUPER",
            "",
            "rgb(xyz)",
            "0x12",
            "wofi --show drun",
        ] {
            assert_eq!(find(v), None, "{v}");
        }
    }

    #[test]
    fn writes_back_in_the_same_form() {
        let red = Rgba {
            r: 1.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let half = Rgba { a: 0.5, ..red };
        let w = |v: &str, c| replace(v, &find(v).unwrap(), c);
        assert_eq!(w("rgb(e2e2e9)", red), "rgb(ff0000)");
        assert_eq!(w("rgb(e2e2e9)", half), "rgba(ff000080)");
        assert_eq!(w("rgba(e2e2e9ff)", red), "rgba(ff0000ff)");
        assert_eq!(w("\"rgb(e2e2e9)\"", red), "\"rgb(ff0000)\"");
        assert_eq!(w("rgb(1, 2, 3)", red), "rgb(255, 0, 0)");
        assert_eq!(w("rgb(1, 2, 3)", half), "rgba(255, 0, 0, 0.5)");
        assert_eq!(w("0xffe2e2e9", half), "0x80ff0000");
        assert_eq!(w("#e2e2e9", red), "#ff0000");
        assert_eq!(w("hsl(0, 0%, 100%)", red), "rgb(ff0000)");
        assert_eq!(
            w("rgba(33ccffee) rgba(00ff99ee) 45deg", red),
            "rgba(ff0000ff) rgba(00ff99ee) 45deg"
        );
    }

    #[test]
    fn hsl_matches_css() {
        // hsl(215 90% 73%) is #7cb0f8 in a browser.
        assert_eq!(parse_css("hsl(215 90% 73%)"), rune::color::hex("#7cb0f8"));
        assert_eq!(parse_css("hsl(0, 0%, 100%)"), rune::color::hex("#ffffff"));
        assert!(parse_css("var(--x)").is_none());
    }

    #[test]
    fn hsv_round_trips() {
        for hex in ["e2e2e9", "33ccff", "000000", "ffffff", "ff0000", "7f3a10"] {
            let c = parse_hex(hex).unwrap();
            assert_eq!(Rgba::from_hsva(c.hsva()).hex(false), hex);
        }
        assert_eq!(parse_hex("#00ff9980").unwrap().hex(true), "00ff9980");
        assert_eq!(parse_hex("00ff9"), None);
    }
}
