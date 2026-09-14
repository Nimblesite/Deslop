//! Parsed form of the `--embeddings={auto,required,off}` flag.
//!
//! The enum lives in the core crate so the pipeline (not the CLI binary)
//! is the authority on embedding-layer behaviour. Keeping the parse
//! logic here also lets future callers (MCP/LSP daemon) reuse it
//! without re-implementing the string → variant match.

use thiserror::Error;

/// How aggressively the pipeline should run the embedding pass.
///
/// - `Off`: skip embeddings entirely; fused scores rely on the two
///   deterministic signals. The shipped CLI default ([FUSED-SIGNALS-
///   THREE-LAYER]): the batch tool must produce a report on a machine
///   that has no reachable provider, and a first run must never block
///   on one.
/// - `Auto`: probe the provider. If reachable, run embeddings; if not,
///   `tracing::warn!` and continue with two signals. The recommended
///   mode for interactive surfaces (VSIX/LSP) where a local provider
///   is expected and the recall it buys is worth probing for.
/// - `Required`: probe the provider. Fail hard when it is not
///   reachable. For CI runs that mandate Type-4 recall.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingMode {
    /// Skip embeddings entirely.
    Off,
    /// Use embeddings when reachable; warn and fall back otherwise.
    Auto,
    /// Use embeddings; fail hard when the provider is unreachable.
    Required,
}

impl EmbeddingMode {
    /// Short CLI-facing string representation. Inverse of
    /// [`EmbeddingMode::from_str`]; kept in sync.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Auto => "auto",
            Self::Required => "required",
        }
    }
}

/// Error returned by [`EmbeddingMode::from_str`] when the input is
/// not one of the three allowed variants.
#[derive(Debug, Error)]
#[error("expected one of auto/required/off, got {value:?}")]
pub struct ParseModeError {
    /// The string the user supplied.
    pub value: String,
}

impl std::str::FromStr for EmbeddingMode {
    type Err = ParseModeError;
    fn from_str(source: &str) -> Result<Self, Self::Err> {
        match source {
            "off" => Ok(Self::Off),
            "auto" => Ok(Self::Auto),
            "required" => Ok(Self::Required),
            other => Err(ParseModeError {
                value: other.to_owned(),
            }),
        }
    }
}
