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
  resolveProbe,
} from "./bubble.helpers";
import { SHORT_VERDICT } from "../../bubble/renderParts";
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
      // The server ignores the cancellation and answers anyway.
      await resolveProbe(requests[0], probe, true);
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
      await resolveProbe(requests[0], probeA);
      const rendered = capture.visible();
      assert.ok(rendered !== undefined, "the in-budget probe must render its bubble");
      assert.match(rendered ?? "", new RegExp(SHORT_VERDICT), "the bubble carries the short duplication verdict");

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
      await resolveProbe(requests[0], probe);
      assert.match(
        capture.visible() ?? "",
        new RegExp(SHORT_VERDICT),
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
