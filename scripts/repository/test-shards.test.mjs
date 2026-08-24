// [CI-RELEASE-BUILD] [TEST-SELECTION] The shard partition must lose
// nothing. A binary placed in no shard never runs, and a suite that never
// runs reports green — the gh #412 failure mode, one layer out.
import { strict as assert } from "node:assert";
import test from "node:test";

const { slice } = await import("./test-shards.mjs");

/** Stand-in binary paths; the partition never reads their contents. */
const BINARIES = Array.from({ length: 200 }, (_, index) => `/t/bin-${index}`);

/** The shard counts CI is allowed to use. */
const SHARD_COUNTS = [1, 2, 3, 4, 6, 8];

test("every binary lands in exactly one shard, for every shard count", () => {
  for (const shards of SHARD_COUNTS) {
    const placed = [];
    for (let shard = 1; shard <= shards; shard += 1) {
      placed.push(...slice(BINARIES, shard, shards));
    }
    assert.equal(
      placed.length,
      BINARIES.length,
      `shards=${shards}: ${BINARIES.length} binaries in, ${placed.length} out — ` +
        "a binary in no shard never runs and the suite reports green without it",
    );
    assert.deepEqual(
      [...placed].sort(),
      [...BINARIES].sort(),
      `shards=${shards}: the union of the shards must be the whole set, with no duplicates`,
    );
  }
});

test("shards stay balanced to within one binary", () => {
  for (const shards of SHARD_COUNTS) {
    const sizes = Array.from({ length: shards }, (_, index) => slice(BINARIES, index + 1, shards).length);
    assert.ok(
      Math.max(...sizes) - Math.min(...sizes) <= 1,
      `shards=${shards}: sizes ${sizes} differ by more than one, so one runner carries the suite`,
    );
  }
});

test("a single shard is the whole suite, so N=1 is always a valid fallback", () => {
  assert.deepEqual(slice(BINARIES, 1, 1), BINARIES);
});
