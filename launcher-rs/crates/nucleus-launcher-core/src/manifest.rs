//! The `nucleus.app.json` manifest model — mirrors `schemas/app-manifest.schema.json`
//! (serde `camelCase` renames) plus a hand-rolled parser/validator.

use serde::{Deserialize, Serialize};

use crate::error::ManifestError;

/// What a tool is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Category {
    Image,
    Vector,
    Video,
    Audio,
    Pdf,
    Text,
    Layout,
    Data,
    Cad,
    System,
    Other,
}

impl Category {
    /// Lowercase key as written in the manifest.
    pub fn as_str(self) -> &'static str {
        match self {
            Category::Image => "image",
            Category::Vector => "vector",
            Category::Video => "video",
            Category::Audio => "audio",
            Category::Pdf => "pdf",
            Category::Text => "text",
            Category::Layout => "layout",
            Category::Data => "data",
            Category::Cad => "cad",
            Category::System => "system",
            Category::Other => "other",
        }
    }
}

/// Release maturity of an app, as written in the manifest.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ReleaseStatus {
    Stable,
    Beta,
    Alpha,
    Planned,
}

impl ReleaseStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            ReleaseStatus::Stable => "stable",
            ReleaseStatus::Beta => "beta",
            ReleaseStatus::Alpha => "alpha",
            ReleaseStatus::Planned => "planned",
        }
    }
}

/// How the app is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchKind {
    /// A Node local HTTP server. `args` runs under node with `PORT` set.
    Server,
    /// Open a URL in the nucleus window shell (or the default browser).
    Url,
    /// Run a command, no window shell and no health probe.
    Command,
}

impl LaunchKind {
    pub fn as_str(self) -> &'static str {
        match self {
            LaunchKind::Server => "server",
            LaunchKind::Url => "url",
            LaunchKind::Command => "command",
        }
    }
}

/// A permission an app declares. `Unknown` keeps forward compatibility with
/// permissions the launcher has not seen yet (behaviour identical, display only).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Fs,
    Network,
    Ai,
    Clipboard,
    #[serde(other)]
    Unknown,
}

impl Permission {
    pub fn as_str(self) -> &'static str {
        match self {
            Permission::Fs => "fs",
            Permission::Network => "network",
            Permission::Ai => "ai",
            Permission::Clipboard => "clipboard",
            Permission::Unknown => "other",
        }
    }
}

/// Update channel as declared by the manifest (`updateChannel`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Channel {
    Stable,
    Beta,
    #[serde(other)]
    Other,
}

impl Channel {
    pub fn as_str(self) -> &'static str {
        match self {
            Channel::Stable => "stable",
            Channel::Beta => "beta",
            Channel::Other => "other",
        }
    }
}

/// The `launch` object: how an app is spawned and where its window points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchSpec {
    pub kind: LaunchKind,
    /// Executable to run. Defaults to the current node binary.
    pub command: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    pub url: Option<String>,
    /// Preferred port. `0`/absent → the launcher picks a free one.
    pub port: Option<u16>,
    /// Open a dedicated app window. Default `true`.
    pub window: Option<bool>,
    /// Path polled until the server answers. Default `/`.
    pub ready_path: Option<String>,
}

/// An app manifest: every field of `nucleus.app.json`.
///
/// The four schema-required string fields carry `#[serde(default)]` so a
/// missing key surfaces as a typed [`ManifestError::MissingField`] from
/// [`parse_manifest`] instead of a generic JSON error.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Manifest {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub core_api_version: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub category: Option<Category>,
    pub status: Option<ReleaseStatus>,
    pub inspired_by: Option<String>,
    pub icon: Option<String>,
    pub entry: Option<String>,
    pub protocol_entry: Option<String>,
    pub mcp_entry: Option<String>,
    pub launch: Option<LaunchSpec>,
    pub protocol_range: Option<String>,
    #[serde(default)]
    pub permissions: Vec<Permission>,
    #[serde(default)]
    pub capabilities: Vec<String>,
    pub min_shell: Option<String>,
    pub update_channel: Option<Channel>,
    pub integrity: Option<String>,
    pub signature: Option<String>,
}

impl Manifest {
    /// The launch spec. `None` only if the manifest bypassed [`parse_manifest`]
    /// (which requires `launch`).
    pub fn launch_spec(&self) -> Option<&LaunchSpec> {
        self.launch.as_ref()
    }
}

