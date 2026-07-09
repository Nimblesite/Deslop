//! E2E coverage for the `refactor.extract` code action
//! ([AUTOFIX-EXTRACT-CODE-ACTION], [AUTOFIX-EXTRACT-WORKSPACE-EDIT],
//! [AUTOFIX-EXTRACT-TESTING] case 1 + naming case 4).
//!
//! Spawns the real `deslop-lsp` binary on a fixture workspace, requests
//! code actions inside a duplicated method body, applies the returned
//! `WorkspaceEdit` client-side, and asserts the resulting buffer
//! matches the shared golden snapshot.

mod common;

use std::{
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use anyhow::{anyhow, ensure, Context, Result};
use common::{call, handshake, spawn_lsp_on_fixture_guarded};
use serde_json::{json, Value};

/// Analysis must settle within this budget on any dev machine; the
/// fixture is a single small file.
const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(20);

/// Poll cadence while waiting for the first analysis pass.
const POLL_INTERVAL: Duration = Duration::from_millis(50);

/// Golden files live beside this test per [AUTOFIX-EXTRACT-TESTING].
fn golden(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("code_action")
        .join(name)
}

/// Builds a `textDocument/codeAction` params payload for `uri` covering
/// the zero-indexed `line` span.
fn code_action_params(uri: &str, start_line: u32, end_line: u32) -> Value {
    json!({
        "textDocument": { "uri": uri },
        "range": {
            "start": { "line": start_line, "character": 0 },
            "end": { "line": end_line, "character": 0 }
        },
        "context": { "diagnostics": [] }
    })
}

/// Applies LSP `TextEdit`s (descending start order) to an ASCII source
/// buffer, mirroring the editor's atomic `workspace/applyEdit`.
fn apply_text_edits(source: &str, edits: &[Value]) -> Result<String> {
    ensure!(
        source.is_ascii(),
        "fixture must stay ASCII for the test's offset math"
    );
    let mut buffer = source.to_owned();
    for edit in edits {
        let start = byte_offset(source, edit.pointer("/range/start").context("edit start")?)?;
        let end = byte_offset(source, edit.pointer("/range/end").context("edit end")?)?;
        let new_text = edit
            .pointer("/newText")
            .and_then(Value::as_str)
            .context("edit newText")?;
        ensure!(start <= end && end <= buffer.len(), "edit range in bounds");
        buffer.replace_range(start..end, new_text);
    }
    Ok(buffer)
}

/// ASCII line/character → byte offset against the ORIGINAL buffer (the
/// edits are descending, so untouched offsets stay valid during apply).
fn byte_offset(source: &str, position: &Value) -> Result<usize> {
    let line = position
        .pointer("/line")
        .and_then(Value::as_u64)
        .context("position line")?;
    let character = position
        .pointer("/character")
        .and_then(Value::as_u64)
        .context("position character")?;
    let line_start = source
        .split_inclusive('\n')
        .scan(0_usize, |offset, text| {
            let start = *offset;
            *offset = offset.saturating_add(text.len());
            Some(start)
        })
        .nth(usize::try_from(line).context("line fits usize")?)
        .context("line exists in fixture")?;
    Ok(line_start.saturating_add(usize::try_from(character).context("character fits usize")?))
}

/// Polls `textDocument/codeAction` until the first analysis pass
/// surfaces the extract action (bounded, no arbitrary sleeps beyond the
/// poll cadence).
fn wait_for_actions(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
    params: &Value,
) -> Result<Vec<Value>> {
    let deadline = Instant::now()
        .checked_add(ANALYSIS_TIMEOUT)
        .unwrap_or_else(Instant::now);
    loop {
        let response = call(stdin, stdout, "textDocument/codeAction", params)?;
        if let Some(actions) = response.pointer("/result").and_then(Value::as_array) {
            if !actions.is_empty() {
                return Ok(actions.clone());
            }
        }
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "no code action surfaced within {ANALYSIS_TIMEOUT:?}"
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// [AUTOFIX-EXTRACT-TESTING] case 1: a C# fixture with two
/// byte-identical method bodies offers `refactor.extract`; applying the
/// returned edit produces the golden buffer; the helper name embeds the
/// cluster id (case 4).
#[test]
fn csharp_type1_fixture_offers_and_applies_extract_action() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) =
        spawn_lsp_on_fixture_guarded("csharp-extract-type1")?;
    let init = handshake(&mut stdin, &mut stdout)?;
    let kinds = init
        .pointer("/result/capabilities/codeActionProvider/codeActionKinds")
        .and_then(Value::as_array)
        .context("codeActionProvider advertised")?;
    ensure!(
        kinds
            == &vec![
                Value::String("refactor.extract".into()),
                Value::String("refactor.rewrite".into())
            ],
        "capability must advertise extract + rewrite kinds, got {kinds:?}"
    );
    ensure!(
        init.pointer("/result/capabilities/codeActionProvider/resolveProvider")
            == Some(&Value::Bool(true)),
        "lazy resolve must be advertised ([AUTOFIX-MERGE-CODE-ACTION])"
    );

    let file = workspace.path().join("InvoiceMath.cs");
    let uri = tower_lsp::lsp_types::Url::from_file_path(&file)
        .map_err(|()| anyhow!("fixture path is absolute"))?;
    // Lines 4..6 sit inside the first duplicated method body.
    let params = code_action_params(uri.as_str(), 4, 6);
    let actions = wait_for_actions(&mut stdin, &mut stdout, &params)?;

    let action = actions
        .iter()
        .find(|action| {
            action.pointer("/title").and_then(Value::as_str)
                == Some("Extract identical code to shared method")
        })
        .context("extract action present")?;
    ensure!(
        action.pointer("/kind").and_then(Value::as_str) == Some("refactor.extract"),
        "action kind must be refactor.extract"
    );

    let edits = action
        .pointer(&format!(
            "/edit/changes/{}",
            uri.as_str().replace('/', "~1")
        ))
        .and_then(Value::as_array)
        .cloned()
        .context("edit targets the fixture document")?;
    ensure!(
        edits.len() == 3,
        "one insertion plus two call-site rewrites expected, got {}",
        edits.len()
    );
    let inserted = edits
        .iter()
        .filter_map(|edit| edit.pointer("/newText").and_then(Value::as_str))
        .find(|text| text.contains("private static"))
        .context("helper insertion present")?;
    ensure!(
        inserted.contains("ExtractedFromCluster_"),
        "helper name must embed the cluster id prefix, got: {inserted}"
    );

    let source = fs::read_to_string(&file).context("fixture source")?;
    let applied = apply_text_edits(&source, &edits)?;
    let expected = fs::read_to_string(golden("InvoiceMath.applied.cs")).context("golden")?;
    ensure!(
        applied == expected,
        "applied buffer must match the shared golden.\n--- applied ---\n{applied}"
    );
    Ok(())
}

/// Compact end-to-end scenario shared by the Rust and Python cases
/// ([AUTOFIX-EXTRACT-TESTING] case 2): action offered inside the first
/// duplicated body, applied edit matches the shared golden.
fn assert_language_case(
    fixture_name: &str,
    file_name: &str,
    body_lines: (u32, u32),
    golden_name: &str,
) -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(fixture_name)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let file = workspace.path().join(file_name);
    let uri = tower_lsp::lsp_types::Url::from_file_path(&file)
        .map_err(|()| anyhow!("fixture path is absolute"))?;
    let params = code_action_params(uri.as_str(), body_lines.0, body_lines.1);
    let actions = wait_for_actions(&mut stdin, &mut stdout, &params)?;
    let edits = actions
        .iter()
        .find_map(|action| {
            action
                .pointer(&format!(
                    "/edit/changes/{}",
                    uri.as_str().replace('/', "~1")
                ))
                .and_then(Value::as_array)
        })
        .cloned()
        .context("extract action carries edits for the fixture document")?;
    let source = fs::read_to_string(&file).context("fixture source")?;
    let applied = apply_text_edits(&source, &edits)?;
    let expected = fs::read_to_string(golden(golden_name)).context("golden")?;
    ensure!(
        applied == expected,
        "{fixture_name}: applied buffer must match golden.\n--- applied ---\n{applied}"
    );
    Ok(())
}

