// [VSIX-LANGUAGE-ICONS] All languages have artwork; Osprey keeps its identity.
import * as assert from "node:assert/strict";
import { ANALYSED_LANGUAGE_IDS, LANGUAGES, languageForPath } from "../../types/languages";
import { languageIconSvg, languageIconUri } from "../../types/languageIcons";

const OSPREY = "osprey";
const PYTHON = "python";
const OSPREY_PATH = "/repo/Example.OSP";
const PYTHON_PATH = "/repo/example.py";
const ML_PATH = "/repo/example.ospml";
const UNKNOWN = "unknown";
const SVG_URI_PREFIX = "data:image/svg+xml,";
const GOLD = "#f5c451";
const PROTOTYPE_NAME = "__proto__";
const OSPREY_FLAVORS = ["osprey", "osprey-ml", "osprey-python"];

suite("language identity and artwork", () => {
  test("every registered language has distinct encoded SVG artwork", () => {
    const icons = LANGUAGES.map(languageIconSvg);
    assert.equal(new Set(icons).size, LANGUAGES.length);
    for (const language of LANGUAGES) {
      assert.ok(languageIconUri(language).startsWith(SVG_URI_PREFIX));
      assert.equal(decodeURIComponent(languageIconUri(language).slice(SVG_URI_PREFIX.length)), languageIconSvg(language));
      assert.notEqual(languageIconSvg(language), languageIconSvg(UNKNOWN));
    }
  });

  test("Osprey leads the registry with gold artwork and a separate editor identity", () => {
    assert.equal(LANGUAGES[0], OSPREY);
    assert.equal(languageForPath(OSPREY_PATH), OSPREY);
    assert.equal(languageForPath(PYTHON_PATH), PYTHON);
    assert.equal(languageForPath(ML_PATH), OSPREY);
    for (const flavor of OSPREY_FLAVORS) {
      assert.equal(languageForPath(PYTHON_PATH, flavor), OSPREY);
    }
    assert.ok(ANALYSED_LANGUAGE_IDS.includes(OSPREY));
    assert.ok(languageIconSvg(OSPREY).includes(GOLD));
    assert.equal(languageIconSvg(PROTOTYPE_NAME), languageIconSvg(UNKNOWN));
  });
});
