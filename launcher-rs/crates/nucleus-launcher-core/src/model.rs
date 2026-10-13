//! Aggregated app metadata assembled by discovery.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::catalog::CatalogEntry;
use crate::installed::InstalledRecord;
use crate::manifest::{LaunchSpec, Manifest};

/// Where an app was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AppSource {
    /// Lives in `<repo>/apps` — always present, never uninstalled.
    Repo,
    /// Lives in `%APPDATA%\Nucleus\apps` — installed from the catalogue (M2).
    UserData,
    /// In the roadmap only; not installed yet.
    Roadmap,
}

impl AppSource {
    pub fn as_str(self) -> &'static str {
        match self {
            AppSource::Repo => "repo",
            AppSource::UserData => "user data",
            AppSource::Roadmap => "roadmap",
        }
    }
}

/// One discovered app, with everything the launcher needs to display and run it.
#[derive(Debug, Clone)]
pub struct AppMeta {
    pub manifest: Manifest,
    /// The app directory, used as the working directory when spawning.
    pub dir: PathBuf,
    pub manifest_path: PathBuf,
    pub source: AppSource,
    /// Install record. M2 reads this from `installed.json`; M1 synthesises a
    /// record for locally-discovered apps so they are treated as installed.
    pub installed: Option<InstalledRecord>,
    /// Catalogue entry when the app comes from the offline catalogue (M2).
    pub catalog: Option<CatalogEntry>,
}

impl AppMeta {
    pub fn id(&self) -> &str {
        &self.manifest.id
    }

    pub fn name(&self) -> &str {
        &self.manifest.name
    }

    pub fn launch_spec(&self) -> Option<&LaunchSpec> {
        self.manifest.launch_spec()
    }
}
