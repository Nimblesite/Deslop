// [VSIX-LIVE-BUBBLE] Stale-probe races, driven through the REAL async path:
// a real LiveBubble + ReportStore + a deferred fake client. Each test
// dispatches `probe()` and settles the deferred findSimilar responses out
// of order — no timers, no sleeps — then asserts the rendered decoration
// text. RA-05: the failure path must obey the same freshness guard as the
// success path, supersession must cancel the stale request, and the store
// revision (not the wire generation) is the freshness token.

import * as assert from "node:assert/strict";
import {
  DEFAULT_BUBBLE_CLUSTER_MASS,
  FIXTURE_KIND_TITLE,
  PRIMARY_BUBBLE_CLUSTER_ID,
  bubbleCluster,
  bubbleFixture,
  deferredProbeClient,
  editAt,
  probeCluster,
  probeReport,
  resolveProbe,
  settled,
  withBubble,
} from "./bubble.helpers";
import { reportWithClusters } from "./report.helpers";

suite("LiveBubble stale-probe races", () => {
  test("a stalled probe rejecting after a newer probe rendered leaves the newer bubble intact", async () => {
    const { client, requests } = deferredProbeClient();
    await withBubble({ generation: 1, client }, async ({ capture, bubble }) => {
      const probeA = bubble.probe(capture.editor, editAt(0, "aaaa"));
      const probeB = bubble.probe(capture.editor, editAt(6, "bbbb"));
      assert.equal(requests.length, 2, "both probes must dispatch a findSimilar request");
      assert.equal(requests[0]?.params.path, "/tmp/A.cs", "probe A carries the document path");
      assert.equal(requests[0]?.params.start_byte, 0, "probe A starts at the edit's byte offset");
      assert.equal(requests[0]?.params.end_byte, 4, "probe A spans the inserted text");
      assert.equal(
        requests[0]?.token.isCancellationRequested,
        true,
        "dispatching probe B must cancel probe A's in-flight request",
      );
      await resolveProbe(requests[1], probeB, false);
      const rendered = capture.visible();
      assert.ok(rendered !== undefined, "probe B must render its bubble");
      assert.match(rendered ?? "", new RegExp(FIXTURE_KIND_TITLE), "B carries the clone kind verdict");
      assert.match(rendered ?? "", /×\s*5/, "B carries the authoritative occurrence count");
      assert.match(rendered ?? "", /A\.cs/, "B names the canonical file");

      // The race under test: the superseded probe now rejects (a stall, a
      // server error, or its own cancellation acknowledgement). The old
      // catch path called clearBubble() unconditionally and erased B.
      requests[0]?.reject(new Error("stalled probe A finally failed"));
      await probeA;
      assert.equal(
        capture.visible(),
        rendered,
        "a stalled probe rejecting after a newer probe rendered must not erase the newer bubble",
      );
      assert.match(
        capture.visible() ?? "",
        /×\s*5/,
        "the surviving bubble text is B's, verbatim",
      );
    
    });
  });

  test("a probe resolving after a newer snapshot dropped its cluster paints nothing", async () => {
    const { client, requests } = deferredProbeClient();
    await withBubble({ generation: 1, client }, async ({ store, capture, bubble }) => {
      const probeA = bubble.probe(capture.editor, editAt(0, "aaaa"));

      // The newer full snapshot omits c-a entirely — and settles every
      // retraction tombstone, which is exactly why the ledger cannot guard
      // this race on its own.
      store.setSnapshot(reportWithClusters([bubbleCluster("c-other", 3)]), 2);
      assert.equal(
        store.current.retractedClusters.size,
        0,
        "a snapshot settles every retraction — the freshness token must catch what the ledger cannot",
      );

      // The refresh that doomed probe A re-issues it against the new
      // revision; A's own completion is still held to the freshness guard.
      assert.equal(requests.length, 2, "the newer snapshot re-issues the in-flight probe");
      await resolveProbe(requests[0], probeA, true);
      assert.equal(
        capture.calls.length,
        0,
        "the stale success must not touch the decoration surface at all",
      );
      assert.equal(capture.visible(), undefined, "no bubble may appear for the dropped cluster");
      assert.equal(capture.visibleHover(), undefined, "and no hover card either");

      // Positive proof the guard discriminates rather than blinds: the
      // re-issued probe, answered against the new snapshot, still renders.
      requests[1]?.resolve([bubbleCluster("c-other", 3)]);
      await settled();
      assert.match(
        capture.visible() ?? "",
        new RegExp(FIXTURE_KIND_TITLE),
        "a fresh probe against the new snapshot must still render its bubble",
      );
      assert.match(capture.visible() ?? "", /×\s*2/, "with the new cluster's occurrence count");
    
    });
  });

  test("generation ABA cannot defeat freshness and the store never rolls backward", async () => {
    const { client, requests } = deferredProbeClient();
    await withBubble({ generation: 3, client }, async ({ store, capture, bubble }) => {
      const revisionAtDispatch = store.current.revision;
      const probeA = bubble.probe(capture.editor, editAt(0, "aaaa"));

      // A stale completion labelled with an older generation must be
      // rejected outright: content and generation both stay put.
      const staleSnapshot = reportWithClusters([bubbleCluster("c-stale", 9)]);
      assert.equal(
        store.setSnapshot(staleSnapshot, 2),
        false,
        "a generation rollback must be rejected",
      );
      assert.equal(store.current.generation, 3, "the generation never moves backward");
      assert.equal(
        store.current.report?.clusters[0]?.id,
        "c-a",
        "the rejected snapshot must not replace the content",
      );
      assert.equal(
        store.current.revision,
        revisionAtDispatch,
        "a rejected snapshot must not advance the revision",
      );

      // The second half of the ABA: a snapshot re-labelled with the SAME
      // generation, but different content — c-a is gone. The wire label
      // reads 3 again; only the client-owned revision records the change.
      assert.equal(
        store.setSnapshot(reportWithClusters([bubbleCluster("c-other", 3)]), 3),
        true,
        "a same-generation replacement is accepted",
      );
      assert.ok(
        store.current.revision > revisionAtDispatch,
        "every accepted snapshot advances the monotonic revision",
      );
      assert.equal(store.current.generation, 3, "the generation label reads 3 again — ABA");

      await resolveProbe(requests[0], probeA);
      assert.equal(
        capture.calls.length,
        0,
        "a probe dispatched before the ABA must not repaint the dropped cluster",
      );
      assert.equal(capture.visible(), undefined, "the surface stays empty");
    
    });
  });

  test("dispose() cancels the in-flight probe and strands its completion", async () => {
    const { client, requests } = deferredProbeClient();
    const { capture, bubble } = await bubbleFixture({ generation: 1, client });
    const probeA = bubble.probe(capture.editor, editAt(0, "aaaa"));
    assert.equal(requests.length, 1, "the probe must dispatch its request");
    assert.equal(requests[0]?.token.isCancellationRequested, false, "live before dispose");

    bubble.dispose();
    await resolveProbe(requests[0], probeA, true);
    assert.equal(
      capture.calls.length,
      0,
      "a completion landing after dispose must not touch the decoration surface",
    );
  });
});

