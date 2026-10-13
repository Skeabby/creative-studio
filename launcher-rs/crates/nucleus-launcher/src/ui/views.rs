//! The three panes: sidebar, main list and optional inspector.

use egui::{Button, RichText, Sense, Ui};

use nucleus_launcher_core::{Card, LaunchKind};

use super::theme;
use super::widgets::{StatusBadge, Tag, category_color, tile};
use crate::runtime::Runtime;

/// Sidebar filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Filter {
    All,
    Installed,
    Upcoming,
}

impl Filter {
    pub fn label(self) -> &'static str {
        match self {
            Filter::All => "All apps",
            Filter::Installed => "Installed",
            Filter::Upcoming => "Coming soon",
        }
    }
}

/// Counts shown in the sidebar and footer.
#[derive(Debug, Clone, Copy, Default)]
pub struct Counts {
    pub all: usize,
    pub installed: usize,
    pub upcoming: usize,
}

/// An action a view asks the app to perform after the frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Open,
    Stop,
}

/// Sidebar: brand, filters, system facts.
pub fn sidebar(ui: &mut Ui, filter: &mut Filter, counts: Counts, facts: &Facts) {
    ui.vertical(|ui| {
        ui.add_space(4.0);
        ui.label(
            RichText::new("Nucleus")
                .size(24.0)
                .strong()
                .color(theme::TEXT),
        );
        ui.label(
            RichText::new("app suite launcher")
                .size(12.0)
                .color(theme::MUTED),
        );
        ui.add_space(12.0);
        ui.separator();

        for option in [Filter::All, Filter::Installed, Filter::Upcoming] {
            let label = match option {
                Filter::All => format!("{}  ·  {}", option.label(), counts.all),
                Filter::Installed => format!("{}  ·  {}", option.label(), counts.installed),
                Filter::Upcoming => format!("{}  ·  {}", option.label(), counts.upcoming),
            };
            let selected = *filter == option;
            if ui.selectable_label(selected, label).clicked() {
                *filter = option;
            }
        }

        ui.add_space(12.0);
        ui.separator();
        ui.add_space(6.0);
        ui.label(
            RichText::new("System")
                .size(11.0)
                .strong()
                .color(theme::CYAN),
        );
        label(ui, "Windows-first · egui");
        label(ui, &format!("repo  {}", facts.repo_root.display()));
        label(ui, &format!("data  {}", facts.data_dir.display()));
        label(ui, &facts.window_shell_line);
        match facts.node {
            NodeFact::Found(ref path) => label(ui, &format!("node  {path}")),
            NodeFact::Missing => {
                ui.label(
                    RichText::new("node not found on PATH — apps cannot start. Set NUCLEUS_NODE.")
                        .size(11.0)
                        .color(theme::AMBER),
                );
            }
        }
        for error in &facts.errors {
            ui.add_space(8.0);
            ui.label(
                RichText::new(format!("⚠ {error}"))
                    .size(11.0)
                    .color(theme::AMBER),
            );
        }
    });
}

fn label(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).size(11.0).color(theme::MUTED));
}

/// Node availability, disclosed honestly in the sidebar and header.
#[derive(Debug, Clone)]
pub enum NodeFact {
    Found(String),
    Missing,
}

/// Facts about the running environment, shown in the sidebar/footer.
#[derive(Debug, Clone)]
pub struct Facts {
    pub repo_root: std::path::PathBuf,
    pub data_dir: std::path::PathBuf,
    pub window_shell_line: String,
    pub node: NodeFact,
    pub errors: Vec<String>,
}

impl Facts {
    pub fn node_ok(&self) -> bool {
        matches!(self.node, NodeFact::Found(_))
    }
}

