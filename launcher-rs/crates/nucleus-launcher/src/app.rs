//! The eframe application: owns the discovery, the runtime, the event channel
//! and the craft-launcher-style egui layout.

use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use egui::{CentralPanel, Margin, RichText, TopBottomPanel};

use nucleus_launcher_core::derive_status;

use crate::discovery::Discovery;
use crate::runtime::{Event, Runtime};
use crate::ui::icons::IconStore;
use crate::ui::{Action, Counts, Facts, NodeFact, Palette, SortKey, View, configure_theme};

/// The Nucleus launcher app window.
pub struct LauncherApp {
    discovery: Discovery,
    runtime: Runtime,
    events: Receiver<Event>,
    log: VecDeque<String>,
    search: String,
    view: View,
    dark: bool,
    sort: (SortKey, bool),
    icons: IconStore,
    facts: Facts,
}

const LOG_CAP: usize = 40;

impl LauncherApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        discovery: Discovery,
        runtime: Runtime,
        events: Receiver<Event>,
        facts: Facts,
    ) -> Self {
        crate::ui::install_fonts(&cc.egui_ctx);
        configure_theme(&cc.egui_ctx, &Palette::dark());
        Self {
            discovery,
            runtime,
            events,
            log: VecDeque::new(),
            search: String::new(),
            view: View::Grid,
            dark: true,
            sort: (SortKey::Name, true),
            icons: IconStore::new(),
            facts,
        }
    }

    fn counts(&self) -> Counts {
        let all = self.discovery.app_count();
        let installed = self
            .discovery
            .apps
            .iter()
            .filter(|app| {
                derive_status(app.installed.as_ref(), app.catalog.as_ref(), false).is_installed()
            })
            .count();
        Counts {
            all,
            installed,
            upcoming: self.discovery.planned_count(),
            running: self.runtime.running_count(),
        }
    }

    /// Drain runtime events into the log.
    fn step_events(&mut self) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                Event::Starting(id) => self.log_line(format!("starting {id}")),
                Event::Ready(id, url) => {
                    if url.is_empty() {
                        self.log_line(format!("{id} launched"));
                    } else {
                        self.log_line(format!("{id} ready at {url}"));
                    }
                }
                Event::Failed(id, message) => self.log_line(format!("{id} failed: {message}")),
                Event::Stopped(id) => self.log_line(format!("{id} stopped")),
            }
        }
    }

    fn log_line(&mut self, line: impl Into<String>) {
        self.log.push_back(line.into());
        while self.log.len() > LOG_CAP {
            self.log.pop_front();
        }
    }

    /// Apply a queued action after the frame's panels are drawn.
    fn apply_action(&mut self, id: String, action: Action) {
        match action {
            Action::Open => {
                if let Some(app) = self.discovery.app(&id) {
                    match self.runtime.launch(app) {
                        Ok(()) => {}
                        Err(message) => self.log_line(message),
                    }
                }
            }
            Action::Stop => {
                self.runtime.stop(&id);
            }
        }
    }
}

impl eframe::App for LauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.step_events();
        self.runtime.reconcile();

        if self.runtime.running_count() > 0 {
            // Keep polling for exited windows/processes while anything runs.
            ctx.request_repaint_after(Duration::from_millis(250));
        }

        // Apply the theme (stateless) then draw with the same palette.
        let palette = if self.dark {
            Palette::dark()
        } else {
            Palette::light()
        };
        configure_theme(ctx, &palette);

        let counts = self.counts();
        let mut actions: Vec<(String, Action)> = Vec::new();
        let border = palette.border;

        // Header (top).
        TopBottomPanel::top("header")
            .exact_height(52.0)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(palette.bg)
                    .inner_margin(Margin::symmetric(16, 10))
                    .stroke(egui::Stroke::new(1.0_f32, border)),
            )
            .show(ctx, |ui| {
                crate::ui::views::header(
                    ui,
                    &mut self.search,
                    &mut self.view,
                    &mut self.dark,
                    palette,
                );
            });

        // Footer (bottom).
        TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(palette.bg)
                    .inner_margin(Margin::symmetric(16, 6))
                    .stroke(egui::Stroke::new(1.0_f32, border)),
            )
            .show(ctx, |ui| {
                let log: Vec<String> = self.log.iter().cloned().collect();
                let node_path = match &self.facts.node {
                    NodeFact::Found(path) => Some(path.as_str()),
                    NodeFact::Missing => None,
                };
                crate::ui::views::footer(ui, counts, node_path, &self.runtime, &log, palette);
            });

        // Main area (central).
        CentralPanel::default()
            .frame(
                egui::Frame::central_panel(&ctx.style())
                    .fill(palette.bg)
                    .inner_margin(Margin::symmetric(20, 14)),
            )
            .show(ctx, |ui| {
                if !self.facts.node_ok() {
                    egui::Frame::NONE
                        .fill(crate::ui::theme::AMBER.gamma_multiply(0.14))
                        .corner_radius(8)
                        .inner_margin(Margin::symmetric(12, 8))
                        .show(ui, |ui| {
                            ui.label(
                                RichText::new(
                                    "Node.js was not found on PATH. Apps cannot start until it is \
                                 installed or NUCLEUS_NODE points at `node.exe`.",
                                )
                                .size(12.0)
                                .color(crate::ui::theme::AMBER),
                            );
                        });
                    ui.add_space(8.0);
                }
                for error in &self.facts.errors {
                    ui.label(
                        RichText::new(format!("⚠ {error}"))
                            .size(12.0)
                            .color(crate::ui::theme::AMBER),
                    );
                }
                if !self.facts.errors.is_empty() {
                    ui.add_space(8.0);
                }

                crate::ui::views::main_area(
                    ui,
                    &self.discovery.cards,
                    &self.runtime,
                    &mut crate::ui::ViewState {
                        view: self.view,
                        search: &mut self.search,
                        sort: &mut self.sort,
                    },
                    &mut actions,
                    &mut self.icons,
                    palette,
                );
            });

        for (id, action) in actions {
            self.apply_action(id, action);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.runtime.stop_all();
    }
}
