//! `.nucleusapp` integrity: SHA-256 hashing, the `DIGESTS.json` ledger, and
//! entry-path sanitiser. Pure logic only — file I/O lives in the launcher crate.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};
use thiserror::Error;

/// The per-file digest ledger's file name inside a `.nucleusapp` archive.
pub const DIGESTS_FILE: &str = "DIGESTS.json";

/// Largest manifest / digest ledger the launcher will read into memory (1 MiB).
pub const MAX_META_BYTES: u64 = 1024 * 1024;

const HEX: &[u8; 16] = b"0123456789abcdef";

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

/// Lowercase hex SHA-256 of `bytes`.
pub fn sha256_hex(bytes: &[u8]) -> String {
    to_hex(&Sha256::digest(bytes))
}

/// Normalise a digest string to a bare lowercase 64-char hex, accepting an
/// optional `sha256:` prefix. `None` when it is not valid SHA-256 hex.
pub fn normalize_sha256(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    let bare = trimmed
        .strip_prefix("sha256:")
        .or_else(|| trimmed.strip_prefix("SHA256:"))
        .unwrap_or(trimmed);
    if bare.len() == 64 && bare.chars().all(|c| c.is_ascii_hexdigit()) {
        Some(bare.to_ascii_lowercase())
    } else {
        None
    }
}

/// The parsed `DIGESTS.json`: sanitised archive entry path → lowercase hex SHA-256.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Digests {
    pub files: BTreeMap<String, String>,
}

impl Digests {
    /// The expected digest for a sanitised entry path, if any.
    pub fn get(&self, path: &str) -> Option<&str> {
        self.files.get(path).map(String::as_str)
    }

