// Unit: report webview occurrence counts. The report webview is TSX,
// so inspect the parsed tree instead of source text.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as ts from "typescript";

function reportWebviewSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/report/main.tsx");
}

function parseReportWebview(): ts.SourceFile {
  const sourcePath = reportWebviewSourcePath();
  const source = fs.readFileSync(sourcePath, "utf8");
  return ts.createSourceFile(sourcePath, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
}

function descendants(root: ts.Node, predicate: (node: ts.Node) => boolean): ts.Node[] {
  const matches: ts.Node[] = [];
  function visit(node: ts.Node): void {
    if (predicate(node)) matches.push(node);
    node.forEachChild(visit);
  }
  visit(root);
  return matches;
}

function isOccurrenceArrayLength(node: ts.Node): boolean {
  return ts.isPropertyAccessExpression(node) &&
    node.name.text === "length" &&
    ts.isPropertyAccessExpression(node.expression) &&
    node.expression.name.text === "occurrences" &&
    ts.isIdentifier(node.expression.expression) &&
    node.expression.expression.text === "cluster";
}

function hasOccurrenceCountCall(root: ts.Node): boolean {
  return descendants(
    root,
    (node) => ts.isCallExpression(node) &&
      ts.isIdentifier(node.expression) &&
      node.expression.text === "occurrenceCount",
  ).length > 0;
}

suite("report webview occurrence counts", () => {
  test("cluster rows render authoritative occurrence counts", () => {
    // GH #26: `cluster.occurrences` may be a capped or filtered slice.
    // Visible totals must use the canonical count helper so the main
    // panel cannot disagree with editor hover/details for the cluster.
    const root = parseReportWebview();
    assert.equal(
      descendants(root, isOccurrenceArrayLength).length,
      0,
      "report rows must not present cluster.occurrences.length as the total occurrence count",
    );
    assert.equal(
      hasOccurrenceCountCall(root),
      true,
      "report rows must use the shared occurrenceCount helper",
    );
  });
});
