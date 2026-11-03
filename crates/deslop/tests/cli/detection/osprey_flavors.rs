//! [LANG-CAND-OSPREY] Every syntax flavor retains its source identity.

use super::*;

const MIN_NODES: &str = "10";
const ML_FIXTURE: &str = "osprey-ml";
const MARKER_FIXTURE: &str = "osprey-python-marker";
const PROJECT_FIXTURE: &str = "osprey-python-project";
const PYTHON_FIXTURE: &str = "python-small";
const ML_FILES: [&str; 2] = ["alpha.ospml", "beta.ospml"];
const PYTHON_FILES: [&str; 2] = ["alpha.py", "beta.py"];
const ML_LANGUAGE: &str = "osprey-ml";
const PYTHON_FLAVOR_LANGUAGE: &str = "osprey-python";
const PYTHON_LANGUAGE: &str = "python";
const IDENTICAL_KIND: &str = "identical";
const EXPECTED_RANK: u64 = 1;
const EXCLUDED_FILE: &str = "tools.py";
const EXCLUDED_SOURCE: &str = "# osprey: flavor=default\ndef ignored():\n    return 0\n";
const CONFIG_FILE: &str = ".deslop.toml";
const EXCLUDED_CONFIG: &str = "[defaults]\nexclude = [\"tools.py\"]\n";
const MALFORMED_ML: &str = "describe value =\n    if value > then\n        value +\n";
const ML_PARSE_ERROR: &str = "tree-sitter failed to produce a parse tree for osprey-ml";
const ML_THIRD_COPY: &str = "gamma.ospml";
const ML_FILES_ANALYSED: usize = 2;
const SKIPPED_HEADLINE: &str = "1 file skipped";
const TOO_DEEP_FILE: &str = "deep.py";
const TOO_DEEP_NESTING: usize = 700;
const TOO_DEEP_REASON: &str = "python source nests deeper than the 500-level AST depth limit";
const MANIFEST: &str = "osprey.toml";
const LEADING_MARKER: &str = "# osprey: flavor=python\n";
const LATE_MARKER: &str = "\n# osprey: flavor=python\n";
const STRING_MARKER: &str = "BANNER = \"# osprey: flavor=python\"\n";
const NO_TEXT: &str = "";
const DEFAULT_FIXTURE: &str = "osprey-identical";
const DEFAULT_FILE: &str = "alpha.osp";
const DEFAULT_FILES: [&str; 2] = [DEFAULT_FILE, "beta.osp"];
const DEFAULT_LANGUAGE: &str = "osprey";
const DEFAULT_PROJECT: &str =
    "[project]\npython_sources = [\"alpha.py\", \"beta.py\"]\nflavor = \"default\"\n";
const EXCLUDED_DIRECTORY: &str = "vendor";
const EXCLUDED_DIRECTORY_CONFIG: &str = "[defaults]\nexclude = [\"vendor/**\"]\n";
const MALFORMED_MANIFEST: &str = "[project\nflavor = \n";
const MANIFEST_PARSE_ERROR: &str = "failed to parse";
const SELECTION_ERROR: &str = "cannot select source language";
const FLAVOR_CONFLICT: &str = "flavor marker contradicts the extension";
const CONFLICTING_MARKER: &str = "// osprey: flavor=ml\n";
const FOREIGN_PYTHON_MARKERS: [&str; 3] = [
    "# osprey: flavor=default\n",
    "# osprey: flavor=ml\n",
    "# osprey: flavor=unheard-of\n",
];
const NESTED_PROJECT: &str = "tools";
const UNREADABLE_MANIFEST: &[u8] = &[0xFF, 0xFE, 0x00];
const OVERRIDING_MANIFEST: &str =
    "[project]\nsource_roots = [\"..\"]\npython_sources = [\"../alpha.py\", \"../beta.py\"]\nflavor = \"default\"\n";

#[test]
fn detects_identical_osprey_ml_functions() -> Result<()> {
    assert_flavor(ML_FIXTURE, ML_FILES, ML_LANGUAGE, true)
}

