//! The eframe application: owns the discovery, the runtime, the event channel
//! and the egui layout.

use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use egui::{CentralPanel, Margin, RichText, SidePanel, TopBottomPanel};

use nucleus_launcher_core::derive_status;

use crate::discovery::Discovery;
use crate::runtime::{Event, Runtime};
use crate::ui::{Action, Counts, Facts, Filter};

/// The Nucleus launcher app window.
pub struct LauncherApp {
    discovery: Discovery,
    runtime: Runtime,
    events: Receiver<Event>,
    log: VecDeque<String>,
    selected: Option<String>,
    filter: Filter,
    facts: Facts,
}

const LOG_CAP: usize = 40;

impl LauncherApp {
    pub fn new(
        _cc: &eframe::CreationContext<'_>,
        discovery: Discovery,
        runtime: Runtime,
        events: Receiver<Event>,
        facts: Facts,
    ) -> Self {
        Self {
            discovery,
            runtime,
            events,
            log: VecDeque::new(),
            selected: None,
            filter: Filter::All,
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

        let counts = self.counts();
        let mut actions: Vec<(String, Action)> = Vec::new();
        let mut selected = self.selected.clone();

        // Sidebar (left).
        SidePanel::left("sidebar")
            .resizable(false)
            .exact_width(240.0)
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(crate::ui::theme::INK)
                    .inner_margin(Margin::same(14)),
            )
            .show(ctx, |ui| {
                crate::ui::views::sidebar(ui, &mut self.filter, counts, &self.facts);
            });

        // Footer (bottom).
        TopBottomPanel::bottom("footer")
            .frame(
                egui::Frame::side_top_panel(&ctx.style())
                    .fill(crate::ui::theme::INK)
                    .inner_margin(Margin::symmetric(14, 6)),
            )
            .show(ctx, |ui| {
                let log: Vec<String> = self.log.iter().cloned().collect();
                crate::ui::views::footer(ui, counts, &log);
            });

        // Inspector (right) when a card is selected.
        if let Some(id) = selected.as_deref()
            && let Some(card) = self.discovery.card(id)
        {
            SidePanel::right("inspector")
                .default_width(320.0)
                .min_width(240.0)
                .frame(
                    egui::Frame::side_top_panel(&ctx.style())
                        .fill(crate::ui::theme::INK.gamma_multiply(0.6))
                        .inner_margin(Margin::same(14)),
                )
                .show(ctx, |ui| {
                    crate::ui::views::inspector(ui, card, &self.runtime, &mut actions);
                });
        }

        // Main list (central).
        CentralPanel::default().show(ctx, |ui| {
            if !self.facts.node_ok() {
                egui::Frame::group(ui.style())
                    .fill(crate::ui::theme::AMBER.gamma_multiply(0.12))
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
            crate::ui::views::main_list(
                ui,
                &self.discovery.cards,
                &self.runtime,
                self.filter,
                &mut selected,
                &mut actions,
            );
        });

        self.selected = selected;
        for (id, action) in actions {
            self.apply_action(id, action);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.runtime.stop_all();
    }
}