/// [AUTOFIX-EXTRACT-TESTING] case 2 (Rust): free function + `DeslopTodo`
/// alias inserted at module scope, both occurrences become calls.
#[test]
fn rust_type1_fixture_offers_and_applies_extract_action() -> Result<()> {
    assert_language_case(
        "rust-extract-type1",
        "metrics.rs",
        (1, 3),
        "metrics.applied.rs",
    )
}

/// [AUTOFIX-EXTRACT-TESTING] case 2 (Python): module-scope `def` with
/// PEP 8 spacing, both occurrences become calls.
#[test]
fn python_type1_fixture_offers_and_applies_extract_action() -> Result<()> {
    assert_language_case(
        "python-extract-type1",
        "metrics.py",
        (1, 3),
        "metrics.applied.py",
    )
}

/// Polls `deslop/reportGet` until the first analysis pass lands, so a
/// negative code-action assertion is made against a live report rather
/// than an empty boot-time snapshot.
fn wait_for_analysis(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
) -> Result<()> {
    let deadline = Instant::now()
        .checked_add(ANALYSIS_TIMEOUT)
        .unwrap_or_else(Instant::now);
    loop {
        let report = call(stdin, stdout, "deslop/reportGet", &json!({}))?;
        if common::cluster_count(&report) > 0 {
            return Ok(());
        }
        if Instant::now() >= deadline {
            return Err(anyhow!(
                "analysis produced no clusters within {ANALYSIS_TIMEOUT:?}"
            ));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}

/// Negative-path scenario ([AUTOFIX-EXTRACT-TESTING] case 3): the
/// fixture's clusters exist, yet no extract action is offered anywhere
/// in the file.
fn assert_no_action_over_file(fixture_name: &str, file_name: &str) -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded(fixture_name)?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    wait_for_analysis(&mut stdin, &mut stdout)?;
    let file = workspace.path().join(file_name);
    let uri = tower_lsp::lsp_types::Url::from_file_path(&file)
        .map_err(|()| anyhow!("fixture path is absolute"))?;
    let params = code_action_params(uri.as_str(), 0, 40);
    let response = call(&mut stdin, &mut stdout, "textDocument/codeAction", &params)?;
    let result = response.pointer("/result").context("result present")?;
    let extract_offered = result.as_array().is_some_and(|actions| {
        actions.iter().any(|action| {
            action.pointer("/kind").and_then(Value::as_str) == Some("refactor.extract")
        })
    });
    ensure!(
        !extract_offered,
        "{fixture_name}: the verbatim extract action must not be offered          (these clusters belong to [AUTOFIX-MERGE] / [AUTOFIX-CONSOLIDATE]), got {result}"
    );
    Ok(())
}

/// Type-2 (renamed identifiers inside the bodies) offers no verbatim
/// action — that cluster belongs to [AUTOFIX-MERGE].
#[test]
fn type2_fixture_offers_no_action() -> Result<()> {
    assert_no_action_over_file("csharp-extract-type2", "RateMath.cs")
}

/// Cross-file identical definitions offer no verbatim action — that
/// cluster belongs to [AUTOFIX-CONSOLIDATE].
#[test]
fn cross_file_fixture_offers_no_action() -> Result<()> {
    assert_no_action_over_file("csharp-extract-crossfile", "InvoiceTotals.cs")
}

/// Same-file occurrences in two different classes offer no action
/// ([AUTOFIX-EXTRACT-PRECONDITIONS] rule 4).
#[test]
fn cross_class_fixture_offers_no_action() -> Result<()> {
    assert_no_action_over_file("csharp-extract-crossclass", "Totals.cs")
}

/// A range that touches no cluster occurrence (the class declaration
/// line) yields no action — the server returns an empty result, never
/// a partial edit ([AUTOFIX-EXTRACT-CODE-ACTION]).
#[test]
fn range_outside_occurrences_offers_no_action() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) =
        spawn_lsp_on_fixture_guarded("csharp-extract-type1")?;
    let _init = handshake(&mut stdin, &mut stdout)?;

    let file = workspace.path().join("InvoiceMath.cs");
    let uri = tower_lsp::lsp_types::Url::from_file_path(&file)
        .map_err(|()| anyhow!("fixture path is absolute"))?;
    // First wait until analysis is live (the in-body range offers an
    // action), then probe the class-declaration line.
    let in_body = code_action_params(uri.as_str(), 4, 6);
    let _ready = wait_for_actions(&mut stdin, &mut stdout, &in_body)?;

    let outside = code_action_params(uri.as_str(), 0, 0);
    let response = call(&mut stdin, &mut stdout, "textDocument/codeAction", &outside)?;
    let result = response.pointer("/result").context("result present")?;
    ensure!(
        result.is_null(),
        "class-declaration line must offer no extract action, got {result}"
    );
    Ok(())
}

