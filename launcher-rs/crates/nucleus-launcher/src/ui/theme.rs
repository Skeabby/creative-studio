//! Design tokens, the Nucleus dark + light themes and the Windows font setup
//! (SPEC: INK/SURFACE/TEXT/ACCENT/CYAN/RUN/AMBER).

use std::sync::Arc;

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, TextStyle, Visuals};

/// `#0B0E14` — page background (dark).
pub const INK: Color32 = Color32::from_rgb(0x0B, 0x0E, 0x14);
/// `#151B26` — panels, cards (dark).
pub const SURFACE: Color32 = Color32::from_rgb(0x15, 0x1B, 0x26);
/// `#E7ECF5` — primary text.
pub const TEXT: Color32 = Color32::from_rgb(0xE7, 0xEC, 0xF5);
/// `#6D5EF8` — accent (Open, selection).
pub const ACCENT: Color32 = Color32::from_rgb(0x6D, 0x5E, 0xF8);
/// `#22D3EE` — cyan (tag chips).
pub const CYAN: Color32 = Color32::from_rgb(0x22, 0xD3, 0xEE);
/// `#34D399` — running.
pub const RUN: Color32 = Color32::from_rgb(0x34, 0xD3, 0x99);
/// `#F59E0B` — update available / warnings.
pub const AMBER: Color32 = Color32::from_rgb(0xF5, 0x9E, 0x0B);
/// `#E85A6C` — destructive (Stop).
pub const DANGER: Color32 = Color32::from_rgb(0xE8, 0x5A, 0x6C);
/// `#8A91A0` — secondary text / unavailable.
pub const MUTED: Color32 = Color32::from_rgb(0x8A, 0x91, 0xA0);

/// A resolved palette: pick dark or light, then build the egui visuals.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color32,
    pub surface: Color32,
    pub surface2: Color32,
    pub border: Color32,
    pub text: Color32,
    pub muted: Color32,
    pub selection_bg: Color32,
}

impl Palette {
    pub fn dark() -> Self {
        Palette {
            bg: INK,
            surface: SURFACE,
            surface2: Color32::from_rgb(0x1E, 0x25, 0x33),
            border: Color32::from_rgba_premultiplied(0xFF, 0xFF, 0xFF, 14),
            text: TEXT,
            muted: Color32::from_rgb(0x8A, 0x91, 0xA0),
            selection_bg: ACCENT.gamma_multiply(0.35),
        }
    }

    pub fn light() -> Self {
        Palette {
            bg: Color32::from_rgb(0xEF, 0xF1, 0xF6),
            surface: Color32::from_rgb(0xFF, 0xFF, 0xFF),
            surface2: Color32::from_rgb(0xF1, 0xF3, 0xF8),
            border: Color32::from_rgba_premultiplied(0x0F, 0x15, 0x2B, 22),
            text: Color32::from_rgb(0x10, 0x15, 0x2B),
            muted: Color32::from_rgb(0x5B, 0x64, 0x74),
            selection_bg: ACCENT.gamma_multiply(0.18),
        }
    }
}

/// Segoe UI from `C:\Windows\Fonts`, registered as the `nucleus*` families so
/// the UI looks native on Windows instead of the embedded egui font.
pub fn install_fonts(ctx: &Context) {
    let mut defs = FontDefinitions::default();
    let default_prop = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let default_mono = defs
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    let fonts_dir = std::path::Path::new("C:\\Windows\\Fonts");
    // (our family name, candidate Windows files in priority order)
    let candidates: &[(&str, &[&str])] = &[
        ("nucleus", &["segoeui.ttf"]),
        ("nucleus-bold", &["segoeuib.ttf"]),
        ("nucleus-light", &["segoeuil.ttf"]),
        ("nucleus-semibold", &["seguisb.ttf", "segoeuib.ttf"]),
        ("nucleus-mono", &["consola.ttf"]),
        ("nucleus-emoji", &["seguiemj.ttf"]),
        ("nucleus-symbol", &["seguisym.ttf"]),
    ];

    let mut loaded: Vec<&str> = Vec::new();
    for &(family, files) in candidates {
        for file in files {
            if let Ok(bytes) = std::fs::read(fonts_dir.join(file)) {
                defs.font_data
                    .insert(family.to_string(), Arc::new(FontData::from_owned(bytes)));
                loaded.push(family);
                break;
            }
        }
    }

    // Faces used as fallbacks for every text family, in order.
    let text_faces: Vec<String> = ["nucleus", "nucleus-emoji", "nucleus-symbol"]
        .iter()
        .filter(|family| loaded.contains(&**family))
        .map(|family| family.to_string())
        .collect();

    // Register every family a view may reference; never leave one empty so
    // layout cannot panic when a font file is absent.
    for &(family, _files) in candidates {
        let mut list: Vec<String> = Vec::new();
        if loaded.contains(&family) {
            list.push(family.to_string());
        }
        if family == "nucleus-mono" {
            list.extend(default_mono.iter().cloned());
        } else {
            for face in &text_faces {
                if !list.contains(face) {
                    list.push(face.clone());
                }
            }
            list.extend(default_prop.iter().cloned());
        }
        list.dedup();
        if list.is_empty() {
            list = default_prop.clone();
        }
        defs.families.insert(FontFamily::Name(family.into()), list);
    }

    // Standard families: prefer Segoe UI, keep everything else as fallback.
    let mut prop: Vec<String> = text_faces.clone();
    for font in &default_prop {
        if !prop.contains(font) {
            prop.push(font.clone());
        }
    }
    if prop.is_empty() {
        prop = default_prop;
    }
    defs.families.insert(FontFamily::Proportional, prop);

    let mut mono: Vec<String> = Vec::new();
    if loaded.contains(&"nucleus-mono") {
        mono.push("nucleus-mono".to_string());
    }
    for font in &default_mono {
        if !mono.contains(font) {
            mono.push(font.clone());
        }
    }
    if mono.is_empty() {
        mono = default_mono;
    }
    defs.families.insert(FontFamily::Monospace, mono);

    ctx.set_fonts(defs);
}

