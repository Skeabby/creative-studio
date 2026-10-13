//! Version comparison helpers.

use semver::Version;

/// Strip a single leading `v`/`V` (crates-style version tags).
fn strip_v(version: &str) -> &str {
    version
        .strip_prefix('v')
        .or_else(|| version.strip_prefix('V'))
        .unwrap_or(version)
}

/// Is `latest` a newer version than `installed`?
///
/// Compares as semver when both sides parse; otherwise falls back to a plain
/// string inequality so invalid-but-different versions are never equal.
pub fn is_update(latest: &str, installed: &str) -> bool {
    match (
        Version::parse(strip_v(latest)),
        Version::parse(strip_v(installed)),
    ) {
        (Ok(latest), Ok(installed)) => latest > installed,
        _ => latest != installed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn semver_comparison() {
        assert!(is_update("0.2.0", "0.1.0"));
        assert!(is_update("1.0.0", "0.9.9"));
        assert!(!is_update("0.1.0", "0.1.0"));
        assert!(!is_update("0.1.0", "0.2.0"));
    }

    #[test]
    fn strips_v_prefix() {
        assert!(is_update("v0.2.0", "0.1.0"));
        assert!(is_update("0.2.0", "v0.1.0"));
        assert!(!is_update("V0.1.0", "v0.1.0"));
    }

    #[test]
    fn prerelease_orders_before_release() {
        assert!(is_update("0.2.0", "0.2.0-rc.1"));
        assert!(!is_update("0.2.0-rc.1", "0.2.0"));
    }

    #[test]
    fn invalid_semver_falls_back_to_string() {
        assert!(is_update("1", "0.1.0")); // "1" not semver → string differs
        assert!(!is_update("0.1", "0.1")); // same string, both invalid
        assert!(is_update("0.2", "0.1"));
    }

    #[test]
    fn empty_latest_falls_back_to_string_inequality() {
        // A blank "latest" can't be compared; the string fallback flags it as
        // different. `derive_status` guards against this by skipping empty
        // versions before calling `is_update`.
        assert!(is_update("", "0.1.0"));
    }
}
