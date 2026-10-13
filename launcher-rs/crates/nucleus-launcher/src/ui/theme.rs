//! Design tokens and the dark theme (SPEC: INK/SURFACE/TEXT/ACCENT/CYAN/RUN/AMBER).

use egui::{Color32, Context, FontFamily, FontId, TextStyle};

/// `#0B0E14` — page background.
pub const INK: Color32 = Color32::from_rgb(0x0B, 0x0E, 0x14);
/// `#151B26` — panels, cards.
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
/// `#E85A6C` — destructive (Stop). Small extension of the spec tokens.
pub const DANGER: Color32 = Color32::from_rgb(0xE8, 0x5A, 0x6C);
/// Muted secondary text.
pub const MUTED: Color32 = Color32::from_rgb(0x8A, 0x91, 0xA0);

/// Apply the Nucleus dark theme to an egui context.
pub fn configure_theme(ctx: &Context) {
    ctx.set_theme(egui::Theme::Dark);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = SURFACE;
    visuals.window_fill = SURFACE;
    visuals.extreme_bg_color = INK;
    visuals.override_text_color = Some(TEXT);
    visuals.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    visuals.selection.stroke = egui::Stroke::new(1.0_f32, ACCENT);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(0x1E, 0x25, 0x33);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(0x26, 0x2E, 0x3D);
    visuals.widgets.active.bg_fill = ACCENT.gamma_multiply(0.45);
    ctx.set_visuals(visuals);

    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(14.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    ctx.set_style(style);
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
}
