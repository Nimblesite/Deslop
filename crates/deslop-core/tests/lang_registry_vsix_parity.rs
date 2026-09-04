//! [PIPELINE-LANG-TRAIT] / [FACET-MODEL] anti-drift — the VS Code manifest
//! must track the parser registry.
//!
//! Shipping a language is a two-repo-half job: the Rust registry gains a
//! parser, and `clients/vscode/package.json` gains the activation events that
//! wake the extension on those files. The second half has now been forgotten
//! three times (#170 → #198 → F#/PHP → Go), and the failure is silent: the
//! engine analyses the file, the editor surfaces stay dark.
//!
//! This is the gate. `watched_source_extensions()` is derived from
//! [`deslop_core::pipeline::default_parsers`], so it cannot be forgotten; the
//! manifest is read as JSON data. Any language added to the registry without a
//! matching `workspaceContains` event turns this test red, and vice versa.
//!
//! The downstream half of the chain (manifest → `types/languages.ts` →
//! hover / inlay / LSP sync / grouping / filters) is guarded by
//! `clients/vscode/src/test/unit/analysedLanguages.unit.test.ts`.

use std::{collections::BTreeSet, fs, path::PathBuf};

use anyhow::{Context, Result};
use deslop_core::pipeline::watched_source_extensions;
use serde_json::Value;

/// Absolute path to the VS Code extension manifest.
fn manifest_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("clients")
        .join("vscode")
        .join("package.json")
}

/// The VSIX manifest parsed as structured data.
fn vsix_manifest() -> Result<Value> {
    deslop_test_support::read_json(&manifest_path())
}

/// Every string in the manifest's top-level `field` array.
fn string_array(manifest: &Value, field: &str) -> Vec<String> {
    manifest
        .get(field)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// Every `activationEvents` entry starting with `prefix`, with the prefix
/// stripped — e.g. `workspaceContains:**/*.` yields the extension set.
fn activation_suffixes(manifest: &Value, prefix: &str) -> BTreeSet<String> {
    string_array(manifest, "activationEvents")
        .iter()
        .filter_map(|event| event.strip_prefix(prefix))
        .map(str::to_owned)
        .collect()
}

/// Registry-declared source extensions, deduplicated for set comparison.
fn registry_extensions() -> BTreeSet<String> {
    watched_source_extensions()
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[test]
fn vsix_workspace_contains_events_cover_every_registered_extension() -> Result<()> {
    let manifest = vsix_manifest()?;
    let declared = activation_suffixes(&manifest, "workspaceContains:**/*.");
    let registered = registry_extensions();

    let missing: Vec<&String> = registered.difference(&declared).collect();
    assert!(
        missing.is_empty(),
        "[FACET-MODEL] the parser registry ships {registered:?} but the VSIX manifest \
         only activates on {declared:?}; add `workspaceContains:**/*.<ext>` for {missing:?} \
         to clients/vscode/package.json or Deslop stays asleep on those repos"
    );

    let stray: Vec<&String> = declared.difference(&registered).collect();
    assert!(
        stray.is_empty(),
        "[FACET-MODEL] the VSIX manifest activates on {stray:?}, which no parser in the \
         registry claims; the extension would wake up and analyse nothing"
    );
    Ok(())
}

#[test]
fn vsix_on_language_events_exist_for_every_registered_language() -> Result<()> {
    let manifest = vsix_manifest()?;
    let on_language = activation_suffixes(&manifest, "onLanguage:");

    // One `onLanguage:` event per editor grammar, so the count can never be
    // below the number of distinct registry ids. `.jsx`/`.tsx` add the React
    // grammars on top, which is why this is a floor and not an equality.
    let language_count = deslop_core::pipeline::language_ids().len();
    assert!(
        on_language.len() >= language_count,
        "[FACET-MODEL] the registry ships {language_count} languages but the VSIX manifest \
         declares only {} `onLanguage:` events: {on_language:?}",
        on_language.len()
    );
    assert!(
        on_language.contains("go"),
        "Go must wake the extension in Go repos, got {on_language:?}"
    );
    assert!(
        on_language.contains("php"),
        "PHP must wake the extension in PHP repos, got {on_language:?}"
    );
    assert!(
        on_language.contains("fsharp"),
        "F# must wake the extension in F# repos, got {on_language:?}"
    );
    Ok(())
}

#[test]
fn vsix_marketplace_keywords_name_every_registered_language() -> Result<()> {
    let manifest = vsix_manifest()?;
    let keywords: BTreeSet<String> = string_array(&manifest, "keywords").into_iter().collect();

    let missing: Vec<&'static str> = deslop_core::pipeline::language_ids()
        .into_iter()
        .filter(|id| !keywords.contains(*id))
        .collect();
    assert!(
        missing.is_empty(),
        "[FACET-MODEL] Marketplace search is how a {missing:?} developer finds Deslop; \
         add those ids to `keywords` in clients/vscode/package.json (have: {keywords:?})"
    );
    Ok(())
}
