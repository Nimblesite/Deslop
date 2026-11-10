// [LIVE-CONFIG-LIVE] [LIVE-CONFIG-STARTUP] [VSIX-METRICS-PANEL] Editing
// `.deslop.toml` while VS Code is open must move the Duplication panel's
// percentage on screen to exactly what the CLI prints for the same tree.
// An Osprey agent excluded generated parsers in `.deslop.toml`; the CLI said
// 5.61% while the panel kept showing 10.3%. This drives the real loop —
// file write → LSP watcher → reportChanged → store signals → the tree view
// VS Code renders — with the bundled CLI as the oracle at every step.
import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import type { ReportStore } from "../../reportStore";
import type { MetricsProvider } from "../../tree/providers";
import { formatPercent } from "../../types/format";
import { scanWithBundledCli, type ScannedMetrics } from "../cli.helpers";
import { fixtureRoot, waitForReport } from "./helpers";

const CONFIG_NAME = ".deslop.toml";
const EXCLUDED_FILE = "Gamma.cs";
const EXCLUDE_SECTION = "[defaults]\nexclude = [\"Gam*.cs\"]\n";
const LOOSE_GATE = 80;
const TIGHT_GATE = 50;
const WITHIN_LOOSE_GATE = "✓ within 80.0% gate";
const OVER_TIGHT_GATE = "⚠ over 50.0% gate";
const GATE_WORD = "gate";
const CALM_ICON = "graph";
const BREACHED_ICON = "warning";
const READY = "ready";
const EXPECTED_CLUSTER_COUNT = 3;
const MIN_TREE_EVENTS = 1;
/** The headline row's identity. The extension runs from its bundle, so its
 * node classes are not the ones this suite imports and `instanceof` cannot
 * recognise them; the context value is what VS Code itself keys menus on. */
const HEADLINE_CONTEXT = "deslop.metricsHeadline";

/** What the Duplication panel shows on its headline row. */
interface Screen {
  label: string;
  description: string;
  icon: string;
}

function gatedConfig(gate: number): string {
  return `[threshold]\nmax_duplication_percent = ${gate}\n\n${EXCLUDE_SECTION}`;
}

function readScreen(view: MetricsProvider): Screen | undefined {
  const node = view.getChildren()[0];
  if (node?.contextValue !== HEADLINE_CONTEXT) return undefined;
  const label = typeof node.label === "string" ? node.label : node.label?.label ?? "";
  const icon = node.iconPath instanceof vscode.ThemeIcon ? node.iconPath.id : "";
  return { label, description: String(node.description ?? ""), icon };
}

function settled(store: ReportStore): Promise<void> {
  return new Promise((resolve) => {
    const check = () => {
      const { visibleReport, lifecycle } = store.current;
      if (lifecycle.kind !== READY || visibleReport?.clusters.length !== EXPECTED_CLUSTER_COUNT) return;
      subscription.dispose();
      resolve();
    };
    const subscription = store.onDidChange(check);
    check();
  });
}

/** Writes the config, then resolves on the first tree refresh whose headline
 * satisfies `changed` once the server is idle. Event-driven: no sleeps. */
function nextScreen(view: MetricsProvider, store: ReportStore, write: () => void, changed: (screen: Screen) => boolean): Promise<{ screen: Screen; events: number }> {
  return new Promise((resolve, reject) => {
    let events = 0;
    const subscription = view.onDidChangeTreeData(() => {
      events += 1;
      const screen = readScreen(view);
      if (store.current.lifecycle.kind !== READY || !screen || !changed(screen)) return;
      subscription.dispose();
      resolve({ screen, events });
    });
    try { write(); } catch (error) { subscription.dispose(); reject(error instanceof Error ? error : new Error(String(error))); }
  });
}

function assertMatchesCli(screen: Screen, metrics: ScannedMetrics): void {
  assert.equal(screen.label, `${formatPercent(metrics.duplication_percent)} duplicated`, "the headline is the CLI's percentage");
  assert.ok(screen.description.includes(`${metrics.analysed_loc.toLocaleString()} LOC analysed`), screen.description);
  assert.ok(screen.description.includes(`${metrics.duplicated_loc.toLocaleString()} duplicated`), screen.description);
  assert.ok(screen.description.includes(`${metrics.clusters_total} clusters across ${metrics.duplicated_files} files`), screen.description);
}

