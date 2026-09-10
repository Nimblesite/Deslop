use crate::FileRegistry;
use std::path::PathBuf;

fn stored_files() {
    let mut registry = FileRegistry::new();
    let stored_file_id = registry.register(PathBuf::from("stored.rs"));
    let requested_file_id = registry.register(PathBuf::from("requested.rs"));
}
