//! Built-in exclusion and report-hide rules ([CONFIG-EXCLUDE-BUILTIN]):
//! the dependency/artefact component lists, generated-code detection,
//! and the corpus-scoped path matching they share. Applied before any
//! user pattern, with no configuration surface except the
//! `include_dependencies` opt-in ([CONFIG-EXCLUDE-DEPENDENCIES]).

use std::path::{Component, Path};

/// Directory components that are always excluded from discovery.
///
/// `.cargo` covers Cargo's vendored registry / git checkout caches
/// — even when the surrounding repo points discovery at
/// the user's home directory by accident, the boilerplate generated
/// code under `.cargo/git/checkouts/...` and
/// `.cargo/registry/src/...` never enters the report.
///
/// `.git` and `.claude` cover working-tree copies that look like source
/// but are not actionable duplication. Claude Code agent
/// workflows create full git worktrees under `.claude/worktrees/<id>/`;
/// each is another checkout of the same repo, so without this exclusion
/// every file is reported as N identical "copies". The initial walk's
/// hidden-dir filter skips dot-dirs, but the live watcher
/// (`live/watcher.rs`) and incremental update (`pipeline/session/change.rs`)
/// have no hidden filter and rely solely on this component list, so the
/// exclusion must live here to cover all three discovery paths.
///
/// `.dart_tool` and `.pub-cache` cover the Dart/Flutter toolchain's own
/// caches: `.dart_tool/` holds `package_config.json`, `build_runner`
/// outputs, and per-package generated `.dart`; `.pub-cache/` is a
/// vendored copy of every dependency's source. On a large Flutter
/// monorepo a hot build churns thousands of `.dart` files under
/// `.dart_tool/`; because the live watcher has no `.gitignore` filter,
/// excluding them here is what keeps that churn from monopolising the
/// session and starving the editor's responsiveness.
///
/// `vendor` is the third-party source copy for Go (`go mod vendor`),
/// PHP (Composer) and Rust (`cargo vendor`). Unlike every other entry it
/// is *conventionally committed* — GitHub's `Go.gitignore` ships the
/// `vendor/` rule commented out — and it is not dot-prefixed, so neither
/// the gitignore pass nor the hidden-directory pass prunes it. Without
/// this entry a vendored repo hands the pipeline tens of thousands of
/// dependency files, and because ranking is worst-offenders-first the
/// resulting third-party duplication outranks every first-party finding
/// the user can actually act on.
/// Third-party *library source* vendored or installed into the corpus.
/// These are real, readable source files the user did not write, so
/// analysing them is a legitimate — if unusual — request: auditing a
/// dependency for duplication, or checking whether first-party code
/// duplicates a library it already depends on. Governed by
/// `[analysis] include_dependencies` ([CONFIG-EXCLUDE-DEPENDENCIES]),
/// which defaults to `false`.
const BUILTIN_DEPENDENCY_COMPONENTS: &[&str] =
    &["node_modules", "vendor", ".cargo", ".pub-cache", ".venv"];

/// Build output, tool caches, and working-tree copies
/// ([CONFIG-EXCLUDE-BUILTIN]). Excluded unconditionally: none of it is
/// source the user wrote, and none of it is a "library the code depends
/// on", so no configuration opts back in.
/// `target`, `dist`, `build`, `__pycache__` and `.dart_tool` are compiler
/// and codegen output; `.git` and `.claude` are whole additional checkouts
/// of the same repository which would otherwise report every file
/// as N identical copies.
const BUILTIN_ARTEFACT_COMPONENTS: &[&str] = &[
    "target",
    "dist",
    "build",
    "__pycache__",
    ".dart_tool",
    ".git",
    ".claude",
];

/// Directory components that are always analysed but hidden from summaries.
const BUILTIN_REPORT_HIDE_COMPONENTS: &[&str] = &["generated"];

/// Non-actionable path component pairs hidden from summaries.
const BUILTIN_REPORT_HIDE_COMPONENT_PAIRS: &[(&str, &str)] = &[
    ("alembic", "versions"),
    ("test", "fixtures"),
    ("tests", "fixtures"),
];

/// File suffixes that are always analysed but hidden from summaries.
const BUILTIN_REPORT_HIDE_SUFFIXES: &[&str] = &[
    ".g.cs",
    ".generated.cs",
    ".designer.cs",
    ".pb.cs",
    ".openapi.cs",
    ".generated.py",
    "_generated.py",
    // Dart code generators. `.g.dart` covers source_gen
    // (json_serializable, retrofit, drift, hive, …); the rest cover
    // freezed, auto_route, injectable, flutter_gen, mockito, and the
    // protoc Dart plugin. All carry "GENERATED CODE - DO NOT MODIFY".
    ".g.dart",
    ".freezed.dart",
    ".gr.dart",
    ".config.dart",
    ".gen.dart",
    ".mocks.dart",
    ".pb.dart",
    ".pbenum.dart",
    ".pbjson.dart",
    ".pbserver.dart",
    ".pbgrpc.dart",
];

