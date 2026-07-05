// Unit: the analysed-language registry must cover every language the core
// parser registry ships — [FACET-MODEL] anti-drift (#170/#198). Regression
// guard for the F#/PHP hover gap: the ClusterHoverProvider and inlay bubble
// only attach to documents whose editor language id is in
// ANALYSED_LANGUAGE_IDS, so a missing id silently disables hover for that
// language even though diagnostics still render.

import * as assert from "node:assert/strict";
import {
  ANALYSED_LANGUAGE_IDS,
  languageDisplayName,
  languageForPath,
} from "../../types/languages";

suite("analysed language registry covers F# and PHP", () => {
  test("F# and PHP editor ids are analysed (hover + inlay attach)", () => {
    assert.ok(
      ANALYSED_LANGUAGE_IDS.includes("fsharp"),
      "fsharp must be an analysed language or the F# hover card never registers",
    );
    assert.ok(
      ANALYSED_LANGUAGE_IDS.includes("php"),
      "php must be an analysed language or the PHP hover card never registers",
    );
  });

  test("F# and PHP source extensions resolve to their language id", () => {
    assert.equal(languageForPath("/repo/Tests.fs"), "fsharp");
    assert.equal(languageForPath("/repo/Script.fsx"), "fsharp");
    assert.equal(languageForPath("/repo/Model.php"), "php");
  });

  test("F# and PHP carry human display names", () => {
    assert.equal(languageDisplayName("fsharp"), "F#");
    assert.equal(languageDisplayName("php"), "PHP");
  });
});
