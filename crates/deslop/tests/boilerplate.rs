//! End-to-end coverage for [PIPELINE-BOILERPLATE-FILTER].
//!
//! These tests drive the CLI as a black box. Import/prologue-only
//! repetition must not become duplicate-code clusters, but teams can
//! opt into structured hygiene hints when they want to clean it up.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Result};
use assert_cmd::Command;
use serde_json::Value;

struct SourceSpec {
    name: &'static str,
    body_marker: &'static str,
    source: &'static str,
}

struct RunOutputs {
    json: PathBuf,
    txt: PathBuf,
}

fn outputs_under(dir: &Path) -> RunOutputs {
    RunOutputs {
        json: dir.join("report.json"),
        txt: dir.join("report.txt"),
    }
}

fn write_sources(root: &Path) -> Result<BTreeMap<String, usize>> {
    fs::create_dir_all(root)?;
    let mut prologues = BTreeMap::new();
    for spec in sources() {
        fs::write(root.join(spec.name), spec.source)?;
        let end = spec
            .source
            .find(spec.body_marker)
            .ok_or_else(|| anyhow!("missing body marker for {}", spec.name))?;
        let _previous = prologues.insert(spec.name.to_owned(), end);
    }
    Ok(prologues)
}

fn run_report(root: &Path, tmp: &Path, config: Option<&Path>) -> Result<RunOutputs> {
    let out = outputs_under(tmp);
    let mut cmd = Command::cargo_bin("deslop")?;
    let mut command = cmd
        .arg(root)
        .args(["--min-nodes", "3", "--output"])
        .arg(tmp.join("report"));
    if let Some(path) = config {
        command = command.arg("--config").arg(path);
    }
    let _assertion = command.assert().success();
    Ok(out)
}

#[test]
fn import_boilerplate_is_suppressed_but_real_clones_still_report() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let prologues = write_sources(&scan_root)?;
    let out = run_report(&scan_root, tmp.path(), None)?;
    let report = load_json(&out.json)?;
    assert_no_prologue_clusters(&report, &prologues)?;
    assert_real_clone_survives(&report, &prologues, &["Alpha.cs", "Beta.cs"])?;
    assert_real_clone_survives(&report, &prologues, &["alpha.js", "beta.js"])?;
    assert_real_clone_survives(&report, &prologues, &["alpha.ts", "beta.ts"])?;
    assert_eq!(hint_count(&report), 0, "default mode must stay quiet");
    Ok(())
}

#[test]
fn import_boilerplate_report_mode_emits_low_noise_hints() -> Result<()> {
    let tmp = tempfile::tempdir()?;
    let scan_root = tmp.path().join("src");
    let prologues = write_sources(&scan_root)?;
    let config = tmp.path().join("deslop.toml");
    fs::write(&config, "[defaults.boilerplate]\nimports = \"report\"\n")?;
    let out = run_report(&scan_root, tmp.path(), Some(&config))?;
    let report = load_json(&out.json)?;
    assert_no_prologue_clusters(&report, &prologues)?;
    assert_hint_languages(
        &report,
        ["csharp", "python", "rust", "javascript", "typescript"],
    )?;
    assert_csharp_global_using_nudge(&report)?;
    assert!(fs::read_to_string(&out.txt)?.contains("boilerplate hints"));
    Ok(())
}

fn load_json(path: &Path) -> Result<Value> {
    Ok(serde_json::from_str(&fs::read_to_string(path)?)?)
}

fn assert_no_prologue_clusters(report: &Value, prologues: &BTreeMap<String, usize>) -> Result<()> {
    for occurrence in cluster_occurrences(report)? {
        let Some(name) = occurrence_name(occurrence) else {
            continue;
        };
        let Some(end) = prologues.get(name) else {
            continue;
        };
        let occurrence_end = occurrence
            .get("end_byte")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or_default();
        assert!(
            occurrence_end > *end,
            "prologue cluster survived: {occurrence}"
        );
    }
    Ok(())
}

fn assert_real_clone_survives(
    report: &Value,
    prologues: &BTreeMap<String, usize>,
    files: &[&str],
) -> Result<()> {
    let cluster = find_cluster(report, files)
        .ok_or_else(|| anyhow!("expected the real clone below imports to survive: {files:?}"))?;
    for occurrence in cluster_occurrences(cluster)? {
        let name = occurrence_name(occurrence).unwrap_or_default();
        let prologue_end = prologues.get(name).copied().unwrap_or_default();
        let end = occurrence.get("end_byte").and_then(Value::as_u64);
        assert!(end
            .and_then(|value| usize::try_from(value).ok())
            .is_some_and(|value| value > prologue_end));
    }
    Ok(())
}

fn assert_hint_languages<const N: usize>(report: &Value, languages: [&str; N]) -> Result<()> {
    let actual: BTreeSet<&str> = hints(report)?
        .iter()
        .filter_map(|hint| hint.get("language").and_then(Value::as_str))
        .collect();
    for language in languages {
        assert!(
            actual.contains(language),
            "missing {language} hint: {actual:?}"
        );
    }
    Ok(())
}

