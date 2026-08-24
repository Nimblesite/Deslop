// Unit: the analysed-language registry must cover every language the core
// parser registry ships — [FACET-MODEL] anti-drift (#170/#198). Regression
// guard for the F#/PHP hover gap: the ClusterHoverProvider and inlay bubble
// only attach to documents whose editor language id is in
// ANALYSED_LANGUAGE_IDS, so a missing id silently disables hover for that
// language even though diagnostics still render.
//
// The named-language tests below are the human-readable half. The derived
// tests underneath them are the half that catches the *next* language: they
// compare the registry against `package.json`'s activation events, which
// whoever adds a language always remembers to touch. The upstream link —
// core parser registry → `package.json` — is guarded in Rust by
// `crates/deslop-core/tests/lang_registry_vsix_parity.rs`.

import * as assert from "node:assert/strict";
import {
  ANALYSED_LANGUAGE_IDS,
  LANGUAGES,
  SOURCE_EXTENSIONS,
  languageDisplayName,
  languageForPath,
} from "../../types/languages";
import { extensionPackage } from "./package.helpers";

// VS Code names two of our grammars differently from our language ids:
// `.tsx` is `typescriptreact` in the editor, and `.jsx` files carry the
// `javascriptreact` grammar while sharing our `javascript` id. Every other
// registry id must appear verbatim in ANALYSED_LANGUAGE_IDS.
const EDITOR_GRAMMAR_ID: Record<string, string> = {
  tsx: "typescriptreact",
};

/** Activation-event suffixes for a given prefix, e.g. every `onLanguage:` id. */
function activationSuffixes(prefix: string): string[] {
  return extensionPackage()
    .activationEvents.filter((event) => event.startsWith(prefix))
    .map((event) => event.slice(prefix.length))
    .sort();
}

suite("analysed language registry covers F#, PHP and Go", () => {
  test("F#, PHP and Go editor ids are analysed (hover + inlay attach)", () => {
    assert.ok(
      ANALYSED_LANGUAGE_IDS.includes("fsharp"),
      "fsharp must be an analysed language or the F# hover card never registers",
    );
    assert.ok(
      ANALYSED_LANGUAGE_IDS.includes("php"),
      "php must be an analysed language or the PHP hover card never registers",
    );
    assert.ok(
      ANALYSED_LANGUAGE_IDS.includes("go"),
      "go must be an analysed language or the LSP never syncs .go buffers, " +
        "which kills the live loop, the hover card and the inlay bubble for Go",
    );
  });

  test("F#, PHP and Go source extensions resolve to their language id", () => {
    assert.equal(languageForPath("/repo/Tests.fs"), "fsharp");
    assert.equal(languageForPath("/repo/Script.fsx"), "fsharp");
    assert.equal(languageForPath("/repo/Model.php"), "php");
    assert.equal(languageForPath("/repo/cmd/server/main.go"), "go");
    assert.equal(languageForPath("/repo/cmd/server/MAIN.GO"), "go");
  });

  test("F#, PHP and Go carry human display names", () => {
    assert.equal(languageDisplayName("fsharp"), "F#");
    assert.equal(languageDisplayName("php"), "PHP");
    assert.equal(languageDisplayName("go"), "Go");
  });

  test("Go is offered as a filter option in the report webview", () => {
    // The `<select>` in webview-ui maps over LANGUAGES; a language absent
    // here is unfilterable even once its clusters are in the report.
    assert.ok(
      LANGUAGES.includes("go"),
      `Go must be a filterable language, got ${JSON.stringify(LANGUAGES)}`,
    );
  });

  test("every registry language has an editor grammar in ANALYSED_LANGUAGE_IDS", () => {
    const analysed = new Set(ANALYSED_LANGUAGE_IDS);
    const missing = LANGUAGES.filter((id) => !analysed.has(EDITOR_GRAMMAR_ID[id] ?? id));
    assert.deepEqual(
      missing,
      [],
      `[FACET-MODEL] these registry languages have no editor grammar id, so hover, ` +
        `the inlay bubble and LSP document sync all skip them: ${JSON.stringify(missing)}`,
    );
  });

  test("no display name falls through to the Other bucket", () => {
    const unnamed = LANGUAGES.filter((id) => languageDisplayName(id) === "Other");
    assert.deepEqual(
      unnamed,
      [],
      `[FACET-MODEL] these languages group under "Other" in Top Offenders because ` +
        `LANGUAGE_DISPLAY has no entry: ${JSON.stringify(unnamed)}`,
    );
  });

  test("onLanguage activation events match ANALYSED_LANGUAGE_IDS exactly", () => {
    assert.deepEqual(
      activationSuffixes("onLanguage:"),
      [...ANALYSED_LANGUAGE_IDS].sort(),
      "[FACET-MODEL] an onLanguage event with no matching analysed id wakes the " +
        "extension into a dead editor; an analysed id with no event never wakes at all",
    );
  });

  test("workspaceContains activation events match the extension registry exactly", () => {
    assert.deepEqual(
      activationSuffixes("workspaceContains:**/*."),
      [...SOURCE_EXTENSIONS].sort(),
      "[FACET-MODEL] every extension the registry resolves must also appear as a " +
        "workspaceContains activation event, or Deslop never starts in that repo",
    );
  });

  test("Marketplace keywords name every registered language", () => {
    const keywords = new Set(extensionPackage().keywords);
    const missing = LANGUAGES.filter((id) => !keywords.has(id));
    assert.deepEqual(
      missing,
      [],
      `Marketplace search is how a developer finds Deslop; these languages are ` +
        `unsearchable: ${JSON.stringify(missing)}`,
    );
  });
});
