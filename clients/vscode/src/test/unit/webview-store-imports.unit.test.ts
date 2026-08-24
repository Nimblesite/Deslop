// Unit: webview store runtime imports (#254). A runtime value imported under
// `import type { … }` is erased by the bundler, so at runtime the binding is
// undefined and the first call throws. The severity helper was imported
// type-only, so the `severityByClusterId` computed threw the moment a cluster
// was selected, Preact aborted the re-render, and the detail panel froze on
// "No cluster selected." across every language. The webview cannot execute
// under vscode-test, so assert the invariant on the parsed store source:
// nothing the store CALLS at runtime may arrive through a type-only import.
//
// The helper is now `clusterBand`, which reads the engine's `rank_band`
// ([SEVERITY-BAND]) instead of classifying a locally computed percentile.
// The import hazard is identical, so the pin moved with the name.

import * as assert from "node:assert/strict";
import * as ts from "typescript";

import { parseWebviewSource } from "./webview-source.helpers";

function parseStore(): ts.SourceFile {
  return parseWebviewSource("store.ts", ts.ScriptKind.TS);
}

function typeOnlyImportedNames(root: ts.SourceFile): Set<string> {
  const names = new Set<string>();
  for (const statement of root.statements) {
    if (!ts.isImportDeclaration(statement) || !statement.importClause) continue;
    const { isTypeOnly, namedBindings } = statement.importClause;
    if (!namedBindings || !ts.isNamedImports(namedBindings)) continue;
    for (const element of namedBindings.elements) {
      if (isTypeOnly || element.isTypeOnly) names.add(element.name.text);
    }
  }
  return names;
}

function calledIdentifierNames(root: ts.SourceFile): Set<string> {
  const names = new Set<string>();
  function visit(node: ts.Node): void {
    if (ts.isCallExpression(node) && ts.isIdentifier(node.expression)) {
      names.add(node.expression.text);
    }
    node.forEachChild(visit);
  }
  visit(root);
  return names;
}

suite("webview store runtime imports (#254)", () => {
  test("no value the store calls at runtime is imported type-only", () => {
    const root = parseStore();
    const typeOnly = typeOnlyImportedNames(root);
    const erasedCalls = [...calledIdentifierNames(root)].filter((name) => typeOnly.has(name)).sort();
    assert.deepEqual(
      erasedCalls,
      [],
      "imported via `import type` yet called at runtime, so the bundler erases them to undefined " +
        `and the call throws (cluster panel freezes on "No cluster selected."): ${erasedCalls.join(", ")}`,
    );
  });

  test("clusterBand is a runtime value import so severityByClusterId can band clusters", () => {
    const root = parseStore();
    assert.ok(
      calledIdentifierNames(root).has("clusterBand"),
      "store must call clusterBand to read each cluster's engine-assigned band",
    );
    assert.ok(
      !typeOnlyImportedNames(root).has("clusterBand"),
      "clusterBand is a runtime function; a type-only import erases it and freezes the cluster panel (#254)",
    );
  });

  test("the store reads the engine's band and re-derives no severity of its own", () => {
    // [PRINCIPLES-ONE-CALCULATION] The percentile formula that used to sit
    // in this computed was a second severity engine, and it banded whatever
    // list the panel happened to be holding rather than the report.
    const source = parseStore().getFullText();
    for (const banned of ["severityOf", "rankPercentile", "0.99", "0.9", "0.5"]) {
      assert.ok(
        !source.includes(banned),
        `the webview store must not carry a severity cut point or formula: ${banned}`,
      );
    }
  });
});
