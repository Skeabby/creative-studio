//! The widget library: hard-edged, hairline-ruled controls with uppercase
//! monospace "HUD" labels.
//!
//! Every widget paints itself from the [`Tokens`] passed in, so the same code
//! renders correctly in both themes. Nothing here uses a corner radius.

use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{
    Align2, Color32, CornerRadius, FontId, Galley, Rect, Response, Sense, Stroke, StrokeKind,
    TextFormat, TextureHandle, Ui, Vec2, vec2,
};

use nucleus_launcher_core::{AppStatus, Card, Category};

use super::theme::{CONTROL_H, HUD_SIZE, Tokens, Tone, fonts};

/// Letter-spacing added to HUD labels, as a multiple of the font size.
const HUD_SPACING: f32 = 0.15;
/// Letter-spacing used inside buttons and tags.
const TIGHT_SPACING: f32 = 0.12;

// --- HUD text ---------------------------------------------------------------

/// Lay out `text` in monospace with letter-spacing. The colour is a
/// placeholder so the galley can be re-coloured at paint time.
fn hud_job(ui: &Ui, text: &str, font: FontId, spacing_mult: f32) -> Arc<Galley> {
    let mut job = LayoutJob::default();
    job.append(
        &text.to_uppercase(),
        0.0,
        TextFormat {
            font_id: font.clone(),
            color: Color32::PLACEHOLDER,
            extra_letter_spacing: font.size * spacing_mult,
            ..Default::default()
        },
    );
    ui.fonts(|f| f.layout_job(job))
}

/// Paint an uppercase, letter-spaced monospace label. Returns its response.
pub fn hud(ui: &mut Ui, text: &str, color: Color32) -> Response {
    hud_sized(ui, text, color, HUD_SIZE, HUD_SPACING)
}

/// [`hud`] with an explicit size and letter-spacing multiplier.
pub fn hud_sized(ui: &mut Ui, text: &str, color: Color32, size: f32, spacing: f32) -> Response {
    let galley = hud_job(ui, text, fonts::mono(size), spacing);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color);
    response
}

/// A HUD label rendered inline inside a horizontal layout.
pub fn hud_inline(ui: &mut Ui, text: &str, color: Color32, size: f32, spacing: f32) -> Response {
    let galley = hud_job(ui, text, fonts::mono(size), spacing);
    let (rect, response) = ui.allocate_exact_size(galley.size(), Sense::hover());
    ui.painter().galley(rect.min, galley, color);
    response
}

/// Lay out a single line of proportional text, elided with `…` if it does not
/// fit `wrap_width`. The colour is a placeholder so it can be chosen at paint
/// time.
pub fn line(ui: &Ui, text: &str, font: FontId, wrap_width: f32) -> Arc<Galley> {
    let mut job = LayoutJob::single_section(
        text.to_string(),
        TextFormat {
            font_id: font,
            color: Color32::PLACEHOLDER,
            ..Default::default()
        },
    );
    job.wrap.max_width = wrap_width.max(1.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    ui.fonts(|f| f.layout_job(job))
}

/// A HUD label as a galley (uppercase + letter-spaced), for manual layout.
pub fn hud_galley(ui: &Ui, text: &str, size: f32, spacing: f32) -> Arc<Galley> {
    hud_job(ui, text, fonts::mono(size), spacing)
}

/// Size a HUD chip (the outlined status pill) would occupy.
pub fn badge_size(ui: &Ui, text: &str) -> Vec2 {
    let galley = hud_job(ui, text, fonts::mono(10.0), 0.10);
    vec2(galley.size().x + 24.0, 20.0)
}

/// A header navigation link: dims when idle, spans an inverted block when
/// hovered or active.
pub fn nav_link(ui: &mut Ui, t: &Tokens, label: &str, active: bool) -> Response {
    let galley = hud_job(ui, label, fonts::mono(HUD_SIZE), HUD_SPACING);
    let size = vec2(galley.size().x + 20.0, CONTROL_H);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let span = rect;
    if active || response.hovered() {
        ui.painter()
            .rect_filled(span, CornerRadius::ZERO, t.invert_bg);
    }
    let ink = if active || response.hovered() {
        t.invert_fg
    } else {
        t.muted
    };
    let pos = rect.center() - galley.size() * 0.5;
    ui.painter().galley(pos, galley, ink);
    response
}

/// A vertical hairline placed between header groups.
pub fn header_rule(ui: &mut Ui, t: &Tokens) {
    ui.add_space(4.0);
    rule(ui, t, 22.0);
    ui.add_space(4.0);
}

// --- Chips ------------------------------------------------------------------

/// A solid chip: the app number tab, "Launcher" tag, "Update" flag.
pub fn tag(ui: &mut Ui, text: &str, fill: Color32, ink: Color32) -> Response {
    let galley = hud_job(ui, text, fonts::mono_bold(HUD_SIZE), TIGHT_SPACING);
    let size = vec2(galley.size().x + 24.0, galley.size().y + 16.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, fill);
    let pos = rect.min + vec2(12.0, 8.0);
    painter.galley(pos, galley, ink);
    response
}

/// An outlined status pill: recessed fill, hairline border, tone-coloured ink.
pub fn badge(ui: &mut Ui, t: &Tokens, text: &str, tone: Tone) -> Response {
    let galley = hud_job(ui, text, fonts::mono(10.0), 0.10);
    let size = vec2(galley.size().x + 24.0, 20.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, t.sunken);
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0_f32, t.line),
        StrokeKind::Inside,
    );
    let pos = rect.center() - galley.size() * 0.5;
    painter.galley(pos, galley, t.tone(tone));
    response
}

