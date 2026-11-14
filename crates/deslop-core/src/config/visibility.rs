//! [EXCLUSION-CONFIG] `[visibility]` validation, and the refusal of the
//! retired `[ranking]` section.
//!
//! Clones rank by duplicated mass alone ([RANK-MASS-SUM]). The `[ranking]`
//! section once reweighted or hid whole categories; nothing reads it any
//! more, and a setting that is accepted and then ignored misleads whoever
//! wrote it, so the load fails and says why.

use std::path::Path;

use super::raw::RawVisibility;
use crate::error::CoreError;

/// The values `[visibility] data_clones` accepts.
const DATA_CLONE_CHOICES: [&str; 2] = ["keep", "ignore"];

/// Diagnostic for a config file that still carries a `[ranking]` section.
pub(super) const RANKING_RETIRED: &str = "the [ranking] section is retired: clones rank by \
     duplicated mass alone and no setting reweights or hides a category; remove the section";

/// Fails the load when the config carries the retired `[ranking]` section.
pub(super) fn reject_retired_ranking(source: &Path, present: bool) -> Result<(), CoreError> {
    if present {
        return Err(config_error(source, RANKING_RETIRED.to_owned()));
    }
    Ok(())
}

/// [EXCLUSION-CONFIG] `[visibility] data_clones` accepts only `keep` or
/// `ignore`; `demote` named a retired ranking multiplier and fails the load.
pub(super) fn validate_visibility(
    source: &Path,
    visibility: &RawVisibility,
) -> Result<(), CoreError> {
    match visibility.data_clones.as_deref() {
        None => Ok(()),
        Some(choice) if DATA_CLONE_CHOICES.contains(&choice) => Ok(()),
        Some(other) => Err(config_error(
            source,
            format!("visibility.data_clones must be \"keep\" or \"ignore\", got {other:?}"),
        )),
    }
}

/// The config-validation error every `[visibility]` and `[ranking]`
/// refusal reports.
fn config_error(source: &Path, message: String) -> CoreError {
    CoreError::ConfigThreshold {
        path: source.to_path_buf(),
        message,
    }
}
