// Unit: cluster detail panel selection survives content-hash id churn (#173).
// A cluster's wire id is a hash of its smallest member, so it changes whenever
// that member's normalised AST shifts under re-analysis. The detail panel must
// keep showing the opened cluster across generations instead of silently
// blanking to "No cluster selected." once the captured id no longer resolves.

import * as assert from "node:assert/strict";
import * as path from "node:path";
import * as vscode from "vscode";

import * as logging from "../../logging";
import { HANDLE_MESSAGE_FAILED, openClusterPanel } from "../../webview/panels";
import { ReportStore } from "../../reportStore";
import { anchorForClusterId, clusterPanelFeed, resolveAnchoredCluster } from "../../clusterSelection";
import { Report, ReportCluster, ReportOccurrence } from "../../types/report";
import { reportWithClusters } from "./report.helpers";
import { wireCluster } from "../cluster.helpers";
import { storeWith } from "./report-store.helpers";

const PRIMARY_CLUSTER_ID = "aaaaaaaa";
const NEIGHBOUR_CLUSTER_ID = "99999999";
const TEST_THIRTY = 30;
const FIRST_OCCURRENCE_INDEX = 0;
const THREE_OCCURRENCES = 3;
const CANONICAL_NODE_COUNT = 42;
const CLEAN_PEER_COUNT = 2;
const INITIAL_GENERATION = 1;
/** #606 Workspaces from one cluster to large enough that shipping it whole is the defect. */
const WORKSPACE_SIZES = [1, 2, 50, 500] as const;
const CLUSTER_ID_WIDTH = 7;
const NAVIGATE_CLUSTER = "navigate/cluster";
const MISSING_ID = "gone-gone-gone";
const NO_POSTS = 0;
const ONE_ERROR = 1;
/** Polls for the failed open's log; the loop ends as soon as it appears. */
const SETTLE_ATTEMPTS = 200;
const SETTLE_INTERVAL_MS = 50;
const LOCATIONS_PER_CLUSTER = 3;
const HALF = 2;
/** Where in the engine's rank order the opened cluster sits. */
const OPENED_POSITIONS = [
  { name: "worst cluster", index: (_size: number) => 0 },
  { name: "middle cluster", index: (size: number) => Math.floor(size / HALF) },
  { name: "last cluster", index: (size: number) => size - 1 },
] as const;

/** `size` clusters in rank order, each with its own files. */
function workspaceOf(size: number): ReportCluster[] {
  return Array.from({ length: size }, (_, index) => ({
    ...clusterOf(`c${String(index).padStart(CLUSTER_ID_WIDTH, "0")}`, size - index,
      Array.from({ length: LOCATIONS_PER_CLUSTER }, (_unused, member) => occ(`/repo/src/f${index}_${member}.cs`, member, member + TEST_THIRTY))),
    rank: index + 1,
  }));
}

/** The last snapshot ships `opened`'s locations — all of them — and nothing else but ids, in rank order. */
function assertShipsOnly(fake: FakePanel, opened: ReportCluster, workspace: ReportCluster[], when: string): void {
  const select = lastOfKind(fake.messages, "select/cluster");
  const snapshot = lastOfKind(fake.messages, "report/snapshot");
  assert.equal(select?.id, opened.id, `${when}: the opened cluster is selected`);
  assert.ok(snapshot?.report, `${when}: the panel receives the opened cluster`);
  const shipped = snapshot.report.clusters.find((cluster) => cluster.id === opened.id);
  assert.ok(shipped, `${when}: the opened cluster is shipped`);
  assert.deepEqual(
    shipped.occurrences.map((occurrence) => [occurrence.path, occurrence.start_byte, occurrence.end_byte]),
    opened.occurrences.map((occurrence) => [occurrence.path, occurrence.start_byte, occurrence.end_byte]),
    `${when}: every location of the opened cluster is shipped, canonical first`,
  );
  const locations = snapshot.report.clusters.reduce((total, cluster) => total + cluster.occurrences.length, 0);
  assert.equal(
    locations,
    opened.occurrences.length,
    `${when}: opening one cluster must ship only its ${opened.occurrences.length} locations; shipped ${locations} ` +
      `across ${snapshot.report.clusters.length} clusters`,
  );
  assert.deepEqual(
    snapshot.report.clusters.map((cluster) => cluster.id),
    [...workspace].sort((left, right) => left.rank - right.rank).map((cluster) => cluster.id),
    `${when}: the navigation list keeps every cluster id in the engine's rank order`,
  );
}