/// [LANG-CAND-OSPREY-UNPARSED] One unparseable file never costs the rest of the scan: the file is
/// named with its reason, left out of the counts, and every other file is still reported.
#[test]
fn malformed_osprey_ml_is_skipped_visibly_and_the_scan_continues() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(ML_FIXTURE)?;
    let source = fs::read_to_string(root.join(ML_FILES[0]))?;
    fs::write(root.join(ML_THIRD_COPY), source)?;
    fs::write(root.join(ML_FILES[0]), MALFORMED_ML)?;

    let prefix = tmp.path().join(REPORT_OUTPUT_STEM);
    let assert = deslop_command(&root, &prefix)?
        .args([MIN_NODES_FLAG, MIN_NODES])
        .assert()
        .success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
    assert_contains(&stderr, ML_PARSE_ERROR, "malformed ML is explicit");
    assert_contains(&stderr, ML_FILES[0], "the skipped file is named");
    assert_contains(&stderr, SKIPPED_HEADLINE, "the skip count is stated");

    let json = fs::read_to_string(with_ext(&prefix, "json"))?;
    assert_skips(&json, ML_FILES_ANALYSED, &[ML_FILES[0]])?;
    let clusters = report_clusters(&json)?;
    assert_eq!(clusters.len(), 1, "the two remaining copies still cluster");
    let cluster = require_cluster_spanning(&clusters, ML_FILES[1], ML_THIRD_COPY)?;
    let occurrences = require_array(cluster, "/occurrences", "surviving cluster")?;
    assert_eq!(occurrences.len(), ML_FILES_ANALYSED, "only parsed files");
    Ok(())
}

/// [LANG-CAND-OSPREY-UNPARSED] A file nesting past the depth limit is skipped on the default
/// incremental path too, and every report format says so.
#[test]
fn a_too_deep_file_is_skipped_visibly_and_the_scan_continues() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(PYTHON_FIXTURE)?;
    let nesting = TOO_DEEP_NESTING;
    let source = format!("x = {}1{}\n", "[".repeat(nesting), "]".repeat(nesting));
    fs::write(root.join(TOO_DEEP_FILE), source)?;

    let prefix = tmp.path().join(REPORT_OUTPUT_STEM);
    let assert = deslop_command(&root, &prefix)?
        .args([MIN_NODES_FLAG, MIN_NODES])
        .assert()
        .success();
    let stderr = String::from_utf8(assert.get_output().stderr.clone())?;
    assert_contains(&stderr, SKIPPED_HEADLINE, "the skip count is stated");
    let line = format!("{TOO_DEEP_FILE}: {TOO_DEEP_REASON}");
    assert_contains(&stderr, &line, "stderr names the file and reason");

    let json = fs::read_to_string(with_ext(&prefix, "json"))?;
    let report: serde_json::Value = serde_json::from_str(&json)?;
    assert_eq!(
        report["files_analysed"].as_u64(),
        Some(PYTHON_FILES.len() as u64)
    );
    assert_eq!(
        report["files_skipped"],
        serde_json::json!([{ "path": TOO_DEEP_FILE, "reason": TOO_DEEP_REASON }]),
    );
    let clusters = report_clusters(&json)?;
    let _spans = require_cluster_spanning(&clusters, PYTHON_FILES[0], PYTHON_FILES[1])?;

    let text = fs::read_to_string(with_ext(&prefix, "txt"))?;
    assert_contains(
        &text,
        &format!("skipped {line}"),
        "the text report names it",
    );
    let html = fs::read_to_string(with_ext(&prefix, "html"))?;
    assert_contains(
        &html,
        "<dt>Files skipped</dt><dd>1</dd>",
        "the HTML report counts it",
    );
    assert_contains(&html, TOO_DEEP_REASON, "the HTML report explains it");
    Ok(())
}

/// [LANG-CAND-OSPREY-UNPARSED] A repository holding nothing but broken ML still finishes clean.
#[test]
fn a_repository_of_only_malformed_osprey_ml_still_reports() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(ML_FIXTURE)?;
    for file in ML_FILES {
        fs::write(root.join(file), MALFORMED_ML)?;
    }
    let json = scan_flavor_root(&root, tmp.path())?;
    assert_skips(&json, 0, &ML_FILES)?;
    assert_eq!(
        report_clusters(&json)?.len(),
        0,
        "nothing parsed, no clusters"
    );
    Ok(())
}

/// [LANG-CAND-OSPREY-UNPARSED] A clean scan reports an empty skip list, not a missing one.
#[test]
fn a_clean_osprey_ml_scan_skips_nothing() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(ML_FIXTURE)?;
    let json = scan_flavor_root(&root, tmp.path())?;
    assert_skips(&json, ML_FILES.len(), &[])?;
    Ok(())
}

/// A manifest that cannot be parsed stops the scan of the Osprey sources it governs, by name.
#[test]
fn malformed_project_manifest_returns_a_visible_error() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(DEFAULT_FIXTURE)?;
    fs::write(root.join(MANIFEST), MALFORMED_MANIFEST)?;
    let stderr = scan_failure(&root, tmp.path())?;
    assert_contains(
        &stderr,
        MANIFEST_PARSE_ERROR,
        "malformed manifest is explicit",
    );
    assert_contains(&stderr, MANIFEST, "the manifest is named");
    Ok(())
}

