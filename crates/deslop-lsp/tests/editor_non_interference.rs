//! E2E proof of [LSP-NON-INTERFERENCE]: Deslop's LSP never registers,
//! answers, or slows the editor's standard language features — Go To
//! Definition (`textDocument/definition`), Hover (`textDocument/hover`),
//! and the rest belong exclusively to the editor's real language server.
//! Deslop contributes only additive surfaces (clone diagnostics + code
//! lens). Drives the real `deslop-lsp` binary over stdio — no mocked
//! transport.
//!
//! Regression guard: on a large Flutter/Windows codebase a
//! `definitionProvider` overload made VS Code's F12 spin (it waits for
//! every provider, and Deslop blocked on its in-flight analysis). The
//! structural fix is to advertise none of the standard providers at all,
//! which these tests pin.

use std::{path::Path, thread, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{call, handshake, spawn_lsp_on_fixture_guarded, LspGuard};
use deslop_core::render::signals::plain_explanation;
use deslop_core::report::ReportSignals;

const DEFINITION: &str = "textDocument/definition";
const HOVER: &str = "textDocument/hover";
const DIAGNOSTIC: &str = "textDocument/diagnostic";
const CODE_LENS: &str = "textDocument/codeLens";
const EXECUTE_COMMAND: &str = "workspace/executeCommand";
const REPORT_GET: &str = "deslop/reportGet";
const REFRESH_REPORT_COMMAND: &str = "deslop.lsp.refreshReport";

/// `csharp-small` is exactly the `Alpha.cs`/`Beta.cs` clone pair.
const FIXTURE_FILE_COUNT: u64 = 2;

/// Every standard language-intelligence capability that belongs to the
/// editor's real language server and that Deslop must never advertise.
const FORBIDDEN_CAPABILITIES: &[&str] = &[
    "definitionProvider",
    "hoverProvider",
    "documentLinkProvider",
    "referencesProvider",
    "implementationProvider",
    "typeDefinitionProvider",
    "declarationProvider",
    "completionProvider",
    "renameProvider",
    "signatureHelpProvider",
    "documentFormattingProvider",
    "documentRangeFormattingProvider",
    "documentHighlightProvider",
    "documentSymbolProvider",
];

#[test]
fn initialize_advertises_no_standard_language_providers() -> Result<()> {
    let (_workspace, _guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded("csharp-small")?;
    let init = handshake(&mut stdin, &mut stdout)?;
    let caps = init
        .pointer("/result/capabilities")
        .ok_or_else(|| anyhow!("capabilities missing from initialize: {init}"))?;

    for forbidden in FORBIDDEN_CAPABILITIES {
        assert!(
            caps.get(forbidden).map_or(true, Value::is_null),
            "Deslop must NOT advertise the standard {forbidden} — registering it would let \
             Deslop intercept, override, or stall the editor's own feature: {caps}"
        );
    }

    // Positive control: the additive, Deslop-owned surfaces stay declared,
    // so removing the standard providers did not lobotomise the linter.
    assert!(
        caps.get("codeLensProvider").is_some(),
        "the additive clone code-lens must still be advertised: {caps}"
    );
    assert!(
        caps.get("executeCommandProvider").is_some(),
        "Deslop's own deslop.* commands must still be advertised: {caps}"
    );
    assert!(
        caps.get("diagnosticProvider").is_some(),
        "the additive clone diagnostics must still be advertised: {caps}"
    );
    Ok(())
}

#[test]
fn go_to_definition_is_never_answered_by_deslop() -> Result<()> {
    // F12 anywhere — including inside a clone range — must yield no Deslop
    // result, so the editor's own Go To Definition is the sole responder.
    let (_workspace, _guard, mut stdin, mut stdout, alpha) = lsp_alpha_session()?;

    let response = call(
        &mut stdin,
        &mut stdout,
        DEFINITION,
        &json!({
            "textDocument": { "uri": file_uri(&alpha)? },
            "position": { "line": 6, "character": 12 }
        }),
    )?;
    assert!(
        definition_location(&response).is_none(),
        "Deslop must contribute no Go To Definition location — F12 belongs to the language server: {response}"
    );
    Ok(())
}

// Tests [LSP-HOVER]
#[test]
fn hover_is_never_answered_by_deslop() -> Result<()> {
    // Hover belongs to the editor's language server. The clone card is an
    // additive client-side provider in the VSIX, not an LSP hover.
    let (_workspace, _guard, mut stdin, mut stdout, alpha) = lsp_alpha_session()?;

    let response = call(
        &mut stdin,
        &mut stdout,
        HOVER,
        &json!({
            "textDocument": { "uri": file_uri(&alpha)? },
            "position": { "line": 6, "character": 12 }
        }),
    )?;
    let has_contents = response
        .get("result")
        .filter(|result| !result.is_null())
        .and_then(|result| result.get("contents"))
        .is_some();
    assert!(
        !has_contents,
        "Deslop must contribute no hover contents — hover belongs to the language server: {response}"
    );
    Ok(())
}

#[test]
fn canonical_navigation_survives_via_additive_clone_diagnostics() -> Result<()> {
    // Removing the F12 overload must not cost the user canonical-occurrence
    // navigation: the additive clone diagnostic still links to the
    // canonical occurrence in the sibling file via `relatedInformation`.
    let (_workspace, _guard, mut stdin, mut stdout, alpha) = lsp_alpha_session()?;
    let report = wait_for_clusters(&mut stdin, &mut stdout)?;

    let response = call(
        &mut stdin,
        &mut stdout,
        DIAGNOSTIC,
        &json!({ "textDocument": { "uri": file_uri(&alpha)? } }),
    )?;
    let items = response
        .pointer("/result/items")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("diagnostic report missing items: {response}"))?;
    assert!(
        !items.is_empty(),
        "the Alpha/Beta clone must publish at least one Deslop diagnostic: {response}"
    );
    let links_to_canonical = items.iter().any(|item| {
        item.get("source").and_then(Value::as_str) == Some("deslop")
            && item
                .pointer("/relatedInformation/0/location/uri")
                .and_then(Value::as_str)
                .is_some_and(|uri| uri.contains("Beta.cs"))
    });
    assert!(
        links_to_canonical,
        "the clone diagnostic must link to the canonical occurrence in Beta.cs so navigation \
         survives without overloading F12: {response}"
    );

    // [FUSION-CONTENT-GATE] #344: the Problems panel is a decision surface.
    // The bucket title alone is unfalsifiable — a corroborated Type-2 rename
    // and an anchor-poor scaffolding family both render structural 1.00 — so
    // the message must also state the fused score and the measured evidence.
    let deslop_item = items
        .iter()
        .find(|item| item.get("source").and_then(Value::as_str) == Some("deslop"))
        .ok_or_else(|| anyhow!("a deslop-sourced diagnostic: {response}"))?;
    let cluster_id = deslop_item
        .pointer("/data/cluster_id")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("diagnostic data carries the cluster id: {deslop_item}"))?;
    let message = deslop_item
        .get("message")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("diagnostic carries a message: {deslop_item}"))?;
    assert!(
        message.contains(" × ") && message.contains("code"),
        "the pre-existing bucket title and occurrence count survive the addition: {message}"
    );
    assert_explains_confidence(message, cluster_signals(&report, cluster_id)?, "diagnostic");
    Ok(())
}