function writeConfig(configPath: string, content: string | null): () => void {
  return () => {
    if (content === null) fs.rmSync(configPath, { force: true });
    else fs.writeFileSync(configPath, content);
  };
}

/** The live loop under test: the rendered panel, its store and the config file. */
interface Loop {
  view: MetricsProvider;
  store: ReportStore;
  configPath: string;
}

/** Step 1 — no config: the screen already equals the CLI. */
function assertPlainScreen(view: MetricsProvider, screen: Screen | undefined, plain: ScannedMetrics): Screen {
  assert.ok(screen, "the headline row is on screen");
  assert.equal(view.headline.value?.label, screen.label, "the screen renders the headline signal");
  assertMatchesCli(screen, plain);
  assert.ok(!screen.description.includes(GATE_WORD), "no gate is configured yet");
  assert.equal(screen.icon, CALM_ICON);
  return screen;
}

/** Step 2 — a wildcard exclusion plus a gate: the figure drops to the CLI's. */
async function excludeAndGate(loop: Loop, before: Screen, plain: ScannedMetrics): Promise<Screen> {
  const { screen, events } = await nextScreen(loop.view, loop.store, writeConfig(loop.configPath, gatedConfig(LOOSE_GATE)), (next) => next.label !== before.label);
  const scoped = scanWithBundledCli(fixtureRoot()).metrics;
  assert.ok(scoped.analysed_loc < plain.analysed_loc, "the exclusion shrinks the corpus");
  assertMatchesCli(screen, scoped);
  assert.ok(screen.description.includes(WITHIN_LOOSE_GATE), screen.description);
  assert.equal(screen.icon, CALM_ICON);
  assert.ok(events >= MIN_TREE_EVENTS, "VS Code was told to repaint");
  const visible = loop.store.current.visibleReport;
  assert.equal(visible?.metrics.duplication_percent, scoped.duplication_percent);
  const leaked = visible?.clusters.some((cluster) => cluster.occurrences.some((occurrence) => occurrence.path.endsWith(EXCLUDED_FILE)));
  assert.equal(leaked, false, "the excluded file left every cluster");
  return screen;
}

/** Step 3 — only the gate tightens: same figure, the verdict flips to breached. */
async function tightenGate(loop: Loop, excluded: Screen): Promise<void> {
  const { screen, events } = await nextScreen(loop.view, loop.store, writeConfig(loop.configPath, gatedConfig(TIGHT_GATE)), (next) => next.icon === BREACHED_ICON);
  assert.equal(screen.label, excluded.label, "a gate change leaves the corpus figure alone");
  assert.ok(screen.description.includes(OVER_TIGHT_GATE), screen.description);
  assert.ok(events >= MIN_TREE_EVENTS, "VS Code was told to repaint");
}

/** Step 4 — the original config back: the screen returns to the CLI's first figure. */
async function restore(loop: Loop, saved: string | null, before: Screen): Promise<void> {
  const { screen } = await nextScreen(loop.view, loop.store, writeConfig(loop.configPath, saved), (next) => next.label === before.label && next.icon === CALM_ICON);
  assert.deepEqual(screen, before, "restoring the config restores the CLI's original figure");
}

suite("config reactivity", () => {
  test("every .deslop.toml edit moves the on-screen duplication figure to the CLI's", async () => {
    const api = await waitForReport();
    const { metricsView: view, reportStore: store } = api;
    assert.ok(view && store, "the Duplication panel and store are exposed");
    await settled(store);
    const loop: Loop = { view, store, configPath: path.join(fixtureRoot(), CONFIG_NAME) };
    const saved = fs.existsSync(loop.configPath) ? fs.readFileSync(loop.configPath, "utf8") : null;
    const plain = scanWithBundledCli(fixtureRoot()).metrics;
    const before = assertPlainScreen(view, readScreen(view), plain);
    try {
      await tightenGate(loop, await excludeAndGate(loop, before, plain));
    } finally {
      await restore(loop, saved, before);
    }
  });
});
