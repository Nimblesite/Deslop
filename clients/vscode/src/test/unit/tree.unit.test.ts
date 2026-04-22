// Unit: tree providers. Drive getChildren() directly against seeded stores.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import * as vscode from "vscode";
import {
  TopOffendersProvider,
  FocusedFileProvider,
  SessionProvider,
  StatusTicker,
} from "../../tree/providers";
import { openOccurrence } from "../../commands/register";
import { ReportStore } from "../../reportStore";
import { Bucket, Report, ReportCluster, ReportOccurrence } from "../../types/report";

function cluster(
  id: string,
  weight: number,
  occurrencePath: string,
  startByte = 0,
  endByte = 20,
  bucket: Bucket = "identical",
): ReportCluster {
  return {
    id,
    weight,
    size: 2,
    canonical_node_count: 4,
    signals: bucketSignals(bucket),
    occurrences: [
      { path: occurrencePath, start_byte: startByte, end_byte: endByte, hidden: false },
      {
        path: `${occurrencePath}.other`,
        start_byte: startByte,
        end_byte: endByte,
        hidden: false,
      },
    ],
    summary: "",
    interpretation: `dup in ${occurrencePath}`,
  };
}

function bucketSignals(bucket: Bucket) {
  if (bucket === "nearly_identical") {
    return { structural: 0.99, token_jaccard: 0.96, embedding_cos: 0, fused: 0.96 };
  }
  if (bucket === "loosely_similar") {
    return { structural: 0.2, token_jaccard: 0.4, embedding_cos: 0, fused: 0.4 };
  }
  if (bucket === "same_behavior") {
    return { structural: 0.2, token_jaccard: 0.3, embedding_cos: 0.9, fused: 0.9 };
  }
  return { structural: 1, token_jaccard: 1, embedding_cos: 0, fused: 1 };
}

function labelText(item: vscode.TreeItem): string {
  return typeof item.label === "string" ? item.label : item.label?.label ?? "";
}

function iconColorId(item: vscode.TreeItem): string {
  const icon = item.iconPath as vscode.ThemeIcon | undefined;
  const color = icon?.color;
  return String(color?.id ?? "");
}

function tooltipText(item: vscode.TreeItem): string {
  if (item.tooltip instanceof vscode.MarkdownString) return item.tooltip.value;
  return String(item.tooltip ?? "");
}

function report(clusters: ReportCluster[]): Report {
  return {
    report_schema_version: 1,
    tool_version: "v",
    min_nodes: 30,
    files_analysed: 5,
    clusters_hidden: 0,
    cache_stats: { hits: 1, misses: 2 },
    metrics: {
      analysed_loc: 100,
      duplicated_loc: 10,
      duplication_percent: 10,
      clusters_total: clusters.length,
      duplicated_files: 2,
      threshold: { percent: 0, breached: false, source: "none" },
    },
    schema_doc: "docs",
    action_hints: [],
    embedding_provenance: {
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      model_version: "1",
      dimensions: 768,
    },
    clusters,
  };
}

