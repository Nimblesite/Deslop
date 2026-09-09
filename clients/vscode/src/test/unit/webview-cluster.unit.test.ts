// Unit: cluster webview occurrence locations. The webview is TSX, so use
// TypeScript's parser instead of brittle source regex checks.

import * as assert from "node:assert/strict";
import * as ts from "typescript";

import { descendants, hasDescendant, parseWebviewSource } from "./webview-source.helpers";

const DOC_TEXT_LINK_COMPONENT = "DocTextLink";
const CLUSTER_ID_TOPIC_CONSTANT = "CLUSTER_ID_TOPIC";
const CLUSTER_ID_TOPIC_VALUE = "cluster-id";
const OCCURRENCE_IDENTIFIER = "occurrence";
const SHORT_OCCURRENCE_IDENTIFIER = "o";

const CLUSTER_WEBVIEW_SOURCE = "cluster/main.tsx";
const OCCURRENCE_LIST_SOURCE = "cluster/OccurrenceList.tsx";
const HELP_BUBBLE_SOURCE = "components/HelpBubble.tsx";
const STORE_SOURCE = "store.ts";
const OCCURRENCE_ROW_TAG = "article";
const TAP_HANDLER_NAME = "tapOccurrenceRow";
const PICKED_SIGNAL_NAME = "pickedOccurrence";
const COMPARE_PAIR_MESSAGE = "compare/pair";
const POST_FUNCTION_NAME = "post";

function parseClusterWebview(): ts.SourceFile {
  return parseWebviewSource(CLUSTER_WEBVIEW_SOURCE);
}

function parseOccurrenceList(): ts.SourceFile {
  return parseWebviewSource(OCCURRENCE_LIST_SOURCE);
}

function parseClusterRenderer(): ts.SourceFile[] {
  // The help copy is a real render surface: the panel's titles fold in
  // PANEL_HELP, so hover-copy assertions must see the same text users see.
  return [parseClusterWebview(), parseOccurrenceList(), parseHelpBubble()];
}