/// The dotted-id pattern `^[a-z0-9]+(\.[a-z0-9-]+)+$`, hand-rolled (no regex dep).
/// First segment has no hyphens; further segments may contain `-`.
pub fn is_valid_id(id: &str) -> bool {
    let mut parts = id.split('.');
    let Some(first) = parts.next() else {
        return false;
    };
    let first_ok = !first.is_empty()
        && first
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit());
    if !first_ok {
        return false;
    }
    let mut rest = 0usize;
    for part in parts {
        rest += 1;
        let ok = !part.is_empty()
            && part
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
        if !ok {
            return false;
        }
    }
    rest > 0
}

/// Parse and validate a manifest file's contents.
///
/// Order of checks: JSON parse → required fields (`id`, `name`, `version`,
/// `coreApiVersion`, `launch`) → id pattern → [`validate`].
pub fn parse_manifest(json: &str) -> Result<Manifest, ManifestError> {
    let manifest: Manifest = serde_json::from_str(json)?;

    for (field, value) in [
        ("id", manifest.id.as_str()),
        ("name", manifest.name.as_str()),
        ("version", manifest.version.as_str()),
        ("coreApiVersion", manifest.core_api_version.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(ManifestError::MissingField {
                field: field.to_string(),
            });
        }
    }
    if manifest.launch.is_none() {
        return Err(ManifestError::MissingField {
            field: "launch".to_string(),
        });
    }
    if !is_valid_id(&manifest.id) {
        return Err(ManifestError::InvalidId {
            id: manifest.id.clone(),
        });
    }
    validate(&manifest)?;
    Ok(manifest)
}