/// The main list: app cards filtered, then the "coming soon" section.
pub fn main_list(
    ui: &mut Ui,
    cards: &[Card],
    runtime: &Runtime,
    filter: Filter,
    selected: &mut Option<String>,
    actions: &mut Vec<(String, Action)>,
) {
    egui::ScrollArea::vertical()
        .id_salt("main-list")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(4.0);
            let apps: Vec<&Card> = cards.iter().filter(|card| app_of(card).is_some()).collect();
            let shown: Vec<&Card> = match filter {
                Filter::Upcoming => Vec::new(),
                _ => apps,
            };
            for card in &shown {
                card_row(ui, card, runtime, selected, actions);
            }
            if filter != Filter::Upcoming && shown.is_empty() {
                ui.label(RichText::new("No apps here yet.").color(theme::MUTED));
            }

            if filter != Filter::Installed {
                ui.add_space(16.0);
                ui.label(
                    RichText::new("Coming soon")
                        .size(16.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.label(
                    RichText::new("From the roadmap — planned, not shipped.")
                        .size(11.0)
                        .color(theme::MUTED),
                );
                ui.add_space(8.0);
                let planned: Vec<&Card> =
                    cards.iter().filter(|card| app_of(card).is_none()).collect();
                for card in planned {
                    if ui
                        .selectable_label(selected.as_deref() == Some(card.id()), card.title())
                        .clicked()
                    {
                        *selected = Some(card.id().to_string());
                    }
                }
            }
        });
}

fn app_of(card: &Card) -> Option<&nucleus_launcher_core::AppMeta> {
    match card {
        Card::App(app) => Some(app.as_ref()),
        Card::Planned(_) => None,
    }
}

/// One app row: tile, name/description, category + version tags, status and a
/// primary Open/Stop button.
fn card_row(
    ui: &mut Ui,
    card: &Card,
    runtime: &Runtime,
    selected: &mut Option<String>,
    actions: &mut Vec<(String, Action)>,
) {
    let id = card.id().to_string();
    let name = card.title().to_string();
    let running = runtime.is_running(&id);

    let app = match app_of(card) {
        Some(app) => app,
        None => return,
    };
    let status = if running {
        nucleus_launcher_core::AppStatus::Running
    } else {
        nucleus_launcher_core::derive_status(app.installed.as_ref(), app.catalog.as_ref(), false)
    };

    let tile_color = if running {
        theme::RUN
    } else {
        category_color(app.manifest.category)
    };

    egui::Frame::group(ui.style())
        .fill(theme::SURFACE)
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let glyph = card
                    .emoji()
                    .map(str::to_string)
                    .unwrap_or_else(|| first_letter(&name));
                tile(ui, &glyph, tile_color);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(&name).strong().size(14.0));
                        if let Some(category) = app.manifest.category {
                            ui.add(Tag::new(category.as_str(), category_color(Some(category))));
                        }
                        ui.add(Tag::new(&app.manifest.version, theme::MUTED));
                    });
                    if let Some(description) = card.description() {
                        ui.label(RichText::new(description).size(12.0).color(theme::MUTED));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    action_button(ui, app, running, actions);
                    ui.add(StatusBadge(status));
                });
            });
        })
        .response
        .context_menu(|ui| {
            if ui.button("Copy id").clicked() {
                ui.ctx().copy_text(id.clone());
                ui.close();
            }
        });
    if ui
        .interact(
            ui.response().rect,
            ui.id().with(&id).with("row"),
            Sense::click(),
        )
        .clicked()
    {
        *selected = Some(id);
    }
}

fn first_letter(name: &str) -> String {
    name.chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "?".into())
}

