// [SEVERITY-MODEL] Runtime helpers must be imported as values.
// A type-only import previously froze the selected-cluster panel.
// Check the parsed webview store so every called import survives bundling.

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

  test("clusterSeverity remains available when the store renders diagnostics", () => {
    const root = parseStore();
    assert.ok(
      calledIdentifierNames(root).has("clusterSeverity"),
      "store must read the assigned diagnostic severity",
    );
    assert.ok(
      !typeOnlyImportedNames(root).has("clusterSeverity"),
      "clusterSeverity must survive bundling as a runtime function",
    );
  });

  test("the store displays assigned severity without calculating rank percentiles", () => {
    // [PRINCIPLES-ONE-CALCULATION] Ranking must not become diagnostic severity.
    const source = parseStore().getFullText();
    for (const banned of ["severityOf", "rankPercentile", "0.99", "0.9", "0.5"]) {
      assert.ok(
        !source.includes(banned),
        `the webview store must not carry a severity cut point or formula: ${banned}`,
      );
    }
  });
});