/// Semantic checks on an otherwise well-typed manifest. Mirrors the TS
/// reference: `kind=server` needs `args`, `kind=url` needs `url`.
pub fn validate(manifest: &Manifest) -> Result<(), ManifestError> {
    let mut problems = Vec::new();
    if let Some(launch) = &manifest.launch {
        match launch.kind {
            LaunchKind::Server => {
                if launch.args.is_empty() || launch.args.iter().any(|a| a.is_empty()) {
                    problems.push("launch.args must be a non-empty list for kind=server".into());
                }
            }
            LaunchKind::Url => {
                let url = launch.url.as_deref().unwrap_or("");
                if url.is_empty() {
                    problems.push("launch.url is required for kind=url".into());
                }
            }
            LaunchKind::Command => {}
        }
    }
    if problems.is_empty() {
        Ok(())
    } else {
        Err(ManifestError::Validation { problems })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The real `apps/pdf-workbench/nucleus.app.json`.
    const PDF_MANIFEST: &str = r#"{
      "id": "nucleus.pdf-workbench",
      "name": "PDF Workbench",
      "version": "0.1.0",
      "coreApiVersion": "1",
      "description": "Merge, reorder, rotate and export PDF pages.",
      "author": "Nucleus",
      "category": "pdf",
      "status": "beta",
      "inspiredBy": "Acrobat",
      "icon": "icon.svg",
      "launch": {
        "kind": "server",
        "args": ["dist/server.js"],
        "window": true,
        "readyPath": "/api/state"
      },
      "permissions": ["fs"],
      "capabilities": ["pdf.edit", "pdf.export"],
      "minShell": "0.1.0",
      "updateChannel": "stable"
    }"#;

    #[test]
    fn parses_real_pdf_manifest() {
        let m = parse_manifest(PDF_MANIFEST).expect("pdf-workbench must parse");
        assert_eq!(m.id, "nucleus.pdf-workbench");
        assert_eq!(m.name, "PDF Workbench");
        assert_eq!(m.version, "0.1.0");
        assert_eq!(m.core_api_version, "1");
        assert_eq!(m.category, Some(Category::Pdf));
        assert_eq!(m.status, Some(ReleaseStatus::Beta));
        assert_eq!(m.inspired_by.as_deref(), Some("Acrobat"));
        assert_eq!(m.min_shell.as_deref(), Some("0.1.0"));
        assert_eq!(m.update_channel, Some(Channel::Stable));
        assert_eq!(m.permissions, vec![Permission::Fs]);
        let launch = m.launch_spec().expect("launch required");
        assert_eq!(launch.kind, LaunchKind::Server);
        assert_eq!(launch.args, vec!["dist/server.js".to_string()]);
        assert_eq!(launch.ready_path.as_deref(), Some("/api/state"));
        assert_eq!(launch.window, Some(true));
    }

    #[test]
    fn rejects_invalid_id() {
        let mut v = serde_json::from_str::<serde_json::Value>(PDF_MANIFEST).unwrap();
        v["id"] = serde_json::json!("Bogus");
        let err = parse_manifest(&v.to_string()).unwrap_err();
        assert!(
            matches!(err, ManifestError::InvalidId { .. }),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_bad_id_with_hyphen_in_first_segment() {
        let mut v = serde_json::from_str::<serde_json::Value>(PDF_MANIFEST).unwrap();
        v["id"] = serde_json::json!("nucleus-app");
        assert!(matches!(
            parse_manifest(&v.to_string()),
            Err(ManifestError::InvalidId { .. })
        ));
    }

    #[test]
    fn rejects_missing_name() {
        let mut v = serde_json::from_str::<serde_json::Value>(PDF_MANIFEST).unwrap();
        v.as_object_mut().unwrap().remove("name");
        let err = parse_manifest(&v.to_string()).unwrap_err();
        assert!(
            matches!(err, ManifestError::MissingField { ref field } if field == "name"),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_missing_launch() {
        let mut v = serde_json::from_str::<serde_json::Value>(PDF_MANIFEST).unwrap();
        v.as_object_mut().unwrap().remove("launch");
        let err = parse_manifest(&v.to_string()).unwrap_err();
        assert!(
            matches!(err, ManifestError::MissingField { ref field } if field == "launch"),
            "got {err:?}"
        );
    }

    #[test]
    fn rejects_server_without_args() {
        let mut v = serde_json::from_str::<serde_json::Value>(PDF_MANIFEST).unwrap();
        v["launch"]["args"] = serde_json::json!([]);
        let err = parse_manifest(&v.to_string()).unwrap_err();
        assert!(matches!(err, ManifestError::Validation { .. }));
    }

    #[test]
    fn rejects_url_kind_without_url() {
        let json = r#"{
          "id": "nucleus.site",
          "name": "Site",
          "version": "0.1.0",
          "coreApiVersion": "1",
          "launch": { "kind": "url" }
        }"#;
        let err = parse_manifest(json).unwrap_err();
        assert!(matches!(err, ManifestError::Validation { .. }));
    }

    #[test]
    fn accepts_minimal_command_manifest() {
        let json = r#"{
          "id": "nucleus.watch",
          "name": "Watch",
          "version": "0.1.0",
          "coreApiVersion": "1",
          "launch": { "kind": "command", "command": "echo", "args": ["hi"] }
        }"#;
        let m = parse_manifest(json).expect("command manifest must parse");
        assert_eq!(m.launch_spec().unwrap().kind, LaunchKind::Command);
    }

    #[test]
    fn ignores_unknown_fields() {
        let mut v = serde_json::from_str::<serde_json::Value>(PDF_MANIFEST).unwrap();
        v["futureField"] = serde_json::json!({"nested": true});
        assert!(parse_manifest(&v.to_string()).is_ok());
    }

    #[test]
    fn unknown_permission_maps_to_unknown() {
        let mut v = serde_json::from_str::<serde_json::Value>(PDF_MANIFEST).unwrap();
        v["permissions"] = serde_json::json!(["fs", "future-thing"]);
        let m = parse_manifest(&v.to_string()).unwrap();
        assert_eq!(m.permissions, vec![Permission::Fs, Permission::Unknown]);
    }

    #[test]
    fn is_valid_id_table() {
        assert!(is_valid_id("nucleus.pdf-workbench"));
        assert!(is_valid_id("a.b"));
        assert!(is_valid_id("nucleus.launcher"));
        assert!(!is_valid_id("nucleus"));
        assert!(!is_valid_id("nucleus-app"));
        assert!(!is_valid_id(".nucleus.app"));
        assert!(!is_valid_id("nucleus..app"));
        assert!(!is_valid_id("Nucleus.app"));
        assert!(!is_valid_id("nucleus.app."));
        assert!(!is_valid_id(""));
    }
}
