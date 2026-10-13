//! Filesystem discovery: apps from every app root plus the roadmap.

use std::fs;
use std::path::{Path, PathBuf};

use nucleus_launcher_core::{
    AppMeta, AppSource, Card, InstalledRecord, RoadmapEntry, merge_roadmap, parse_manifest,
};

use crate::paths::Paths;

/// The result of discovery: app cards, roadmap cards and any non-fatal errors
/// encountered along the way (an unreadable manifest is skipped, never fatal).
#[derive(Debug, Clone)]
pub struct Discovery {
    /// Discovered apps, in discovery order (repo first, then user data).
    pub apps: Vec<AppMeta>,
    /// Merged `apps + upcoming` cards, in display order.
    pub cards: Vec<Card>,
    /// Raw roadmap entries loaded from `apps/launcher/roadmap.json`.
    pub roadmap: Vec<RoadmapEntry>,
    /// Reasons some manifests were skipped (e.g. invalid JSON).
    pub errors: Vec<String>,
}

impl Discovery {
    pub fn app(&self, id: &str) -> Option<&AppMeta> {
        self.apps.iter().find(|app| app.id() == id)
    }

    pub fn card(&self, id: &str) -> Option<&Card> {
        self.cards.iter().find(|card| card.id() == id)
    }

    pub fn app_count(&self) -> usize {
        self.apps.len()
    }

    pub fn planned_count(&self) -> usize {
        self.cards.iter().filter(|c| c.as_app().is_none()).count()
    }
}

/// Discover the whole suite: for each app root (in order) take the root itself
/// plus its immediate child directories, and for every candidate load
/// `<dir>/nucleus.app.json` **and** `<dir>/current/nucleus.app.json`
/// (first one wins per id). Then load and merge the roadmap.
pub fn discover(paths: &Paths) -> Discovery {
    let mut apps: Vec<AppMeta> = Vec::new();
    let mut errors: Vec<String> = Vec::new();

    for (root, source) in [
        (paths.app_roots[0].as_path(), AppSource::Repo),
        (paths.app_roots[1].as_path(), AppSource::UserData),
    ] {
        if !root.is_dir() {
            continue;
        }
        // The root itself is a candidate (harmless when it has no manifest).
        add_candidate(root, source, &mut apps, &mut errors);
        // Then each immediate child directory, sorted for deterministic order.
        let mut children = match fs::read_dir(root) {
            Ok(entries) => entries
                .filter_map(|entry| entry.ok())
                .map(|entry| entry.path())
                .filter(|path| path.is_dir() && !is_hidden(path))
                .collect::<Vec<PathBuf>>(),
            Err(err) => {
                errors.push(format!("cannot read {}: {err}", root.display()));
                continue;
            }
        };
        children.sort();
        for child in &children {
            add_candidate(child, source, &mut apps, &mut errors);
        }
    }

    let roadmap = load_roadmap(&paths.repo_root).unwrap_or_else(|err| {
        errors.push(err);
        Vec::new()
    });
    let cards = merge_roadmap(&apps, &roadmap);

    Discovery {
        apps,
        cards,
        roadmap,
        errors,
    }
}

/// Add `<dir>/nucleus.app.json` and `<dir>/current/nucleus.app.json` if both
/// exist and the id is new (first source wins).
fn add_candidate(dir: &Path, source: AppSource, apps: &mut Vec<AppMeta>, errors: &mut Vec<String>) {
    for manifest_name in ["nucleus.app.json", "current/nucleus.app.json"] {
        let manifest_path = dir.join(manifest_name);
        if !manifest_path.is_file() {
            continue;
        }
        let raw = match fs::read_to_string(&manifest_path) {
            Ok(raw) => raw,
            Err(err) => {
                errors.push(format!("cannot read {}: {err}", manifest_path.display()));
                continue;
            }
        };
        let manifest = match parse_manifest(&raw) {
            Ok(manifest) => manifest,
            Err(err) => {
                errors.push(format!("{}: {err}", manifest_path.display()));
                continue;
            }
        };
        let id = manifest.id.clone();
        if apps.iter().any(|app| app.id() == id) {
            continue; // first wins on id
        }
        // M1: synthesize an install record so locally-found apps are treated
        // as installed (Open is enabled). `Repo` records must never be
        // uninstalled; M2 reads real records from installed.json instead.
        let installed = Some(InstalledRecord {
            version: manifest.version.clone(),
            installed_at: String::new(),
            source,
            channel: manifest.update_channel.map(|c| c.as_str().to_string()),
        });
        apps.push(AppMeta {
            manifest,
            dir: manifest_path
                .parent()
                .expect("join keeps a parent")
                .to_path_buf(),
            manifest_path,
            source,
            installed,
            catalog: None,
        });
    }
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(|name| name.starts_with('.') || name.starts_with('_'))
        .unwrap_or(false)
}

/// Load `<repo>/apps/launcher/roadmap.json`. Missing/unreadable → error string.
fn load_roadmap(repo_root: &Path) -> Result<Vec<RoadmapEntry>, String> {
    let path = repo_root.join("apps").join("launcher").join("roadmap.json");
    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("cannot read roadmap {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("invalid roadmap {}: {err}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use nucleus_launcher_core::{AppMeta, AppSource};

    fn app(id: &str) -> AppMeta {
        AppMeta {
            manifest: nucleus_launcher_core::parse_manifest(&format!(
                r#"{{"id":"{id}","name":"{id}","version":"0.1.0","coreApiVersion":"1","launch":{{"kind":"server","args":["server.js"]}}}}"#
            ))
            .unwrap(),
            dir: PathBuf::from("."),
            manifest_path: PathBuf::from("."),
            source: AppSource::Repo,
            installed: None,
            catalog: None,
        }
    }

    #[test]
    fn hidden_dir_is_hidden() {
        assert!(is_hidden(Path::new(".git")));
        assert!(is_hidden(Path::new("_wip")));
        assert!(!is_hidden(Path::new("notes")));
    }

    #[test]
    fn card_and_app_lookup() {
        let apps = vec![app("nucleus.launcher")];
        let discovery = Discovery {
            apps: apps.clone(),
            cards: merge_roadmap(&apps, &[]),
            roadmap: vec![],
            errors: vec![],
        };
        assert!(discovery.app("nucleus.launcher").is_some());
        assert!(discovery.app("nope").is_none());
        assert_eq!(
            discovery.card("nucleus.launcher").unwrap().id(),
            "nucleus.launcher"
        );
        assert_eq!(discovery.app_count(), 1);
        assert_eq!(discovery.planned_count(), 0);
    }

    #[test]
    fn planned_count_only_counts_planned_cards() {
        let roadmap = vec![nucleus_launcher_core::RoadmapEntry {
            id: "nucleus.audio".into(),
            name: "Audio".into(),
            description: None,
            category: None,
            inspired_by: None,
            emoji: None,
            color: None,
        }];
        let apps = vec![app("nucleus.launcher")];
        let discovery = Discovery {
            apps: apps.clone(),
            cards: merge_roadmap(&apps, &roadmap),
            roadmap,
            errors: vec![],
        };
        assert_eq!(discovery.app_count(), 1);
        assert_eq!(discovery.planned_count(), 1);
    }
}
