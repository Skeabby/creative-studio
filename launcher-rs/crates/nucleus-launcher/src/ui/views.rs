//! The craft-launcher-style views: header, card grid, sortable table and the
//! "coming soon" section.

use egui::{Color32, RichText, Sense, Stroke, StrokeKind, Ui};

use nucleus_launcher_core::{AppStatus, Card};

use super::icons::IconStore;
use super::theme::{self, ACCENT, AMBER, Palette, RUN, fonts};
use super::widgets::{self, Tag, category_color};
use crate::runtime::Runtime;

/// Which main view is active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    Grid,
    Table,
}

/// Sortable table column.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortKey {
    Name,
    Category,
    Version,
    Status,
}

/// Counts shown in the header and footer.
#[derive(Debug, Clone, Copy, Default)]
pub struct Counts {
    pub all: usize,
    pub installed: usize,
    pub upcoming: usize,
    pub running: usize,
}

/// An action a view asks the app to perform after the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Stop,
}

/// Node availability, disclosed honestly in the header.
#[derive(Debug, Clone)]
pub enum NodeFact {
    Found(String),
    Missing,
}

/// Facts about the running environment.
#[derive(Debug, Clone)]
pub struct Facts {
    pub node: NodeFact,
    pub errors: Vec<String>,
}

impl Facts {
    pub fn node_ok(&self) -> bool {
        matches!(self.node, NodeFact::Found(_))
    }
}

/// The top bar: brand, search, view + theme switches.
pub fn header(
    ui: &mut Ui,
    search: &mut String,
    view: &mut View,
    dark: &mut bool,
    palette: Palette,
) {
    ui.horizontal(|ui| {
        // Brand.
        ui.add_space(2.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
        ui.painter().circle_filled(rect.center(), 6.0, ACCENT);
        ui.label(
            RichText::new("Nucleus")
                .size(19.0)
                .color(palette.text)
                .font(fonts::bold(19.0)),
        );
        ui.label(
            RichText::new("app suite launcher")
                .size(11.0)
                .color(palette.muted)
                .font(fonts::body(11.0)),
        );

        ui.add_space(24.0);
        // Search.
        widgets::search_field(ui, search);

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Theme toggle.
            let sun = if *dark { "☀" } else { "🌙" };
            if widgets::icon_button(ui, sun, false).clicked() {
                *dark = !*dark;
            }
            // View switch (grid | table).
            if widgets::icon_button(ui, "▦", *view == View::Grid).clicked() {
                *view = View::Grid;
            }
            ui.add_space(4.0);
            if widgets::icon_button(ui, "≡", *view == View::Table).clicked() {
                *view = View::Table;
            }
        });
    });
}

/// Whether a card matches the current search query.
pub fn matches(query: &str, card: &Card) -> bool {
    if query.trim().is_empty() {
        return true;
    }
    let q = query.to_lowercase();
    let hay = [
        Some(card.id().to_string()),
        Some(card.title().to_string()),
        card.description().map(str::to_string),
        card.category().map(|c| c.as_str().to_string()),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" ")
    .to_lowercase();
    hay.contains(&q)
}

fn card_category(card: &Card) -> Option<nucleus_launcher_core::Category> {
    match card {
        Card::App(app) => app.manifest.category,
        Card::Planned(entry) => entry.category,
    }
}

fn card_color(card: &Card, palette: Palette) -> Color32 {
    match card {
        Card::App(app) => category_color(app.manifest.category),
        Card::Planned(entry) => entry
            .color
            .as_deref()
            .and_then(theme::hex_color)
            .unwrap_or(palette.muted),
    }
}

/// The app-level view state the views read/mutate this frame.
pub struct ViewState<'a> {
    pub view: View,
    pub search: &'a mut String,
    pub sort: &'a mut (SortKey, bool),
}