suite("TopOffendersProvider", () => {
  test("renders an Analysing… placeholder before the first report arrives", () => {
    const store = new ReportStore();
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("renders a 'no duplication' placeholder when the report is empty", () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("renders one root node per cluster", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 10, "/f1"), cluster("b", 5, "/f2")]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 2);
  });

  test("groups Top Offenders rows by representative file while preserving impact rank", () => {
    // [VSIX-TOP-OFFENDERS-FILE-GROUPS] Issue #10: the tree must be
    // triageable by file without losing current impact/rank ordering.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("rank-1-beta", 100, "/repo/src/b/Beta.cs"),
        cluster("rank-2-alpha", 80, "/repo/src/a/Alpha.cs"),
        cluster("rank-3-alpha", 60, "/repo/src/a/Alpha.cs"),
        cluster("rank-4-gamma", 40, "/repo/src/c/Gamma.cs"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const nodes = provider.getChildren();
    const labels = nodes.map(labelText);
    const descriptions = nodes.map((node) => String(node.description ?? ""));
    const label0 = labels[0] ?? "";
    const label1 = labels[1] ?? "";
    const label2 = labels[2] ?? "";
    const label3 = labels[3] ?? "";

    assert.equal(nodes.length, 4, "one top-level row must render per cluster");
    assert.match(label0, /#2\b/, "Alpha's highest-impact cluster keeps rank #2");
    assert.match(label1, /#3\b/, "Alpha's lower-impact cluster keeps rank #3");
    assert.match(label2, /#1\b/, "Beta keeps its original global rank");
    assert.match(label3, /#4\b/, "Gamma keeps its original global rank");
    assert.match(label0, /Alpha\.cs/, "first Alpha row must expose file context");
    assert.match(label1, /Alpha\.cs/, "second Alpha row must expose file context");
    assert.match(label2, /Beta\.cs/, "Beta row must expose file context");
    assert.match(label3, /Gamma\.cs/, "Gamma row must expose file context");
    assert.deepEqual(descriptions, [
      "rank-2-alpha",
      "rank-3-alpha",
      "rank-1-beta",
      "rank-4-gamma",
    ]);
    assert.equal(nodes[0]?.command?.command, "deslop.openCluster");
    assert.deepEqual(nodes[0]?.command?.arguments, ["rank-2-alpha"]);
    assert.equal(provider.getChildren(nodes[0]).length, 2);
  });

  test("renders distinct accessible category color metadata on Top Offenders rows", () => {
    // [VSIX-TOP-OFFENDERS-CATEGORY-COLORS] Issue #10: category colour
    // is metadata, not the only signal; labels and a11y text still name it.
    const store = new ReportStore();
    store.setSnapshot(
      report([
        cluster("exact", 100, "/repo/src/a/Exact.cs", 0, 20, "identical"),
        cluster("near", 90, "/repo/src/b/Near.cs", 0, 20, "nearly_identical"),
      ]),
      0,
    );
    const provider = new TopOffendersProvider(store, new StatusTicker());

    const [exact, near] = provider.getChildren();
    assert.ok(exact, "exact duplicate row must render");
    assert.ok(near, "near duplicate row must render");
    assert.ok(exact.iconPath instanceof vscode.ThemeIcon);
    assert.ok(near.iconPath instanceof vscode.ThemeIcon);
    assert.notEqual(iconColorId(exact), "", "exact duplicate must carry a theme color");
    assert.notEqual(iconColorId(near), "", "near duplicate must carry a theme color");
    assert.notEqual(
      iconColorId(exact),
      iconColorId(near),
      "exact and near duplicate categories must have distinct theme colors",
    );
    assert.match(labelText(exact), /Identical code/);
    assert.match(labelText(near), /Nearly identical code/);
    assert.match(labelText(exact), /Exact\.cs/);
    assert.match(labelText(near), /Near\.cs/);
    assert.match(exact.accessibilityInformation?.label ?? "", /Identical code/);
    assert.match(near.accessibilityInformation?.label ?? "", /Nearly identical code/);
    assert.match(exact.accessibilityInformation?.label ?? "", /Exact\.cs/);
    assert.match(near.accessibilityInformation?.label ?? "", /Near\.cs/);
    assert.match(tooltipText(exact), /\/repo\/src\/a\/Exact\.cs/);
    assert.match(tooltipText(near), /\/repo\/src\/b\/Near\.cs/);
  });

  test("expanding a cluster node yields OccurrenceNode children", () => {
    const store = new ReportStore();
    const c = cluster("a", 10, "/f1");
    store.setSnapshot(report([c]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const roots = provider.getChildren();
    const kids = provider.getChildren(roots[0]);
    assert.equal(kids.length, c.occurrences.length);
  });

  test("occurrence row reports and opens the exact file, line, and column", async () => {
    // [VSIX-ACTIVITY-BAR] Issue #8: tree occurrence rows must show
    // path:line:column, not machine-oriented start_byte..end_byte.
    const dir = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-issue-8-tree-"));
    const occurrencePath = path.join(dir, "ChatProtocol.cs");
    const source = "namespace Demo;\n\npublic sealed class ChatProtocol {\n    void Send() {}\n}\n";
    const startByte = Buffer.byteLength(source.slice(0, source.indexOf("void Send")), "utf8");
    const endByte = startByte + Buffer.byteLength("void Send", "utf8");
    fs.writeFileSync(occurrencePath, source, "utf8");

    try {
      const store = new ReportStore();
      store.setSnapshot(report([cluster("issue-8", 10, occurrencePath, startByte, endByte)]), 0);
      const provider = new TopOffendersProvider(store, new StatusTicker());
      const [root] = provider.getChildren();
      assert.ok(root, "cluster root must exist");

      const [occurrence] = provider.getChildren(root);
      assert.ok(occurrence, "occurrence child must exist");
      const label = typeof occurrence.label === "string"
        ? occurrence.label
        : occurrence.label?.label ?? "";
      const description = String(occurrence.description ?? "");
      const rendered = `${label} ${description}`;

      assert.ok(occurrence.command, "occurrence row must be tappable");
      assert.equal(occurrence.command.command, "deslop.openOccurrence");
      const commandArguments = occurrence.command.arguments;
      assert.ok(commandArguments, "occurrence command must carry arguments");
      const argument = commandArguments[0] as ReportOccurrence | undefined;
      assert.ok(argument, "occurrence command must carry the occurrence payload");

      await openOccurrence(argument);

      const editor = vscode.window.activeTextEditor;
      assert.ok(editor, "tapping the occurrence must open an editor");
      assert.equal(editor.document.uri.fsPath, occurrencePath);
      assert.equal(editor.selection.start.line, 3, "cursor should move to line 4");
      assert.equal(editor.selection.start.character, 4, "cursor should move to column 5");
      assert.equal(editor.selection.end.character, 13, "selection should cover the occurrence");

      assert.deepEqual(
        {
          hasFileName: /ChatProtocol\.cs/.test(rendered),
          hasLineAndColumn: /ChatProtocol\.cs:4:5/.test(rendered) ||
            /line\s+4,\s*column\s+5/i.test(rendered),
          exposesRawByteRange: new RegExp(`\\b${startByte}\\.\\.${endByte}\\b`).test(rendered),
          usesByteTerminology: /\bbytes?\b/i.test(rendered),
        },
        {
          hasFileName: true,
          hasLineAndColumn: true,
          exposesRawByteRange: false,
          usesByteTerminology: false,
        },
        `occurrence row must report the same human target it navigates to, got: ${rendered}`,
      );
    } finally {
      await vscode.commands.executeCommand("workbench.action.closeActiveEditor");
      fs.rmSync(dir, { recursive: true, force: true });
    }
  });

  test("getTreeItem returns the node verbatim", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 10, "/f1")]), 0);
    const provider = new TopOffendersProvider(store, new StatusTicker());
    const [root] = provider.getChildren();
    assert.ok(root, "root node must exist");
    assert.strictEqual(provider.getTreeItem(root), root);
  });

  test("reacts to LSP-fed store snapshots and deltas", () => {
    const store = new ReportStore();
    const ticker = new StatusTicker();
    const provider = new TopOffendersProvider(store, ticker);
    let treeRefreshes = 0;
    const sub = provider.onDidChangeTreeData(() => {
      treeRefreshes += 1;
    });

    try {
      store.setSnapshot(report([cluster("stale", 1, "/stale.cs")]), 1);
      assert.equal(treeRefreshes, 1, "snapshot must refresh the tree");
      assert.equal(String(provider.getChildren()[0]?.description ?? ""), "stale");

      store.applyDelta({
        from_generation: 1,
        to_generation: 2,
        clusters_added: [cluster("fresh", 50, "/fresh.cs")],
        clusters_removed: ["stale"],
        clusters_updated: [],
        cache_stats: { hits: 2, misses: 0 },
        tool_version: "v2",
      });

      assert.equal(treeRefreshes, 2, "delta must refresh the tree");
      assert.equal(String(provider.getChildren()[0]?.description ?? ""), "fresh");
    } finally {
      sub.dispose();
      provider.dispose();
      ticker.dispose();
    }
  });
});

suite("FocusedFileProvider", () => {
  test("renders 'No active editor' when no editor is focused", async () => {
    await vscode.commands.executeCommand("workbench.action.closeAllEditors");
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("returns [] when no report is loaded yet", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "x",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 0);
  });

  test("returns cluster overlap for the active editor", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "content",
      language: "plaintext",
    });
    const editor = await vscode.window.showTextDocument(doc);
    const activePath = editor.document.uri.fsPath;
    const store = new ReportStore();
    store.setSnapshot(
      report([cluster("a", 10, activePath), cluster("b", 5, "/other")]),
      0,
    );
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.ok(nodes.length >= 1);
    const kids = provider.getChildren(nodes[0]);
    assert.ok(kids.length >= 1);
  });

  test("returns an empty hint when no clusters match the active file", async () => {
    const doc = await vscode.workspace.openTextDocument({
      content: "z",
      language: "plaintext",
    });
    await vscode.window.showTextDocument(doc);
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 1, "/does-not-match")]), 0);
    const provider = new FocusedFileProvider(store, new StatusTicker());
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });
});

