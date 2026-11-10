//! The app's colours. `~/.config/hyprconfig/theme.css` (written by the app, and by matugen when
//! its template is set up) holds shadcn-style custom properties for light (`:root`) and dark
//! (`.dark`) appearances. Three of them seed Rune's kit: `--background`, `--foreground` and
//! `--primary`; anything that doesn't parse falls back to the kit's neutral theme.

use rune::Color;
use rune_kit::{Kit, PaletteSeed};

/// The seeds for both appearances. A file with only `:root` uses it for both.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Palettes {
    pub light: Option<Seed>,
    pub dark: Option<Seed>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seed {
    pub bg: Color,
    pub fg: Color,
    pub accent: Color,
    pub danger: Option<Color>,
}

impl Palettes {
    pub fn parse(css: &str) -> Palettes {
        let css = strip_comments(css);
        let light = block(&css, ":root").and_then(seed);
        let dark = block(&css, ".dark").and_then(seed);
        Palettes {
            light,
            dark: dark.or(light),
        }
    }

    /// The kit for this appearance: the theme file's colours, or Rune's neutral theme.
    pub fn kit(&self, dark: bool) -> Kit {
        let seed = if dark { self.dark } else { self.light };
        // A seed whose background disagrees with the appearance (a light matugen scheme in dark
        // mode) would put light text on light layers; the neutral theme is safer there.
        match seed.filter(|s| (s.bg.luminance() < 0.18) == dark) {
            Some(s) => {
                let mut k = Kit::from_palette(PaletteSeed {
                    bg: s.bg,
                    fg: s.fg,
                    accent: s.accent,
                });
                if let Some(d) = s.danger {
                    k.c.danger.icon = d;
                    k.c.danger.fg = d;
                }
                k
            }
            None => Kit::neutral(dark),
        }
    }
}

fn seed(vars: &str) -> Option<Seed> {
    Some(Seed {
        bg: var(vars, "--background")?,
        fg: var(vars, "--foreground")?,
        accent: var(vars, "--primary")?,
        danger: var(vars, "--destructive"),
    })
}

/// The declarations inside the first rule whose selector is exactly `selector`.
fn block<'a>(css: &'a str, selector: &str) -> Option<&'a str> {
    let mut rest = css;
    while let Some(open) = rest.find('{') {
        let head = &rest[..open];
        let close = open + rest[open..].find('}')?;
        if head
            .trim()
            .rsplit(['}', ';'])
            .next()
            .is_some_and(|h| h.trim() == selector)
        {
            return Some(&rest[open + 1..close]);
        }
        rest = &rest[close + 1..];
    }
    None
}

fn strip_comments(s: &str) -> String {
    let mut out = String::new();
    let mut rest = s;
    while let Some(i) = rest.find("/*") {
        out.push_str(&rest[..i]);
        match rest[i..].find("*/") {
            Some(j) => rest = &rest[i + j + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// `--name: value;` within a block, as a colour.
fn var(vars: &str, name: &str) -> Option<Color> {
    vars.split(';').find_map(|decl| {
        let (k, v) = decl.split_once(':')?;
        (k.trim() == name).then(|| color(v.trim())).flatten()
    })
}

/// `#rgb`, `#rrggbb`, `#rrggbbaa`, `hsl(h s% l%)` (commas optional) or `rgb(r g b)`.
pub fn color(v: &str) -> Option<Color> {
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

#[cfg(test)]
mod tests {
    use super::*;

    const CSS: &str = r#"/* header { not a rule } */
:root {
  --background: hsl(0 0% 100%);
  --foreground: hsl(240 10% 3.9%);
  --primary: #3355ff;
}
.dark {
  /* Tokyo Night */
  --background: hsl(235 16% 13%);
  --foreground: hsl(230 70% 85%);
  --primary: hsl(215 90% 73%);
  --destructive: rgb(247 118 142);
}"#;

    #[test]
    fn reads_both_appearances() {
        let p = Palettes::parse(CSS);
        let light = p.light.unwrap();
        let dark = p.dark.unwrap();
        assert_eq!(light.bg, rune::color::hex("#ffffff").unwrap());
        assert_eq!(light.accent, rune::color::hex("#3355ff").unwrap());
        assert!(light.danger.is_none());
        assert!(dark.bg.luminance() < 0.05);
        assert!(dark.danger.is_some());
    }

    #[test]
    fn root_only_serves_both() {
        let p = Palettes::parse(
            ":root { --background: #101010; --foreground: #eeeeee; --primary: #88aaff; }",
        );
        assert_eq!(p.light, p.dark);
    }

    #[test]
    fn hsl_matches_css() {
        // hsl(215 90% 73%) is #7cb0f8 in a browser.
        assert_eq!(color("hsl(215 90% 73%)"), rune::color::hex("#7cb0f8"));
        assert_eq!(color("hsl(0, 0%, 100%)"), rune::color::hex("#ffffff"));
    }

    #[test]
    fn garbage_falls_back() {
        assert_eq!(Palettes::parse("not css"), Palettes::default());
        assert!(color("var(--x)").is_none());
    }
}
