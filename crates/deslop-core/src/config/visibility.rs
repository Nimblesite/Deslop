//! [EXCLUSION-CONFIG] `[visibility]` validation.

use std::path::Path;

use super::raw::RawVisibility;
use crate::error::CoreError;

/// The values `[visibility] data_clones` accepts.
const DATA_CLONE_CHOICES: [&str; 2] = ["keep", "ignore"];

/// [EXCLUSION-CONFIG] `[visibility] data_clones` accepts only `keep` or
/// `ignore`.
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

/// The config-validation error every `[visibility]` refusal reports.
fn config_error(source: &Path, message: String) -> CoreError {
    CoreError::ConfigThreshold {
        path: source.to_path_buf(),
        message,
    }
}
