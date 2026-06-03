// Unit: the trailing-edge debounce that collapses a burst of UI-thread work into
// a single invocation ([VSIX-PERF]). Coalescing is asserted deterministically
// through an injected scheduler — no wall-clock waits.

import * as assert from "node:assert/strict";
import { debounce, ScheduleFn } from "../../util/debounce";

suite("debounce util (VSIX-PERF)", () => {
  test("collapses a burst of calls into a single trailing invocation", () => {
    const armed: { callback: (() => void) | null } = { callback: null };
    let scheduleCount = 0;
    let cancelCount = 0;
    const schedule: ScheduleFn = (callback) => {
      scheduleCount += 1;
      armed.callback = callback;
      return () => {
        cancelCount += 1;
        armed.callback = null;
      };
    };
    let runs = 0;
    const flush = debounce(() => {
      runs += 1;
    }, 50, schedule);

    flush();
    flush();
    flush();
    assert.equal(runs, 0, "nothing runs until the trailing timer fires");
    assert.equal(scheduleCount, 3, "each call re-arms the trailing timer");
    assert.equal(cancelCount, 2, "each re-arm cancels the previously armed timer");
    assert.ok(armed.callback, "exactly one trailing callback is armed after the burst");

    armed.callback?.();
    assert.equal(runs, 1, "the burst collapses to exactly one invocation");
  });

  test("cancel disarms a pending flush so the function never runs", () => {
    const armed: { callback: (() => void) | null } = { callback: null };
    let cancelCount = 0;
    const schedule: ScheduleFn = (callback) => {
      armed.callback = callback;
      return () => {
        cancelCount += 1;
        armed.callback = null;
      };
    };
    let runs = 0;
    const flush = debounce(() => {
      runs += 1;
    }, 50, schedule);

    flush();
    flush.cancel();
    assert.equal(cancelCount, 1, "cancel releases the armed timer");
    assert.equal(armed.callback, null, "no callback remains pending after cancel");
    flush.cancel(); // idempotent — a second cancel with nothing armed is a no-op
    assert.equal(runs, 0, "a cancelled flush never invokes the function");
  });

  test("the default scheduler arms a real timer that cancel clears", () => {
    // Exercises the production setTimeout/clearTimeout path deterministically: a
    // long-delay timer cannot fire within the synchronous test, and cancel()
    // clears it, so the function never runs regardless of wall-clock timing.
    let runs = 0;
    const flush = debounce(() => {
      runs += 1;
    }, 60_000);

    flush();
    flush();
    assert.equal(runs, 0, "the trailing flush is deferred, never synchronous");
    flush.cancel();
    assert.equal(runs, 0, "cancel clears the real timer before it can fire");
  });
});