#[test]
fn additive_code_lens_carries_deslops_own_jump_command_not_definition() -> Result<()> {
    // The additive clone code lens is how Deslop offers occurrence
    // navigation — via its own command, never by overloading F12.
    let (_workspace, _guard, mut stdin, mut stdout, alpha) = lsp_alpha_session()?;
    let report = wait_for_clusters(&mut stdin, &mut stdout)?;

    let response = call(
        &mut stdin,
        &mut stdout,
        CODE_LENS,
        &json!({ "textDocument": { "uri": file_uri(&alpha)? } }),
    )?;
    let lenses = response
        .pointer("/result")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("code lens result missing: {response}"))?;
    let first_lens = lenses.first().ok_or_else(|| {
        anyhow!(
            "Deslop must contribute its additive clone code lens on a duplicated file: {response}"
        )
    })?;
    let command = first_lens
        .pointer("/command/command")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("code lens command missing: {response}"))?;
    assert_eq!(
        command, "deslop.jumpToNextOccurrence",
        "the lens must navigate via Deslop's own command, never textDocument/definition: {response}"
    );

    // [FUSION-CONTENT-GATE] #344: the lens is the inline decision surface, so
    // it carries the same explanation the Problems panel does.
    let cluster_id = first_lens
        .pointer("/command/arguments/0")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("lens command names its cluster: {first_lens}"))?;
    let title = first_lens
        .pointer("/command/title")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow!("lens carries a title: {first_lens}"))?;
    assert!(
        title.starts_with("●● ") && title.ends_with(" — jump to next"),
        "the pre-existing glyph, count and action survive the addition: {title}"
    );
    assert!(
        !title.contains('`'),
        "a lens title is rendered verbatim — Markdown code spans would show as \
         literal backticks: {title}"
    );
    assert_explains_confidence(title, cluster_signals(&report, cluster_id)?, "code lens");
    Ok(())
}