/// [AUTOFIX-MERGE-CODE-ACTION]: the leaf-gap fixture offers a lazily
/// resolved `refactor.rewrite`; `codeAction/resolve` attaches the
/// transactional edit; applying it matches the merge golden.
#[test]
fn merge_fixture_offers_and_resolves_rewrite_action() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) =
        spawn_lsp_on_fixture_guarded("csharp-merge-leafgap")?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let file = workspace.path().join("RateLimits.cs");
    let uri = tower_lsp::lsp_types::Url::from_file_path(&file)
        .map_err(|()| anyhow!("fixture path is absolute"))?;
    let params = code_action_params(uri.as_str(), 4, 6);
    let actions = wait_for_actions(&mut stdin, &mut stdout, &params)?;
    let offer = rewrite_offer(&actions, "Merge duplicates into one parameterised helper")?;

    let resolved = call(&mut stdin, &mut stdout, "codeAction/resolve", offer)?;
    let edits = resolved
        .pointer("/result/edit/documentChanges/0/edits")
        .and_then(Value::as_array)
        .cloned()
        .context("resolve attaches the transactional edit")?;
    ensure!(
        edits.len() == 3,
        "insertion + two rewrites, got {}",
        edits.len()
    );
    let source = fs::read_to_string(&file)?;
    let applied = apply_text_edits(&source, &edits)?;
    let expected = fs::read_to_string(golden("RateLimits.merged.cs")).context("golden")?;
    ensure!(
        applied == expected,
        "resolved merge must match the shared golden.\n--- applied ---\n{applied}"
    );
    Ok(())
}