/// The main scrolling area: grid or table of apps, then "coming soon".
pub fn main_area(
    ui: &mut Ui,
    cards: &[Card],
    runtime: &Runtime,
    state: &mut ViewState<'_>,
    actions: &mut Vec<(String, Action)>,
    icons: &mut IconStore,
    palette: Palette,
) {
    let view = state.view;
    let search = &mut *state.search;
    let sort = &mut *state.sort;
    egui::ScrollArea::vertical()
        .id_salt("main-area")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);

            let apps: Vec<&Card> = cards
                .iter()
                .filter(|card| matches(search, card) && card.as_app().is_some())
                .collect();
            let planned: Vec<&Card> = cards
                .iter()
                .filter(|card| matches(search, card) && card.as_app().is_none())
                .collect();

            if apps.is_empty() && planned.is_empty() {
                ui.add_space(48.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new("No apps match your search.")
                            .size(14.0)
                            .color(palette.muted)
                            .font(fonts::body(14.0)),
                    );
                });
                return;
            }

            if !apps.is_empty() {
                widgets::section_header(ui, "Apps", &format!("{} found", apps.len()), palette);
                ui.add_space(4.0);
                match view {
                    View::Grid => grid(ui, &apps, runtime, actions, icons, palette),
                    View::Table => table(ui, &apps, runtime, actions, sort, icons, palette),
                }
            }

            if !planned.is_empty() {
                ui.add_space(20.0);
                widgets::section_header(
                    ui,
                    "Coming soon",
                    &format!("{} in the roadmap", planned.len()),
                    palette,
                );
                ui.add_space(4.0);
                for card in planned {
                    planned_row(ui, card, palette);
                }
            }
        });
}

/// Icons for a real app card, or a coloured letter tile.
fn app_icon(ui: &mut Ui, icons: &mut IconStore, card: &Card, size: f32, palette: Palette) {
    let letter = card.title();
    let color = card_color(card, palette);
    let mut texture = None;
    if let Card::App(app) = card
        && let Some(icon) = &app.manifest.icon
    {
        let path = app.dir.join(icon);
        if path.exists() {
            texture = icons.load(ui.ctx(), app.id(), &path, size);
        }
    }
    widgets::icon_tile(ui, texture, letter, color, size, palette);
}

/// One card in the grid (craft-launcher style).
fn card(
    ui: &mut Ui,
    card: &Card,
    runtime: &Runtime,
    actions: &mut Vec<(String, Action)>,
    icons: &mut IconStore,
    palette: Palette,
) {
    let id = card.id().to_string();
    let running = runtime.is_running(&id);
    let status = if running {
        AppStatus::Running
    } else {
        status_of(card)
    };

    let frame = egui::Frame::NONE
        .fill(palette.surface)
        .corner_radius(12)
        .inner_margin(egui::Margin::same(14));
    let response = frame.show(ui, |ui| {
        ui.set_min_width(240.0);
        ui.set_width(ui.available_width());
        ui.vertical(|ui| {
            // Top: icon + identity.
            ui.horizontal(|ui| {
                app_icon(ui, icons, card, 46.0, palette);
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(card.title())
                            .size(15.0)
                            .color(palette.text)
                            .font(fonts::semibold(15.0)),
                    );
                    ui.horizontal(|ui| {
                        if let Some(category) = card_category(card) {
                            let cc = category_color(Some(category));
                            ui.add(Tag {
                                text: category.as_str(),
                                color: cc,
                                palette,
                            });
                        }
                        if let Card::App(app) = card {
                            ui.add(Tag {
                                text: &app.manifest.version,
                                color: palette.muted,
                                palette,
                            });
                        }
                    });
                });
            });

            // Description.
            if let Some(description) = card.description()
                && !description.is_empty()
            {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(description)
                        .size(12.0)
                        .color(palette.muted)
                        .font(fonts::body(12.0)),
                );
            }

            // Footer: status + action.
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.add(widgets::StatusBadge { status });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    action_button(ui, card, running, actions, palette);
                });
            });
        });
    });

    // Hover border over the painted card.
    if response.response.hovered() {
        ui.painter().rect(
            response.response.rect,
            egui::CornerRadius::same(12),
            Color32::TRANSPARENT,
            Stroke::new(1.0_f32, theme::hover_border(&palette).gamma_multiply(0.5)),
            StrokeKind::Inside,
        );
    }
}

fn status_of(card: &Card) -> AppStatus {
    match card {
        Card::App(app) => nucleus_launcher_core::derive_status(
            app.installed.as_ref(),
            app.catalog.as_ref(),
            false,
        ),
        Card::Planned(_) => AppStatus::NotInstalled,
    }
}

/// The primary/secondary action per card.
fn action_button(
    ui: &mut Ui,
    card: &Card,
    running: bool,
    actions: &mut Vec<(String, Action)>,
    palette: Palette,
) {
    let id = card.id().to_string();
    if running {
        if widgets::ghost_button(ui, "Stop", RUN).clicked() {
            actions.push((id, Action::Stop));
        }
        return;
    }
    match card {
        Card::App(_) => match status_of(card) {
            AppStatus::Installed | AppStatus::UpdateAvailable => {
                if widgets::primary_button(ui, "Open", true, ACCENT).clicked() {
                    actions.push((id, Action::Open));
                }
            }
            AppStatus::NotInstalled => {
                widgets::primary_button(ui, "Install", false, ACCENT);
            }
            AppStatus::Unavailable | AppStatus::Running => {
                widgets::primary_button(ui, "—", false, palette.muted);
            }
        },
        Card::Planned(_) => {
            widgets::primary_button(ui, "Install", false, ACCENT);
        }
    }
}