    /// Does `bytes` match the digest recorded for `path`?
    pub fn matches(&self, path: &str, bytes: &[u8]) -> bool {
        match self.get(path) {
            Some(expected) => sha256_hex(bytes) == expected,
            None => false,
        }
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Errors from parsing a `DIGESTS.json` ledger.
#[derive(Debug, Error)]
pub enum DigestError {
    #[error("invalid DIGESTS.json: {0}")]
    Parse(#[from] serde_json::Error),

    #[error("DIGESTS.json lists no files")]
    Empty,

    #[error("invalid sha256 for `{path}`: `{value}`")]
    InvalidHash { path: String, value: String },

    #[error("unsafe path `{path}` in DIGESTS.json")]
    UnsafePath { path: String },
}

/// Parse a `DIGESTS.json` ledger, rejecting any non-SHA-256 value and any key
/// that is not a safe relative path (the same rules as zip entries).
pub fn parse_digests(json: &str) -> Result<Digests, DigestError> {
    // Tolerate a Windows BOM (Notepad / PowerShell `Set-Content` add one).
    let json = json.strip_prefix('\u{feff}').unwrap_or(json);
    let raw: BTreeMap<String, String> = serde_json::from_str(json)?;
    if raw.is_empty() {
        return Err(DigestError::Empty);
    }
    let mut files = BTreeMap::new();
    for (path, value) in raw {
        let clean = sanitize_entry_path(&path).ok_or_else(|| DigestError::UnsafePath {
            path: path.clone(),
        })?;
        let hash = normalize_sha256(&value).ok_or(DigestError::InvalidHash {
            path: path.clone(),
            value,
        })?;
        files.insert(clean, hash);
    }
    Ok(Digests { files })
}

/// Is `part` a Windows reserved device name (`CON`, `NUL`, `COM1`, …)?
///
/// Win32 rewrites `NUL`/`CON`/`COM1` and friends to device paths, so a payload
/// file with such a name could be silently discarded or block — never write it.
fn is_reserved_device_name(part: &str) -> bool {
    let base = part.split(['.', ':']).next().unwrap_or(part);
    let upper = base.trim_end().to_ascii_uppercase();
    if matches!(
        upper.as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$"
    ) {
        return true;
    }
    let bytes = upper.as_bytes();
    bytes.len() == 4
        && (upper.starts_with("COM") || upper.starts_with("LPT"))
        && bytes[3].is_ascii_digit()
        && bytes[3] != b'0'
}

/// Sanitise a zip entry path into a safe, forward-slash relative path.
///
/// Rejects absolute paths, Windows drive/UNC prefixes (`:`), and *any* path
/// component that Windows would normalise into a traversal or a device name:
/// components ending in a dot or space (`.. `, `...`, `a.`), and reserved
/// device names. This is the classic zip-slip guard hardened for the
/// "MagicDot" class (CVE-2023-36396) where Win32 strips trailing dots/spaces.
pub fn sanitize_entry_path(name: &str) -> Option<String> {
    let unified = name.replace('\\', "/");
    if unified.is_empty() || unified.starts_with('/') || unified.contains(':') {
        return None;
    }
    let mut parts: Vec<&str> = Vec::new();
    for part in unified.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        // Win32 silently strips trailing dots/spaces and mangles leading spaces,
        // so `.. ` → `..` and `a.` → `a` — reject rather than alias.
        if part.starts_with(' ') || part.ends_with('.') || part.ends_with(' ') {
            return None;
        }
        if is_reserved_device_name(part) {
            return None;
        }
        parts.push(part);
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("/"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256_hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn normalize_accepts_prefixed_and_cased() {
        let hex = "BA7816BF8F01CFEA414140DE5DAE2223B00361A396177A9CB410FF61F20015AD";
        let lower = hex.to_ascii_lowercase();
        assert_eq!(normalize_sha256(hex).as_deref(), Some(lower.as_str()));
        assert_eq!(
            normalize_sha256(&format!("sha256:{lower}")).as_deref(),
            Some(lower.as_str())
        );
        assert_eq!(normalize_sha256("nope"), None);
        assert_eq!(normalize_sha256(&"a".repeat(63)), None);
    }

    #[test]
    fn parse_digests_roundtrip_and_rejects_bad() {
        let good = r#"{ "payload/dist/server.js": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" }"#;
        let digests = parse_digests(good).expect("valid ledger");
        assert_eq!(digests.len(), 1);
        assert!(digests.matches("payload/dist/server.js", b"abc"));

        assert!(matches!(parse_digests("{}"), Err(DigestError::Empty)));
        assert!(matches!(
            parse_digests(r#"{"a":"not-a-hash"}"#),
            Err(DigestError::InvalidHash { .. })
        ));
        assert!(parse_digests("not json").is_err());

        let with_bom = format!("\u{feff}{good}");
        assert!(parse_digests(&with_bom).is_ok());
    }

    #[test]
    fn ledger_keys_are_sanitised() {
        // A backslash key must normalise to the same entry as a slash key.
        let json = r#"{ "payload\\win\\a.js": "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad" }"#;
        let digests = parse_digests(json).expect("sanitisable key");
        assert!(digests.get("payload/win/a.js").is_some());
        assert!(matches!(
            parse_digests(r#"{"payload/../evil":"sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"}"#),
            Err(DigestError::UnsafePath { .. })
        ));
    }

    #[test]
    fn sanitize_blocks_traversal_and_absolute() {
        assert_eq!(
            sanitize_entry_path("payload/dist/server.js").as_deref(),
            Some("payload/dist/server.js")
        );
        assert_eq!(
            sanitize_entry_path("payload\\win\\path.js").as_deref(),
            Some("payload/win/path.js")
        );
        assert_eq!(
            sanitize_entry_path("./payload/./a").as_deref(),
            Some("payload/a")
        );
        assert_eq!(sanitize_entry_path("payload/../evil"), None);
        assert_eq!(sanitize_entry_path("/etc/passwd"), None);
        assert_eq!(sanitize_entry_path("C:/Windows/system32"), None);
        assert_eq!(sanitize_entry_path(".."), None);
        assert_eq!(sanitize_entry_path(""), None);
    }

    #[test]
    fn sanitize_blocks_windows_normalisation_tricks() {
        // MagicDot class: trailing dots/spaces alias `.`/`..` and other names.
        for evil in [
            "payload/.. /evil.txt",
            "payload/.. .\\evil.txt",
            "payload/.../evil.txt",
            "payload/../evil.txt",
            "payload/a./b",
            "payload/trailing ",
            ".. ",
            "...",
        ] {
            assert_eq!(sanitize_entry_path(evil), None, "should reject `{evil}`");
        }
        // Reserved device names (a payload file that Win32 would rewrite).
        for device in [
            "payload/NUL",
            "payload/nul.txt",
            "payload/CON",
            "payload/COM1.js",
            "payload/sub/LPT9",
        ] {
            assert_eq!(sanitize_entry_path(device), None, "should reject `{device}`");
        }
        // Ordinary names that merely contain a dot are fine.
        assert_eq!(
            sanitize_entry_path("payload/dist/server.js").as_deref(),
            Some("payload/dist/server.js")
        );
        assert_eq!(
            sanitize_entry_path("payload/COM.js").as_deref(),
            Some("payload/COM.js")
        );
    }
}
