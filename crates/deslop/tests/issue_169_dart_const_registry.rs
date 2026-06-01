//! Regression coverage for #169: Dart const data registries — runs of
//! `static const Foo NAME = Foo(<distinct values>);` (icon tables, colour
//! palettes, design tokens) — are un-refactorable data, not duplicate
//! logic, yet they ranked among the worst handwritten offenders on real
//! Flutter code. They cluster via sibling-window fingerprints spanning
//! several consecutive const declarations, which the single-member Dart
//! field filter missed. Such clusters must be hidden from the ranked
//! report.

use std::{
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

#[test]
fn dart_const_constructor_registry_is_hidden() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let src = tmp.path().join("src");
    fs::create_dir(&src)?;

    // An icon-table-style const registry: every entry is a static const
    // initialised by a constructor call with named arguments, differing
    // only in name and codepoint. Pure data — nothing to extract.
    let mut file = String::from("class IconRegistry {\n  static const String family = \"icons\";\n");
    for index in 0..80 {
        file.push_str(&format!(
            "  static const IconData icon{index} = IconData(0x{:x}, fontFamily: family, matchTextDirection: true);\n",
            0xf000 + index,
        ));
    }
    file.push_str("}\n");
    fs::write(src.join("icons.dart"), file)?;

    let report = report_path(tmp.path());
    let mut cmd = Command::cargo_bin("deslop")?;
    let _assertion = cmd
        .arg(&src)
        .arg("--min-nodes")
        .arg("30")
        .arg("--embeddings")
        .arg("off")
        .arg("--notext")
        .arg("--nohtml")
        .arg("--output")
        .arg(tmp.path().join("report"))
        .assert()
        .success();

    let body = fs::read_to_string(&report)?;
    let json: Value = serde_json::from_str(&body)?;
    assert!(
        clusters(&json).is_empty(),
        "a const data registry (sibling static-const constructor calls) must \
         not rank as duplication: {body}"
    );
    Ok(())
}