function parseHelpBubble(): ts.SourceFile {
  return parseWebviewSource(HELP_BUBBLE_SOURCE);
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

function clusterBadgeLabelTemplates(root: ts.Node): string[] {
  const out: string[] = [];
  function visit(node: ts.Node): void {
    if (
      (ts.isJsxSelfClosingElement(node) || ts.isJsxOpeningElement(node)) &&
      ts.isIdentifier(node.tagName) &&
      node.tagName.text === "ClusterBadge"
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
      // [VSIX-PAIR-COMPARE] The mass help copy explains the ranking metric
      // with the honest term — weight/bucket language is retired.
      "This cluster's duplicated mass",
      "Canonical occurrence",
      "Hidden means this path matched report_hide configuration",
      "Open this occurrence in VS Code",
      "Compare is disabled on the canonical occurrence",
      "Compare opens a diff between this occurrence and the canonical occurrence in one click",
      // [VSIX-PAIR-COMPARE] Rows are the selection: each row and the list
      // header explain the two-row tap in their hover copy.
      "Tap this row to pick it, then tap a second row to compare the two",
      "Picked for comparison. Tap another row to compare it with this one",
      "Tap one row, then another, to compare those two",
      "Previous cluster",
      "Next cluster",
      "Detailed keyboard help",
      "semantic match",
    ]) {
      assert.match(corpus, new RegExp(escapeRegExp(phrase)), `missing hover copy: ${phrase}`);
    }
    // The removed two-step selection and weight/bucket copy must stay gone:
    // no per-row "Select for comparison" button and no gated compare button.
    for (const gone of [
      "Select two occurrences to enable compare",
      "Select for comparison",
      "Compare selected occurrences",
      "Weight is this cluster's duplicated mass",
    ]) {
      assert.doesNotMatch(corpus, new RegExp(escapeRegExp(gone)), `retired copy resurfaced: ${gone}`);
    }
  });

  test("tapping an occurrence row picks it and a second row hands both endpoints to the host", () => {
    // [VSIX-PAIR-COMPARE] The rows are the selection control. The tap state
    // machine lives in the store, and its second tap posts compare/pair.
    const rows = descendants(
      parseOccurrenceList(),
      (n) => ts.isJsxOpeningElement(n) && jsxTagName(n) === OCCURRENCE_ROW_TAG,
    ) as ts.JsxOpeningElement[];
    assert.equal(rows.length, 1, "one row element renders every occurrence");
    const row = rows[0];
    assert.ok(row && jsxAttribute(row, "onClick"), "the row itself answers a tap");
    assert.ok(row && jsxAttribute(row, "title"), "the row explains the tap in its hover copy");
    const store = parseWebviewSource(STORE_SOURCE, ts.ScriptKind.TS);
    const tapHandlers = descendants(
      store,
      (n) => ts.isFunctionDeclaration(n) && n.name?.text === TAP_HANDLER_NAME,
    );
    assert.equal(tapHandlers.length, 1, "the store owns the tap state machine");
    const handler = tapHandlers[0];
    assert.ok(handler && hasDescendant(handler, (n) => ts.isCallExpression(n) && ts.isIdentifier(n.expression) && n.expression.text === POST_FUNCTION_NAME), "the second tap posts to the host");
    assert.ok(hasDescendant(store, (n) => ts.isStringLiteral(n) && n.text === COMPARE_PAIR_MESSAGE), "the store names the compare/pair message");
    const pickSignals = descendants(
      store,
      (n) => ts.isVariableDeclaration(n) && ts.isIdentifier(n.name) && n.name.text === PICKED_SIGNAL_NAME,
    );
    assert.equal(pickSignals.length, 1, "the picked row is one store signal, not component state");
  });

  test("cluster webview links visible explanations to website docs", () => {
    // The panel is the cluster view plus its help bubble; every docs topic
    // it carries is a cluster-level fact. Pair-only signal topics have no
    // place here because the panel renders no pair evidence
    // ([FUSED-PAIR-SIGNALS]).
    const corpus = clusterRendererCorpus();
    for (const phrase of [
      "cluster-id",
      "clone-kind",
      "ai-match",
      "rank",
      "mass",
      "occurrence-count",
      "canonical",
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
    for (const gone of [
      "content-evidence",
      "structural",
      "jaccard",
      "agreement",
      "rename-consistency",
      "literal-fraction",
    ]) {
      assert.doesNotMatch(
        corpus,
        new RegExp(escapeRegExp(gone)),
        `pair-only signal topic must not appear on the cluster panel: ${gone}`,
      );
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

  test("cluster badge label leads with the stable slug, not the volatile #N rank (#146)", () => {
    // [VSIX-TOP-OFFENDERS-CLUSTER-ID] applies to every cluster-row surface,
    // including the cluster detail webview. Rank is volatile (re-numbered on
    // every snapshot); the slug is stable. Both humans and AI agents reading
    // the rendered panel must see the same slug everywhere
    // ([VSIX-CLUSTER-ID-CONSISTENCY]) so cross-message references survive
    // re-analysis.
    const root = parseClusterWebview();
    const badgeLabels = clusterBadgeLabelTemplates(root);
    assert.ok(
      badgeLabels.length > 0,
      "cluster panel must render a ClusterBadge in the header",
    );
    for (const label of badgeLabels) {
      assert.doesNotMatch(
        label,
        /^#\$\{rank/,
        `cluster badge must not lead with the volatile #\${rank}, got: ${label}`,
      );
      assert.doesNotMatch(
        label,
        /^#\d/,
        `cluster badge must not lead with a literal #N, got: ${label}`,
      );
      assert.match(
        label,
        /\bslug\b/i,
        `cluster badge must reference the cluster slug, got: ${label}`,
      );
    }
  });

  test("the cluster panel renders no signal hover copy", () => {
    // The admission signals are pair measurements and never touch the
    // cluster ([FUSED-PAIR-SIGNALS]). The panel carries no signal strip,
    // no signal formatter, and no signal help copy — asserted negatively so
    // the leak cannot quietly return.
    const corpus = [stringCorpus(parseClusterWebview()), stringCorpus(parseHelpBubble())].join("\n");
    for (const gone of [
      "Combined clone score",
      "AST-shape similarity",
      "Token-overlap similarity",
      "Semantic similarity",
      "Current value",
      "How much of the matched content",
      "consistent identifier renaming",
      "literal data rather than logic",
      "CONTENT EVIDENCE",
      "ELECTED PAIR",
      "SignalStrip",
    ]) {
      assert.doesNotMatch(
        corpus,
        new RegExp(escapeRegExp(gone)),
        `pair-only signal copy must not render on the cluster panel: ${gone}`,
      );
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

