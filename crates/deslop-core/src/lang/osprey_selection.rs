//! [LANG-CAND-OSPREY] Resolve syntax from project metadata and parsed comments.

use super::osprey::{LANGUAGE_ID, ML_LANGUAGE_ID, PYTHON_LANGUAGE_ID};
use crate::{config::ExclusionConfig, error::CoreError, lang::shared::parse_source};
use serde::Deserialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Osprey project metadata filename, also watched for live reloads.
pub(crate) const MANIFEST: &str = "osprey.toml";
/// Ordinary Python identity before an explicit Osprey opt-in.
const PYTHON_ID: &str = "python";
/// Metadata payload recognized inside a leading comment node.
const MARKER: &str = "osprey: flavor=";
/// The one flavor that opts an ordinary `.py` file into Osprey.
const PYTHON_FLAVOR: &str = "python";
/// Marker/extension disagreement requires an explicit project override.
const FLAVOR_CONFLICT: &str = "flavor marker contradicts the extension";

/// Project rules compiled once for a discovery or manifest refresh.
#[derive(Debug, Default)]
pub(crate) struct OspreyProjects {
    /// Readable manifests, deepest first so nested projects win.
    projects: Vec<Project>,
    /// Unusable manifests, reported only for the Osprey sources they govern.
    broken: Vec<BrokenManifest>,
}

/// A manifest that could not be read or parsed.
#[derive(Debug)]
struct BrokenManifest {
    /// Canonical manifest directory, compared against canonical source paths.
    directory: PathBuf,
    /// What went wrong, naming the manifest.
    problem: String,
}

#[derive(Debug)]
/// Resolved source membership and optional project-wide syntax override.
struct Project {
    /// Manifest directory, used to prefer nested projects.
    directory: PathBuf,
    /// Absolute production-source roots.
    roots: Vec<PathBuf>,
    /// Explicit Python-flavor source and entry paths.
    python: Vec<PathBuf>,
    /// Compiler-compatible project syntax override.
    flavor: Option<String>,
}

#[derive(Deserialize, Default)]
/// Only the project section influences language identity.
struct Manifest {
    /// Project source selection; unrelated manifest sections remain untouched.
    #[serde(default)]
    project: Settings,
}

#[derive(Deserialize, Default)]
/// Relevant keys parsed by the TOML parser rather than source heuristics.
struct Settings {
    /// Configured source roots or compiler-compatible defaults.
    source_roots: Option<Vec<PathBuf>>,
    /// Literal opt-in paths for Osprey Python.
    #[serde(default)]
    python_sources: Vec<PathBuf>,
    /// Application entry, also opting a Python entry into Osprey.
    entry: Option<PathBuf>,
    /// Explicit syntax override.
    flavor: Option<String>,
}

impl OspreyProjects {
    /// Read real TOML, keeping malformed manifests for [`Self::language`] to
    /// report: only an Osprey source can need one, so a stray `osprey.toml`
    /// never stops the scan of a repository written in anything else.
    /// Excluded manifests sit outside the analysed workspace, so they can
    /// neither fail the scan nor select a flavor.
    pub(crate) fn load(root: &Path, config: &ExclusionConfig) -> Self {
        let (projects, broken): (Vec<_>, Vec<_>) = ignore::WalkBuilder::new(root)
            .build()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_some_and(|kind| kind.is_file()))
            .filter(|entry| entry.file_name() == MANIFEST)
            .filter(|entry| !config.is_excluded(entry.path(), None))
            .map(|entry| Project::read(entry.path()))
            .partition(Result::is_ok);
        let mut projects: Vec<_> = projects.into_iter().filter_map(Result::ok).collect();
        projects.sort_by_key(|project| std::cmp::Reverse(project.directory.components().count()));
        let broken: Vec<_> = broken.into_iter().filter_map(Result::err).collect();
        if !broken.is_empty() {
            tracing::warn!(manifest_count = broken.len(), "osprey_manifests_unusable");
        }
        Self { projects, broken }
    }

    /// Source bytes include unsaved editor text on the live path.
    pub(crate) fn language(
        &self,
        path: &Path,
        source: &[u8],
        fallback: &'static str,
    ) -> Result<&'static str, CoreError> {
        if !matches!(fallback, LANGUAGE_ID | PYTHON_ID | ML_LANGUAGE_ID) {
            return Ok(fallback);
        }
        self.require_usable_manifest(path, fallback)?;
        let project = self
            .projects
            .iter()
            .find(|project| project.claims(path, fallback));
        let marker = flavor_marker(source, fallback)?;
        if fallback == PYTHON_ID && project.is_none() && !opts_into_python(marker.as_deref()) {
            return Ok(fallback);
        }
        if let Some(flavor) = project.and_then(|project| project.flavor.as_deref()) {
            return flavor_language(flavor, path);
        }
        extension_flavor(path, marker.as_deref(), fallback)
    }

    /// Osprey syntax under an unusable manifest cannot be selected. A `.py`
    /// file stays ordinary Python, because it opts in only explicitly.
    fn require_usable_manifest(&self, path: &Path, fallback: &str) -> Result<(), CoreError> {
        if fallback == PYTHON_ID {
            return Ok(());
        }
        self.broken
            .iter()
            .find(|manifest| path.starts_with(&manifest.directory))
            .map_or(Ok(()), |manifest| {
                Err(selection_error(path, &manifest.problem))
            })
    }
}

