// [VSIX-LIVE-BUBBLE] Stale-probe races, driven through the REAL async path:
// a real LiveBubble + ReportStore + a deferred fake client. Each test
// dispatches `probe()` and settles the deferred findSimilar responses out
// of order — no timers, no sleeps — then asserts the rendered decoration
// text. RA-05: the failure path must obey the same freshness guard as the
// success path, supersession must cancel the stale request, and the store
// revision (not the wire generation) is the freshness token.

import * as assert from "node:assert/strict";
import {
  bubbleCluster,
  bubbleFixture,
  deferredProbeClient,
  editAt,
  resolveProbe,
} from "./bubble.helpers";
import { SHORT_VERDICT } from "../../bubble/renderParts";
import { reportWithClusters } from "./report.helpers";

suite("LiveBubble stale-probe races", () => {
  test("a stalled probe rejecting after a newer probe rendered leaves the newer bubble intact", async () => {
    const { client, requests } = deferredProbeClient();
    const { capture, bubble } = await bubbleFixture({ generation: 1, client });
    try {
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
      assert.match(rendered ?? "", new RegExp(SHORT_VERDICT), "B carries the short duplication verdict");
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
    } finally {
      bubble.dispose();
    }
  });

  test("a probe resolving after a newer snapshot dropped its cluster paints nothing", async () => {
    const { client, requests } = deferredProbeClient();
    const { store, capture, bubble } = await bubbleFixture({ generation: 1, client });
    try {
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

      await resolveProbe(requests[0], probeA);
      assert.equal(
        capture.calls.length,
        0,
        "the stale success must not touch the decoration surface at all",
      );
      assert.equal(capture.visible(), undefined, "no bubble may appear for the dropped cluster");
      assert.equal(capture.visibleHover(), undefined, "and no hover card either");

      // Positive proof the guard discriminates rather than blinds: a fresh
      // probe against the new snapshot still renders.
      const probeB = bubble.probe(capture.editor, editAt(6, "bbbb"));
      await resolveProbe(requests[1], probeB, undefined, [
        bubbleCluster("c-other", 3),
      ]);
      assert.match(
        capture.visible() ?? "",
        new RegExp(SHORT_VERDICT),
        "a fresh probe against the new snapshot must still render its bubble",
      );
      assert.match(capture.visible() ?? "", /×\s*2/, "with the new cluster's occurrence count");
    } finally {
      bubble.dispose();
    }
  });

  test("generation ABA cannot defeat freshness and the store never rolls backward", async () => {
    const { client, requests } = deferredProbeClient();
    const { store, capture, bubble } = await bubbleFixture({ generation: 3, client });
    try {
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
    } finally {
      bubble.dispose();
    }
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
