//! Design tokens, themes and embedded fonts for the Nucleus launcher.
//!
//! The look is deliberately flat and editorial: **hard edges** (every corner
//! radius is zero), **hairline rules** instead of shadows, a single **brand
//! violet** accent reserved for the launcher's own chrome (active cells, the
//! view switch, primary actions), and three typefaces — Archivo for display,
//! Inter for body text and Geist Mono for the uppercase "HUD" labels.
//!
//! Two motifs say *Nucleus* rather than any other launcher: the **orbit** — a
//! ring and a satellite drawn around every app icon, echoing the brand mark —
//! and **corner brackets** on the frame and on the hovered card.
//!
//! Fonts are embedded under the SIL Open Font License; their licence texts
//! ship next to the binaries in `assets/fonts/OFL-*.txt`.

use std::sync::Arc;

use egui::{Color32, Context, FontData, FontDefinitions, FontFamily, TextStyle, Visuals};

/// Height of every control: buttons, inputs, filter cells, glyph buttons.
pub const CONTROL_H: f32 = 34.0;
/// Height of the top chrome bar.
pub const HEADER_H: f32 = 48.0;
/// Height of the bottom status bar.
pub const FOOTER_H: f32 = 34.0;
/// Size of the monospace "HUD" labels.
pub const HUD_SIZE: f32 = 11.0;

// --- Raw palette ------------------------------------------------------------

/// `#121316` — page background (dark).
pub const BG_DARK: Color32 = rgb(0x121316);
/// `#060607` — wells, badges, input backgrounds (dark).
pub const SUNKEN_DARK: Color32 = rgb(0x06_06_07);
/// `#F2F1EE` — warm off-white ink (dark theme).
pub const INK_DARK: Color32 = rgb(0xF2_F1_EE);
/// `#F2F1EE` — page background (light).
pub const BG_LIGHT: Color32 = rgb(0xF2_F1_EE);
/// `#E9E8E4` — wells, badges, input backgrounds (light).
pub const SUNKEN_LIGHT: Color32 = rgb(0xE9_E8_E4);
/// `#101014` — ink (light theme).
pub const INK_LIGHT: Color32 = rgb(0x10_10_14);
/// `#6D5EF8` — brand violet: the Nucleus accent, both themes. It is reserved
/// for the launcher's own chrome, so no app colour may reuse it.
pub const ACCENT: Color32 = rgb(0x6D_5E_F8);

const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb(
        ((hex >> 16) & 0xFF) as u8,
        ((hex >> 8) & 0xFF) as u8,
        (hex & 0xFF) as u8,
    )
}

/// The same colour at a given alpha (used for muted/faint/line inks).
/// The same colour at a different opacity.
pub fn alpha(color: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), a)
}

/// Semantic tone of a status, mapped to a token colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Good,
    Warn,
    Info,
    Neutral,
    Faint,
}

/// A fully resolved theme. Construct with [`Tokens::dark`] / [`Tokens::light`]
/// and pass by value (it is `Copy`).
#[derive(Debug, Clone, Copy)]
pub struct Tokens {
    /// Whether this is the dark variant (used for the odd branch).
    pub dark: bool,
    /// Page background — also the surface every card is painted on.
    pub bg: Color32,
    /// Recessed wells: badges, search field, hovers.
    pub sunken: Color32,
    /// Body text.
    pub ink: Color32,
    /// Headings and emphasised text.
    pub ink_strong: Color32,
    /// Secondary text.
    pub muted: Color32,
    /// Tertiary text and labels.
    pub faint: Color32,
    /// Hairline rules.
    pub line: Color32,
    /// Stronger hairline for outlined controls.
    pub line_strong: Color32,
    /// Brand violet surface: active cells, the active view switch, primary
    /// actions, hovered glyph buttons.
    pub accent: Color32,
    /// Brand violet as text/icon ink.
    pub accent_ink: Color32,
    /// Ink painted on top of [`Tokens::accent`].
    pub on_accent: Color32,
    pub good: Color32,
    pub warn: Color32,
    pub bad: Color32,
}

impl Tokens {
    pub fn dark() -> Self {
        let ink = INK_DARK;
        Tokens {
            dark: true,
            bg: BG_DARK,
            sunken: SUNKEN_DARK,
            ink,
            ink_strong: Color32::WHITE,
            muted: alpha(ink, 153),
            faint: alpha(ink, 97),
            line: alpha(ink, 38),
            line_strong: alpha(ink, 102),
            accent: ACCENT,
            accent_ink: rgb(0xA7_9B_FF),
            on_accent: Color32::WHITE,
            good: rgb(0x3F_D6_B8),
            warn: rgb(0xFF_C1_4D),
            bad: rgb(0xFF_62_59),
        }
    }

