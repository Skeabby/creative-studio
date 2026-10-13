//! Typed errors for manifest parsing.

use thiserror::Error;

/// Errors produced while parsing or validating a `nucleus.app.json` manifest.
#[derive(Debug, Error)]
pub enum ManifestError {
    /// The file was not valid JSON.
    #[error("invalid JSON: {0}")]
    Parse(#[from] serde_json::Error),

    /// The `id` field does not match the required dotted pattern.
    #[error(r"invalid id `{id}`: expected the form `nucleus.app` (^[a-z0-9]+(\.[a-z0-9-]+)+$)")]
    InvalidId { id: String },

    /// A field the launcher requires is missing or empty.
    #[error("missing or empty required field `{field}`")]
    MissingField { field: String },

    /// One or more semantic problems with an otherwise well-typed manifest.
    #[error("invalid manifest: {}", .problems.join("; "))]
    Validation { problems: Vec<String> },
}
