// [VSIX-LIVE-BUBBLE] RA-05: the 250 ms probe budget is a hard deadline,
// not a cooperative request. `$/cancelRequest` is advisory — a server that
// ignores it still settles the response promise — so the deadline records
// an expired state and the completion is rejected either way. Driven with
// an injected budget scheduler and deferred responses: no timers, no
// sleeps.

import * as assert from "node:assert/strict";
import {
  bubbleFixture,
  deferredProbeClient,
  editAt,
  probeCluster,
} from "./bubble.helpers";
import { BudgetScheduler } from "../../bubble/live";

/** Captures scheduled budget deadlines so tests fire them on demand. */
function manualBudget(): {
  scheduler: BudgetScheduler;
  deadlines: Array<{ expire: () => void; disposed: boolean; ms: number }>;
} {
  const deadlines: Array<{ expire: () => void; disposed: boolean; ms: number }> = [];
  return {
    deadlines,
    scheduler: (expire, ms) => {
      const entry = { expire, disposed: false, ms };
      deadlines.push(entry);
      return {
        dispose: () => {
          entry.disposed = true;
        },
      };
    },
  };
}

suite("LiveBubble probe budget deadline", () => {
  test("a success arriving after the deadline renders nothing, even when cancellation is ignored", async () => {
    const { client, requests } = deferredProbeClient();
    const { scheduler, deadlines } = manualBudget();
    const { capture, bubble } = await bubbleFixture({ generation: 1, client, budget: scheduler });
    try {
      const probe = bubble.probe(capture.editor, editAt(0, "aaaa"));
      assert.equal(requests.length, 1, "the probe must dispatch a findSimilar request");
      assert.equal(deadlines.length, 1, "the probe must arm its budget deadline");
      assert.equal(deadlines[0]?.ms, 250, "the budget is the specified 250 ms");

      deadlines[0]?.expire();
      assert.equal(
        requests[0]?.token.isCancellationRequested,
        true,
        "the deadline still cancels so a compliant server stops working",
      );

      // The server ignores the cancellation and answers anyway.
      requests[0]?.resolve([probeCluster("c-a", 10, 0.95)]);
      await probe;
      assert.equal(
        capture.visible(),
        undefined,
        "a late success must not render — the edit cycle is skipped outright",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("a failure arriving after the deadline leaves the previously rendered bubble intact", async () => {
    const { client, requests } = deferredProbeClient();
    const { scheduler, deadlines } = manualBudget();
    const { capture, bubble } = await bubbleFixture({ generation: 1, client, budget: scheduler });
    try {
      const probeA = bubble.probe(capture.editor, editAt(0, "aaaa"));
      requests[0]?.resolve([probeCluster("c-a", 10, 0.95)]);
      await probeA;
      const rendered = capture.visible();
      assert.ok(rendered !== undefined, "the in-budget probe must render its bubble");
      assert.match(rendered ?? "", /Identical code/, "the bubble carries the bucket title");

      const probeB = bubble.probe(capture.editor, editAt(6, "bbbb"));
      deadlines[1]?.expire();
      requests[1]?.reject(new Error("server failed after the deadline"));
      await probeB;
      assert.equal(
        capture.visible(),
        rendered,
        "an expired probe's failure must not clear the earlier bubble",
      );
    } finally {
      bubble.dispose();
    }
  });

  test("a response inside the budget renders and disposes its deadline", async () => {
    const { client, requests } = deferredProbeClient();
    const { scheduler, deadlines } = manualBudget();
    const { capture, bubble } = await bubbleFixture({ generation: 1, client, budget: scheduler });
    try {
      const probe = bubble.probe(capture.editor, editAt(0, "aaaa"));
      requests[0]?.resolve([probeCluster("c-a", 10, 0.95)]);
      await probe;
      assert.match(
        capture.visible() ?? "",
        /Identical code/,
        "an in-budget response renders normally",
      );
      assert.equal(
        deadlines[0]?.disposed,
        true,
        "a settled probe must disarm its budget deadline",
      );
    } finally {
      bubble.dispose();
    }
  });
});