/// Responsive card grid.
fn grid(
    ui: &mut Ui,
    apps: &[&Card],
    runtime: &Runtime,
    actions: &mut Vec<(String, Action)>,
    icons: &mut IconStore,
    palette: Palette,
) {
    const CARD_MIN: f32 = 280.0;
    const GAP: f32 = 12.0;
    let columns = ((ui.available_width() - GAP) / (CARD_MIN + GAP))
        .max(1.0)
        .floor() as usize;

    egui::Grid::new("apps-grid")
        .num_columns(columns)
        .spacing(egui::vec2(GAP, GAP))
        .show(ui, |ui| {
            for (index, app_card) in apps.iter().enumerate() {
                card(ui, app_card, runtime, actions, icons, palette);
                if (index + 1) % columns == 0 && index + 1 < apps.len() {
                    ui.end_row();
                }
            }
            // End the last row for a full-width trailing border.
            if !apps.is_empty() {
                ui.end_row();
            }
        });
}

/// Sort the current app list for the table view.
fn sort_apps(apps: &mut Vec<&Card>, sort: &(SortKey, bool)) {
    let (key, asc) = sort;
    apps.sort_by(|a, b| {
        let ordering = match key {
            SortKey::Name => a.title().to_lowercase().cmp(&b.title().to_lowercase()),
            SortKey::Category => card_category(a)
                .map(|c| c.as_str())
                .unwrap_or("")
                .cmp(card_category(b).map(|c| c.as_str()).unwrap_or("")),
            SortKey::Version => match (a, b) {
                (Card::App(a), Card::App(b)) => a.manifest.version.cmp(&b.manifest.version),
                _ => a.title().cmp(b.title()),
            },
            SortKey::Status => status_order(&status_of(a)).cmp(&status_order(&status_of(b))),
        };
        if *asc { ordering } else { ordering.reverse() }
    });
}

fn status_order(status: &AppStatus) -> u8 {
    match status {
        AppStatus::Running => 0,
        AppStatus::Installed => 1,
        AppStatus::UpdateAvailable => 2,
        AppStatus::NotInstalled => 3,
        AppStatus::Unavailable => 4,
    }
}

/// The sortable table view.
fn table(
    ui: &mut Ui,
    apps_in: &[&Card],
    runtime: &Runtime,
    actions: &mut Vec<(String, Action)>,
    sort: &mut (SortKey, bool),
    icons: &mut IconStore,
    palette: Palette,
) {
    let mut apps = apps_in.to_vec();
    sort_apps(&mut apps, sort);

    egui::Frame::NONE
        .fill(palette.surface)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(10, 4))
        .show(ui, |ui| {
            egui::Grid::new("apps-table")
                .num_columns(5)
                .spacing(egui::vec2(18.0, 6.0))
                .striped(palette.bg.r() >= 128)
                .show(ui, |ui| {
                    for (key, label) in [
                        (SortKey::Name, "Name"),
                        (SortKey::Category, "Category"),
                        (SortKey::Version, "Version"),
                        (SortKey::Status, "Status"),
                    ] {
                        let arrow = if sort.0 == key {
                            if sort.1 { " ↑" } else { " ↓" }
                        } else {
                            ""
                        };
                        if ui
                            .add(
                                egui::Button::new(
                                    RichText::new(format!("{label}{arrow}"))
                                        .size(12.0)
                                        .color(if sort.0 == key { ACCENT } else { palette.muted })
                                        .font(fonts::semibold(12.0)),
                                )
                                .frame(false)
                                .corner_radius(4),
                            )
                            .clicked()
                        {
                            if sort.0 == key {
                                sort.1 = !sort.1;
                            } else {
                                *sort = (key, true);
                            }
                        }
                        ui.end_row();
                    }
                    for &card in &apps {
                        row(ui, card, runtime, actions, icons, palette);
                    }
                });
        });
}

