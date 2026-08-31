// Unit: cluster webview occurrence locations. The webview is TSX, so use
// TypeScript's parser instead of brittle source regex checks.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as ts from "typescript";
import { SIGNAL_HELP, signalTitle } from "../../types/signals";

const DOC_TEXT_LINK_COMPONENT = "DocTextLink";
const CLUSTER_ID_TOPIC_CONSTANT = "CLUSTER_ID_TOPIC";
const CLUSTER_ID_TOPIC_VALUE = "cluster-id";
const OCCURRENCE_IDENTIFIER = "occurrence";
const SHORT_OCCURRENCE_IDENTIFIER = "o";

function clusterWebviewSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/cluster/main.tsx");
}

function occurrenceListSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/cluster/OccurrenceList.tsx");
}

function signalStripSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/components/SignalStrip.tsx");
}

function helpBubbleSourcePath(): string {
  return path.resolve(__dirname, "../../../webview-ui/src/components/HelpBubble.tsx");
}

function parseSource(sourcePath: string): ts.SourceFile {
  const source = fs.readFileSync(sourcePath, "utf8");
  return ts.createSourceFile(sourcePath, source, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
}

function parseClusterWebview(): ts.SourceFile {
  return parseSource(clusterWebviewSourcePath());
}

function parseOccurrenceList(): ts.SourceFile {
  return parseSource(occurrenceListSourcePath());
}

function parseClusterRenderer(): ts.SourceFile[] {
  return [parseClusterWebview(), parseOccurrenceList()];
}

function parseSignalStrip(): ts.SourceFile {
  return parseSource(signalStripSourcePath());
}

function parseHelpBubble(): ts.SourceFile {
  return parseSource(helpBubbleSourcePath());
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
    [OCCURRENCE_IDENTIFIER, SHORT_OCCURRENCE_IDENTIFIER].includes(node.expression.text) &&
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

function clusterRendererButtons(): ts.JsxOpeningLikeElement[] {
  return parseClusterRenderer().flatMap(jsxButtons);
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

function clusterRendererCorpus(): string {
  return parseClusterRenderer().map(stringCorpus).join("\n");
}

// The source text of a template expression: its head, every
// interpolated expression verbatim, and every span's literal tail — the
// same reconstruction `stringCorpus` performs, extended with expression
// text so assertions can pin which variable a label is built from.
function templateText(expr: ts.TemplateExpression): string {
  const parts = [expr.head.text];
  for (const span of expr.templateSpans) {
    parts.push(span.expression.getText());
    parts.push(span.literal.text);
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
          attr.initializer.expression
        ) {
          const expr = attr.initializer.expression;
          if (ts.isTemplateExpression(expr)) {
            out.push(templateText(expr));
          } else if (ts.isNoSubstitutionTemplateLiteral(expr)) {
            out.push(expr.text);
          }
        }
      }
    }
    node.forEachChild(visit);
  }
  visit(root);
  return out;
}

