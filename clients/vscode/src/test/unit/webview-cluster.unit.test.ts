// Unit: cluster webview occurrence locations. The webview is TSX, so use
// TypeScript's parser instead of brittle source regex checks.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as ts from "typescript";

function clusterWebviewSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/cluster/main.tsx");
}

function parseClusterWebview(): ts.SourceFile {
  const sourcePath = clusterWebviewSourcePath();
  const source = fs.readFileSync(sourcePath, "utf8");
  return ts.createSourceFile(sourcePath, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
}

function hasDescendant(node: ts.Node, predicate: (node: ts.Node) => boolean): boolean {
  if (predicate(node)) return true;
  let found = false;
  node.forEachChild((child) => {
    if (!found) found = hasDescendant(child, predicate);
  });
  return found;
}

function hasOccurrenceByteAccess(node: ts.Node, propertyName: string): boolean {
  return ts.isPropertyAccessExpression(node) &&
    ts.isIdentifier(node.expression) &&
    node.expression.text === "o" &&
    node.name.text === propertyName;
}

function hasOccurrencePathAccess(node: ts.Node): boolean {
  return hasOccurrenceByteAccess(node, "path");
}

function hasHumanLocationText(node: ts.Node): boolean {
  if (!ts.isJsxText(node) && !ts.isStringLiteral(node)) return false;
  const text = node.getText().toLowerCase();
  return text.includes("line") && (text.includes("column") || text.includes("position"));
}

function hasHumanLocationRendering(node: ts.Node): boolean {
  return hasDescendant(node, hasOccurrencePathAccess) &&
    hasDescendant(node, hasHumanLocationText);
}

function findOccurrenceLocationRenderings(root: ts.SourceFile): string[] {
  const renderings = new Set<string>();
  function visit(node: ts.Node): void {
    if (ts.isJsxElement(node) && hasHumanLocationRendering(node)) {
      renderings.add("file + human line/column");
    }
    node.forEachChild(visit);
  }
  visit(root);
  return [...renderings];
}

suite("cluster webview occurrence locations", () => {
  test("renders occurrence file, line, and column for human readers", () => {
    // [VSIX-WEBVIEW] / issue #8: cluster detail occurrence rows must
    // show the same human editor target the Open button navigates to.
    assert.deepEqual(
      findOccurrenceLocationRenderings(parseClusterWebview()),
      ["file + human line/column"],
      "cluster detail webview must show occurrence file plus human line and column",
    );
  });
});