/// Reference font ids used across the views.
pub mod fonts {
    use egui::{FontFamily, FontId};

    pub fn body(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("nucleus".into()))
    }

    pub fn bold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("nucleus-bold".into()))
    }

    pub fn semibold(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("nucleus-semibold".into()))
    }

    pub fn mono(size: f32) -> FontId {
        FontId::new(size, FontFamily::Name("nucleus-mono".into()))
    }
}

/// Apply a palette to the egui context (dark or light).
pub fn configure_theme(ctx: &Context, palette: &Palette) {
    let mut visuals = Visuals::dark();
    if palette.bg == Palette::light().bg {
        visuals = Visuals::light();
    }

    visuals.panel_fill = palette.bg;
    visuals.window_fill = palette.surface;
    visuals.extreme_bg_color = palette.bg;
    visuals.faint_bg_color = palette.surface2;
    visuals.override_text_color = Some(palette.text);
    visuals.selection.bg_fill = palette.selection_bg;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.inactive.bg_fill = palette.surface2;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, palette.text);
    visuals.widgets.hovered.bg_fill = lighten(palette.surface2, 0.05);
    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, palette.text);
    visuals.hyperlink_color = ACCENT;
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(10.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(0);
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    style
        .text_styles
        .insert(TextStyle::Heading, fonts::bold(22.0));
    style.text_styles.insert(TextStyle::Body, fonts::body(13.5));
    style
        .text_styles
        .insert(TextStyle::Button, fonts::semibold(13.0));
    style
        .text_styles
        .insert(TextStyle::Small, fonts::body(11.0));
    ctx.set_style(style);
}

/// Lighten a colour a little (mixing toward white when dark, toward black when light).
pub fn lighten(color: Color32, amount: f32) -> Color32 {
    let toward = if color.r() as u16 + color.g() as u16 + color.b() as u16 > 384 {
        Color32::BLACK
    } else {
        Color32::WHITE
    };
    Color32::from_rgba_premultiplied(
        lerp(color.r(), toward.r(), amount),
        lerp(color.g(), toward.g(), amount),
        lerp(color.b(), toward.b(), amount),
        255,
    )
}

fn lerp(a: u8, b: u8, t: f32) -> u8 {
    (a as f32 + (b as f32 - a as f32) * t)
        .round()
        .clamp(0.0, 255.0) as u8
}

/// The card border colour for a hover state.
pub fn hover_border(p: &Palette) -> Color32 {
    ACCENT.gamma_multiply(if p.bg == Palette::dark().bg { 0.8 } else { 0.6 })
}

/// Approximate parse of `#RRGGBB` (roadmap tile colours). Returns None on
/// garbage so the caller can fall back to a default.
pub fn hex_color(hex: &str) -> Option<Color32> {
    let hex = hex.trim().strip_prefix('#')?;
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rrggbb() {
        assert_eq!(hex_color("#6D5EF8"), Some(ACCENT));
        assert_eq!(hex_color("#22D3EE"), Some(CYAN));
        assert_eq!(hex_color("no"), None);
        assert_eq!(hex_color("#fff"), None);
        assert_eq!(hex_color(""), None);
    }

    #[test]
    fn palettes_differ_but_share_status_colours() {
        let dark = Palette::dark();
        let light = Palette::light();
        assert_ne!(dark.bg, light.bg);
        assert_eq!(dark.text, TEXT);
    }
}
