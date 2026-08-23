// Unit: cluster detail panel selection survives content-hash id churn (#173).
// A cluster's wire id is a hash of its smallest member, so it changes whenever
// that member's normalised AST shifts under re-analysis. The detail panel must
// keep showing the opened cluster across generations instead of silently
// blanking to "No cluster selected." once the captured id no longer resolves.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";

import { openClusterPanel } from "../../webview/panels";
import { ReportStore } from "../../reportStore";
import { anchorForClusterId, clusterPanelFeed, resolveAnchoredCluster } from "../../clusterSelection";
import { Report, ReportCluster, ReportOccurrence } from "../../types/report";
import { reportWithClusters } from "./report.helpers";
import { bucketSignals } from "../signals.helpers";
import { wireCluster } from "../cluster.helpers";

const PRIMARY_CLUSTER_ID = "aaaaaaaa";
const TEST_THIRTY = 30;

interface PostedMessage {
  kind?: string;
  id?: string | null;
  report?: Report;
}

interface FakePanel {
  panel: vscode.WebviewPanel;
  messages: PostedMessage[];
  fireReady(): void;
  fireDispose(): void;
}

function fakeWebviewPanel(): FakePanel {
  const messages: PostedMessage[] = [];
  const receivers: ((message: unknown) => void)[] = [];
  const disposers: (() => void)[] = [];
  const webview = {
    cspSource: "vscode-resource:",
    html: "",
    asWebviewUri: (uri: vscode.Uri) => uri,
    postMessage: (message: PostedMessage) => {
      messages.push(message);
      return Promise.resolve(true);
    },
    onDidReceiveMessage: (handler: (message: unknown) => void) => {
      receivers.push(handler);
      return {
        dispose: () => {
          const index = receivers.indexOf(handler);
          if (index >= 0) receivers.splice(index, 1);
        },
      };
    },
  };
  const panel = {
    webview,
    reveal: () => {},
    onDidDispose: (handler: () => void) => {
      disposers.push(handler);
      return { dispose: () => {} };
    },
  } as unknown as vscode.WebviewPanel;
  return {
    panel,
    messages,
    fireReady: () => receivers.slice().forEach((handler) => handler({ kind: "ready" })),
    fireDispose: () => disposers.slice().forEach((handler) => handler()),
  };
}

function fakeCtx(): vscode.ExtensionContext {
  const root = path.join(__dirname, "..", "..", "..");
  return {
    extensionPath: root,
    extensionUri: vscode.Uri.file(root),
    subscriptions: [] as vscode.Disposable[],
  } as unknown as vscode.ExtensionContext;
}

function occ(filePath: string, startByte: number, endByte: number): ReportOccurrence {
  return { path: filePath, start_byte: startByte, end_byte: endByte, hidden: false };
}

function clusterOf(id: string, weight: number, occurrences: ReportOccurrence[]): ReportCluster {
  return wireCluster({
    id,
    weight,
    canonical_node_count: 0,
    bucket: "identical",
    signals: bucketSignals("identical"),
    occurrences,
  });
}

function reportOf(clusters: ReportCluster[]): Report {
  return reportWithClusters(clusters, { files_analysed: 4 });
}

function lastOfKind(messages: PostedMessage[], kind: string): PostedMessage | undefined {
  return [...messages].reverse().find((message) => message.kind === kind);
}

// Opens a cluster panel against a stubbed webview, drives the `ready`
// handshake, then runs `body` with the captured message stream.
function withClusterPanel(
  store: ReportStore,
  clusterId: string,
  body: (fake: FakePanel) => void,
): void {
  const fake = fakeWebviewPanel();
  const windowApi = vscode.window as unknown as { createWebviewPanel: unknown };
  const original = windowApi.createWebviewPanel;
  windowApi.createWebviewPanel = () => fake.panel;
  try {
    openClusterPanel(fakeCtx(), store, clusterId);
    fake.fireReady();
    body(fake);
  } finally {
    fake.fireDispose(); // tear down the module-level activePanels entry
    windowApi.createWebviewPanel = original;
  }
}

