//! Precision guards for the #169 Dart const-data-registry filter: it must
//! suppress un-refactorable const *data* without ever hiding genuine
//! duplicate *logic*.
//!
//! Two ways the data filter could wrongly hide real duplication:
//!  1. A field initialised by a closure/lambda (`= (x) { ... }` or
//!     `= (x) => ...`) carries executable logic. Dart emits
//!     `function_expression` for these, not `function_body`, so a naive
//!     "no `function_body`" test misclassifies them as data.
//!  2. A *verbatim* copy of a field block (byte-identical members) is a
//!     real copy-paste, unlike a registry of distinct entries — it must
//!     survive, matching the Python #104 design (`raw_snippet_texts_differ`).

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use assert_cmd::Command;
use serde_json::Value;

fn report_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    path
}

fn clusters(report: &Value) -> &[Value] {
    report
        .get("clusters")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default()
}

fn run(src: &Path, out_dir: &Path, min_nodes: &str) -> Result<Value> {
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(src)
        .arg("--min-nodes")
        .arg(min_nodes)
        .arg("--embeddings")
        .arg("off")
        .arg("--notext")
        .arg("--nohtml")
        .arg("--output")
        .arg(out_dir.join("report"))
        .assert()
        .success();
    let body = fs::read_to_string(report_path(out_dir))?;
    Ok(serde_json::from_str(&body)?)
}

#[test]
fn dart_duplicated_closure_field_logic_stays_visible() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    fs::create_dir(&src)?;

    // Several fields holding the *same* closure body — real copy-pasted
    // logic that a developer could extract into a shared function. The
    // const-data filter must not mistake logic-bearing closures for data.
    let mut file = String::from("class Handlers {\n");
    for name in ["onTap", "onHold", "onSwipe", "onDrag"] {
        let _written = writeln!(
            file,
            "  static final {name} = (int e) {{ final r = e * 2; \
             final s = r + e; return s - 1; }};"
        );
    }
    file.push_str("}\n");
    fs::write(src.join("handlers.dart"), file)?;

    let report = run(&src, tmp.path(), "15")?;
    assert!(
        !clusters(&report).is_empty(),
        "a closure body copy-pasted across several fields is duplicate logic \
         and must still be reported: {report}"
    );
    Ok(())
}

#[test]
fn dart_verbatim_field_block_copy_stays_visible() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    fs::create_dir(&src)?;

    // A byte-identical run of const declarations copied between two files —
    // a real copy-paste, not a registry of distinct entries. Distinct
    // surrounding members keep the whole-class subtrees apart so only the
    // verbatim field block can cluster.
    let block = "  static const int alpha = 1;\n  static const int bravo = 2;\n  \
        static const int charlie = 3;\n  static const int delta = 4;\n";
    fs::write(
        src.join("first.dart"),
        format!("class First {{\n  int seed() => 11;\n{block}}}\n"),
    )?;
    fs::write(
        src.join("second.dart"),
        format!("class Second {{\n  String tag() => 'x';\n{block}}}\n"),
    )?;

    let report = run(&src, tmp.path(), "5")?;
    let spans_both = clusters(&report).iter().any(|cluster| {
        let paths: Vec<&str> = cluster
            .get("occurrences")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or_default()
            .iter()
            .filter_map(|occ| occ.get("path").and_then(Value::as_str))
            .collect();
        paths.iter().any(|path| path.ends_with("first.dart"))
            && paths.iter().any(|path| path.ends_with("second.dart"))
    });
    assert!(
        spans_both,
        "a verbatim-identical const field block copied across files is a real \
         copy-paste and must still be reported: {report}"
    );
    Ok(())
}
