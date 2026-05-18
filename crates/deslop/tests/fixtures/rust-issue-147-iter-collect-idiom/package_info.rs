//! Package info — name extraction for dependency graph.

pub struct Package {
    pub name: String,
    pub release_year: u32,
}

pub fn package_names(packages: &[Package]) -> Vec<&str> {
    let names: Vec<&str> = packages.iter().map(|package| package.name.as_str()).collect();
    names
}

pub fn release_labels(packages: &[Package]) -> Vec<String> {
    let labels: Vec<String> = packages
        .iter()
        .map(|package| package.name.to_lowercase())
        .collect();
    labels
}
