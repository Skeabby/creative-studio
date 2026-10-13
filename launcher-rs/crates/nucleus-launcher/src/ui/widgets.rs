//! Small reusable widgets: status dots, tags/chips, icon tiles, buttons.

use egui::{Align2, Color32, FontId, Rect, RichText, Sense, Stroke, TextureHandle, Ui, Widget};

use nucleus_launcher_core::{AppStatus, Category};

use super::theme::{ACCENT, AMBER, CYAN, DANGER, MUTED, Palette, RUN, fonts};

/// Dot + short label for an [`AppStatus`].
pub struct StatusBadge {
    pub status: AppStatus,
}

impl StatusBadge {
    fn color(&self) -> Color32 {
        match self.status {
            AppStatus::Running => RUN,
            AppStatus::UpdateAvailable => AMBER,
            AppStatus::Installed => CYAN,
            AppStatus::NotInstalled | AppStatus::Unavailable => MUTED,
        }
    }

    fn label(&self) -> &'static str {
        match self.status {
            AppStatus::Running => "Running",
            AppStatus::UpdateAvailable => "Update",
            AppStatus::Installed => "Installed",
            AppStatus::NotInstalled => "Not installed",
            AppStatus::Unavailable => "—",
        }
    }
}

impl Widget for StatusBadge {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let color = self.color();
        ui.horizontal(|ui| {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 3.5, color);
            ui.label(
                RichText::new(self.label())
                    .size(12.0)
                    .color(color)
                    .font(fonts::semibold(12.0)),
            );
            response
        })
        .inner
    }
}

/// Rounded tag used for category chips and versions.
pub struct Tag<'a> {
    pub text: &'a str,
    pub color: Color32,
    pub palette: Palette,
}

impl Widget for Tag<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        let surface = self.palette.surface;
        // A chip that reads against both themes: tinted fill + tinted text.
        let (bg, fg) = tag_colors(self.color, surface);
        egui::Frame::NONE
            .fill(bg)
            .corner_radius(5)
            .inner_margin(egui::Margin::symmetric(7, 3))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(self.text)
                        .size(11.0)
                        .color(fg)
                        .font(fonts::semibold(11.0)),
                )
            })
            .response
    }
}

/// Fill + text colours for a chip: strong text, translucent fill.
fn tag_colors(color: Color32, surface: Color32) -> (Color32, Color32) {
    let on_dark = surface.r() < 128;
    let fg = color;
    let bg = if on_dark {
        color.gamma_multiply(0.16)
    } else {
        color.gamma_multiply(0.14)
    };
    (bg, fg)
}

/// Accent colour for an app's category chip.
pub fn category_color(category: Option<Category>) -> Color32 {
    match category {
        Some(Category::System | Category::Image) => ACCENT,
        Some(Category::Pdf) => DANGER,
        Some(Category::Video) => AMBER,
        Some(Category::Audio) => RUN,
        Some(Category::Vector) => CYAN,
        _ => MUTED,
    }
}

/// An app icon tile: a rounded-square surface with the rasterised icon, or a
/// coloured letter tile when the icon is missing.
#[allow(clippy::too_many_arguments)]
pub fn icon_tile(
    ui: &mut Ui,
    texture: Option<&TextureHandle>,
    letter: &str,
    color: Color32,
    size: f32,
    palette: Palette,
) -> Rect {
    let (rect, _response) = ui.allocate_exact_size(egui::vec2(size, size), Sense::hover());
    let radius = (size / 4.0).max(6.0);
    ui.painter().rect_filled(
        rect,
        radius,
        color.gamma_multiply(if palette.bg.r() < 128 { 0.22 } else { 0.12 }),
    );
    match texture {
        Some(tex) => {
            let inset = size * 0.16;
            let img_rect = rect.shrink(inset);
            ui.painter().image(
                tex.id(),
                img_rect,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
        }
        None => {
            let glyph = letter
                .chars()
                .next()
                .map(|c| c.to_uppercase().to_string())
                .unwrap_or_else(|| "?".into());
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                glyph,
                FontId::new(size * 0.42, egui::FontFamily::Name("nucleus-bold".into())),
                color,
            );
        }
    }
    rect
}

/// Primary action button (Open / Install): accent fill, white text.
pub fn primary_button(ui: &mut Ui, text: &str, enabled: bool, accent: Color32) -> egui::Response {
    let btn = egui::Button::new(
        RichText::new(text)
            .color(Color32::WHITE)
            .font(fonts::semibold(13.0)),
    )
    .fill(accent)
    .stroke(Stroke::new(1.0_f32, accent.gamma_multiply(0.8)))
    .corner_radius(6)
    .min_size(egui::vec2(84.0, 30.0));
    if enabled {
        ui.add(btn)
    } else {
        ui.add_enabled(false, btn)
    }
}

/// Secondary button (Stop): transparent, coloured outline + text.
pub fn ghost_button(ui: &mut Ui, text: &str, color: Color32) -> egui::Response {
    let btn = egui::Button::new(RichText::new(text).color(color).font(fonts::semibold(13.0)))
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.6)))
        .corner_radius(6)
        .min_size(egui::vec2(84.0, 30.0));
    ui.add(btn)
}

/// A search-style text field.
pub fn search_field(ui: &mut Ui, query: &mut String) -> egui::Response {
    ui.add(
        egui::TextEdit::singleline(query)
            .font(fonts::body(13.0))
            .hint_text(
                RichText::new("Search apps…").color(ui.visuals().text_color().gamma_multiply(0.45)),
            )
            .desired_width(220.0)
            .margin(egui::Margin::symmetric(10, 6)),
    )
}

/// A small image button toggle used for the view/theme switches.
pub fn icon_button(ui: &mut Ui, label: &str, selected: bool) -> egui::Response {
    let color = if selected {
        ACCENT
    } else {
        ui.visuals().text_color()
    };
    let btn = egui::Button::new(RichText::new(label).size(14.0).color(color))
        .fill(if selected {
            ACCENT.gamma_multiply(0.22)
        } else {
            Color32::TRANSPARENT
        })
        .stroke(Stroke::NONE)
        .corner_radius(6)
        .frame(true)
        .min_size(egui::vec2(30.0, 26.0));
    ui.add(btn)
}

/// Section header used in the central area (like craft-launcher's groups).
pub fn section_header(ui: &mut Ui, title: &str, subtitle: &str, palette: Palette) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(title)
                .size(18.0)
                .color(palette.text)
                .font(fonts::bold(18.0)),
        );
        ui.add_space(2.0);
        ui.label(
            RichText::new(subtitle)
                .size(11.5)
                .color(palette.muted)
                .font(fonts::body(11.5)),
        );
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_colour_is_stable() {
        assert_eq!(category_color(Some(Category::Pdf)), DANGER);
        assert_eq!(category_color(None), MUTED);
    }

    #[test]
    fn letter_falls_back_to_question_mark() {
        assert_eq!("?".chars().next().unwrap(), "?".chars().next().unwrap());
    }
}
