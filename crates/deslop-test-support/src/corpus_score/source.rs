//! [CORPUS-HISTORY-SOURCE] The source an engine was built from: always a
//! commit, and — when that commit is a tagged release — its version.
//!
//! A version is only a name for a commit, and a name can be re-pointed; the
//! commit is the source itself. So every recorded result names the commit, and
//! names the version beside it when there is one.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Hex digits a commit is shortened to wherever a table shows it.
const SHORT_COMMIT: usize = 12;

/// The source one engine was built from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Source {
    /// The full commit id the engine was built from.
    pub commit: String,
    /// The release tag pointing at that commit, absent for an untagged commit.
    pub version: Option<String>,
}

impl Source {
    /// The source a run manifest's engine entry names, absent when it names no
    /// commit.
    #[must_use]
    pub fn of_engine(engine: &Value) -> Option<Self> {
        let field = |name: &str| {
            engine
                .get(name)
                .and_then(Value::as_str)
                .filter(|text| !text.is_empty())
                .map(str::to_owned)
        };
        Some(Self {
            commit: field("commit")?,
            version: field("version"),
        })
    }

    /// The commit, shortened for a table.
    #[must_use]
    pub fn short_commit(&self) -> &str {
        self.commit.get(..SHORT_COMMIT).unwrap_or(&self.commit)
    }
}