interface PostedMessage {
  kind?: string;
  id?: string | null;
  report?: Report;
}

interface FakePanel {
  panel: vscode.WebviewPanel;
  messages: PostedMessage[];
  fireReady(): void;
  fireMessage(message: unknown): void;
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
    fireMessage: (message: unknown) => receivers.slice().forEach((handler) => handler(message)),
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
  return { path: filePath, start_byte: startByte, end_byte: endByte, start_line: 1, end_line: 2, hidden: false };
}

function clusterOf(id: string, mass: number, occurrences: ReportOccurrence[]): ReportCluster {
  return wireCluster({
    id,
    mass,
    canonical_node_count: 0,
        occurrences,
  });
}

function reportOf(clusters: ReportCluster[]): Report {
  return reportWithClusters(clusters, { files_analysed: 4 });
}

function lastOfKind(messages: PostedMessage[], kind: string): PostedMessage | undefined {
  return [...messages].reverse().find((message) => message.kind === kind);
}

/** The two clusters of the navigation fixture, as the report publishes them. */
function publishedClusters(report: Report): { opened: ReportCluster; neighbour: ReportCluster } {
  const opened = report.clusters.find((cluster) => cluster.id === PRIMARY_CLUSTER_ID);
  const neighbour = report.clusters.find((cluster) => cluster.id === NEIGHBOUR_CLUSTER_ID);
  assert.ok(opened && neighbour, "the fixture report must publish both clusters");
  return { opened, neighbour };
}

/** The dirty-projected form of the neighbour cluster, asserted present. */
function projectedNeighbour(store: ReportStore): ReportCluster {
  const projected = store.current.visibleReport?.clusters.find(
    (cluster) => cluster.id === NEIGHBOUR_CLUSTER_ID,
  );
  assert.ok(projected, "the neighbour must survive the projection with two members");
  return projected;
}

// Opens a cluster panel against a stubbed webview, drives the `ready`
// handshake, then runs `body` with the captured message stream.
function withClusterPanel(
  store: ReportStore,
  clusterId: string,
  body: (fake: FakePanel) => void,
  ready = true,
): void {
  const fake = fakeWebviewPanel();
  const windowApi = vscode.window as unknown as { createWebviewPanel: unknown };
  const original = windowApi.createWebviewPanel;
  windowApi.createWebviewPanel = () => fake.panel;
  try {
    openClusterPanel(fakeCtx(), store, clusterId);
    if (ready) fake.fireReady();
    body(fake);
  } finally {
    fake.fireDispose(); // tear down the module-level activePanels entry
    windowApi.createWebviewPanel = original;
  }
}

