//! # nucleus-launcher
//!
//! The Nucleus desktop launcher: discover the local suite, open/stop app
//! servers, and show them in a light egui window. Windows-first (M1).

#![cfg_attr(not(windows), allow(dead_code))]

#[cfg(not(windows))]
compile_error!("nucleus-launcher currently supports Windows only (M1)");

mod app;
mod discovery;
mod paths;
mod ports;
mod probe;
mod process;
mod runtime;
mod ui;
mod window;

use std::path::PathBuf;

use discovery::Discovery;
use serde_json::json;

use crate::ui::{Facts, NodeFact};

fn main() {
    // --- CLI: --smoke | --list | --repo <path> -------------------------------
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut smoke = false;
    let mut list = false;
    let mut repo: Option<PathBuf> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--smoke" => smoke = true,
            "--list" => list = true,
            "--repo" => {
                index += 1;
                match args.get(index) {
                    Some(path) => repo = Some(PathBuf::from(path)),
                    None => {
                        eprintln!("error: --repo requires a path");
                        std::process::exit(2);
                    }
                }
            }
            other => {
                eprintln!("error: unknown argument `{other}`");
                eprintln!("usage: nucleus-launcher [--smoke] [--list] [--repo <path>]");
                std::process::exit(2);
            }
        }
        index += 1;
    }

    // --- Resolve the repository + data locations ----------------------------
    let paths = match paths::Paths::resolve(repo.as_deref()) {
        Ok(paths) => paths,
        Err(message) => {
            eprintln!("error: {message}");
            std::process::exit(1);
        }
    };
    let discovery = discovery::discover(&paths);
    let node = process::known_node();

    if smoke {
        print_smoke(&discovery, node.as_deref());
        return;
    }
    if list {
        print_list(&discovery);
        return;
    }

    // --- Engine + window ----------------------------------------------------
    let (runtime, events) = runtime::Runtime::new(runtime::RuntimeConfig {
        node_path: node.clone(),
        shell_exe: paths.shell_exe.clone(),
        icon_ico: paths.icon_ico.clone(),
        window_profiles_dir: paths.window_profiles_dir.clone(),
        window_size: window::APP_WINDOW_SIZE,
    });
    let facts = Facts {
        repo_root: paths.repo_root.clone(),
        data_dir: paths.data_dir.clone(),
        window_shell_line: match &paths.shell_exe {
            Some(shell) => format!("window shell  {}", shell.display()),
            None => "window shell  default browser (shell not built)".to_string(),
        },
        node: match node {
            Some(path) => NodeFact::Found(path),
            None => NodeFact::Missing,
        },
        errors: discovery.errors.clone(),
    };

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Nucleus")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    let result = eframe::run_native(
        "Nucleus",
        native_options,
        Box::new(move |cc| {
            ui::configure_theme(&cc.egui_ctx);
            Ok(Box::new(app::LauncherApp::new(
                cc, discovery, runtime, events, facts,
            )))
        }),
    );
    if let Err(err) = result {
        eprintln!("error: {err}");
    }
}

/// `--smoke`: a short JSON summary, exit 0 (exit 1 already handled above on a
/// serious error like an unresolvable repository).
fn print_smoke(discovery: &Discovery, node: Option<&str>) {
    let discovered_ids: Vec<&str> = discovery.apps.iter().map(|app| app.id()).collect();
    let planned_ids: Vec<&str> = discovery
        .roadmap
        .iter()
        .map(|entry| entry.id.as_str())
        .collect();
    let summary = json!({
        "discovered": discovery.app_count(),
        "packed": discovered_ids.len(),
        "apps": discovered_ids,
        "roadmap": discovery.roadmap.len(),
        "planned": planned_ids,
        "cards": discovery.cards.len(),
        "node": node,
        "errors": discovery.errors,
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&summary).unwrap_or_else(|_| "{}".into())
    );
}

/// `--list`: a human-readable listing.
fn print_list(discovery: &Discovery) {
    println!(
        "{} app{} discovered, {} planned",
        discovery.app_count(),
        plural(discovery.app_count()),
        discovery.planned_count()
    );
    for app in &discovery.apps {
        println!(
            "{:<28} {:<20} {:<8} {:<6} {}",
            app.id(),
            app.name(),
            app.manifest
                .category
                .map(|c| c.as_str().to_string())
                .unwrap_or_else(|| "-".into()),
            app.manifest
                .status
                .map(|s| s.as_str().to_string())
                .unwrap_or_else(|| "-".into()),
            app.manifest.version
        );
    }
    println!();
    println!("Coming soon:");
    for entry in &discovery.roadmap {
        println!("  {:<28} {}", entry.id, entry.name);
    }
    if !discovery.errors.is_empty() {
        println!();
        println!("warnings:");
        for error in &discovery.errors {
            println!("  - {error}");
        }
    }
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}