/// Returns true when `path` sits in a built-in excluded tree **inside the
/// analysed corpus** ([CONFIG-EXCLUDE-BUILTIN]).
///
/// Only components at or below `scan_root` are considered. The rule prunes
/// dependency and build trees *within the corpus the user asked for*; a
/// directory name above the scan root describes where the checkout happens
/// to sit on disk and says nothing about its contents. Matching those
/// ancestors excluded every file in any repository nested under e.g.
/// `dist`, `build`, `target`, `vendor` or `node_modules`, yielding
/// `files_analysed: 0`, `clusters: []`, `threshold.breached: false` and a
/// successful exit — a total, silent false negative. This mirrors the
/// carve-out [`scan_root_contains_component_pair`] already applies to the
/// report-hide tier.
///
/// When `include_dependencies` is set, [`BUILTIN_DEPENDENCY_COMPONENTS`]
/// stops applying and third-party library source enters the analysis;
/// [`BUILTIN_ARTEFACT_COMPONENTS`] applies regardless
/// ([CONFIG-EXCLUDE-DEPENDENCIES]).
///
/// A path outside `scan_root`, or a config with no known root, is not part
/// of any corpus this rule governs, so the rule does not fire. That
/// direction can only admit a file for analysis — it can never silently
/// discard one.
///
/// Pinned by `crates/deslop/tests/issue_342_scan_root_under_excluded_ancestor.rs`
/// and `crates/deslop/tests/go_vendor_exclusion.rs`.
pub(super) fn corpus_built_in_excluded(
    path: &Path,
    scan_root: Option<&Path>,
    include_dependencies: bool,
) -> bool {
    let Some(components) = corpus_components(path, scan_root) else {
        return false;
    };
    components.into_iter().any(|component| {
        BUILTIN_ARTEFACT_COMPONENTS
            .iter()
            .chain(dependency_components(include_dependencies))
            .any(|excluded| component == *excluded)
    })
}

/// The dependency component list when it applies, or nothing when the user
/// opted into analysing libraries ([CONFIG-EXCLUDE-DEPENDENCIES]).
fn dependency_components(include_dependencies: bool) -> &'static [&'static str] {
    if include_dependencies {
        &[]
    } else {
        BUILTIN_DEPENDENCY_COMPONENTS
    }
}

/// Returns the lowercased components of `path` lying strictly below
/// `scan_root` — the part of the path the user asked deslop to analyse.
/// `None` when no corpus boundary is known or `path` sits outside it.
fn corpus_components(path: &Path, scan_root: Option<&Path>) -> Option<Vec<String>> {
    let relative = scan_root.and_then(|root| path.strip_prefix(root).ok())?;
    Some(path_components(relative).collect())
}

/// Returns true when built-in generated-code rules hide a path from summaries.
pub(super) fn built_in_report_hidden(path: &Path, scan_root: Option<&Path>) -> bool {
    has_hidden_component(path)
        || has_hidden_component_pair(path, scan_root)
        || has_hidden_suffix(path)
}

/// Returns true when the path has a generated-code directory component.
fn has_hidden_component(path: &Path) -> bool {
    path_components(path).any(|component| {
        BUILTIN_REPORT_HIDE_COMPONENTS
            .iter()
            .any(|hidden| component == *hidden)
    })
}

/// Returns true when the path contains a generated-code component pair.
fn has_hidden_component_pair(path: &Path, scan_root: Option<&Path>) -> bool {
    let components: Vec<String> = path_components(path).collect();
    BUILTIN_REPORT_HIDE_COMPONENT_PAIRS.iter().any(|pair| {
        contains_component_pair(&components, *pair)
            && !scan_root_contains_component_pair(scan_root, *pair)
    })
}

/// Returns true when the scan root itself is inside a hidden component
/// pair. In that case the user intentionally asked to analyse that
/// corpus, so the built-in dogfood hide rule must not erase every
/// positive fixture cluster.
fn scan_root_contains_component_pair(scan_root: Option<&Path>, pair: (&str, &str)) -> bool {
    let Some(root) = scan_root else {
        return false;
    };
    let components: Vec<String> = path_components(root).collect();
    contains_component_pair(&components, pair)
}

/// Returns true when adjacent path components match `pair`.
fn contains_component_pair(components: &[String], pair: (&str, &str)) -> bool {
    components
        .windows(2)
        .any(|window| matches!(window, [first, second] if first == pair.0 && second == pair.1))
}

/// Recognises the unambiguous machine-generated banners build tools emit
/// in a file's head — `@generated` (linguist convention), `GENERATED CODE`
/// (`build_runner` / `source_gen`), `AUTO[- ]GENERATED` (ffigen),
/// `Autogenerated` (jnigen), and `automatically generated` (Flutter/Dart
/// localizations, intl messages). Catches generators that emit no stable file
/// suffix, e.g. ffigen/jnigen FFI bindings, so they join the suffix list in
/// being hidden from the ranked report ([EXCLUSION-CONFIG]). Scans only
/// the first kilobyte so a stray phrase deep in hand-written source cannot
/// trip it, and matches ASCII-case-insensitively without allocating.
#[must_use]
pub(crate) fn has_generated_header(source: &[u8]) -> bool {
    const MARKERS: &[&[u8]] = &[
        b"@generated",
        b"generated code",
        b"auto generated",
        b"auto-generated",
        b"autogenerated",
        // Flutter/Dart codegen (generated localizations, intl messages) and
        // a wide class of generators emit "automatically generated … do not
        // edit". Without this marker these dominated worst-offenders on a
        // stock Flutter analysis.
        b"automatically generated",
    ];
    let head = source.get(..source.len().min(1024)).unwrap_or(source);
    MARKERS.iter().any(|marker| {
        head.windows(marker.len())
            .any(|window| window.eq_ignore_ascii_case(marker))
    })
}

/// Returns true when the path's file name has a generated-code suffix.
fn has_hidden_suffix(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(std::ffi::OsStr::to_str) else {
        return false;
    };
    let lower = file_name.to_ascii_lowercase();
    BUILTIN_REPORT_HIDE_SUFFIXES
        .iter()
        .any(|suffix| lower.ends_with(suffix))
}

/// Returns lowercase normal path components for case-insensitive matching.
fn path_components(path: &Path) -> impl Iterator<Item = String> + '_ {
    path.components().filter_map(|component| match component {
        Component::Normal(value) => value.to_str().map(str::to_ascii_lowercase),
        _ => None,
    })
}
