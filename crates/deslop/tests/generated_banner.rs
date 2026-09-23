//! [EXCLUSION-GENERATED-BANNER] A file that opens with a generator's
//! banner is hidden from the ranked report and from the duplication
//! percentage; a file that merely talks about generators is not.
//!
//! The Flutter framework's localisation bundles (#165) are the case that
//! started it: they carry "This file has been automatically generated.
//! Please do not edit it manually." under an ordinary file name, so no
//! path rule can hide them, and they dominated the worst-offenders
//! ranking of a stock Flutter analysis.

#[path = "generated_banner/languages.rs"]
mod languages;
#[path = "generated_banner/registry.rs"]
mod registry;

use std::{fs, path::Path};

use anyhow::{anyhow, Result};
use serde_json::Value;

use crate::common::{
    scan_dir::{report_path, temp_scan_dir},
    *,
};

/// Small enough that a four-statement function clusters, so every
/// fixture here stays short enough to read in one screen.
const MIN_NODES: &str = "5";

/// Where [`scan`] writes the report set it then reads back.
const REPORT_PREFIX: &str = "report";

/// One scan configuration for every fixture in this file: deterministic,
/// no embeddings, JSON only.
const SCAN_ARGS: [&str; 6] = [
    "--min-nodes",
    MIN_NODES,
    "--embeddings",
    "off",
    "--notext",
    "--nohtml",
];

/// Scans `src`, returning the parsed report and its raw body, so a
/// failing assertion can print exactly what the tool produced.
fn scan(tmp: &Path, src: &Path) -> Result<(Value, String)> {
    let report = report_path(tmp);
    let mut cmd = deslop_cmd(src, &tmp.join(REPORT_PREFIX))?;
    let _assertion = cmd.args(SCAN_ARGS).assert().success();
    let body = fs::read_to_string(&report)?;
    Ok((serde_json::from_str(&body)?, body))
}

/// Asserts `file`'s occurrence in `cluster` carries `hidden`, and that its
/// lines reach `duplicated_loc` exactly when the occurrence is shown: the
/// row and the percentage read one decision.
fn assert_visibility(report: &Value, cluster: &Value, file: &str, hidden: bool) -> Result<()> {
    let occurrence = row_for_path(occurrences(cluster), file)
        .ok_or_else(|| anyhow!("{file} has no occurrence beside its copies: {report:#}"))?;
    assert_eq!(
        occurrence_is_hidden(occurrence),
        hidden,
        "{file}: wrong `hidden` flag on its occurrence: {report:#}"
    );
    assert_eq!(
        duplicated_loc_for(report, file) == 0,
        hidden,
        "{file}: its lines must count exactly when it is shown: {report:#}"
    );
    Ok(())
}

#[test]
fn dart_automatically_generated_banner_is_hidden() -> Result<()> {
    let (tmp, src) = temp_scan_dir("src")?;

    // The Flutter framework banner verbatim, followed by a non-trivial
    // duplicated function body so a genuine clone cluster would surface
    // were the header not recognised.
    let header = "// This file has been automatically generated. \
        Please do not edit it manually.\n";
    let dup_body = "String formatLabel(int value) {\n  \
        final doubled = value * 2;\n  \
        final label = 'item-$doubled';\n  \
        return label.toUpperCase();\n}\n";
    fs::write(src.join("messages_en.dart"), format!("{header}{dup_body}"))?;
    fs::write(src.join("messages_fr.dart"), format!("{header}{dup_body}"))?;

    // Positive control: a DIFFERENT duplicated body, header-less, must
    // still cluster — proving the engine detects clones at --min-nodes 5
    // and that the banner pair's absence is due to the header, not a setup
    // error that produced no clusters at all. A distinct body keeps this a
    // separate cluster from the banner pair (whose body is normalised
    // identically regardless of the comment).
    let control_body = "int blendChannels(int base, int overlay) {\n  \
        final mixed = base + overlay;\n  \
        final clamped = mixed - base;\n  \
        return clamped * overlay;\n}\n";
    fs::write(src.join("plain_a.dart"), control_body)?;
    fs::write(src.join("plain_b.dart"), control_body)?;

    let (json, body) = scan(tmp.path(), &src)?;
    assert!(
        clusters(&json).iter().any(|cluster| cluster_spans(
            cluster,
            "plain_a.dart",
            "plain_b.dart"
        )),
        "positive control: the header-less duplicate body must still cluster: {body}"
    );
    let leaked = clusters(&json)
        .iter()
        .any(|cluster| cluster_spans(cluster, "messages_en.dart", "messages_fr.dart"));
    assert!(
        !leaked,
        "files carrying the 'automatically generated' banner must be hidden \
         from the ranked report: {body}"
    );
    Ok(())
}

/// Prose *about* generated files, in a file that is not one. The phrase a
/// generator emits as its entire banner appears here inside a sentence
/// that says the opposite, which is what a header comment on hand-written
/// code normally does.
const PROSE_ABOUT_GENERATORS: &str =
    "// Maintenance note: the localisation bundles this helper feeds are\n\
     // automatically generated and must not be edited by hand. This file\n\
     // is not one of them: it is hand-written and reviewed like any other\n\
     // source file.\n";

/// Copied verbatim into both hand-written files, so their duplication is
/// not in doubt.
const HAND_WRITTEN_BODY: &str = "String describeTotal(int count) {\n  \
     final doubled = count * 2;\n  \
     final label = 'total-$doubled';\n  \
     return label.toLowerCase();\n}\n";

/// The two hand-written copies. Neither is generator output.
const HAND_WRITTEN_LEFT: &str = "totals_helper.dart";
const HAND_WRITTEN_RIGHT: &str = "totals_helper_copy.dart";

/// [EXCLUSION-GENERATED-BANNER] `report_hide` covers machine-generated **output**.
/// A hand-written file that merely describes a generator is not output,
/// so hiding it loses a real duplicate from the ranked report and drops
/// its lines out of `metrics.duplicated_loc` — a false negative in the
/// listing and in the headline percentage at once.
#[test]
fn prose_about_generators_does_not_hide_hand_written_duplication() -> Result<()> {
    let (tmp, src) = temp_scan_dir("src")?;
    let source = format!("{PROSE_ABOUT_GENERATORS}{HAND_WRITTEN_BODY}");
    fs::write(src.join(HAND_WRITTEN_LEFT), &source)?;
    fs::write(src.join(HAND_WRITTEN_RIGHT), &source)?;

    let (json, body) = scan(tmp.path(), &src)?;
    assert!(
        report_spans(&json, HAND_WRITTEN_LEFT, HAND_WRITTEN_RIGHT),
        "two hand-written copies of one function are a duplicate, whatever \
         their comments describe: {body}"
    );
    assert!(
        duplicated_loc_for(&json, HAND_WRITTEN_LEFT) > 0,
        "{HAND_WRITTEN_LEFT} is hand-written, so its duplicated lines belong \
         in the headline percentage: {body}"
    );
    Ok(())
}
