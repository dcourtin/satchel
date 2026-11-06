//! Reads the active KDE colour scheme (as copied into `kdeglobals`) and derives Satchel's palette
//! from it, so that Satchel follows « Configuration du système » like every KDE app.
//! Single source of truth: the colour scheme + accent colour. Catppuccin Mocha is only the fallback.

use std::collections::HashMap;
use std::path::Path;

pub type Rgb = (u8, u8, u8);

/// Satchel's palette slots (names kept from the Catppuccin origin of the design tokens).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Palette {
    pub crust: Rgb,
    pub mantle: Rgb,
    pub base: Rgb,
    pub surface0: Rgb,
    pub surface1: Rgb,
    pub surface2: Rgb,
    pub overlay0: Rgb,
    pub overlay1: Rgb,
    pub subtext0: Rgb,
    pub text: Rgb,
    /// Accent (KDE accent colour, else the scheme's focus colour).
    pub accent: Rgb,
    /// Selection background of the scheme.
    pub selection: Rgb,
    pub selection_text: Rgb,
    pub negative: Rgb,
    pub neutral: Rgb,
    pub positive: Rgb,
    pub warm: Rgb,
}

impl Default for Palette {
    /// Catppuccin Mocha — used when kdeglobals is missing or incomplete.
    fn default() -> Self {
        Palette {
            crust: (17, 17, 27),
            mantle: (24, 24, 37),
            base: (30, 30, 46),
            surface0: (49, 50, 68),
            surface1: (69, 71, 90),
            surface2: (88, 91, 112),
            overlay0: (108, 112, 134),
            overlay1: (127, 132, 156),
            subtext0: (166, 173, 200),
            text: (205, 214, 244),
            accent: (203, 166, 247),
            selection: (137, 180, 250),
            selection_text: (17, 17, 27),
            negative: (243, 139, 168),
            neutral: (249, 226, 175),
            positive: (166, 227, 161),
            warm: (250, 179, 135),
        }
    }
}

/// `a` mixed towards `b` by `t` (0 = a, 1 = b).
pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    (m(a.0, b.0), m(a.1, b.1), m(a.2, b.2))
}

fn parse_rgb(v: &str) -> Option<Rgb> {
    let mut it = v.split(',').map(|p| p.trim().parse::<u8>());
    let (r, g, b) = (it.next()?.ok()?, it.next()?.ok()?, it.next()?.ok()?);
    Some((r, g, b))
}

/// INI-ish groups → keys → values (KConfig syntax, enough for colours).
fn parse_groups(text: &str) -> HashMap<String, HashMap<String, String>> {
    let mut groups: HashMap<String, HashMap<String, String>> = HashMap::new();
    let mut current = String::new();
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') && line.ends_with(']') {
            current = line[1..line.len() - 1].to_string();
        } else if let Some((k, v)) = line.split_once('=') {
            groups.entry(current.clone()).or_default().insert(k.trim().to_string(), v.trim().to_string());
        }
    }
    groups
}

/// Palette from kdeglobals text. Missing values fall back to the default palette.
pub fn palette_from_kdeglobals(text: &str) -> Palette {
    let g = parse_groups(text);
    let d = Palette::default();
    if !g.contains_key("Colors:View") {
        return d; // no KDE colour scheme applied: keep the reference palette exactly
    }
    let get = |group: &str, key: &str| g.get(group).and_then(|m| m.get(key)).and_then(|v| parse_rgb(v));
    let base = get("Colors:View", "BackgroundNormal").unwrap_or(d.base);
    let mantle = get("Colors:Window", "BackgroundNormal").unwrap_or(d.mantle);
    let text_c = get("Colors:View", "ForegroundNormal").unwrap_or(d.text);
    let surface0 = get("Colors:Button", "BackgroundNormal").unwrap_or(d.surface0);
    let accent = get("General", "AccentColor").or_else(|| get("Colors:View", "DecorationFocus")).unwrap_or(d.accent);
    let negative = get("Colors:View", "ForegroundNegative").unwrap_or(d.negative);
    let neutral = get("Colors:View", "ForegroundNeutral").unwrap_or(d.neutral);
    Palette {
        crust: mix(mantle, (0, 0, 0), 0.3),
        mantle,
        base,
        surface0,
        surface1: mix(surface0, text_c, 0.12),
        surface2: mix(surface0, text_c, 0.24),
        overlay0: mix(text_c, base, 0.55),
        overlay1: mix(text_c, base, 0.45),
        subtext0: get("Colors:View", "ForegroundInactive").unwrap_or(d.subtext0),
        text: text_c,
        accent,
        selection: get("Colors:Selection", "BackgroundNormal").unwrap_or(d.selection),
        selection_text: get("Colors:Selection", "ForegroundNormal").unwrap_or(d.selection_text),
        negative,
        neutral,
        positive: get("Colors:View", "ForegroundPositive").unwrap_or(d.positive),
        warm: mix(negative, neutral, 0.5),
    }
}

pub fn read_palette(home: &Path) -> Palette {
    std::fs::read_to_string(home.join(".config/kdeglobals")).map(|t| palette_from_kdeglobals(&t)).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_scheme_and_accent() {
        let kg = "[Colors:View]\nBackgroundNormal=1,2,3\nForegroundNormal=250,250,250\nForegroundNegative=200,0,0\n\
                  [Colors:Window]\nBackgroundNormal=10,10,10\n[Colors:Selection]\nBackgroundNormal=0,100,200\n\
                  [General]\nAccentColor=255,0,255\n";
        let p = palette_from_kdeglobals(kg);
        assert_eq!(p.base, (1, 2, 3));
        assert_eq!(p.mantle, (10, 10, 10));
        assert_eq!(p.text, (250, 250, 250));
        assert_eq!(p.accent, (255, 0, 255));
        assert_eq!(p.selection, (0, 100, 200));
        assert_eq!(p.negative, (200, 0, 0));
        assert_eq!(p.crust, (7, 7, 7));
        // Missing keys keep the fallback.
        assert_eq!(p.positive, Palette::default().positive);
    }

    #[test]
    fn accent_falls_back_to_focus_colour() {
        let p = palette_from_kdeglobals("[Colors:View]\nDecorationFocus=1,1,1\n");
        assert_eq!(p.accent, (1, 1, 1));
        assert_eq!(palette_from_kdeglobals(""), Palette::default());
    }

    #[test]
    fn mixing() {
        assert_eq!(mix((0, 0, 0), (255, 255, 255), 0.5), (128, 128, 128));
        assert_eq!(mix((10, 20, 30), (10, 20, 30), 0.7), (10, 20, 30));
    }
}
