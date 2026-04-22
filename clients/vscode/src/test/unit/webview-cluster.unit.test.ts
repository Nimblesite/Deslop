// Unit: cluster webview occurrence locations. The webview is TSX, so use
// TypeScript's parser instead of brittle source regex checks.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as ts from "typescript";

function clusterWebviewSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/cluster/main.tsx");
}

function signalStripSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/components/SignalStrip.tsx");
}

function parseSource(sourcePath: string): ts.SourceFile {
  const source = fs.readFileSync(sourcePath, "utf8");
  return ts.createSourceFile(sourcePath, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
}

function parseClusterWebview(): ts.SourceFile {
  return parseSource(clusterWebviewSourcePath());
}

function parseSignalStrip(): ts.SourceFile {
  return parseSource(signalStripSourcePath());
}

function hasDescendant(node: ts.Node, predicate: (node: ts.Node) => boolean): boolean {
  if (predicate(node)) return true;
  let found = false;
  node.forEachChild((child) => {
    if (!found) found = hasDescendant(child, predicate);
  });
  return found;
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

function hasOccurrenceByteAccess(node: ts.Node, propertyName: string): boolean {
  return ts.isPropertyAccessExpression(node) &&
    ts.isIdentifier(node.expression) &&
    node.expression.text === "o" &&
    node.name.text === propertyName;
}

function hasOccurrencePathAccess(node: ts.Node): boolean {
  return hasOccurrenceByteAccess(node, "path");
}

function hasRenderedByteLocation(node: ts.Node): boolean {
  const hasStart = hasDescendant(node, (n) => hasOccurrenceByteAccess(n, "start_byte"));
  const hasEnd = hasDescendant(node, (n) => hasOccurrenceByteAccess(n, "end_byte"));
  const hasByteText = hasDescendant(node, (n) => {
    if (!ts.isJsxText(n) && !ts.isStringLiteral(n)) return false;
    return /\bbytes?\b/i.test(n.getText());
  });
  return hasStart && hasEnd && hasByteText;
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

function findRenderedByteLocations(root: ts.SourceFile): string[] {
  const renderings = new Set<string>();
  function visit(node: ts.Node): void {
    if (ts.isJsxElement(node) && hasRenderedByteLocation(node)) {
      renderings.add("visible byte offset");
    }
    node.forEachChild(visit);
  }
  visit(root);
  return [...renderings];
}

function jsxTagName(node: ts.JsxOpeningLikeElement): string {
  const name = node.tagName;
  return ts.isIdentifier(name) ? name.text : name.getText();
}

function jsxAttribute(node: ts.JsxOpeningLikeElement, name: string): ts.JsxAttribute | undefined {
  return node.attributes.properties.find(
    (attr): attr is ts.JsxAttribute => ts.isJsxAttribute(attr) && attr.name.getText() === name,
  );
}

function jsxButtons(root: ts.SourceFile): ts.JsxOpeningLikeElement[] {
  return descendants(
    root,
    (node): node is ts.JsxOpeningLikeElement =>
      (ts.isJsxOpeningElement(node) || ts.isJsxSelfClosingElement(node)) &&
      jsxTagName(node) === "button",
  ) as ts.JsxOpeningLikeElement[];
}

function onClickText(button: ts.JsxOpeningLikeElement): string {
  return jsxAttribute(button, "onClick")?.initializer?.getText() ?? "";
}

function stringCorpus(root: ts.SourceFile): string {
  const parts: string[] = [];
  function visit(node: ts.Node): void {
    if (ts.isStringLiteral(node) || ts.isJsxText(node)) {
      parts.push(node.text);
    }
    if (ts.isTemplateExpression(node)) {
      parts.push(node.head.text);
      for (const span of node.templateSpans) parts.push(span.literal.text);
    }
    if (ts.isNoSubstitutionTemplateLiteral(node)) parts.push(node.text);
    node.forEachChild(visit);
  }
  visit(root);
  return parts.join("\n");
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

  test("does not render byte offsets as the visible occurrence location", () => {
    assert.deepEqual(
      findRenderedByteLocations(parseClusterWebview()),
      [],
      "cluster detail webview must not show start_byte/end_byte as user-facing location text",
    );
  });

  test("cluster navigation buttons use local selected-cluster behavior", () => {
    const root = parseClusterWebview();
    const sourceText = root.getFullText();
    const handlers = jsxButtons(root).map(onClickText).join("\n");
    assert.match(
      handlers,
      /selectPreviousCluster\(list, rank\)/,
      "prev cluster button must call the same local selection helper as the p shortcut",
    );
    assert.match(
      handlers,
      /selectNextCluster\(list, rank\)/,
      "next cluster button must call the same local selection helper as the n shortcut",
    );
    assert.doesNotMatch(
      sourceText,
      /kind:\s*"navigate\/(?:next|prev)"/,
      "cluster navigation must not post host messages that have no behavior behind them",
    );
  });

  test("every cluster webview button has hover text and an accessible label", () => {
    const buttons = jsxButtons(parseClusterWebview());
    assert.ok(buttons.length >= 5, "Open, Compare, prev, next, and help buttons must render");
    for (const button of buttons) {
      assert.ok(jsxAttribute(button, "title"), `button missing hover title: ${button.getText()}`);
      assert.ok(
        jsxAttribute(button, "aria-label"),
        `button missing aria-label: ${button.getText()}`,
      );
    }
  });

  test("cluster webview hover copy explains visible data and actions", () => {
    const corpus = stringCorpus(parseClusterWebview());
    for (const phrase of [
      "Cluster ",
      "Rank ",
      "Weight is Deslop's duplication impact score",
      "Canonical occurrence",
      "Hidden means this path matched report_hide configuration",
      "Open this occurrence in VS Code",
      "Compare is disabled on the canonical occurrence",
      "Previous cluster",
      "Next cluster",
      "Detailed keyboard help",
      "AI match",
    ]) {
      assert.match(corpus, new RegExp(escapeRegExp(phrase)), `missing hover copy: ${phrase}`);
    }
  });

  test("signal strip hover copy explains every score", () => {
    const corpus = stringCorpus(parseSignalStrip());
    for (const phrase of [
      "Structural score",
      "Jaccard score",
      "Embedding score",
      "Fused score",
      "Current value",
    ]) {
      assert.match(corpus, new RegExp(escapeRegExp(phrase)), `missing signal hover: ${phrase}`);
    }
  });
});

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