/// The primary button: Open or Stop (disabled for non-installed apps).
fn action_button(
    ui: &mut Ui,
    app: &nucleus_launcher_core::AppMeta,
    running: bool,
    actions: &mut Vec<(String, Action)>,
) {
    if running {
        let text = RichText::new("Stop").color(theme::RUN);
        if ui
            .add(
                Button::new(text)
                    .fill(theme::SURFACE)
                    .min_size(egui::vec2(76.0, 30.0)),
            )
            .clicked()
        {
            actions.push((app.id().to_string(), Action::Stop));
        }
    } else {
        match nucleus_launcher_core::derive_status(
            app.installed.as_ref(),
            app.catalog.as_ref(),
            false,
        ) {
            nucleus_launcher_core::AppStatus::Installed
            | nucleus_launcher_core::AppStatus::UpdateAvailable => {
                if ui
                    .add(
                        Button::new(RichText::new("Open").color(theme::TEXT))
                            .fill(theme::ACCENT)
                            .min_size(egui::vec2(76.0, 30.0)),
                    )
                    .clicked()
                {
                    actions.push((app.id().to_string(), Action::Open));
                }
            }
            nucleus_launcher_core::AppStatus::NotInstalled => {
                ui.add_enabled(
                    false,
                    Button::new(RichText::new("Install").color(theme::MUTED))
                        .min_size(egui::vec2(76.0, 30.0)),
                );
            }
            nucleus_launcher_core::AppStatus::Unavailable
            | nucleus_launcher_core::AppStatus::Running => {
                ui.add_enabled(
                    false,
                    Button::new(RichText::new("—").color(theme::MUTED))
                        .min_size(egui::vec2(76.0, 30.0)),
                );
            }
        }
    }
}

/// Optional right-side inspector for the selected card.
pub fn inspector(ui: &mut Ui, card: &Card, runtime: &Runtime, actions: &mut Vec<(String, Action)>) {
    egui::ScrollArea::vertical()
        .id_salt("inspector")
        .show(ui, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new(card.title()).size(18.0).strong());
            });
            ui.label(RichText::new(card.id()).size(11.0).color(theme::MUTED));
            ui.add_space(6.0);
            ui.separator();

            match card {
                Card::App(app) => {
                    let running = runtime.is_running(app.id());
                    let mut launch_spec_lines: Vec<String> = Vec::new();
                    if let Some(spec) = app.launch_spec() {
                        match spec.kind {
                            LaunchKind::Server => {
                                launch_spec_lines.push("kind  server".to_string());
                                if let Some(port) = spec.port {
                                    launch_spec_lines.push(format!("preferred port  {port}"));
                                }
                                launch_spec_lines.push(format!("args  {}", spec.args.join(" ")));
                                if let Some(path) = &spec.ready_path {
                                    launch_spec_lines.push(format!("ready path  {path}"));
                                }
                            }
                            LaunchKind::Url => {
                                launch_spec_lines.push("kind  url".to_string());
                                if let Some(url) = &spec.url {
                                    launch_spec_lines.push(format!("url  {url}"));
                                }
                            }
                            LaunchKind::Command => {
                                launch_spec_lines.push("kind  command".to_string());
                                if let Some(command) = &spec.command {
                                    launch_spec_lines.push(format!("command  {command}"));
                                }
                                if !spec.args.is_empty() {
                                    launch_spec_lines
                                        .push(format!("args  {}", spec.args.join(" ")));
                                }
                            }
                        }
                    }

                    field(ui, "name", Some(app.name().to_string()));
                    field(
                        ui,
                        "category",
                        app.manifest.category.map(|c| c.as_str().to_string()),
                    );
                    field(
                        ui,
                        "status",
                        app.manifest.status.map(|s| s.as_str().to_string()),
                    );
                    field(ui, "version", Some(app.manifest.version.clone()));
                    field(ui, "core api", Some(app.manifest.core_api_version.clone()));
                    field(ui, "author", app.manifest.author.clone());
                    field(ui, "inspired by", app.manifest.inspired_by.clone());
                    field(ui, "source", Some(app.source.as_str().to_string()));
                    if let Some(status) = running_info_status(app, runtime) {
                        field(ui, "runtime status", Some(status));
                    }
                    field(
                        ui,
                        "permissions",
                        joined(app.manifest.permissions.iter().map(|p| p.as_str())),
                    );
                    field(
                        ui,
                        "capabilities",
                        joined(app.manifest.capabilities.iter().map(|s| s.as_str())),
                    );
                    if let Some(channel) = app.manifest.update_channel {
                        field(ui, "channel", Some(channel.as_str().to_string()));
                    }
                    if let Some(min) = &app.manifest.min_shell {
                        field(ui, "min shell", Some(min.clone()));
                    }
                    field(ui, "dir", Some(app.dir.display().to_string()));
                    if !launch_spec_lines.is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("Launch")
                                .size(12.0)
                                .strong()
                                .color(theme::CYAN),
                        );
                        for line in launch_spec_lines {
                            field(ui, "", Some(line));
                        }
                    }
                    ui.add_space(10.0);
                    if running {
                        if ui
                            .add(
                                Button::new(RichText::new("Stop").color(theme::RUN))
                                    .fill(theme::SURFACE),
                            )
                            .clicked()
                        {
                            actions.push((app.id().to_string(), Action::Stop));
                        }
                    } else {
                        let openable = nucleus_launcher_core::derive_status(
                            app.installed.as_ref(),
                            app.catalog.as_ref(),
                            running,
                        )
                        .is_installed();
                        ui.add_enabled(
                            openable,
                            Button::new(RichText::new("Open").color(theme::TEXT))
                                .fill(theme::ACCENT),
                        )
                        .clicked()
                        .then(|| actions.push((app.id().to_string(), Action::Open)));
                    }
                }
                Card::Planned(entry) => {
                    field(ui, "id", Some(entry.id.clone()));
                    field(ui, "description", entry.description.clone());
                    field(
                        ui,
                        "category",
                        entry.category.map(|c| c.as_str().to_string()),
                    );
                    field(ui, "inspired by", entry.inspired_by.clone());
                    if let Some(color) = &entry.color
                        && let Some(parsed) = theme::hex_color(color)
                    {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new("accent").size(12.0).color(theme::MUTED));
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(14.0, 14.0), Sense::hover());
                            ui.painter().rect_filled(rect, 3.0, parsed);
                        });
                    }
                    if let Some(emoji) = &entry.emoji {
                        field(ui, "emoji", Some(emoji.clone()));
                    }
                    ui.add_space(8.0);
                    ui.label(
                        RichText::new("Not shipped yet — planned in the roadmap.")
                            .size(12.0)
                            .color(theme::AMBER),
                    );
                }
            }
        });
}