// --- Segmented control ------------------------------------------------------

/// One cell of a [`cells`] row.
pub struct Cell<'a> {
    pub label: &'a str,
    /// Optional `(count, highlight)` rendered after the label.
    pub count: Option<(usize, Color32)>,
}

impl<'a> Cell<'a> {
    pub fn new(label: &'a str) -> Self {
        Cell { label, count: None }
    }

    pub fn with_count(label: &'a str, count: usize, highlight: Color32) -> Self {
        Cell {
            label,
            count: Some((count, highlight)),
        }
    }
}

/// A mutually-exclusive row of cells (filter tabs): outline + internal
/// vertical hairlines, the selected cell inverted. Returns the clicked index.
pub fn cells(ui: &mut Ui, t: &Tokens, items: &[Cell<'_>], selected: usize) -> Option<usize> {
    // Measure first so the row can be a single allocation.
    let mut widths: Vec<f32> = Vec::with_capacity(items.len());
    let mut galleys: Vec<Arc<Galley>> = Vec::with_capacity(items.len());
    for item in items {
        let mut label = item.label.to_string();
        if let Some((count, _)) = item.count {
            label = format!("{} {count}", item.label);
        }
        let galley = hud_job(ui, &label, fonts::mono(HUD_SIZE), HUD_SPACING);
        widths.push(galley.size().x + 24.0);
        galleys.push(galley);
    }
    let total: f32 = widths.iter().sum();
    let (rect, _) = ui.allocate_exact_size(vec2(total, CONTROL_H), Sense::hover());

    let mut clicked = None;
    let mut x = rect.min.x;
    for (index, galley) in galleys.into_iter().enumerate() {
        let cell = Rect::from_min_size(egui::pos2(x, rect.min.y), vec2(widths[index], CONTROL_H));
        let response = ui.interact(
            cell,
            ui.make_persistent_id(("nucleus-cell", index)),
            Sense::click(),
        );
        let active = index == selected;
        let mut fill = Color32::TRANSPARENT;
        let mut ink = t.muted;
        if active {
            fill = t.invert_bg;
            ink = t.invert_fg;
        } else if response.hovered() {
            fill = t.sunken;
            ink = t.ink_strong;
        }
        if fill != Color32::TRANSPARENT {
            ui.painter().rect_filled(cell, CornerRadius::ZERO, fill);
        }
        let pos = egui::pos2(
            cell.center().x - galley.size().x * 0.5,
            cell.center().y - galley.size().y * 0.5,
        );
        ui.painter().galley(pos, galley, ink);
        if index > 0 {
            ui.painter()
                .vline(cell.left(), cell.y_range(), Stroke::new(1.0_f32, t.line));
        }
        if response.clicked() {
            clicked = Some(index);
        }
        x += widths[index];
    }

    ui.painter().rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0_f32, t.line),
        StrokeKind::Inside,
    );
    clicked
}

// --- Buttons ----------------------------------------------------------------

/// Which button variant to paint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    /// Solid inverted: the page's single clear call to action.
    Primary,
    /// Outlined, inverting on hover.
    Secondary,
}

/// A button with an uppercase monospace label. `min_width` sets the floor.
pub fn button(
    ui: &mut Ui,
    t: &Tokens,
    label: &str,
    kind: ButtonKind,
    enabled: bool,
    min_width: f32,
) -> Response {
    let galley = hud_job(ui, label, fonts::mono_bold(HUD_SIZE), TIGHT_SPACING);
    let width = (galley.size().x + 36.0).max(min_width);
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(width, CONTROL_H), sense);
    let hovered = enabled && response.hovered();

    let (fill, ink, stroke) = match kind {
        ButtonKind::Primary => {
            let fill = if hovered {
                t.invert_bg.gamma_multiply(0.82)
            } else {
                t.invert_bg
            };
            (fill, t.invert_fg, Stroke::NONE)
        }
        ButtonKind::Secondary => {
            if hovered {
                (t.invert_bg, t.invert_fg, Stroke::NONE)
            } else {
                (t.bg, t.ink, Stroke::new(1.0_f32, t.line_strong))
            }
        }
    };

    let painter = ui.painter();
    if fill != Color32::TRANSPARENT {
        painter.rect_filled(rect, CornerRadius::ZERO, fill);
    }
    if stroke != Stroke::NONE {
        painter.rect_stroke(rect, CornerRadius::ZERO, stroke, StrokeKind::Inside);
    }
    let pos = rect.center() - galley.size() * 0.5;
    let ink = if enabled {
        ink
    } else {
        ink.gamma_multiply(0.5)
    };
    painter.galley(pos, galley, ink);
    response
}