/// Finds the lazily-resolved `refactor.rewrite` offer with `title`
/// among `actions`, asserting the shared offer shape
/// ([AUTOFIX-MERGE-CODE-ACTION] step 1): kind `refactor.rewrite`, edit
/// omitted, cluster id in `data`.
fn rewrite_offer<'a>(actions: &'a [Value], title: &str) -> Result<&'a Value> {
    let offer = actions
        .iter()
        .find(|action| action.pointer("/title").and_then(Value::as_str) == Some(title))
        .with_context(|| format!("rewrite offer `{title}` present"))?;
    ensure!(
        offer.pointer("/kind").and_then(Value::as_str) == Some("refactor.rewrite"),
        "offer kind must be refactor.rewrite"
    );
    ensure!(
        offer.pointer("/edit").is_none(),
        "the offer omits the edit — lazy resolve"
    );
    ensure!(
        offer
            .pointer("/data/cluster_id")
            .and_then(Value::as_str)
            .is_some(),
        "the offer carries the cluster id"
    );
    Ok(offer)
}

/// [AUTOFIX-CONSOLIDATE-SURFACE] (issue #277): a cross-file identical
/// definition offers a lazily resolved `refactor.rewrite`; resolving
/// attaches the consolidation edit; applying it removes the duplicate
/// definition and imports the canonical symbol.
#[test]
fn cross_file_fixture_offers_and_resolves_consolidate_action() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) =
        spawn_lsp_on_fixture_guarded("rust-consolidate")?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let file = workspace.path().join("pricing_b.rs");
    let uri = tower_lsp::lsp_types::Url::from_file_path(&file)
        .map_err(|()| anyhow!("fixture path is absolute"))?;
    let params = code_action_params(uri.as_str(), 2, 5);
    let actions = wait_for_actions(&mut stdin, &mut stdout, &params)?;
    let offer = rewrite_offer(
        &actions,
        "Consolidate identical duplicates into one canonical definition",
    )?;
    let resolved = call(&mut stdin, &mut stdout, "codeAction/resolve", offer)?;
    let changes = resolved
        .pointer("/result/edit/documentChanges")
        .and_then(Value::as_array)
        .cloned()
        .context("resolve attaches the consolidation edit ([AUTOFIX-CONSOLIDATE-EDIT])")?;
    let (target_uri, edits) = changes
        .iter()
        .find_map(|change| {
            let uri_text = change
                .pointer("/textDocument/uri")
                .and_then(Value::as_str)?;
            let edits = change.pointer("/edits").and_then(Value::as_array)?;
            (!edits.is_empty()).then(|| (uri_text.to_owned(), edits.clone()))
        })
        .context("one duplicate file receives edits")?;
    let target = tower_lsp::lsp_types::Url::parse(&target_uri)?
        .to_file_path()
        .map_err(|()| anyhow!("edited uri is a file path"))?;
    let source = fs::read_to_string(&target)?;
    let applied = apply_text_edits(&source, &edits)?;
    ensure!(
        applied.starts_with("use crate::pricing_"),
        "duplicate imports the canonical symbol:\n{applied}"
    );
    ensure!(
        !applied.contains("pub fn normalise_labels"),
        "duplicate no longer defines the symbol:\n{applied}"
    );
    Ok(())
}

/// Resolving a drifted cluster disables the action with the routing
/// reason instead of attaching an edit.
#[test]
fn drifted_fixture_resolve_disables_with_reason() -> Result<()> {
    let (workspace, _guard, mut stdin, mut stdout) =
        spawn_lsp_on_fixture_guarded("csharp-merge-drift")?;
    let _init = handshake(&mut stdin, &mut stdout)?;
    let file = workspace.path().join("DriftLimits.cs");
    let uri = tower_lsp::lsp_types::Url::from_file_path(&file)
        .map_err(|()| anyhow!("fixture path is absolute"))?;
    let params = code_action_params(uri.as_str(), 4, 6);
    let actions = wait_for_actions(&mut stdin, &mut stdout, &params)?;
    let offer = rewrite_offer(&actions, "Merge duplicates into one parameterised helper")?;
    let resolved = call(&mut stdin, &mut stdout, "codeAction/resolve", offer)?;
    ensure!(
        resolved.pointer("/result/edit").is_none(),
        "no edit attaches to a refused merge"
    );
    let reason = resolved
        .pointer("/result/disabled/reason")
        .and_then(Value::as_str)
        .context("refusal reason surfaces on the action")?;
    ensure!(!reason.is_empty(), "reason is human-readable");
    Ok(())
}