    pub fn light() -> Self {
        let ink = INK_LIGHT;
        Tokens {
            dark: false,
            bg: BG_LIGHT,
            sunken: SUNKEN_LIGHT,
            ink,
            ink_strong: Color32::BLACK,
            muted: alpha(ink, 158),
            faint: alpha(ink, 107),
            line: alpha(ink, 41),
            line_strong: alpha(ink, 102),
            accent: ACCENT,
            accent_ink: rgb(0x4A_3A_C8),
            on_accent: Color32::WHITE,
            good: rgb(0x0A_75_63),
            warn: rgb(0x8A_58_00),
            bad: rgb(0xB3_26_1E),
        }
    }

    /// The colour a semantic tone paints in.
    pub fn tone(&self, tone: Tone) -> Color32 {
        match tone {
            Tone::Good => self.good,
            Tone::Warn => self.warn,
            Tone::Info => self.accent_ink,
            Tone::Neutral => self.muted,
            Tone::Faint => self.faint,
        }
    }
}

/// Linear blend from `from` to `to` by `t` (0..=1).
pub fn blend(from: Color32, to: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let mix = |a: u8, b: u8| {
        (a as f32 + (b as f32 - a as f32) * t)
            .round()
            .clamp(0.0, 255.0) as u8
    };
    Color32::from_rgba_unmultiplied(
        mix(from.r(), to.r()),
        mix(from.g(), to.g()),
        mix(from.b(), to.b()),
        mix(from.a(), to.a()),
    )
}

/// Parse `#RRGGBB` or `RRGGBB` (roadmap tile colours); `None` on anything else.
pub fn hex_color(hex: &str) -> Option<Color32> {
    let hex = hex.trim();
    let hex = hex.strip_prefix('#').unwrap_or(hex);
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some(Color32::from_rgb(r, g, b))
}

// --- Fonts ------------------------------------------------------------------

/// The typefaces, embedded so the look does not depend on the host machine.
///
/// * `display` — Archivo (wordmarks, headings)
/// * `body` — Inter (paragraphs, buttons, captions)
/// * `mono` — Geist Mono (the uppercase HUD labels)
pub mod fonts {
    use egui::{FontFamily, FontId};

    fn named(name: &'static str, size: f32) -> FontId {
        FontId::new(size, FontFamily::Name(name.into()))
    }

    pub fn display(size: f32) -> FontId {
        named("nucleus-display", size)
    }

    pub fn body(size: f32) -> FontId {
        named("nucleus-body", size)
    }

    pub fn mono(size: f32) -> FontId {
        named("nucleus-mono", size)
    }

    pub fn mono_bold(size: f32) -> FontId {
        named("nucleus-mono-bold", size)
    }
}

/// Register the embedded faces. Every named family is guaranteed non-empty so
/// layout can never panic on a missing face.
pub fn install_fonts(ctx: &Context) {
    let mut defs = FontDefinitions::default();
    let text_fallback: Vec<String> = defs
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let mono_fallback: Vec<String> = defs
        .families
        .get(&FontFamily::Monospace)
        .cloned()
        .unwrap_or_default();

    // Colour emoji straight from Windows, when present.
    let mut emoji: Vec<String> = Vec::new();
    if let Ok(bytes) = std::fs::read("C:\\Windows\\Fonts\\seguiemj.ttf") {
        defs.font_data.insert(
            "nucleus-emoji".to_string(),
            Arc::new(FontData::from_owned(bytes)),
        );
        emoji.push("nucleus-emoji".to_string());
    }

    let faces: [(&str, &[u8]); 3] = [
        (
            "nucleus-display",
            include_bytes!("../../assets/fonts/Archivo-Variable.ttf"),
        ),
        (
            "nucleus-body",
            include_bytes!("../../assets/fonts/Inter-Variable.ttf"),
        ),
        (
            "nucleus-mono",
            include_bytes!("../../assets/fonts/GeistMono-Variable.ttf"),
        ),
    ];
    for (name, bytes) in faces {
        defs.font_data
            .insert(name.to_string(), Arc::new(FontData::from_static(bytes)));
    }

    let text_chain = |primary: &str| -> Vec<String> {
        let mut chain = vec![primary.to_string()];
        chain.extend(emoji.iter().cloned());
        chain.extend(text_fallback.iter().cloned());
        chain
    };
    defs.families.insert(
        FontFamily::Name("nucleus-body".into()),
        text_chain("nucleus-body"),
    );
    defs.families.insert(
        FontFamily::Name("nucleus-display".into()),
        text_chain("nucleus-display"),
    );
    for family in ["nucleus-mono", "nucleus-mono-bold"] {
        let mut chain = vec!["nucleus-mono".to_string()];
        chain.extend(mono_fallback.iter().cloned());
        defs.families.insert(FontFamily::Name(family.into()), chain);
    }

    let mut proportional = text_chain("nucleus-body");
    proportional.extend(text_fallback.iter().cloned());
    defs.families.insert(FontFamily::Proportional, proportional);

    let mut monospace = vec!["nucleus-mono".to_string()];
    monospace.extend(mono_fallback.iter().cloned());
    defs.families.insert(FontFamily::Monospace, monospace);

    ctx.set_fonts(defs);
}