/// A square glyph button (34 × 34): hairline when idle, inverted on hover.
pub fn glyph_button(ui: &mut Ui, t: &Tokens, glyph: &str, enabled: bool) -> Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, response) = ui.allocate_exact_size(vec2(CONTROL_H, CONTROL_H), sense);
    let hovered = enabled && response.hovered();
    let painter = ui.painter();
    if hovered {
        painter.rect_filled(rect, CornerRadius::ZERO, t.invert_bg);
    }
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0_f32, if hovered { t.invert_bg } else { t.line }),
        StrokeKind::Inside,
    );
    let ink = if hovered { t.invert_fg } else { t.muted };
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        glyph,
        fonts::mono(14.0),
        ink,
    );
    response
}

// --- Other ------------------------------------------------------------------

/// A vertical hairline rule of the given height.
pub fn rule(ui: &mut Ui, t: &Tokens, height: f32) {
    let (rect, _) = ui.allocate_exact_size(vec2(1.0, height), Sense::hover());
    ui.painter().vline(
        rect.center().x,
        rect.y_range(),
        Stroke::new(1.0_f32, t.line),
    );
}

/// A flat icon tile: recessed square, hairline border, then the rasterised
/// icon (or a letter when the app has no icon).
pub fn icon_tile(
    ui: &mut Ui,
    t: &Tokens,
    texture: Option<&TextureHandle>,
    letter: &str,
    color: Color32,
    size: f32,
) -> Rect {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, t.sunken);
    match texture {
        Some(texture) => {
            let inset = size * 0.10;
            painter.image(
                texture.id(),
                rect.shrink(inset),
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
            painter.text(
                rect.center(),
                Align2::CENTER_CENTER,
                glyph,
                fonts::display(size * 0.45),
                color,
            );
        }
    }
    painter.rect_stroke(
        rect,
        CornerRadius::ZERO,
        Stroke::new(1.0_f32, t.line),
        StrokeKind::Inside,
    );
    rect
}

/// The accent colour of a card: the roadmap colour for planned entries, the
/// category colour for real apps.
pub fn card_color(card: &Card, t: &Tokens) -> Color32 {
    match card {
        Card::App(app) => category_color(app.manifest.category, t),
        Card::Planned(entry) => entry
            .color
            .as_deref()
            .and_then(super::theme::hex_color)
            .unwrap_or(t.muted),
    }
}

/// Category → accent colour. Unknown categories fall back to muted grey.
pub fn category_color(category: Option<Category>, t: &Tokens) -> Color32 {
    match category {
        Some(Category::System) => t.accent,
        Some(Category::Image) => super::theme::hex_color("#6D5EF8").unwrap_or(t.accent),
        Some(Category::Vector) => super::theme::hex_color("#22D3EE").unwrap_or(t.accent),
        Some(Category::Pdf) => super::theme::hex_color("#E85A6C").unwrap_or(t.bad),
        Some(Category::Video) => super::theme::hex_color("#F59E0B").unwrap_or(t.warn),
        Some(Category::Audio) => super::theme::hex_color("#34D399").unwrap_or(t.good),
        Some(Category::Text) => super::theme::hex_color("#8AB4F8").unwrap_or(t.accent_ink),
        Some(Category::Layout) => super::theme::hex_color("#C084FC").unwrap_or(t.accent),
        Some(Category::Data) => super::theme::hex_color("#38BDF8").unwrap_or(t.accent_ink),
        Some(Category::Cad) => super::theme::hex_color("#FB923C").unwrap_or(t.warn),
        Some(Category::Other) | None => t.muted,
    }
}

/// Human label for a card's status badge, plus its tone.
pub fn status_label(status: AppStatus) -> (&'static str, Tone) {
    match status {
        AppStatus::Running => ("Running", Tone::Good),
        AppStatus::UpdateAvailable => ("Update available", Tone::Warn),
        AppStatus::Installed => ("Installed", Tone::Neutral),
        AppStatus::NotInstalled => ("Available", Tone::Info),
        AppStatus::Unavailable => ("Not available here", Tone::Faint),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_labels_are_stable() {
        assert_eq!(status_label(AppStatus::Running).0, "Running");
        assert_eq!(status_label(AppStatus::Installed).0, "Installed");
        assert_eq!(status_label(AppStatus::Unavailable).1, Tone::Faint);
    }

    #[test]
    fn categories_map_to_distinct_colours() {
        let t = Tokens::dark();
        assert_ne!(
            category_color(Some(Category::Pdf), &t),
            category_color(Some(Category::Audio), &t)
        );
        assert_eq!(category_color(None, &t), t.muted);
    }
}
