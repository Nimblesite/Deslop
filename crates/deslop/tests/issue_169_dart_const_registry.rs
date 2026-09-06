//! Regression coverage for #169: Dart const data registries — runs of
//! `static const Foo NAME = Foo(<distinct values>);` (icon tables, colour
//! palettes, design tokens) — are un-refactorable data, not duplicate
//! logic, yet they ranked among the worst handwritten offenders on real
//! Flutter code. They cluster via sibling-window fingerprints spanning
//! several consecutive const declarations, which the single-member Dart
//! field filter missed. Such clusters must be hidden from the ranked
//! report.

use std::{
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

use anyhow::Result;
use serde_json::Value;

mod common;
use crate::common::*;

fn report_path(tmp: &Path) -> PathBuf {
    let mut path = tmp.join("report");
    let _replaced = path.set_extension("json");
    path
}

fn any_cluster_touches(report: &Value, file_name: &str) -> bool {
    clusters(report).iter().any(|cluster| {
        occurrence_paths(cluster)
            .iter()
            .any(|path| path.ends_with(file_name))
    })
}

fn any_cluster_spans(report: &Value, left: &str, right: &str) -> bool {
    clusters(report).iter().any(|cluster| {
        let paths = occurrence_paths(cluster);
        paths.iter().any(|path| path.ends_with(left))
            && paths.iter().any(|path| path.ends_with(right))
    })
}

#[test]
fn dart_const_constructor_registry_is_hidden() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    fs::create_dir(&src)?;

    // An icon-table-style const registry: every entry is a static const
    // initialised by a constructor call with named arguments, differing
    // only in name and codepoint. Pure data — nothing to extract.
    let mut file =
        String::from("class IconRegistry {\n  static const String family = \"icons\";\n");
    for index in 0..80_u32 {
        let codepoint = 0xf000_u32.saturating_add(index);
        let _written = writeln!(
            file,
            "  static const IconData icon{index} = IconData(0x{codepoint:x}, \
             fontFamily: family, matchTextDirection: true);"
        );
    }
    file.push_str("}\n");
    fs::write(src.join("icons.dart"), file)?;

    // Positive control in the SAME run: a genuinely duplicated method body
    // copied across two classes MUST still cluster at this `--min-nodes`,
    // so an empty-registry result cannot pass for the wrong reason (parse
    // failure, threshold too high, etc.).
    let method = "  int score(int v) {\n    final w = v * 3;\n    final z = w + v;\n    \
        final q = z - w;\n    final r = q * z;\n    return r + w - v;\n  }\n";
    fs::write(
        src.join("scorer_a.dart"),
        format!("class ScorerA {{\n{method}}}\n"),
    )?;
    fs::write(
        src.join("scorer_b.dart"),
        format!("class ScorerB {{\n{method}}}\n"),
    )?;

    let report = report_path(tmp.path());
    let mut cmd = deslop_cmd(&src, &tmp.path().join("report"))?;
    let _assertion = cmd
        .args([
            "--min-nodes",
            "30",
            "--embeddings",
            "off",
            "--notext",
            "--nohtml",
        ])
        .assert()
        .success();

    let body = fs::read_to_string(&report)?;
    let json: Value = serde_json::from_str(&body)?;
    assert!(
        any_cluster_spans(&json, "scorer_a.dart", "scorer_b.dart"),
        "positive control: the duplicated score() method must still cluster: {body}"
    );
    assert!(
        !any_cluster_touches(&json, "icons.dart"),
        "a const data registry (sibling static-const constructor calls) must \
         not rank as duplication: {body}"
    );
    Ok(())
}
