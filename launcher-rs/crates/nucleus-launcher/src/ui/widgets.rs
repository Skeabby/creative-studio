//! Small reusable widgets: status dots, tags/chips, tiles.

use egui::{Align2, Color32, FontId, RichText, Sense, Stroke, Ui, Widget};

use nucleus_launcher_core::{AppStatus, Category};

use super::theme::{ACCENT, AMBER, CYAN, DANGER, MUTED, RUN};

/// Dot + short label for an [`AppStatus`].
pub struct StatusBadge(pub AppStatus);

impl StatusBadge {
    fn color(&self) -> Color32 {
        match self.0 {
            AppStatus::Running => RUN,
            AppStatus::UpdateAvailable => AMBER,
            AppStatus::Installed => CYAN,
            AppStatus::NotInstalled | AppStatus::Unavailable => MUTED,
        }
    }

    fn label(&self) -> &'static str {
        match self.0 {
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
        let label = self.label();
        ui.horizontal(|ui| {
            let (rect, response) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), Sense::hover());
            ui.painter().circle_filled(rect.center(), 3.5, color);
            ui.label(RichText::new(label).size(12.0).color(color));
            response
        })
        .inner
    }
}

/// Rounded tag used for category chips and versions.
pub struct Tag<'a> {
    pub text: &'a str,
    pub fill: Color32,
    pub text_color: Color32,
}

impl<'a> Tag<'a> {
    pub fn new(text: &'a str, text_color: Color32) -> Self {
        Tag {
            text,
            fill: text_color,
            text_color,
        }
    }
}

impl Widget for Tag<'_> {
    fn ui(self, ui: &mut Ui) -> egui::Response {
        egui::Frame::group(ui.style())
            .fill(self.fill.gamma_multiply(0.15))
            .corner_radius(4)
            .inner_margin(egui::Margin::symmetric(6, 2))
            .show(ui, |ui| {
                ui.label(RichText::new(self.text).size(11.0).color(self.text_color))
            })
            .response
    }
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

/// A 44px rounded tile with a letter (SVG icons are not rasterised cheaply in
/// egui; SPEC: emoji/letter tiles in M1) or an emoji.
pub fn tile(ui: &mut Ui, glyph: &str, fill: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 44.0), Sense::hover());
    ui.painter().rect(
        rect,
        egui::CornerRadius::same(10),
        fill.gamma_multiply(0.35),
        Stroke::new(1.0_f32, fill.gamma_multiply(0.6)),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        FontId::proportional(20.0),
        fill,
    );
}
