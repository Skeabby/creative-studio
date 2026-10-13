//! Roadmap entries ("coming soon") and the merge of discovered + planned apps.

use serde::{Deserialize, Serialize};

use crate::manifest::Category;
use crate::model::AppMeta;

/// One "coming soon" entry, mirroring `apps/launcher/roadmap.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoadmapEntry {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub category: Option<Category>,
    pub inspired_by: Option<String>,
    pub emoji: Option<String>,
    /// `#RRGGBB` used as the planned tile colour.
    pub color: Option<String>,
}

/// A card shown in the launcher: either a real app or a planned one.
#[derive(Debug, Clone)]
pub enum Card {
    App(Box<AppMeta>),
    Planned(RoadmapEntry),
}

impl Card {
    pub fn id(&self) -> &str {
        match self {
            Card::App(app) => app.id(),
            Card::Planned(entry) => &entry.id,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Card::App(app) => app.name(),
            Card::Planned(entry) => &entry.name,
        }
    }

    pub fn description(&self) -> Option<&str> {
        match self {
            Card::App(app) => app.manifest.description.as_deref(),
            Card::Planned(entry) => entry.description.as_deref(),
        }
    }

    pub fn category(&self) -> Option<Category> {
        match self {
            Card::App(app) => app.manifest.category,
            Card::Planned(entry) => entry.category,
        }
    }

    pub fn emoji(&self) -> Option<&str> {
        match self {
            Card::App(_) => None,
            Card::Planned(entry) => entry.emoji.as_deref(),
        }
    }

    /// `#RRGGBB` accent for the planned tile.
    pub fn color_hex(&self) -> Option<&str> {
        match self {
            Card::App(_) => None,
            Card::Planned(entry) => entry.color.as_deref(),
        }
    }

    pub fn as_app(&self) -> Option<&AppMeta> {
        match self {
            Card::App(app) => Some(app.as_ref()),
            Card::Planned(_) => None,
        }
    }
}

/// Combine discovered apps with roadmap entries into a single card list.
///
/// Apps come first (in discovery order); roadmap entries whose id matches an
/// existing app are dropped (dedup by id), so a planned app that ships becomes
/// a real card automatically.
pub fn merge_roadmap(apps: &[AppMeta], roadmap: &[RoadmapEntry]) -> Vec<Card> {
    let mut cards: Vec<Card> = apps
        .iter()
        .cloned()
        .map(|a| Card::App(Box::new(a)))
        .collect();
    let mut seen: std::collections::BTreeSet<&str> = apps.iter().map(AppMeta::id).collect();
    for entry in roadmap {
        if seen.insert(&entry.id) {
            cards.push(Card::Planned(entry.clone()));
        }
    }
    cards
}

#[cfg(test)]
mod tests {
    use super::*;

    fn app(id: &str) -> AppMeta {
        AppMeta {
            manifest: crate::manifest::parse_manifest(&format!(
                r#"{{"id":"{id}","name":"{id}","version":"0.1.0","coreApiVersion":"1","launch":{{"kind":"server","args":["server.js"]}}}}"#
            ))
            .unwrap(),
            dir: std::path::PathBuf::from("."),
            manifest_path: std::path::PathBuf::from("."),
            source: crate::model::AppSource::Repo,
            installed: None,
            catalog: None,
        }
    }

    fn planned(id: &str) -> RoadmapEntry {
        RoadmapEntry {
            id: id.into(),
            name: id.into(),
            description: None,
            category: None,
            inspired_by: None,
            emoji: None,
            color: None,
        }
    }

    #[test]
    fn parses_real_roadmap() {
        let json = r##"[
          { "id": "nucleus.image-editor", "name": "Image Editor",
            "description": "Layers, masks, non-destructive adjustments and export.",
            "category": "image", "inspiredBy": "Photoshop",
            "emoji": "🖼️", "color": "#6D5EF8" }
        ]"##;
        let entries: Vec<RoadmapEntry> = serde_json::from_str(json).unwrap();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, "nucleus.image-editor");
        assert_eq!(entries[0].category, Some(Category::Image));
        assert_eq!(entries[0].emoji.as_deref(), Some("🖼️"));
        assert_eq!(entries[0].color.as_deref(), Some("#6D5EF8"));
    }

    #[test]
    fn merge_preserves_order_and_dedups() {
        let apps = vec![app("nucleus.launcher"), app("nucleus.notes")];
        let roadmap = vec![planned("nucleus.notes"), planned("nucleus.audio")];
        let cards = merge_roadmap(&apps, &roadmap);
        assert_eq!(cards.len(), 3);
        assert!(matches!(cards[0], Card::App(_)));
        assert!(matches!(cards[1], Card::App(_)));
        assert!(matches!(cards[2], Card::Planned(_)));
        // Deduplicated id == "nucleus.notes" dropped.
        let planned_ids: Vec<&str> = cards
            .iter()
            .filter_map(|c| match c {
                Card::Planned(e) => Some(e.id.as_str()),
                Card::App(_) => None,
            })
            .collect();
        assert_eq!(planned_ids, vec!["nucleus.audio"]);
    }
}