/// A file named `osprey.toml` says nothing about ordinary Python beside it.
#[test]
fn broken_project_manifests_leave_ordinary_python_analysable() -> Result<()> {
    for manifest in [MALFORMED_MANIFEST.as_bytes(), UNREADABLE_MANIFEST] {
        let (tmp, root) = seeded_fixture_root(PROJECT_FIXTURE)?;
        fs::write(root.join(MANIFEST), manifest)?;
        let json = scan_flavor_root(&root, tmp.path())?;
        assert_flavor_report(&json, PYTHON_FILES, PYTHON_LANGUAGE, true)?;
        assert_eq!(report_clusters(&json)?.len(), 1, "one ordinary pair");
    }
    Ok(())
}

/// A broken manifest governs its own directory, never Osprey sources outside it.
#[test]
fn broken_nested_manifests_leave_sources_outside_them_analysable() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(DEFAULT_FIXTURE)?;
    let nested = root.join(NESTED_PROJECT);
    fs::create_dir_all(&nested)?;
    fs::write(nested.join(MANIFEST), MALFORMED_MANIFEST)?;
    let json = scan_flavor_root(&root, tmp.path())?;
    assert_flavor_report(&json, DEFAULT_FILES, DEFAULT_LANGUAGE, true)?;
    assert_eq!(report_clusters(&json)?.len(), 1, "one Osprey pair");
    Ok(())
}

/// A marker contradicting an Osprey extension stops the scan and names the file.
#[test]
fn conflicting_flavor_marker_returns_a_visible_error() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(DEFAULT_FIXTURE)?;
    let file = root.join(DEFAULT_FILE);
    let source = fs::read_to_string(&file)?;
    fs::write(&file, format!("{CONFLICTING_MARKER}{source}"))?;
    let stderr = scan_failure(&root, tmp.path())?;
    assert_contains(
        &stderr,
        SELECTION_ERROR,
        "the selection failure is explicit",
    );
    assert_contains(&stderr, FLAVOR_CONFLICT, "the conflict is explained");
    assert_contains(&stderr, DEFAULT_FILE, "the conflicting source is named");
    Ok(())
}

/// Only `flavor=python` opts a `.py` file in; any other comment leaves it Python.
#[test]
fn foreign_markers_leave_ordinary_python_analysable() -> Result<()> {
    for marker in FOREIGN_PYTHON_MARKERS {
        assert_comment_selection(marker, NO_TEXT, PYTHON_LANGUAGE)?;
    }
    Ok(())
}

/// Assert the analysed count and, in path order, every skipped file with its reason.
fn assert_skips(json: &str, analysed: usize, skipped: &[&str]) -> Result<()> {
    let report: serde_json::Value = serde_json::from_str(json)?;
    assert_eq!(
        report["files_analysed"].as_u64(),
        u64::try_from(analysed).ok()
    );
    let expected: Vec<serde_json::Value> = skipped
        .iter()
        .map(|path| serde_json::json!({ "path": path, "reason": ML_PARSE_ERROR }))
        .collect();
    assert_eq!(report["files_skipped"], serde_json::Value::Array(expected));
    Ok(())
}

/// Run a scan that must fail and return its stderr.
fn scan_failure(root: &Path, output: &Path) -> Result<String> {
    let prefix = output.join(REPORT_OUTPUT_STEM);
    let stderr = deslop_command(root, &prefix)?
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    Ok(String::from_utf8(stderr)?)
}

#[test]
fn identifies_osprey_python_by_marker() -> Result<()> {
    assert_flavor(MARKER_FIXTURE, PYTHON_FILES, PYTHON_FLAVOR_LANGUAGE, true)
}

#[test]
fn identifies_osprey_python_by_project_selection() -> Result<()> {
    assert_flavor(PROJECT_FIXTURE, PYTHON_FILES, PYTHON_FLAVOR_LANGUAGE, true)
}

#[test]
fn ordinary_python_keeps_its_language_and_duplicates() -> Result<()> {
    assert_flavor(PYTHON_FIXTURE, PYTHON_FILES, PYTHON_LANGUAGE, false)
}

#[test]
fn excluded_flavor_markers_leave_selected_sources_analysable() -> Result<()> {
    let json = excluded_report()?;
    assert_flavor_report(&json, PYTHON_FILES, PYTHON_FLAVOR_LANGUAGE, true)
}