suite("cluster webview occurrence locations", () => {
  test("renders occurrence file, line, and column for human readers", () => {
    // [VSIX-WEBVIEW] / issue #8: cluster detail occurrence rows must
    // show the same human editor target the Open button navigates to.
    assert.deepEqual(
      findOccurrenceLocationRenderings(parseOccurrenceList()),
      ["file + human line/column"],
      "cluster detail webview must show occurrence file plus human line and column",
    );
  });

  test("does not render byte offsets as the visible occurrence location", () => {
    assert.deepEqual(
      findRenderedByteLocations(parseOccurrenceList()),
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
    const buttons = clusterRendererButtons();
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
    const corpus = clusterRendererCorpus();
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

  test("cluster webview links visible explanations to website docs", () => {
    // The panel is the cluster view plus the signal strip it embeds; both
    // carry docs topics, so both are in scope for this assertion.
    const corpus = `${clusterRendererCorpus()}\n${stringCorpus(parseSignalStrip())}`;
    for (const phrase of [
      "cluster-id",
      "clone-bucket",
      "ai-match",
      "rank",
      "weight",
      "size",
      "occurrence-count",
      "canonical",
      "signals",
      "content-evidence",
      "occurrences",
      "occurrence-location",
      "hidden-occurrence",
      "open-action",
      "compare-action",
      "cluster-navigation",
      "keyboard-shortcuts",
    ]) {
      assert.match(corpus, new RegExp(escapeRegExp(phrase)), `missing docs topic: ${phrase}`);
    }
  });

  test("cluster id is rendered as a docs link", () => {
    const root = parseClusterWebview();
    const topicConstant = descendants(root, (node) => {
      if (!ts.isVariableDeclaration(node) || !ts.isIdentifier(node.name)) return false;
      const initializer = node.initializer;
      return node.name.text === CLUSTER_ID_TOPIC_CONSTANT &&
        initializer !== undefined &&
        ts.isStringLiteral(initializer) &&
        initializer.text === CLUSTER_ID_TOPIC_VALUE;
    });
    const linkedTopics = descendants(root, (node) => {
      if (!ts.isJsxOpeningElement(node)) return false;
      if (node.tagName.getText(root) !== DOC_TEXT_LINK_COMPONENT) return false;
      const topic = node.attributes.properties.find(
        (property): property is ts.JsxAttribute =>
          ts.isJsxAttribute(property) && property.name.getText(root) === "topic",
      );
      if (topic?.initializer === undefined || !ts.isJsxExpression(topic.initializer)) return false;
      const expression = topic.initializer.expression;
      return expression !== undefined &&
        ts.isIdentifier(expression) &&
        expression.text === CLUSTER_ID_TOPIC_CONSTANT;
    });
    assert.equal(topicConstant.length, 1, "cluster-id docs topic must have one named constant");
    assert.ok(linkedTopics.length > 0, "cluster id must link to its docs section");
  });

  test("severity badge label leads with the stable slug, not the volatile #N rank (#146)", () => {
    // [VSIX-TOP-OFFENDERS-CLUSTER-ID] applies to every cluster-row surface,
    // including the cluster detail webview. Rank is volatile (re-numbered on
    // every snapshot); the slug is stable. Both humans and AI agents reading
    // the rendered panel must see the same slug everywhere
    // ([VSIX-CLUSTER-ID-CONSISTENCY]) so cross-message references survive
    // re-analysis.
    const root = parseClusterWebview();
    const badgeLabels = severityBadgeLabelTemplates(root);
    assert.ok(
      badgeLabels.length > 0,
      "cluster panel must render a SeverityBadge in the header",
    );
    for (const label of badgeLabels) {
      assert.doesNotMatch(
        label,
        /^#\$\{rank/,
        `severity badge must not lead with the volatile #\${rank}, got: ${label}`,
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

  test("signal strip hover copy explains every score", () => {
    // The signal copy moved into the shared `types/signals` formatter
    // (#344) so the strip, its tooltips and the docs anchors cannot
    // describe the same number two ways. The corpus follows it there and
    // covers the three content-evidence axes. There is no combined-score
    // hover: that axis is gone from the wire, and its old hover copy with
    // it — asserted negatively so it cannot quietly return.
    const corpus = [
      stringCorpus(parseSignalStrip()),
      stringCorpus(parseHelpBubble()),
      Object.values(SIGNAL_HELP).join("\n"),
      signalTitle({ topic: "agreement", label: "agreement", value: 0.08 }),
    ].join("\n");
    assert.doesNotMatch(
      corpus,
      /Combined clone score/,
      "the combined-score hover must stay deleted with the fused axis",
    );
    for (const phrase of [
      "AST-shape similarity",
      "Token-overlap similarity",
      "Semantic similarity",
      "Current value",
      "How much of the matched content the locations genuinely share",
      "one consistent identifier renaming explains every difference",
      "literal data rather than logic",
      "sibling boilerplate",
    ]) {
      assert.match(corpus, new RegExp(escapeRegExp(phrase)), `missing signal hover: ${phrase}`);
    }
  });

  test("help bubbles point at deslop.live docs", () => {
    const source = parseHelpBubble();
    const corpus = stringCorpus(source);
    assert.match(corpus, /https:\/\/deslop\.live\/docs\/vscode-cluster-panel\//);
    assert.match(corpus, /More details/);
    assert.match(source.getFullText(), /data-doc-topic/);
  });
});

function escapeRegExp(text: string): string {
  return text.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}
