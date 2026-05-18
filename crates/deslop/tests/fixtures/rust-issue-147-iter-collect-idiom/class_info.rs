//! Class info — name extraction for analytics renderer.

pub struct ClassInfo {
    pub name: String,
    pub line_count: usize,
}

pub fn class_names(classes: &[ClassInfo]) -> Vec<&str> {
    let names: Vec<&str> = classes.iter().map(|class| class.name.as_str()).collect();
    names
}

pub fn uppercased_names(classes: &[ClassInfo]) -> Vec<String> {
    let firsts: Vec<String> = classes
        .iter()
        .map(|class| class.name.to_uppercase())
        .collect();
    firsts
}
