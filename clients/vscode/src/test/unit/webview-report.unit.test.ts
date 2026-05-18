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

function templateText(node: ts.TemplateLiteral): string {
  if (ts.isNoSubstitutionTemplateLiteral(node)) return node.text;
  const parts: string[] = [node.head.text];
  for (const span of node.templateSpans) {
    parts.push("${", span.expression.getText(), "}", span.literal.text);
  }
  return parts.join("");
}

function severityBadgeLabelTemplates(root: ts.Node): string[] {
  const out: string[] = [];
  function visit(node: ts.Node): void {
    if (
      (ts.isJsxSelfClosingElement(node) || ts.isJsxOpeningElement(node)) &&
      ts.isIdentifier(node.tagName) &&
      node.tagName.text === "SeverityBadge"
    ) {
      for (const attr of node.attributes.properties) {
        if (
          ts.isJsxAttribute(attr) &&
          attr.name.getText() === "label" &&
          attr.initializer &&
          ts.isJsxExpression(attr.initializer) &&
          attr.initializer.expression &&
          ts.isTemplateExpression(attr.initializer.expression)
        ) {
          out.push(templateText(attr.initializer.expression));
        } else if (
          ts.isJsxAttribute(attr) &&
          attr.name.getText() === "label" &&
          attr.initializer &&
          ts.isJsxExpression(attr.initializer) &&
          attr.initializer.expression &&
          ts.isNoSubstitutionTemplateLiteral(attr.initializer.expression)
        ) {
          out.push(attr.initializer.expression.text);
        }
      }
    }
    node.forEachChild(visit);
  }
  visit(root);
  return out;
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

  test("severity badge label leads with the stable slug, not the volatile #N rank (#146)", () => {
    // [VSIX-TOP-OFFENDERS-CLUSTER-ID] applies to every cluster-row surface, not
    // just the activity-bar tree. The volatile rank/index is never the row's
    // identity — humans and AI agents scraping the report panel must see the
    // same stable 7-hex slug everywhere ([VSIX-CLUSTER-ID-CONSISTENCY]).
    const root = parseReportWebview();
    const badgeLabels = severityBadgeLabelTemplates(root);
    assert.ok(
      badgeLabels.length > 0,
      "report panel must render a SeverityBadge per cluster row",
    );
    for (const label of badgeLabels) {
      assert.doesNotMatch(
        label,
        /^#\$\{/,
        `severity badge must not lead with the volatile #N rank, got: ${label}`,
      );
      assert.doesNotMatch(
        label,
        /^#\d/,
        `severity badge must not lead with a literal #N, got: ${label}`,
      );
      assert.match(
        label,
        /\bslug\b/i,
        `severity badge must reference the cluster slug, got: ${label}`,
      );
    }
  });
});