suite("LiveBubble probe outlived by a report refresh", () => {
  // [VSIX-LIVE-BUBBLE] [LIVE-OPEN-BUFFERS] The debounce that dispatches the
  // probe is the same pause that makes the server re-analyse the dirty
  // buffer, so the refreshed report routinely lands while the probe is in
  // flight. The probe's answer is then stale by the store's own rule and
  // must not paint — but the edit cycle still earned its bubble.
  test("a report refresh landing mid-probe re-issues the probe instead of dropping the edit cycle", async () => {
    const { client, requests } = deferredProbeClient();
    await withBubble({ generation: 1, client }, async ({ store, capture, bubble }) => {
      const probeA = bubble.probe(capture.editor, editAt(0, "aaaa"));
      assert.equal(requests.length, 1, "the edit dispatches one probe");

      assert.equal(store.setSnapshot(probeReport(), 2), true, "the re-analysis of the dirty buffer is accepted");
      assert.equal(requests.length, 2, "the refresh re-issues the probe against the new revision");
      assert.equal(requests[0]?.token.isCancellationRequested, true, "the probe the refresh doomed is cancelled");
      assert.equal(requests[1]?.token.isCancellationRequested, false, "the re-issued probe is live");
      assert.deepEqual(requests[1]?.params, requests[0]?.params, "the re-issued probe asks about the same range");

      await resolveProbe(requests[0], probeA);
      assert.equal(capture.calls.length, 0, "the stale answer never touches the surface");

      requests[1]?.resolve([probeCluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS)]);
      await settled();
      const rendered = capture.visible();
      assert.ok(rendered !== undefined, "the re-issued probe renders the bubble this edit earned");
      assert.match(rendered ?? "", new RegExp(FIXTURE_KIND_TITLE), "with the clone kind verdict");
      assert.match(rendered ?? "", /×\s*5/, "and the refreshed report's occurrence count");
      assert.match(rendered ?? "", /A\.cs/, "naming the canonical file");
    });
  });
});