suite("cluster detail panel selection (#173)", () => {
  const groupOccurrences = [occ("/repo/Alpha.cs", 10, 20), occ("/repo/Beta.cs", TEST_THIRTY, 40)];
  const otherOccurrences = [occ("/repo/Gamma.cs", 0, 9), occ("/repo/Delta.cs", 0, 9)];
  // A third member so this cluster still clears the projection's two-visible
  // floor after its canonical occurrence goes dirty.
  const neighbourOccurrences = [...otherOccurrences, occ("/repo/Epsilon.cs", 0, 9)];

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
    const store = storeWith(reportOf([clusterOf("dddddddd", TEST_THIRTY, groupOccurrences)]), 1);

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

  test("a surviving projection preserves the opened cluster's original canonical and figures", () => {
    // [VSIX-PAIR-COMPARE] A clean peer is never silently promoted to canonical.
    const canonical = groupOccurrences[FIRST_OCCURRENCE_INDEX];
    assert.ok(canonical);
    const occurrences = [...groupOccurrences, ...otherOccurrences].slice(FIRST_OCCURRENCE_INDEX, THREE_OCCURRENCES);
    const original = { ...clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, occurrences), canonical_node_count: CANONICAL_NODE_COUNT };
    const report = reportOf([original]);
    const store = storeWith(report, FIRST_OCCURRENCE_INDEX);
    store.markFileDirty(canonical.path);
    const visible = store.current.visibleReport;
    assert.ok(visible);
    assert.equal(visible.clusters[FIRST_OCCURRENCE_INDEX]?.occurrences.length, CLEAN_PEER_COUNT);
    const feed = clusterPanelFeed(report, visible, anchorForClusterId(report, original.id));
    assert.equal(feed.selectedId, original.id);
    assert.deepEqual(feed.report.clusters[FIRST_OCCURRENCE_INDEX], report.clusters[FIRST_OCCURRENCE_INDEX]);
    assert.equal(feed.report.clusters[FIRST_OCCURRENCE_INDEX]?.occurrences[FIRST_OCCURRENCE_INDEX], canonical);
    assert.equal(feed.report.clusters[FIRST_OCCURRENCE_INDEX]?.canonical_node_count, CANONICAL_NODE_COUNT);
  });

  // [VSIX-PAIR-COMPARE] `n` / `p` move the panel's selection inside the
  // webview, with no host round trip — the host keeps no second copy of
  // cluster selection state. So every cluster in the feed is a cluster the
  // user can detail, and every one of them must arrive with the engine's own
  // membership. Hand the panel a projection that elided a dirty canonical and
  // its first surviving peer takes the canonical slot: mislabelled as
  // canonical, and its own Compare action suppressed as a self-comparison.
  test("a cluster the panel can navigate to keeps its canonical, never a promoted peer", () => {
    const opened = clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, groupOccurrences);
    const neighbour = clusterOf(NEIGHBOUR_CLUSTER_ID, 5, neighbourOccurrences);
    const neighbourCanonical = neighbourOccurrences[FIRST_OCCURRENCE_INDEX];
    assert.ok(neighbourCanonical);
    const report = reportOf([opened, neighbour]);
    // `reportOf` stamps the ranking the engine would have stamped, so the
    // published clusters — not the raw fixtures — are what the panel must get.
    const published = publishedClusters(report);
    const store = storeWith(report, INITIAL_GENERATION);
    try {
      store.markFileDirty(neighbourCanonical.path);
      const projected = projectedNeighbour(store);
      assert.equal(projected.occurrences.length, CLEAN_PEER_COUNT, "precondition: the edit elides one member");
      assert.notEqual(
        projected.occurrences[FIRST_OCCURRENCE_INDEX],
        neighbourCanonical,
        "precondition: the projection alone leaves a peer in the canonical slot",
      );

      // The panel is anchored on the OPENED cluster: the neighbour reaches it
      // as a navigation entry only — id, rank and counts, no locations (#606).
      const visible = store.current.visibleReport;
      assert.ok(visible);
      const feed = clusterPanelFeed(report, visible, anchorForClusterId(report, PRIMARY_CLUSTER_ID));
      assert.equal(feed.selectedId, PRIMARY_CLUSTER_ID, "the opened cluster stays selected");
      const entry = feed.report.clusters.find((cluster) => cluster.id === NEIGHBOUR_CLUSTER_ID);
      assert.ok(entry, "a cluster the user can navigate to must reach the panel");
      assert.deepEqual(entry.occurrences, [], "a cluster the panel does not show travels without its locations");
      assert.equal(entry.occurrence_count, THREE_OCCURRENCES, "its entry still counts the engine's occurrences");
      assert.equal(entry.rank, published.neighbour.rank, "its entry keeps the engine's rank");
      assert.equal(
        feed.report.clusters.find((cluster) => cluster.id === PRIMARY_CLUSTER_ID),
        published.opened,
        "the opened cluster arrives as the engine published it",
      );

      // Navigating `n` re-anchors the panel on the neighbour, as the host does
      // for a `navigate/cluster` request: it arrives whole, canonical first.
      const navigated = clusterPanelFeed(report, visible, anchorForClusterId(report, NEIGHBOUR_CLUSTER_ID));
      assert.equal(navigated.selectedId, NEIGHBOUR_CLUSTER_ID, "navigation selects the neighbour");
      const fed = navigated.report.clusters.find((cluster) => cluster.id === NEIGHBOUR_CLUSTER_ID);
      assert.ok(fed, "the navigated cluster reaches the panel");
      assert.equal(
        fed.occurrences[FIRST_OCCURRENCE_INDEX],
        neighbourCanonical,
        "the canonical slot holds the engine's canonical occurrence, not the first clean peer",
      );
      assert.equal(fed.occurrences.length, THREE_OCCURRENCES, "every peer stays comparable against it");
      assert.equal(fed.occurrence_count, THREE_OCCURRENCES, "the panel counts the engine's occurrences");
      assert.equal(fed, published.neighbour, "the navigable cluster arrives as the engine published it");
      assert.deepEqual(
        navigated.report.clusters.find((cluster) => cluster.id === PRIMARY_CLUSTER_ID)?.occurrences,
        [],
        "the cluster navigated away from travels as an entry only",
      );
    } finally {
      store.dispose();
    }
  });

  test("clears the selection and surfaces the dead id when the cluster leaves the report entirely", () => {
    const store = storeWith(reportOf([clusterOf("eeeeeeee", TEST_THIRTY, groupOccurrences)]), 1);

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

  // #606: the panel showed "No cluster selected." for seconds on React
  // because the host serialised the whole report — 10,774 clusters and 44,447
  // locations, each enriched by reading its source file — to show one cluster.
  // The panel needs the opened cluster's locations; the rest of the workspace
  // is at most a navigation list of ids, in the engine's rank order.
  for (const size of WORKSPACE_SIZES) {
    for (const position of OPENED_POSITIONS) {
      test(`opening the ${position.name} of ${size} clusters ships only that cluster's locations (#606)`, () => {
        const workspace = workspaceOf(size);
        const opened = workspace[position.index(size)];
        assert.ok(opened, "the fixture has a cluster at that position");
        const store = storeWith(reportOf(workspace), INITIAL_GENERATION);

        withClusterPanel(store, opened.id, (fake) => {
          assertShipsOnly(fake, opened, workspace, "first push");

          // A re-analysis renames every cluster id (content hashes churn) while
          // locations stay put: the panel follows the opened cluster by its
          // canonical location and still ships only its locations.
          const reanalysed = workspace.map((cluster) => ({ ...cluster, id: `${cluster.id}-next` }));
          store.setSnapshot(reportOf(reanalysed), INITIAL_GENERATION + 1);
          const followed = reanalysed[position.index(size)];
          assert.ok(followed);
          assertShipsOnly(fake, followed, reanalysed, "after a re-analysis");
        });
      });
    }
  }

  test("an opened cluster the dirty projection elided ships only its own locations (#606)", () => {
    const workspace = workspaceOf(WORKSPACE_SIZES[WORKSPACE_SIZES.length - 1] ?? 1);
    const opened = clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, groupOccurrences);
    const store = storeWith(reportOf([opened, ...workspace]), INITIAL_GENERATION);
    withClusterPanel(store, PRIMARY_CLUSTER_ID, (fake) => {
      store.markFileDirty(groupOccurrences[FIRST_OCCURRENCE_INDEX]?.path ?? "");
      assertShipsOnly(fake, opened, [opened, ...workspace], "with the canonical file unsaved");
    });
  });

  test("n / p navigation asks the host, which re-anchors and ships only the navigated cluster (#606)", () => {
    const workspace = workspaceOf(WORKSPACE_SIZES[WORKSPACE_SIZES.length - 1] ?? 1);
    const [first, second, third] = workspace;
    assert.ok(first && second && third);
    const store = storeWith(reportOf(workspace), INITIAL_GENERATION);
    withClusterPanel(store, first.id, (fake) => {
      assertShipsOnly(fake, first, workspace, "opened");
      for (const target of [second, third, first, workspace[workspace.length - 1]]) {
        assert.ok(target);
        fake.fireMessage({ kind: NAVIGATE_CLUSTER, id: target.id });
        assertShipsOnly(fake, target, workspace, `navigated to ${target.id}`);
      }

      // A re-analysis after navigating follows the navigated cluster, not the opened one.
      const reanalysed = workspace.map((cluster) => ({ ...cluster, id: `${cluster.id}-next` }));
      store.setSnapshot(reportOf(reanalysed), INITIAL_GENERATION + 1);
      const last = reanalysed[reanalysed.length - 1];
      assert.ok(last);
      assertShipsOnly(fake, last, reanalysed, "after navigating and a re-analysis");

      // Messages that are not a navigation, or name no cluster, change nothing.
      const pushes = fake.messages.length;
      for (const ignored of [{ kind: NAVIGATE_CLUSTER }, { kind: NAVIGATE_CLUSTER, id: 7 }, { kind: "navigate", id: first.id }, null, "navigate/cluster"]) {
        fake.fireMessage(ignored);
      }
      assert.equal(fake.messages.length, pushes, "a malformed navigation pushes nothing");

      // A navigation to an id the report no longer holds clears the selection.
      fake.fireMessage({ kind: NAVIGATE_CLUSTER, id: MISSING_ID });
      assert.equal(lastOfKind(fake.messages, "select/cluster")?.id, null, "an unknown cluster clears the selection");
      const snapshot = lastOfKind(fake.messages, "report/snapshot");
      assert.ok(snapshot?.report);
      assert.equal(
        snapshot.report.clusters.reduce((total, cluster) => total + cluster.occurrences.length, 0),
        0,
        "with nothing selected the panel receives no locations at all",
      );
    });
  });

  test("nothing reaches the panel before it is ready, and a navigation asked before then is honoured on ready (#606)", () => {
    const workspace = workspaceOf(WORKSPACE_SIZES[WORKSPACE_SIZES.length - 1] ?? 1);
    const [first, second] = workspace;
    assert.ok(first && second);
    const store = storeWith(reportOf(workspace), INITIAL_GENERATION);
    withClusterPanel(store, first.id, (fake) => {
      assert.equal(fake.messages.length, NO_POSTS, "an unmounted panel is sent nothing");
      fake.fireMessage({ kind: NAVIGATE_CLUSTER, id: second.id });
      fake.fireMessage({ kind: "not-ready" });
      fake.fireMessage(undefined);
      assert.equal(fake.messages.length, NO_POSTS, "messages before ready push nothing");
      fake.fireReady();
      assertShipsOnly(fake, second, workspace, "on ready, after an early navigation");
    }, false);
  });

  test("a panel opened before any report sends nothing until one arrives (#606)", () => {
    const store = new ReportStore();
    try {
      withClusterPanel(store, PRIMARY_CLUSTER_ID, (fake) => {
        assert.equal(fake.messages.length, NO_POSTS, "no report, no feed");
        const opened = clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, groupOccurrences);
        store.setSnapshot(reportOf([opened]), INITIAL_GENERATION);
        assertShipsOnly(fake, opened, [opened], "once the first report arrives");
      });
    } finally {
      store.dispose();
    }
  });

  test("a message whose command fails is logged, never thrown into the webview", async () => {
    const store = storeWith(reportOf([clusterOf(PRIMARY_CLUSTER_ID, TEST_THIRTY, groupOccurrences)]), INITIAL_GENERATION);
    const logged: unknown[] = [];
    const logger = logging as unknown as { logError: (error: unknown, context: string) => void };
    const original = logger.logError;
    logger.logError = (error: unknown, context: string) => { logged.push([context, error]); };
    try {
      withClusterPanel(store, PRIMARY_CLUSTER_ID, (fake) => {
        assert.ok(lastOfKind(fake.messages, "report/snapshot"), "the panel is wired and fed");
        // A location whose file does not exist: the real open command rejects.
        fake.fireMessage({ kind: "open/occurrence", occurrence: occ("/definitely/missing/file.cs", 0, 1) });
      });
      for (let attempt = 0; attempt < SETTLE_ATTEMPTS && logged.length < ONE_ERROR; attempt += 1) {
        await new Promise<void>((resolve) => {
          setTimeout(resolve, SETTLE_INTERVAL_MS);
        });
      }
    } finally {
      logger.logError = original;
      store.dispose();
    }
    assert.equal(logged.length, ONE_ERROR, "the failed command is logged once");
    assert.equal((logged[0] as [string])[0], HANDLE_MESSAGE_FAILED, "the log names the failed handler");
  });

  test("does not re-push the feed on embedding-progress, lifecycle, or pending-model ticks (VSIX-PERF)", () => {
    const store = storeWith(reportOf([clusterOf("ffffffff", TEST_THIRTY, groupOccurrences)]), 1);

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