fn assert_csharp_global_using_nudge(report: &Value) -> Result<()> {
    let hint = hints(report)?
        .iter()
        .find(|hint| hint.get("language").and_then(Value::as_str) == Some("csharp"))
        .ok_or_else(|| anyhow!("missing csharp hint"))?;
    assert_eq!(hint.get("severity").and_then(Value::as_str), Some("info"));
    assert_contains(hint, "recommendation", "GlobalUsings.cs");
    assert!(hint
        .get("occurrences")
        .and_then(Value::as_array)
        .is_some_and(|v| v.len() >= 2));
    Ok(())
}

fn hints(report: &Value) -> Result<&[Value]> {
    report
        .get("boilerplate_hints")
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| anyhow!("boilerplate_hints missing"))
}

fn hint_count(report: &Value) -> usize {
    report
        .get("boilerplate_hints")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
}

fn cluster_occurrences(report_or_cluster: &Value) -> Result<Vec<&Value>> {
    if let Some(items) = report_or_cluster
        .get("occurrences")
        .and_then(Value::as_array)
    {
        return Ok(items.iter().collect());
    }
    let clusters = report_or_cluster
        .get("clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("clusters missing"))?;
    Ok(clusters.iter().flat_map(cluster_occurrences_flat).collect())
}

fn cluster_occurrences_flat(cluster: &Value) -> Vec<&Value> {
    cluster
        .get("occurrences")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, |items| items.iter().collect())
}

fn find_cluster<'a>(report: &'a Value, required: &[&str]) -> Option<&'a Value> {
    report
        .get("clusters")?
        .as_array()?
        .iter()
        .find(|cluster| cluster_has_files(cluster, required))
}

fn cluster_has_files(cluster: &Value, required: &[&str]) -> bool {
    let names: BTreeSet<&str> = cluster_occurrences_flat(cluster)
        .into_iter()
        .filter_map(occurrence_name)
        .collect();
    required.iter().all(|name| names.contains(name))
}

fn occurrence_name(occurrence: &Value) -> Option<&str> {
    let path = occurrence.get("path")?.as_str()?;
    Path::new(path).file_name()?.to_str()
}

fn assert_contains(value: &Value, key: &str, needle: &str) {
    let text = value.get(key).and_then(Value::as_str).unwrap_or_default();
    assert!(
        text.contains(needle),
        "{key} did not contain {needle}: {text}"
    );
}

fn sources() -> [SourceSpec; 10] {
    [
        SourceSpec {
            name: "Alpha.cs",
            body_marker: "public sealed class",
            source: CSHARP_ALPHA,
        },
        SourceSpec {
            name: "Beta.cs",
            body_marker: "public sealed class",
            source: CSHARP_BETA,
        },
        SourceSpec {
            name: "alpha.rs",
            body_marker: "pub fn",
            source: RUST_ALPHA,
        },
        SourceSpec {
            name: "beta.rs",
            body_marker: "pub struct",
            source: RUST_BETA,
        },
        SourceSpec {
            name: "alpha.py",
            body_marker: "def",
            source: PYTHON_ALPHA,
        },
        SourceSpec {
            name: "beta.py",
            body_marker: "class",
            source: PYTHON_BETA,
        },
        SourceSpec {
            name: "alpha.js",
            body_marker: "export function shared",
            source: JS_ALPHA,
        },
        SourceSpec {
            name: "beta.js",
            body_marker: "export function shared",
            source: JS_BETA,
        },
        SourceSpec {
            name: "alpha.ts",
            body_marker: "export function shared",
            source: TS_ALPHA,
        },
        SourceSpec {
            name: "beta.ts",
            body_marker: "export function shared",
            source: TS_BETA,
        },
    ]
}

const CSHARP_ALPHA: &str = r"using System;
using System.Collections.Generic;
using Microsoft.Extensions.Logging;

namespace Example.Tests;

public sealed class Alpha
{
    public int Shared(int value)
    {
        var total = value + 1;
        return total * 3;
    }
}
";

const CSHARP_BETA: &str = r"using System;
using System.Collections.Generic;
using Microsoft.Extensions.Logging;

namespace Example.Tests;

public sealed class Beta
{
    public int Shared(int value)
    {
        var total = value + 1;
        return total * 3;
    }
}
";

const RUST_ALPHA: &str = r"use std::collections::HashMap;
use std::sync::Arc;

pub fn alpha(seed: i32) -> i32 {
    match seed {
        0 => 11,
        other => other * 17,
    }
}
";

const RUST_BETA: &str = r"use std::collections::HashMap;
use std::sync::Arc;

pub struct Beta {
    total: i32,
}

impl Beta {
    pub fn open(total: i32) -> Self {
        Self { total }
    }
}
";

const PYTHON_ALPHA: &str = r#"import os
import sys
from pathlib import Path

def alpha(seed):
    table = {"seed": seed}
    return table["seed"] + 41
"#;

const PYTHON_BETA: &str = r"import os
import sys
from pathlib import Path

class Beta:
    def __init__(self, name):
        self.name = name
";

const JS_ALPHA: &str = r#"import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { z } from "zod";

export function shared(value) {
  const total = value + 1;
  return total * 3;
}
"#;

const JS_BETA: &str = r#"import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { z } from "zod";

export function shared(amount) {
  const sum = amount + 1;
  return sum * 3;
}
"#;

const TS_ALPHA: &str = r#"import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { z } from "zod";

export function shared(value: number): number {
  const total = value + 1;
  return total * 3;
}
"#;

const TS_BETA: &str = r#"import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { z } from "zod";

export function shared(amount: number): number {
  const sum = amount + 1;
  return sum * 3;
}
"#;