suite("LiveBubble probe answered before the acknowledging refresh", () => {
  // [VSIX-LIVE-BUBBLE] [VSIX-STATE-DIRTY] The other ordering of the same
  // race: the probe answers first, against a projection that still hides
  // the edited file's occurrences, so the cluster is not visible and the
  // surface stays empty. The refresh that acknowledges the buffer then
  // reveals the cluster — and must re-issue the probe, or the edit never
  // gets the bubble it earned.
  test("a probe answered while the buffer was still hidden is re-issued when the acknowledging refresh lands", async () => {
    const { client, requests } = deferredProbeClient();
    await withBubble({ generation: 1, client }, async ({ store, capture, bubble }) => {
      const path = capture.editor.document.uri.fsPath;
      const version = capture.editor.document.version;
      store.markFileDirty(path, version);
      assert.equal(store.isFileDirty(path), true, "the edited buffer awaits analysis");
      assert.equal(
        store.current.visibleReport?.clusters.length,
        0,
        "the projection hides the only cluster while its file is dirty",
      );

      const probeA = bubble.probe(capture.editor, editAt(0, "aaaa"));
      await resolveProbe(requests[0], probeA);
      assert.equal(requests.length, 1, "one probe so far");
      assert.equal(capture.visible(), undefined, "a hidden cluster paints nothing");

      assert.equal(
        store.setSnapshot(probeReport(), 2, { path, version }),
        true,
        "the refresh acknowledging this buffer version is accepted",
      );
      assert.equal(store.isFileDirty(path), false, "the buffer is analysed");
      assert.equal(requests.length, 2, "the acknowledging refresh re-issues the probe");
      assert.deepEqual(requests[1]?.params, requests[0]?.params, "for the same edited range");

      requests[1]?.resolve([probeCluster(PRIMARY_BUBBLE_CLUSTER_ID, DEFAULT_BUBBLE_CLUSTER_MASS)]);
      await settled();
      const rendered = capture.visible();
      assert.ok(rendered !== undefined, "the re-issued probe paints the bubble");
      assert.match(rendered ?? "", new RegExp(FIXTURE_KIND_TITLE), "with the clone kind verdict");
      assert.match(rendered ?? "", /×\s*5/, "and the acknowledged report's occurrence count");

      // Settled against an analysed buffer, the probe is no longer
      // provisional: a later unrelated refresh does not re-issue it.
      assert.equal(store.setSnapshot(probeReport(), 3), true, "an unrelated refresh is accepted");
      assert.equal(requests.length, 2, "an analysed edit is not re-probed on every refresh");
    });
  });
});