suite("SessionProvider", () => {
  test("renders five session rows when a report is loaded", () => {
    const store = new ReportStore();
    store.setSnapshot(report([cluster("a", 1, "/f")]), 0);
    const provider = new SessionProvider(store, new StatusTicker(), () => undefined);
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 5);
    assert.equal(provider.getChildren(nodes[0]).length, 0);
  });

  test("renders a 'no session' placeholder before a report arrives", () => {
    const store = new ReportStore();
    const provider = new SessionProvider(store, new StatusTicker(), () => undefined);
    const nodes = provider.getChildren();
    assert.equal(nodes.length, 1);
  });

  test("marks state as running when the clientFactory returns a value", () => {
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const state = nodes.find((n) => typeof n.label === "string" && n.label === "State");
    assert.ok(state);
  });

  test("SessionProvider renders an Embedding progress row while a swap is in flight", () => {
    // [VSIX-SESSION-PROGRESS] The Session panel must show progress
    // while the selected model refresh is running.
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    store.setEmbeddingProgress({
      phase: "starting",
      provider_id: "ollama",
      model_id: "nomic-embed-text",
      done: 0,
      total: 23797,
    });
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const progress = nodes.find(
      (n) => typeof n.label === "string" && n.label === "Embedding",
    );
    assert.ok(progress, "Embedding progress row must be present");
    assert.match(
      String(progress.description ?? ""),
      /0\s*\/\s*23[,.]?797/,
      "progress description must carry done / total",
    );
  });

  test("Embedding model row shows the pending id with a loading suffix while a swap is in flight", () => {
    // [VSIX-SESSION-PROGRESS] The pending selected model is visible
    // before the LSP returns a fresh embedded report.
    const store = new ReportStore();
    store.setSnapshot(report([]), 0);
    store.setPendingEmbeddingModel("nomic-embed-text");
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const embeddingRow = nodes.find(
      (n) => typeof n.label === "string" && n.label === "Embedding model",
    );
    assert.ok(embeddingRow, "Embedding model row must be rendered");
    assert.match(
      String(embeddingRow.description ?? ""),
      /nomic-embed-text.*loading/i,
      "pending model id must be visible with a loading hint",
    );
  });

  test("Embedding model row prompts for selection when live embeddings are off", () => {
    // [LIVE-EMBEDDING-CONSENT] Fresh live sessions must guide the user
    // to select a model instead of implying embeddings already ran.
    const store = new ReportStore();
    const snapshot = report([]);
    snapshot.embedding_provenance = null;
    store.setSnapshot(snapshot, 0);
    const provider = new SessionProvider(store, new StatusTicker(), () => ({}) as never);
    const nodes = provider.getChildren();
    const embeddingRow = nodes.find(
      (n) => typeof n.label === "string" && n.label === "Embedding model",
    );
    assert.ok(embeddingRow, "Embedding model row must be rendered");
    assert.match(
      String(embeddingRow.description ?? ""),
      /select model/i,
      "session panel must make model selection discoverable",
    );
  });
});
