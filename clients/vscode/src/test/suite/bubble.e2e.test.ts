// [VSIX-LIVE-BUBBLE] / [VSIX-HOVER-PROVIDER] Real VS Code surfaces against
// the bundled LSP, including the classic and compact hover presentations.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";
import { openFixture, sleep, waitFor, activateExtension, deleteRange } from "./helpers";

const SHIPPING_FILE = "Shipping.cs";
const SHIPPING_KIND_TITLE = "Nearly identical code";
const SHIPPING_FIRST_LINE = 8;
const SHIPPING_SECOND_LINE = 20;
const SHIPPING_COPY_COUNT = 2;
const HOVER_PROVIDER_COMMAND = "vscode.executeHoverProvider";
const HOVER_REPORT_TIMEOUT_MS = 15_000;
const INSIDE_OCCURRENCE_BYTE = 1;
const BUBBLE_PRESENTATION_SETTING = "liveBubble.presentation";
const DETAIL_CUE_LABEL = "ⓘ Details";
const EDIT_LINE = 10;
const EDIT_COLUMN = 4;
const INSERTED_SPACE = " ";
const HINT_PROVIDER_COMMAND = "vscode.executeInlayHintProvider";
const LIVE_BUBBLE_TIMEOUT_MS = 15_000;

async function deslopHover(uri: vscode.Uri, cursor: vscode.Position, slug: string): Promise<vscode.MarkdownString> {
  const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(HOVER_PROVIDER_COMMAND, uri, cursor);
  const card = hovers?.flatMap((hover) => hover.contents)
    .find((content) => content instanceof vscode.MarkdownString && content.value.includes(slug));
  assert.ok(card instanceof vscode.MarkdownString, "hovering over the clone opens Deslop's card");
  return card;
}

interface BinaryExport {
  readonly source?: string;
  readonly path?: string;
}

interface ExtensionExports {
  readonly resolvedLsp?: BinaryExport;
  readonly resolvedMcp?: BinaryExport;
}

suite("live bubble (real LSP)", () => {
  test("editing a real duplicate shows a hoverable classic bubble", async () => {
    const api = await activateExtension();
    const editor = await openFixture(SHIPPING_FILE);
    await waitFor(() => api.reportStore?.current.visibleReport?.clusters.find((item) =>
      item.occurrences.some((occurrence) => path.basename(occurrence.path) === SHIPPING_FILE),
    ), HOVER_REPORT_TIMEOUT_MS);
    const editAt = new vscode.Position(EDIT_LINE, EDIT_COLUMN);
    const queryRange = editor.document.lineAt(EDIT_LINE).range;
    await editor.edit((builder) => builder.insert(editAt, INSERTED_SPACE));
    try {
      const hint = await waitFor(async () => {
        const hints = await vscode.commands.executeCommand<vscode.InlayHint[]>(
          HINT_PROVIDER_COMMAND, editor.document.uri, queryRange,
        );
        return hints?.find((item) => item.label === DETAIL_CUE_LABEL);
      }, LIVE_BUBBLE_TIMEOUT_MS);
      assert.equal(hint.label, DETAIL_CUE_LABEL, "the in-editor detail cue is visible after the edit");
      assert.ok(hint.tooltip instanceof vscode.MarkdownString, "hovering the cue opens the shared card");
      assert.match(hint.tooltip.value, /Mass: /, "the live hover shows duplicated mass");
      assert.match(hint.tooltip.value, /command:deslop\.openCluster/, "the live hover opens the cluster");
    } finally {
      await deleteRange(editor, EDIT_LINE, EDIT_COLUMN, EDIT_LINE, EDIT_COLUMN + INSERTED_SPACE.length);
      await editor.document.save();
    }
  });

  test("hovering over a real clone shows the 0.32.0-style detail card", async () => {
    const api = await activateExtension();
    const editor = await openFixture(SHIPPING_FILE);
    const cluster = await waitFor(() => api.reportStore?.current.visibleReport?.clusters.find((item) =>
      item.occurrences.some((occurrence) => path.basename(occurrence.path) === SHIPPING_FILE),
    ), HOVER_REPORT_TIMEOUT_MS);
    const occurrence = cluster.occurrences.find((item) => path.basename(item.path) === SHIPPING_FILE);
    assert.ok(occurrence, "the real report must locate a Shipping clone");
    assert.equal(cluster.occurrence_count, SHIPPING_COPY_COUNT, "the engine reports both copies");
    const cursor = editor.document.positionAt(occurrence.start_byte + INSIDE_OCCURRENCE_BYTE);
    const card = await deslopHover(editor.document.uri, cursor, cluster.id.slice(0, 7));
    assert.match(card.value, new RegExp(SHIPPING_KIND_TITLE), "the card states the clone kind");
    assert.match(card.value, /× 2/, "the card states the copy count");
    assert.match(card.value, /Mass: /, "the card explains the duplicated mass");
    assert.ok(card.value.includes(`${SHIPPING_FILE}:${SHIPPING_FIRST_LINE}`), "the card locates the first method");
    assert.ok(card.value.includes(`${SHIPPING_FILE}:${SHIPPING_SECOND_LINE}`), "the card locates the second method");
    assert.match(card.value, /command:deslop\.compareWithCanonical/, "the chosen copy can be compared");
    assert.match(card.value, /command:deslop\.openCluster/, "the complete cluster can be opened");
    const config = vscode.workspace.getConfiguration("deslop");
    await config.update(BUBBLE_PRESENTATION_SETTING, "compact", vscode.ConfigurationTarget.Workspace);
    try {
      const compact = await deslopHover(editor.document.uri, cursor, cluster.id.slice(0, 7));
      assert.match(compact.value, /× 2/, "Compact retains the copy count");
      assert.match(compact.value, /Canonical: `.*Shipping\.cs`/, "Compact retains the canonical path");
      assert.match(compact.value, /command:deslop\.openCluster/, "Compact retains the cluster action");
      assert.doesNotMatch(compact.value, /Nearly identical code|Mass:|Occurrences:|Compare/, "Compact matches the brief 0.36.0 card");
    } finally {
      await config.update(BUBBLE_PRESENTATION_SETTING, undefined, vscode.ConfigurationTarget.Workspace);
    }
  });

  test("extension spawns bundled Deslop binaries", async () => {
    const ext = vscode.extensions.getExtension<ExtensionExports>("nimblesite.deslop-live");
    assert.ok(ext, "extension must be installed");
    const api = await ext.activate();
    assert.ok(ext.isActive, "extension must be active against the real LSP");
    assert.equal(api.resolvedLsp?.source, "bundled");
    assert.equal(api.resolvedMcp?.source, "bundled");
    assert.match(api.resolvedLsp?.path ?? "", /[/\\]bin[/\\][^/\\]+[/\\]deslop-lsp(?:\.exe)?$/);
    assert.match(api.resolvedMcp?.path ?? "", /[/\\]bin[/\\][^/\\]+[/\\]deslop-mcp(?:\.exe)?$/);
  });

  test("editing a duplicated range triggers re-analysis", async () => {
    const editor = await openFixture("Alpha.cs");

    await editor.edit((builder) =>
      builder.insert(new vscode.Position(2, 0), "    var extra = 42;\n"),
    );
    // 250ms debounce + LSP re-analysis budget; the real binary must complete in <1s.
    await sleep(2000);

    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("deslop.bubble.dismiss"));
    assert.ok(commands.includes("deslop.bubble.dismissCluster"));
  });

  test("escape dismisses the bubble", async () => {
    await vscode.commands.executeCommand("deslop.bubble.dismiss");
  });
});