// --- egui style -------------------------------------------------------------

/// Apply a theme: colours, zeroed corners and the spacing grid.
pub fn configure_theme(ctx: &Context, t: &Tokens) {
    let mut visuals = if t.dark {
        Visuals::dark()
    } else {
        Visuals::light()
    };

    visuals.panel_fill = t.bg;
    visuals.window_fill = t.bg;
    visuals.extreme_bg_color = t.sunken;
    visuals.faint_bg_color = t.sunken;
    visuals.override_text_color = Some(t.ink);
    visuals.hyperlink_color = t.accent_ink;
    visuals.window_stroke = egui::Stroke::new(1.0_f32, t.line_strong);
    visuals.window_corner_radius = egui::CornerRadius::ZERO;
    visuals.menu_corner_radius = egui::CornerRadius::ZERO;
    visuals.selection.bg_fill = t.accent;
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, egui::Color32::WHITE);

    let widgets = &mut visuals.widgets;
    widgets.noninteractive.bg_fill = t.bg;
    widgets.noninteractive.weak_bg_fill = t.bg;
    widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, t.line);
    widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, t.ink);

    widgets.inactive.bg_fill = t.sunken;
    widgets.inactive.weak_bg_fill = t.sunken;
    widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, t.line_strong);
    widgets.inactive.fg_stroke = egui::Stroke::new(1.0_f32, t.ink);

    widgets.hovered.bg_fill = t.accent;
    widgets.hovered.weak_bg_fill = t.accent;
    widgets.hovered.bg_stroke = egui::Stroke::NONE;
    widgets.hovered.fg_stroke = egui::Stroke::new(1.0_f32, t.on_accent);

    widgets.active.bg_fill = t.accent;
    widgets.active.weak_bg_fill = t.accent;
    widgets.active.bg_stroke = egui::Stroke::NONE;
    widgets.active.fg_stroke = egui::Stroke::new(1.0_f32, t.on_accent);

    for widget in [
        &mut widgets.noninteractive,
        &mut widgets.inactive,
        &mut widgets.hovered,
        &mut widgets.active,
        &mut widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::ZERO;
        widget.expansion = 0.0;
    }

    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 6.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.interact_size.y = CONTROL_H;
    style.spacing.menu_margin = egui::Margin::same(4);
    style.spacing.scroll = egui::style::ScrollStyle::solid();
    style
        .text_styles
        .insert(TextStyle::Heading, fonts::display(24.0));
    style.text_styles.insert(TextStyle::Body, fonts::body(14.0));
    style
        .text_styles
        .insert(TextStyle::Button, fonts::mono_bold(11.0));
    style
        .text_styles
        .insert(TextStyle::Small, fonts::body(12.0));
    style
        .text_styles
        .insert(TextStyle::Monospace, fonts::mono(13.0));
    ctx.set_style(style);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rrggbb() {
        assert_eq!(hex_color("#6D5EF8"), Some(ACCENT));
        assert_eq!(hex_color("6d5ef8"), Some(ACCENT));
        assert_eq!(hex_color("no"), None);
        assert_eq!(hex_color("#fff"), None);
        assert_eq!(hex_color(""), None);
    }

    #[test]
    fn themes_differ_but_share_the_accent() {
        let dark = Tokens::dark();
        let light = Tokens::light();
        assert_ne!(dark.bg, light.bg);
        assert_eq!(dark.accent, light.accent);
        assert!(dark.dark && !light.dark);
    }

    #[test]
    fn tone_lookup_is_stable() {
        let t = Tokens::dark();
        assert_eq!(t.tone(Tone::Good), t.good);
        assert_eq!(t.tone(Tone::Warn), t.warn);
        assert_eq!(t.tone(Tone::Info), t.accent_ink);
        assert_eq!(t.tone(Tone::Neutral), t.muted);
        assert_eq!(t.tone(Tone::Faint), t.faint);
    }

    #[test]
    fn blend_hits_both_ends() {
        assert_eq!(blend(Color32::BLACK, Color32::WHITE, 0.0), Color32::BLACK);
        assert_eq!(blend(Color32::BLACK, Color32::WHITE, 1.0), Color32::WHITE);
    }
}