/// One table row.
fn row(
    ui: &mut Ui,
    card: &Card,
    runtime: &Runtime,
    actions: &mut Vec<(String, Action)>,
    icons: &mut IconStore,
    palette: Palette,
) {
    let id = card.id().to_string();
    let running = runtime.is_running(&id);

    ui.horizontal(|ui| {
        app_icon(ui, icons, card, 24.0, palette);
        ui.add_space(2.0);
        ui.label(
            RichText::new(card.title())
                .size(13.0)
                .color(palette.text)
                .font(fonts::body(13.0)),
        );
    });
    if let Some(category) = card_category(card) {
        let cc = category_color(Some(category));
        ui.add(Tag {
            text: category.as_str(),
            color: cc,
            palette,
        });
    } else {
        ui.label(RichText::new("—").color(palette.muted));
    }
    match card {
        Card::App(app) => {
            ui.label(
                RichText::new(&app.manifest.version)
                    .size(12.0)
                    .color(palette.muted)
                    .monospace(),
            );
            let status = if running {
                AppStatus::Running
            } else {
                status_of(card)
            };
            ui.add(widgets::StatusBadge { status });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if running {
                    if widgets::ghost_button(ui, "Stop", RUN).clicked() {
                        actions.push((id, Action::Stop));
                    }
                } else {
                    match status_of(card) {
                        AppStatus::Installed | AppStatus::UpdateAvailable => {
                            if widgets::primary_button(ui, "Open", true, ACCENT).clicked() {
                                actions.push((id, Action::Open));
                            }
                        }
                        _ => {
                            widgets::primary_button(ui, "Install", false, ACCENT);
                        }
                    }
                }
            });
        }
        Card::Planned(_) => {
            ui.label(RichText::new("—").color(palette.muted));
            ui.add(widgets::StatusBadge {
                status: AppStatus::NotInstalled,
            });
            widgets::primary_button(ui, "Install", false, ACCENT);
        }
    }
    ui.end_row();
}

/// A compact "coming soon" row.
fn planned_row(ui: &mut Ui, card: &Card, palette: Palette) {
    egui::Frame::NONE
        .fill(palette.surface)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(14, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let color = card_color(card, palette);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), Sense::hover());
                ui.painter().rect_filled(rect, 7, color.gamma_multiply(0.2));
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    card.title()
                        .chars()
                        .next()
                        .map(|c| c.to_uppercase().to_string())
                        .unwrap_or_default(),
                    egui::FontId::new(15.0, egui::FontFamily::Name("nucleus-bold".into())),
                    color,
                );
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(card.title())
                            .size(13.0)
                            .color(palette.text)
                            .font(fonts::semibold(13.0)),
                    );
                    if let Some(description) = card.description() {
                        ui.label(RichText::new(description).size(11.5).color(palette.muted));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add(Tag {
                        text: "Planned",
                        color: AMBER,
                        palette,
                    });
                });
            });
        });
}

/// Bottom footer: counts + running apps + node path + last log line.
pub fn footer(
    ui: &mut Ui,
    counts: Counts,
    node: Option<&str>,
    runtime: &Runtime,
    log: &[String],
    palette: Palette,
) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{} app{} · {} installed · {} planned · {} running",
                counts.all,
                plural(counts.all),
                counts.installed,
                counts.upcoming,
                counts.running
            ))
            .size(11.0)
            .color(palette.muted)
            .font(fonts::body(11.0)),
        );
        ui.separator();
        for id in runtime.running_ids() {
            let detail = runtime
                .running_info(&id)
                .map(|(url, port)| match (url, port) {
                    (Some(url), _) => format!("· {url}"),
                    (None, Some(port)) => format!("· :{port}"),
                    (None, None) => String::new(),
                })
                .unwrap_or_default();
            ui.label(
                RichText::new(format!("▶ {id}{detail}"))
                    .size(11.0)
                    .color(RUN)
                    .font(fonts::mono(11.0)),
            );
        }
        if let Some(node_path) = node {
            ui.separator();
            ui.label(
                RichText::new(format!("node {}", node_path.trim_end_matches("node.exe")))
                    .size(11.0)
                    .color(palette.muted)
                    .font(fonts::mono(11.0)),
            );
        }
        if let Some(last) = log.last() {
            ui.separator();
            ui.label(
                RichText::new(last)
                    .size(11.0)
                    .color(palette.muted)
                    .font(fonts::mono(11.0)),
            );
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new("egui · Rust · Windows")
                    .size(10.0)
                    .color(palette.muted.gamma_multiply(0.8))
                    .font(fonts::body(10.0)),
            );
        });
    });
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
