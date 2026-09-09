use crate::FileRegistry;
use std::path::PathBuf;

fn compared_files() {
    let mut registry = FileRegistry::new();
    let left = registry.register(PathBuf::from("left.py"));
    let right = registry.register(PathBuf::from("right.py"));
}