fn running_info_status(app: &nucleus_launcher_core::AppMeta, runtime: &Runtime) -> Option<String> {
    let (url, port) = runtime.running_info(app.id())?;
    match (url, port) {
        (Some(url), Some(port)) => Some(format!("running · {url} · port {port}")),
        (Some(url), None) => Some(format!("running · {url}")),
        (None, Some(port)) => Some(format!("running · port {port}")),
        (None, None) => Some("running".to_string()),
    }
}

fn field(ui: &mut Ui, label: &str, value: Option<String>) {
    if let Some(value) = value {
        if label.is_empty() {
            ui.label(
                RichText::new(value)
                    .size(12.0)
                    .color(theme::TEXT)
                    .monospace(),
            );
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(label).size(12.0).color(theme::MUTED));
                ui.label(
                    RichText::new(value)
                        .size(12.0)
                        .color(theme::TEXT)
                        .monospace(),
                );
            });
        }
    }
}

fn joined<'a>(values: impl Iterator<Item = &'a str>) -> Option<String> {
    let joined: Vec<String> = values.map(str::to_string).collect();
    if joined.is_empty() {
        None
    } else {
        Some(joined.join(", "))
    }
}

/// Bottom footer: counts + honest source line.
pub fn footer(ui: &mut Ui, counts: Counts, log: &[String]) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!(
                "{} apps · {} coming soon",
                counts.all, counts.upcoming
            ))
            .size(11.0)
            .color(theme::MUTED),
        );
        ui.separator();
        if let Some(last) = log.last() {
            ui.label(RichText::new(last).size(11.0).color(theme::MUTED));
        }
    });
}
