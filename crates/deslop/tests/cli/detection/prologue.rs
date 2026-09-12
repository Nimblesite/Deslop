//! Prologue and import suppression contracts.

use super::*;

// Returns the byte slice of `path` spanned by `occurrence`'s reported
// `[start_byte, end_byte)`. Used by the prologue-cluster regressions
// to read the source text the report claims is a clone.
pub(super) fn occurrence_source(
    scan_root: &Path,
    occurrence: &serde_json::Value,
) -> Option<Vec<u8>> {
    let path = occurrence.get("path").and_then(serde_json::Value::as_str)?;
    let start = occurrence_byte(occurrence, "start_byte")?;
    let end = occurrence_byte(occurrence, "end_byte")?;
    let bytes = fs::read(scan_root.join(path)).ok()?;
    bytes.get(start..end).map(<[u8]>::to_vec)
}

// The `key` byte offset an occurrence reports, narrowed to a `usize`.
fn occurrence_byte(occurrence: &serde_json::Value, key: &str) -> Option<usize> {
    usize::try_from(occurrence.get(key).and_then(serde_json::Value::as_u64)?).ok()
}

// Returns true when `text` opens with a top-level import/prologue
// construct in any of the languages Deslop currently parses: a Python
// triple-quoted module docstring, `import` / `from` / `if TYPE_CHECKING:`,
// a C# `using` directive or `namespace` declaration, or a Rust
// `use` / `extern crate` statement. The check looks only at the first
// non-whitespace line so a window that starts with prologue and
// extends into real code still counts as prologue-anchored.
fn opens_with_prologue_keyword(text: &str) -> bool {
    let trimmed = text.trim_start();
    if trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''") {
        return true;
    }
    let first_line = trimmed.lines().next().unwrap_or("").trim_start();
    if first_line.starts_with("if TYPE_CHECKING") {
        return true;
    }
    let token = first_line.split_whitespace().next().unwrap_or("");
    matches!(
        token,
        "use" | "using" | "namespace" | "import" | "from" | "extern"
    )
}

// Asserts no cluster in `report` is a cross-file prologue false
// positive: a multi-file cluster whose every occurrence starts on an
// import/use/namespace/docstring line. Drives all three issue-#34
// regression tests (Python, C#, Rust).
pub(super) fn assert_no_cross_file_prologue_cluster(
    report: &serde_json::Value,
    scan_root: &Path,
    label: &str,
) {
    let clusters = array_or_empty(report, "/clusters");
    for cluster in &clusters {
        let occurrences = array_or_empty(cluster, "/occurrences");
        let all_prologue = !occurrences.is_empty()
            && occurrences.iter().all(|occurrence| {
                let bytes = occurrence_source(scan_root, occurrence).unwrap_or_default();
                opens_with_prologue_keyword(std::str::from_utf8(&bytes).unwrap_or(""))
            });
        let files = cluster_file_paths(cluster);
        assert!(
            !(all_prologue && files.len() > 1),
            "{label}: cluster {} is a cross-file prologue cluster spanning \
             {files:?}; import / use / namespace / docstring scaffolding must never \
             anchor a cross-file clone",
            cluster_id(cluster),
        );
    }
}

// Drives `fixture_name` with the CLI's default flags and asserts its
// report carries no cross-file prologue cluster. Shared by the three
// issue-#34 prologue regressions (Python, C#, Rust).
fn assert_no_prologue_false_positive(fixture_name: &str, label: &str) -> Result<()> {
    let (scan_root, report) = run_with_args(fixture_name, &[])?;
    assert_no_cross_file_prologue_cluster(&report, &scan_root, label);
    Ok(())
}

// Audience: HUMAN. Issue #34. Python test suites conventionally open
// with a module docstring, `from __future__ import annotations`,
// `import pytest`, `from typing import TYPE_CHECKING`, and an
// `if TYPE_CHECKING:` import block. That prologue is pure
// import/prologue boilerplate: it carries no semantic content a human
// would recognise as "copy-pasted code". Before the fix for #34 the
// prologue subtree survived the boilerplate filter (no
// `future_import_statement` carrier, no module-docstring carrier, and
// the `if_statement` wrapper around imports was not treated as an
// imports-only subtree), so deslop reported the prologue as a
// cross-file clone spanning every Python file in the repo. For a
// 40-file repo that produced a 109-member cluster; even a 6-file
// fixture reproduces the symptom.
#[test]
fn python_module_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    assert_no_prologue_false_positive("python-prologue-false-positive", "python prologue")
}

// Audience: HUMAN. Issue #34, C# arm. The same prologue
// false-positive that hits Python `from __future__ import` /
// `if TYPE_CHECKING:` blocks also hits C# files: when many `.cs`
// files share the same `using ...;` block + `namespace X;` prologue
// but have entirely different class bodies, the sibling-window pass
// emits windows that span from the prologue into the class
// declaration. The `using_directive`/`file_scoped_namespace_declaration`
// k-grams dominate the window's token signature, so token Jaccard
// approaches 1.00 and LSH-only matching links every file into one
// cross-file cluster — even though the structural Merkle hashes
// disagree. The reported cluster from the user's repo had 109
// occurrences pinned at line 1, column 1 across the codebase; six
// distinct files reproduce the same shape here.
#[test]
fn csharp_using_namespace_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    assert_no_prologue_false_positive("csharp-prologue-false-positive", "csharp prologue")
}

// Audience: HUMAN. Issue #34, Rust arm. Six Rust files share the same
// eight-line `use ...;` block but contain completely different items
// (a function, a struct, an async fetcher, a trait, a CSV parser, a
// retry policy). Sibling windows that begin on a `use_declaration`
// and extend into the next `function_item` / `struct_item` /
// `trait_item` carry token signatures dominated by
// `use_declaration __ident__` k-grams, pushing token Jaccard to 1.00.
// `use_declaration` is already a boilerplate carrier for subtree
// fingerprints, but the sibling-window emitter still produces windows
// that *start* inside the use block — and those windows do anchor
// cross-file LSH-only clusters. The fix must keep import scaffolding
// from anchoring cross-file matches in any language we parse.
#[test]
fn rust_use_prologue_never_becomes_a_cross_file_cluster() -> Result<()> {
    assert_no_prologue_false_positive("rust-prologue-false-positive", "rust prologue")
}
