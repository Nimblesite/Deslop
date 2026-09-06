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

mod common;

use std::{path::Path, thread, time::Duration};

use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::common::{call, handshake, spawn_lsp_on_fixture};

const DEFINITION: &str = "textDocument/definition";
const HOVER: &str = "textDocument/hover";
const DIAGNOSTIC: &str = "textDocument/diagnostic";
const CODE_LENS: &str = "textDocument/codeLens";
const EXECUTE_COMMAND: &str = "workspace/executeCommand";
const REPORT_GET: &str = "deslop/reportGet";

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
    let (_workspace, mut child, mut stdin, mut stdout, _stderr) =
        spawn_lsp_on_fixture("csharp-small")?;
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
    let _ = child.kill();
    Ok(())
}

#[test]
fn go_to_definition_is_never_answered_by_deslop() -> Result<()> {
    // F12 anywhere — including inside a clone range — must yield no Deslop
    // result, so the editor's own Go To Definition is the sole responder.
    let (_workspace, mut child, mut stdin, mut stdout, _stderr, alpha) = lsp_alpha_session()?;

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
    let _ = child.kill();
    Ok(())
}

// Tests [LSP-HOVER]
#[test]
fn hover_is_never_answered_by_deslop() -> Result<()> {
    // Hover belongs to the editor's language server. The clone card is an
    // additive client-side provider in the VSIX, not an LSP hover.
    let (_workspace, mut child, mut stdin, mut stdout, _stderr, alpha) = lsp_alpha_session()?;

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
    let _ = child.kill();
    Ok(())
}

#[test]
fn canonical_navigation_survives_via_additive_clone_diagnostics() -> Result<()> {
    // Removing the F12 overload must not cost the user canonical-occurrence
    // navigation: the additive clone diagnostic still links to the
    // canonical occurrence in the sibling file via `relatedInformation`.
    let (_workspace, mut child, mut stdin, mut stdout, _stderr, alpha) = lsp_alpha_session()?;
    wait_for_clusters(&mut stdin, &mut stdout)?;

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
    let _ = child.kill();
    Ok(())
}

#[test]
fn additive_code_lens_carries_deslops_own_jump_command_not_definition() -> Result<()> {
    // The additive clone code lens is how Deslop offers occurrence
    // navigation — via its own command, never by overloading F12.
    let (_workspace, mut child, mut stdin, mut stdout, _stderr, alpha) = lsp_alpha_session()?;
    wait_for_clusters(&mut stdin, &mut stdout)?;

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
    let _ = child.kill();
    Ok(())
}

#[test]
fn refresh_command_re_evaluates_the_corpus_after_an_edit() -> Result<()> {
    // The additive `deslop.lsp.refreshReport` verb re-runs analysis:
    // editing Alpha.cs away from its Beta.cs twin drops the clone, and the
    // refresh reports the removal. Exercises Deslop's own command surface,
    // which is wholly separate from any standard editor request.
    let (_workspace, mut child, mut stdin, mut stdout, _stderr, alpha) = lsp_alpha_session()?;
    wait_for_clusters(&mut stdin, &mut stdout)?;

    std::fs::write(
        &alpha,
        "namespace Alpha { public class Solo { public int One() { return 1; } } }\n",
    )?;
    let response = call(
        &mut stdin,
        &mut stdout,
        EXECUTE_COMMAND,
        &json!({ "command": "deslop.lsp.refreshReport" }),
    )?;
    let removed = response
        .pointer("/result/clustersRemoved")
        .and_then(Value::as_u64)
        .ok_or_else(|| anyhow!("refresh response missing clustersRemoved: {response}"))?;
    assert!(
        removed >= 1,
        "editing Alpha.cs away from Beta.cs must drop the clone on refresh: {response}"
    );
    let _ = child.kill();
    Ok(())
}

/// Copies the `csharp-small` fixture, spawns the LSP, completes the
/// handshake, and returns the workspace (keep it bound — dropping it
/// deletes the workspace), the child, its stdin/stdout, the child's
/// stderr (keep it bound — dropping the read end early stalls the
/// heavily-logging LSP on a full stderr pipe so it never answers), and
/// the path to `Alpha.cs`.
fn lsp_alpha_session() -> Result<(
    tempfile::TempDir,
    std::process::Child,
    std::process::ChildStdin,
    std::io::BufReader<std::process::ChildStdout>,
    std::process::ChildStderr,
    std::path::PathBuf,
)> {
    let (workspace, child, mut stdin, mut stdout, stderr) = spawn_lsp_on_fixture("csharp-small")?;
    let alpha = workspace.path().join("Alpha.cs");
    let _init = handshake(&mut stdin, &mut stdout)?;
    Ok((workspace, child, stdin, stdout, stderr, alpha))
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
/// cluster, so the diagnostic pull has clone data to project.
fn wait_for_clusters(
    stdin: &mut std::process::ChildStdin,
    stdout: &mut std::io::BufReader<std::process::ChildStdout>,
) -> Result<()> {
    for _ in 0..60 {
        let response = call(stdin, stdout, REPORT_GET, &json!({}))?;
        let has_clusters = response
            .pointer("/result/clusters")
            .and_then(Value::as_array)
            .is_some_and(|clusters| !clusters.is_empty());
        if has_clusters {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(500));
    }
    Err(anyhow!("no clusters produced within 30s"))
}

fn file_uri(path: &Path) -> Result<String> {
    tower_lsp::lsp_types::Url::from_file_path(path)
        .map(|url| url.to_string())
        .map_err(|()| anyhow!("path not absolute: {}", path.display()))
}