/// Any other marker in an unclaimed `.py` file is just a comment.
fn opts_into_python(marker: Option<&str>) -> bool {
    let Some(flavor) = marker else {
        return false;
    };
    if flavor != PYTHON_FLAVOR {
        tracing::warn!("osprey_marker_ignored_in_ordinary_python");
    }
    flavor == PYTHON_FLAVOR
}

impl Project {
    /// Decode project metadata and resolve its paths.
    fn read(path: &Path) -> Result<Self, BrokenManifest> {
        let directory = path.parent().unwrap_or(Path::new("."));
        let broken = |problem: String| BrokenManifest {
            directory: crate::pipeline::canonical_source_path(directory),
            problem,
        };
        let text = fs::read_to_string(path)
            .map_err(|source| broken(format!("failed to read {}: {source}", path.display())))?;
        let manifest: Manifest = toml::from_str(&text)
            .map_err(|source| broken(format!("failed to parse {}: {source}", path.display())))?;
        Ok(Self::from_settings(directory, manifest.project))
    }

    /// Compile relative membership rules against the manifest directory.
    fn from_settings(directory: &Path, settings: Settings) -> Self {
        let roots = settings
            .source_roots
            .unwrap_or_else(|| default_roots(directory));
        let python = settings.python_sources.into_iter().chain(
            settings
                .entry
                .filter(|entry| entry.extension().is_some_and(|extension| extension == "py")),
        );
        Self {
            directory: directory.to_path_buf(),
            roots: roots
                .into_iter()
                .map(|root| absolute(directory, &root))
                .collect(),
            python: python.map(|file| absolute(directory, &file)).collect(),
            flavor: settings.flavor,
        }
    }

    /// Python membership is explicit; other Osprey sources follow source roots.
    fn claims(&self, path: &Path, fallback: &str) -> bool {
        if fallback == PYTHON_ID {
            return self.python.iter().any(|file| file == path);
        }
        self.roots.iter().any(|root| path.starts_with(root))
    }
}

/// Inspect only leading comment nodes; markers in strings or later code cannot opt in.
fn flavor_marker(source: &[u8], fallback: &'static str) -> Result<Option<String>, CoreError> {
    let grammar = match fallback {
        PYTHON_ID => tree_sitter_python::LANGUAGE.into(),
        ML_LANGUAGE_ID => tree_sitter_osprey_ml::LANGUAGE.into(),
        _ => tree_sitter_osprey::LANGUAGE.into(),
    };
    let tree = parse_source(fallback, &grammar, source)?;
    Ok(leading_marker(tree.root_node(), source))
}

/// Stop at the first code node, so strings and later comments never select syntax.
fn leading_marker(root: tree_sitter::Node<'_>, source: &[u8]) -> Option<String> {
    let mut cursor = root.walk();
    let marker = root
        .named_children(&mut cursor)
        .take_while(is_leading_comment)
        .filter_map(|node| node.utf8_text(source).ok())
        .find_map(comment_flavor);
    marker
}

/// Comment families shared by Osprey's braces/ML and Python CSTs.
fn is_leading_comment(node: &tree_sitter::Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "line_comment" | "doc_comment" | "inner_doc_comment" | "block_comment"
    )
}

/// Decode flavor metadata only after the CST identified a real comment.
fn comment_flavor(text: &str) -> Option<String> {
    text.strip_prefix("//")
        .or_else(|| text.strip_prefix('#'))?
        .trim()
        .strip_prefix(MARKER)
        .map(|flavor| flavor.trim().to_owned())
}

/// Refuse markers that disagree with the extension without a project override.
fn extension_flavor(
    path: &Path,
    marker: Option<&str>,
    fallback: &'static str,
) -> Result<&'static str, CoreError> {
    let expected = if fallback == PYTHON_ID {
        PYTHON_LANGUAGE_ID
    } else {
        fallback
    };
    let selected = marker
        .map(|flavor| flavor_language(flavor, path))
        .transpose()?;
    if selected.is_some_and(|language| language != expected) {
        return Err(selection_error(path, FLAVOR_CONFLICT));
    }
    Ok(selected.unwrap_or(expected))
}

/// Match Osprey's manifest defaults for source-root selection.
fn default_roots(directory: &Path) -> Vec<PathBuf> {
    vec![PathBuf::from(if directory.join("src").is_dir() {
        "src"
    } else {
        "."
    })]
}

/// Map the compiler's three flavor names to independent grammar/cache identities.
fn flavor_language(flavor: &str, path: &Path) -> Result<&'static str, CoreError> {
    match flavor {
        "default" => Ok(LANGUAGE_ID),
        "ml" => Ok(ML_LANGUAGE_ID),
        "python" => Ok(PYTHON_LANGUAGE_ID),
        _ => Err(selection_error(path, "unknown Osprey flavor")),
    }
}

/// Surface invalid metadata instead of silently selecting another grammar.
fn selection_error(path: &Path, message: &str) -> CoreError {
    CoreError::LanguageSelection {
        path: path.to_path_buf(),
        message: message.to_owned(),
    }
}

/// Share live-path canonicalization, including not-yet-created files.
fn absolute(directory: &Path, path: &Path) -> PathBuf {
    crate::pipeline::canonical_source_path(&directory.join(path))
}