#[test]
fn python_opt_in_requires_a_leading_comment() -> Result<()> {
    for (prefix, suffix, language) in [
        (LEADING_MARKER, NO_TEXT, PYTHON_FLAVOR_LANGUAGE),
        (NO_TEXT, LATE_MARKER, PYTHON_LANGUAGE),
        (STRING_MARKER, NO_TEXT, PYTHON_LANGUAGE),
    ] {
        assert_comment_selection(prefix, suffix, language)?;
    }
    Ok(())
}

#[test]
fn explicit_project_flavor_overrides_the_file_extension() -> Result<()> {
    let (tmp, root) = seeded_fixture_root(PROJECT_FIXTURE)?;
    let source = fs::read_to_string(fixture(DEFAULT_FIXTURE).join(DEFAULT_FILE))?;
    fs::write(root.join(MANIFEST), DEFAULT_PROJECT)?;
    for file in PYTHON_FILES {
        fs::write(root.join(file), &source)?;
    }
    let json = scan_flavor_root(&root, tmp.path())?;
    assert_flavor_report(&json, PYTHON_FILES, DEFAULT_LANGUAGE, true)
}

#[test]
fn excluded_project_manifests_never_block_or_steer_the_scan() -> Result<()> {
    for manifest in [MALFORMED_MANIFEST, OVERRIDING_MANIFEST] {
        let json = excluded_manifest_report(manifest)?;
        assert_flavor_report(&json, PYTHON_FILES, PYTHON_FLAVOR_LANGUAGE, true)?;
        assert_eq!(report_clusters(&json)?.len(), 1, "one selected pair");
    }
    Ok(())
}

/// A manifest under an excluded path is outside the analysed workspace.
fn excluded_manifest_report(manifest: &str) -> Result<String> {
    let (tmp, root) = seeded_fixture_root(PROJECT_FIXTURE)?;
    let excluded = root.join(EXCLUDED_DIRECTORY);
    fs::create_dir_all(&excluded)?;
    fs::write(excluded.join(MANIFEST), manifest)?;
    fs::write(root.join(CONFIG_FILE), EXCLUDED_DIRECTORY_CONFIG)?;
    scan_flavor_root(&root, tmp.path())
}

/// Markers inside text or after implementation cannot change the language.
fn assert_comment_selection(prefix: &str, suffix: &str, language: &str) -> Result<()> {
    let (tmp, root) = seeded_fixture_root(PROJECT_FIXTURE)?;
    fs::remove_file(root.join(MANIFEST))?;
    let source = fs::read_to_string(root.join(PYTHON_FILES[0]))?;
    for file in PYTHON_FILES {
        fs::write(root.join(file), format!("{prefix}{source}{suffix}"))?;
    }
    let json = scan_flavor_root(&root, tmp.path())?;
    assert_flavor_report(&json, PYTHON_FILES, language, true)
}

/// A source excluded by path must never influence parser selection.
fn excluded_report() -> Result<String> {
    let (tmp, root) = seeded_fixture_root(PROJECT_FIXTURE)?;
    fs::write(root.join(EXCLUDED_FILE), EXCLUDED_SOURCE)?;
    fs::write(root.join(CONFIG_FILE), EXCLUDED_CONFIG)?;
    scan_flavor_root(&root, tmp.path())
}

/// Render every selection scenario through the same black-box CLI path.
fn scan_flavor_root(root: &Path, output: &Path) -> Result<String> {
    let prefix = output.join(REPORT_OUTPUT_STEM);
    run_scan(root, &prefix, &[MIN_NODES_FLAG, MIN_NODES])?;
    Ok(fs::read_to_string(with_ext(&prefix, "json"))?)
}

/// Assert clone membership and parser identity on the rendered report.
fn assert_flavor(
    fixture_name: &str,
    files: [&str; 2],
    language: &str,
    identical: bool,
) -> Result<()> {
    let json = run_min_nodes(fixture_name, MIN_NODES)?;
    assert_flavor_report(&json, files, language, identical)
}

/// Check exact occurrence membership, rank, bucket and source identity together.
fn assert_flavor_report(
    json: &str,
    files: [&str; 2],
    language: &str,
    identical: bool,
) -> Result<()> {
    assert_type2_report(json, files[0], files[1])?;
    let clusters = report_clusters(json)?;
    let cluster = require_cluster_spanning(&clusters, files[0], files[1])?;
    assert_eq!(field(cluster, "rank").as_u64(), Some(EXPECTED_RANK));
    if identical {
        assert_eq!(field(cluster, "kind").as_str(), Some(IDENTICAL_KIND));
    }
    for occurrence in require_array(cluster, "/occurrences", "flavor cluster")? {
        assert_eq!(field(occurrence, "language").as_str(), Some(language));
    }
    Ok(())
}
