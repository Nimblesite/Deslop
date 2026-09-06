// Unit: webview store runtime imports (#254). A runtime value imported under
// `import type { … }` is erased by the bundler, so at runtime the binding is
// undefined and the first call throws. `severityOf` was imported type-only, so
// the `severityByClusterId` computed threw the moment a cluster was selected,
// Preact aborted the re-render, and the detail panel froze on
// "No cluster selected." across every language. The webview cannot execute
// under vscode-test, so assert the invariant on the parsed store source:
// nothing the store CALLS at runtime may arrive through a type-only import.

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

  test("severityOf is a runtime value import so severityByClusterId can bucket clusters", () => {
    const root = parseStore();
    assert.ok(
      calledIdentifierNames(root).has("severityOf"),
      "store must call severityOf to assign each cluster a severity",
    );
    assert.ok(
      !typeOnlyImportedNames(root).has("severityOf"),
      "severityOf is a runtime function; a type-only import erases it and freezes the cluster panel (#254)",
    );
  });
});