suite("cluster detail panel selection (#173)", () => {
  const groupOccurrences = [occ("/repo/Alpha.cs", 10, 20), occ("/repo/Beta.cs", TEST_THIRTY, 40)];
  const otherOccurrences = [occ("/repo/Gamma.cs", 0, 9), occ("/repo/Delta.cs", 0, 9)];

  test("re-resolves the selected cluster when its content-hash id churns across a re-analysis", () => {
    const store = new ReportStore();
    // The opened duplicate group's canonical occurrence is Alpha.cs:10-20.
    store.setSnapshot(
      reportOf([clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, groupOccurrences), clusterOf("cccccccc", 5, otherOccurrences)]),
      1,
    );

    withClusterPanel(store, PRIMARY_CLUSTER_ID, (fake) => {
      // After the webview mounts, the panel selects the opened cluster and the
      // cluster is present in the snapshot it was given.
      assert.equal(
        lastOfKind(fake.messages, "select/cluster")?.id,
        PRIMARY_CLUSTER_ID,
        "panel must select the opened cluster once the webview is ready",
      );
      assert.ok(
        lastOfKind(fake.messages, "report/snapshot")?.report?.clusters.some((c) => c.id === PRIMARY_CLUSTER_ID),
        "opened cluster must be present in the first snapshot the panel receives",
      );

      // Re-analysis: the same duplicate group survives, but its smallest member's
      // normalised hash shifted, so the wire id churns aaaaaaaa -> bbbbbbbb. The
      // canonical occurrence (Alpha.cs:10-20) is unchanged.
      store.setSnapshot(
        reportOf([clusterOf("bbbbbbbb", TEST_THIRTY, groupOccurrences), clusterOf("cccccccc", 5, otherOccurrences)]),
        2,
      );

      const select = lastOfKind(fake.messages, "select/cluster");
      const snapshot = lastOfKind(fake.messages, "report/snapshot");
      assert.ok(select, "panel must have a current selection after the id churn");
      assert.ok(snapshot?.report, "panel must have a current snapshot after the id churn");
      assert.ok(
        snapshot.report.clusters.some((c) => c.id === select.id),
        `selected cluster id ${String(select.id)} must exist in the live snapshot — ` +
          `otherwise the detail panel renders "No cluster selected."`,
      );
      assert.equal(
        select.id,
        "bbbbbbbb",
        "selection must follow the duplicate group to its new id, not stay pinned to the dead id",
      );
    });
  });

  test("keeps the opened cluster visible when an unsaved edit would elide it from the projection", () => {
    const store = new ReportStore();
    store.setSnapshot(reportOf([clusterOf("dddddddd", TEST_THIRTY, groupOccurrences)]), 1);

    withClusterPanel(store, "dddddddd", (fake) => {
      // Editing Alpha.cs drops the cluster below two visible occurrences, so the
      // visible projection elides it entirely (#117). The detail panel must still
      // show the cluster the user explicitly opened.
      store.markFileDirty("/repo/Alpha.cs");
      assert.equal(
        store.current.visibleReport?.clusters.length,
        0,
        "precondition: the dirty edit elides the cluster from the visible projection",
      );

      const select = lastOfKind(fake.messages, "select/cluster");
      const snapshot = lastOfKind(fake.messages, "report/snapshot");
      assert.equal(select?.id, "dddddddd", "panel keeps the opened cluster selected through the edit");
      assert.ok(
        snapshot?.report?.clusters.some((c) => c.id === "dddddddd"),
        "opened cluster must be re-injected into the snapshot from canonical truth",
      );
    });
  });

  test("clears the selection and surfaces the dead id when the cluster leaves the report entirely", () => {
    const store = new ReportStore();
    store.setSnapshot(reportOf([clusterOf("eeeeeeee", TEST_THIRTY, groupOccurrences)]), 1);

    withClusterPanel(store, "eeeeeeee", (fake) => {
      // The duplication is resolved — the cluster is gone from the next report.
      store.setSnapshot(reportOf([clusterOf("cccccccc", 5, otherOccurrences)]), 2);

      const select = lastOfKind(fake.messages, "select/cluster");
      const snapshot = lastOfKind(fake.messages, "report/snapshot");
      assert.equal(select?.id, null, "selection must clear when the opened cluster no longer exists");
      assert.ok(snapshot?.report, "panel still receives the live snapshot after the cluster is gone");
      assert.ok(
        !snapshot.report.clusters.some((c) => c.id === "eeeeeeee"),
        "the dead cluster must not be re-injected into the snapshot",
      );
    });
  });

  test("does not re-push the feed on embedding-progress, lifecycle, or pending-model ticks (VSIX-PERF)", () => {
    const store = new ReportStore();
    store.setSnapshot(reportOf([clusterOf("ffffffff", TEST_THIRTY, groupOccurrences)]), 1);

    withClusterPanel(store, "ffffffff", (fake) => {
      const afterReady = fake.messages.length;
      assert.ok(afterReady >= 1, "the ready handshake pushes the initial feed");

      // Non-report store churn must NOT re-push the (potentially heavy) cluster
      // feed — the effect tracks only report + visibleReport.
      store.setEmbeddingProgress({
        phase: "starting",
        provider_id: "ollama",
        model_id: "nomic-embed-text",
        done: 0,
        total: 10,
        percent: 0,
        message: undefined,
      });
      store.setLifecycle({ kind: "analysing" });
      store.setPendingEmbeddingModel("nomic-embed-text");
      assert.equal(
        fake.messages.length,
        afterReady,
        "embedding-progress / lifecycle / pending-model ticks do not re-push the feed",
      );

      // A genuine report change still re-pushes.
      store.setSnapshot(reportOf([clusterOf("ffffffff", TEST_THIRTY, groupOccurrences)]), 2);
      assert.ok(
        fake.messages.length > afterReady,
        "a report change re-pushes the feed",
      );
    });
  });

  test("resolveAnchoredCluster falls back to the id when the canonical occurrence has moved", () => {
    const report = reportOf([clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, [occ("/repo/Alpha.cs", 99, 110)])]);
    const anchor = anchorForClusterId(reportOf([clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, groupOccurrences)]), PRIMARY_CLUSTER_ID);
    // The anchored occurrence (Alpha.cs:10-20) is gone, but the id survives — the
    // id fallback keeps the selection rather than blanking the panel.
    assert.equal(resolveAnchoredCluster(report, anchor)?.id, PRIMARY_CLUSTER_ID);
    assert.equal(resolveAnchoredCluster(null, anchor), undefined);
    const feed = clusterPanelFeed(report, report, anchor);
    assert.equal(feed.selectedId, PRIMARY_CLUSTER_ID);
  });
});