#[test]
fn refresh_command_re_evaluates_the_corpus_after_an_edit() -> Result<()> {
    // The additive `deslop.lsp.refreshReport` verb re-runs analysis:
    // editing Alpha.cs away from its Beta.cs twin drops the clone, and the
    // refresh reports the removal. Exercises Deslop's own command surface,
    // which is wholly separate from any standard editor request.
    let (_workspace, _guard, mut stdin, mut stdout, alpha) = lsp_alpha_session()?;
    let _report = wait_for_clusters(&mut stdin, &mut stdout)?;

    std::fs::write(
        &alpha,
        "namespace Alpha { public class Solo { public int One() { return 1; } } }\n",
    )?;
    let response = call(
        &mut stdin,
        &mut stdout,
        EXECUTE_COMMAND,
        &json!({ "command": REFRESH_REPORT_COMMAND }),
    )?;
    assert_eq!(
        response.pointer("/result/command").and_then(Value::as_str),
        Some(REFRESH_REPORT_COMMAND),
        "the refresh response names Deslop's own verb: {response}"
    );
    // GH #312: the response's delta counts diff against whichever snapshot
    // came immediately before, so they race the live watcher — on a busy
    // machine the watcher drops the clone first and `clustersRemoved` is
    // legitimately zero. The refresh re-runs analysis synchronously, so
    // the end state is deterministic whichever side ran first: assert it
    // instead of the delta.
    let refreshed = call(&mut stdin, &mut stdout, REPORT_GET, &json!({}))?;
    let clusters = refreshed
        .pointer("/result/clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("refreshed report missing clusters: {refreshed}"))?;
    assert!(
        clusters.is_empty(),
        "editing Alpha.cs away from Beta.cs must leave no clone in the refreshed \
         report — the pre-edit pass above proved the pair was detected: {refreshed:#}"
    );
    assert_eq!(
        refreshed
            .pointer("/result/files_analysed")
            .and_then(Value::as_u64),
        Some(FIXTURE_FILE_COUNT),
        "both fixture files stay analysed after the refresh: {refreshed:#}"
    );
    Ok(())
}

/// Copies the `csharp-small` fixture, spawns the LSP under an armed
/// [`LspGuard`], completes the handshake, and returns the workspace (keep it
/// bound — dropping it deletes the workspace), the guard (keep it bound — it
/// reaps the child and drains its stderr for the whole test, GH #370), the
/// child's stdin/stdout, and the path to `Alpha.cs`.
fn lsp_alpha_session() -> Result<(
    tempfile::TempDir,
    LspGuard,
    std::process::ChildStdin,
    std::io::BufReader<std::process::ChildStdout>,
    std::path::PathBuf,
)> {
    let (workspace, guard, mut stdin, mut stdout) = spawn_lsp_on_fixture_guarded("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let _init = handshake(&mut stdin, &mut stdout)?;
    Ok((workspace, guard, stdin, stdout, alpha))
}

/// Extracts a definition target URI from any of the shapes the LSP allows
/// (`Location`, `Location[]`, `LocationLink[]`). Returns `None` when the
/// response carries no usable definition — including a method-not-found
/// error or a null result, both of which mean Deslop declined to answer.
fn definition_location(response: &Value) -> Option<&Value> {
    let result = response.get("result").filter(|value| !value.is_null())?;
    result
        .pointer("/uri")
        .or_else(|| result.pointer("/0/uri"))
        .or_else(|| result.pointer("/0/targetUri"))
}

/// Polls `deslop/reportGet` until the analysis has produced at least one
/// cluster, so the diagnostic pull has clone data to project. Returns the
/// settled report so a caller can pin a rendered surface against the exact
/// signal numbers the wire published.
fn wait_for_clusters(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
) -> Result<Value> {
    for _ in 0..60 {
        let response = call(stdin, stdout, REPORT_GET, &json!({}))?;
        let has_clusters = response
            .pointer("/result/clusters")
            .and_then(Value::as_array)
            .is_some_and(|clusters| !clusters.is_empty());
        if has_clusters {
            return Ok(response);
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err(anyhow!("no clusters produced within 30s"))
}

/// Reads the wire signals of `cluster_id` out of a `deslop/reportGet`
/// response.
fn cluster_signals(report: &Value, cluster_id: &str) -> Result<ReportSignals> {
    let clusters = report
        .pointer("/result/clusters")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("report carries no clusters: {report}"))?;
    let signals = clusters
        .iter()
        .find(|cluster| cluster.get("id").and_then(Value::as_str) == Some(cluster_id))
        .and_then(|cluster| cluster.get("signals"))
        .ok_or_else(|| anyhow!("cluster {cluster_id} missing from report: {report}"))?;
    Ok(serde_json::from_value(signals.clone())?)
}

/// The confidence explanation [FUSION-CONTENT-GATE] every plain-text Deslop
/// surface must carry. Spelled out here rather than borrowed from the
/// renderer, so a surface that quietly drops the fused score or the measured
/// content evidence fails this test instead of agreeing with itself.
fn expected_explanation(signals: ReportSignals) -> String {
    format!(
        "structural {structural:.2} · jaccard {jaccard:.2} · embedding {embedding:.2} · \
         fused {fused:.2} · agreement {agreement:.2} · rename {rename:.2} · \
         literal {literal:.2}",
        structural = signals.structural,
        jaccard = signals.token_jaccard,
        embedding = signals.embedding_cos,
        fused = signals.fused,
        agreement = signals.agreement,
        rename = signals.rename_consistency,
        literal = signals.literal_fraction,
    )
}

/// Asserts the shared renderer and this test agree on the explanation, then
/// that `rendered` carries it verbatim.
fn assert_explains_confidence(rendered: &str, signals: ReportSignals, surface: &str) {
    let expected = expected_explanation(signals);
    assert_eq!(
        plain_explanation(signals),
        expected,
        "the {surface} must use the shared render::signals rendering, never a second format"
    );
    assert!(
        rendered.contains(&expected),
        "the {surface} must state the fused confidence and the measured content evidence \
         [FUSION-CONTENT-GATE]: expected `{expected}` inside `{rendered}`"
    );
    assert!(
        signals.fused > 0.0 && signals.structural > 0.0,
        "a published clone must carry positive support, else the {surface} pins nothing: \
         {signals:?}"
    );
}

fn file_uri(path: &Path) -> Result<String> {
    tower_lsp::lsp_types::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|()| anyhow!("path not absolute: {}", path.display()))
}
