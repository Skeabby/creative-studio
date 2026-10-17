//! Locating and loading the offline catalogue (`catalog.json`).
//!
//! Precedence:
//! 1. the `NUCLEUS_CATALOG` environment variable — a path, `file://` or
//!    `http(s)://` URL, fetched with [`crate::net::fetch_catalog`];
//! 2. `<data_dir>/catalog.json` — a catalogue previously placed on disk;
//! 3. the copy embedded at build time from `launcher-rs/mirror/catalog.json`,
//!    so the launcher always has *some* catalogue and works fully offline.
//!
//! Loading never fails: a source that cannot be read logs a warning and the
//! next source is tried; the embedded catalogue is always available.

use std::path::Path;

use nucleus_launcher_core::{Catalog, parse_catalog};

/// The catalogue compiled into the binary (repo `launcher-rs/mirror/catalog.json`).
pub const EMBEDDED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../mirror/catalog.json"
));

/// The embedded catalogue. Parsing is lenient, so this never fails.
pub fn embedded() -> Catalog {
    parse_catalog(EMBEDDED)
}

/// Load the catalogue, returning it with any non-fatal warnings.
pub fn load(data_dir: &Path) -> (Catalog, Vec<String>) {
    let mut warnings = Vec::new();

    if let Ok(source) = std::env::var("NUCLEUS_CATALOG") {
        let source = source.trim();
        if !source.is_empty() {
            match crate::net::fetch_catalog(source) {
                Ok(catalog) => return (resolve_relative_urls(catalog, source), warnings),
                Err(err) => warnings.push(format!("NUCLEUS_CATALOG: {err}")),
            }
        }
    }

    let local = data_dir.join("catalog.json");
    if local.is_file() {
        let source = local.to_string_lossy();
        match crate::net::fetch_catalog(&source) {
            Ok(catalog) => return (resolve_relative_urls(catalog, &source), warnings),
            Err(err) => warnings.push(format!("{}: {err}", local.display())),
        }
    }

    (embedded(), warnings)
}

/// Rewrite every app `url` that is a relative path into an absolute path
/// resolved against the catalogue's own directory. This lets a catalogue be a
/// plain folder on disk (`mirror/catalog.json` next to `mirror/apps/*`), which
/// is what makes `NUCLEUS_CATALOG=…/catalog.json` install **offline**.
pub(crate) fn resolve_relative_urls(mut catalog: Catalog, source: &str) -> Catalog {
    let Some(base) = crate::net::local_path(source)
        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
    else {
        return catalog;
    };
    for entry in &mut catalog.apps {
        if let Some(url) = &entry.url
            && !url.contains("://")
            && !Path::new(url).is_absolute()
        {
            entry.url = Some(base.join(url).to_string_lossy().into_owned());
        }
    }
    catalog
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!(
            "nucleus-catalogue-{label}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_embedded_catalogue_parses() {
        // The embedded file must be valid enough to parse and carry the
        // metadata the launcher relies on; a broken one would silently yield an
        // empty catalogue, defeating M2.
        let catalog = embedded();
        assert!(
            catalog.generated_at.is_some(),
            "embedded catalogue has generatedAt"
        );
        assert!(
            !catalog.suite_version.is_empty(),
            "embedded catalogue has suiteVersion"
        );
    }

    #[test]
    fn loads_a_catalogue_from_the_data_dir() {
        if std::env::var("NUCLEUS_CATALOG").is_ok() {
            return; // the env override legitimately takes precedence
        }
        let dir = scratch("data");
        fs::write(
            dir.join("catalog.json"),
            r#"{"suiteVersion":"9.9.9","apps":[]}"#,
        )
        .unwrap();
        let (catalog, warnings) = load(&dir);
        assert_eq!(catalog.suite_version, "9.9.9");
        assert!(warnings.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn relative_asset_urls_resolve_against_the_catalogue_dir() {
        let dir = scratch("relative");
        let catalog_path = dir.join("catalog.json");
        fs::write(
            &catalog_path,
            r#"{"suiteVersion":"1","apps":[
              {"id":"nucleus.notes","repo":"Skeabby/nucleus","releaseAsset":"notes.nucleusapp",
               "url":"apps/notes.nucleusapp","version":"0.1.0"}
            ]}"#,
        )
        .unwrap();
        let (catalog, _) = load(&dir);
        let url = catalog.apps[0].url.as_deref().unwrap();
        assert!(
            Path::new(url).is_absolute(),
            "resolved to an absolute path: {url}"
        );
        assert!(url.ends_with("notes.nucleusapp"));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn falls_back_to_the_embedded_catalogue() {
        if std::env::var("NUCLEUS_CATALOG").is_ok() {
            return;
        }
        let dir = scratch("empty");
        let (catalog, warnings) = load(&dir);
        assert_eq!(catalog.suite_version, embedded().suite_version);
        assert!(warnings.is_empty());
        let _ = fs::remove_dir_all(&dir);
    }
}
