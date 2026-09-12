// [SEVERITY-CONFIG] The extension exposes validated settings; Rust owns defaults.
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const manifest = JSON.parse(readFileSync(new URL("../package.json", import.meta.url), "utf8"));
const properties = manifest.contributes.configuration.properties;
const KINDS = ["identical", "nearly_identical", "same_behavior", "loosely_similar", "structural_only"];
const LEVELS = ["none", "hint", "information", "warning", "error"];

test("diagnostics default off with open-file scope", () => {
  assert.equal(properties["deslop.diagnostics.enabled"]?.default, false);
  assert.equal(properties["deslop.diagnostics.scope"]?.default, "open-files");
  assert.deepEqual(properties["deslop.diagnostics.scope"]?.enum, ["open-files", "workspace"]);
});

test("each category accepts only diagnostic levels, with omitted entries resolved by Rust", () => {
  const setting = properties["deslop.diagnostics.severityByKind"];
  assert.ok(setting);
  assert.deepEqual(setting.default, {});
  assert.equal(setting.additionalProperties, false);
  assert.deepEqual(Object.keys(setting.properties), KINDS);
  for (const kind of KINDS) assert.deepEqual(setting.properties[kind].enum, LEVELS);
});
