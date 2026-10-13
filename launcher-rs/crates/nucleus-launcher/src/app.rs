//! The eframe application: owns discovery, the runtime, the event channel and
//! the three-panel flat layout (header / central / footer).

use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::time::Duration;

use egui::{CentralPanel, Frame, Margin, RichText, Stroke, TopBottomPanel};

use nucleus_launcher_core::{AppStatus, Card, derive_status};

use crate::discovery::Discovery;
use crate::runtime::{Event, Runtime};
use crate::ui::icons::IconStore;
use crate::ui::theme::{FOOTER_H, HEADER_H, fonts};
use crate::ui::{Action, Counts, Facts, Filter, SortKey, Tokens, View, ViewState, configure_theme};

/// The Nucleus launcher app window.
pub struct LauncherApp {
    discovery: Discovery,
    runtime: Runtime,
    events: Receiver<Event>,
    log: VecDeque<String>,
    search: String,
    view: View,
    sort: (SortKey, bool),
    filter: Filter,
    dark: bool,
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
        configure_theme(&cc.egui_ctx, &Tokens::dark());
        Self {
            discovery,
            runtime,
            events,
            log: VecDeque::new(),
            search: String::new(),
            view: View::Grid,
            sort: (SortKey::Name, true),
            filter: Filter::All,
            dark: true,
            icons: IconStore::new(),
            facts,
        }
    }

    /// The totals shown in the filter row and footer.
    fn counts(&self) -> Counts {
        let mut counts = Counts::default();
        for card in &self.discovery.cards {
            match card {
                Card::App(app) => {
                    let status = derive_status(
                        app.installed.as_ref(),
                        app.catalog.as_ref(),
                        self.runtime.is_running(app.id()),
                    );
                    counts.all += 1;
                    if status.is_installed() {
                        counts.installed += 1;
                    } else {
                        counts.not_installed += 1;
                    }
                    if status == AppStatus::UpdateAvailable {
                        counts.updates += 1;
                    }
                }
                Card::Planned(_) => counts.planned += 1,
            }
        }
        counts.running = self.runtime.running_count();
        counts
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
                if let Some(app) = self.discovery.app(&id)
                    && let Err(message) = self.runtime.launch(app)
                {
                    self.log_line(message);
                }
            }
            Action::Stop => {
                self.runtime.stop(&id);
            }
            Action::Reveal => {
                if let Some(app) = self.discovery.app(&id) {
                    crate::process::reveal(&app.dir);
                }
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

        let t = if self.dark {
            Tokens::dark()
        } else {
            Tokens::light()
        };
        configure_theme(ctx, &t);

        let counts = self.counts();
        let mut actions: Vec<(String, Action)> = Vec::new();
        let border = Stroke::new(1.0_f32, t.line);

        {
            let mut state = ViewState {
                view: &mut self.view,
                search: &mut self.search,
                sort: &mut self.sort,
                filter: &mut self.filter,
                dark: &mut self.dark,
            };

            // --- header, doubling as the title bar --------------------------
            TopBottomPanel::top("nucleus-header")
                .exact_height(HEADER_H)
                .frame(
                    Frame::NONE
                        .fill(t.bg)
                        .inner_margin(Margin::symmetric(20, 0))
                        .stroke(border),
                )
                .show(ctx, |ui| {
                    crate::ui::views::header(ui, &t, &mut state);
                });

            // --- footer, the status bar -------------------------------------
            TopBottomPanel::bottom("nucleus-footer")
                .exact_height(FOOTER_H)
                .frame(
                    Frame::NONE
                        .fill(t.bg)
                        .inner_margin(Margin::symmetric(20, 0))
                        .stroke(border),
                )
                .show(ctx, |ui| {
                    let log: Vec<String> = self.log.iter().cloned().collect();
                    crate::ui::views::footer(ui, &t, counts, &self.facts, &self.runtime, &log);
                });

            // --- central: toolbar + grid/table ------------------------------
            let discovery = &self.discovery;
            let runtime = &self.runtime;
            let facts = &self.facts;
            let icons = &mut self.icons;
            CentralPanel::default()
                .frame(
                    Frame::NONE
                        .fill(t.bg)
                        .inner_margin(Margin::symmetric(20, 0)),
                )
                .show(ctx, |ui| {
                    ui.add_space(14.0);
                    crate::ui::views::toolbar(ui, &t, &mut state, counts);
                    ui.add_space(18.0);

                    if !facts.node_ok() || !facts.errors.is_empty() {
                        Frame::NONE
                            .fill(t.sunken)
                            .stroke(border)
                            .inner_margin(Margin::symmetric(14, 10))
                            .show(ui, |ui| {
                                if !facts.node_ok() {
                                    ui.label(
                                        RichText::new(
                                            "Node.js was not found on PATH. Apps cannot start \
                                             until it is installed or NUCLEUS_NODE points at \
                                             `node.exe`.",
                                        )
                                        .font(fonts::body(13.0))
                                        .color(t.warn),
                                    );
                                }
                                for error in &facts.errors {
                                    ui.label(
                                        RichText::new(error).font(fonts::body(12.0)).color(t.warn),
                                    );
                                }
                            });
                        ui.add_space(14.0);
                    }

                    crate::ui::views::main_area(
                        ui,
                        &t,
                        &discovery.cards,
                        runtime,
                        &mut state,
                        &mut actions,
                        icons,
                    );
                });
        }

        for (id, action) in actions {
            self.apply_action(id, action);
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.runtime.stop_all();
    }
}
