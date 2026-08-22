#!/usr/bin/env node
// [CI-RELEASE-BUILD] Partitions the workspace's release test binaries so
// CI can run them in parallel.
//
// The suite's cost is its run phase. One runner executes the binaries
// serially; N runners each execute their own slice. [TEST-ONE-BINARY]
// collapsed 200 binaries into 13 — most of the 2924s that motivated the
// split was per-process startup, not test work, and libtest parallelises
// within a binary — so the slices are coarse and unbalanced by
// construction. Correctness of the partition is what matters here; the
// balance is bounded by the largest single binary either way.
//
// [TEST-SELECTION] The partition is over *binaries*, never test names.
// `cargo test --skip` matches a substring of the test name and silently
// dropped whole suites that way (gh #412). Cargo reports the binaries as
// structured JSON, every one is placed, and `--list` prints the whole set
// so the union is checkable.
import { execFileSync } from "node:child_process";
import { basename, resolve } from "node:path";
import { argv, exit } from "node:process";
import { fileURLToPath } from "node:url";

/** Cargo's arguments for enumerating the shardable binaries.
 *
 *  [TEST-SELECTION] The feature set is passed in rather than repeated
 *  here. A second copy drifts, and a feature this script does not enable
 *  is a test binary cargo never builds — the shard then reports green
 *  over a partition that is missing a target rather than failing. The
 *  Makefile's `_TEST_FEATURES` is the one definition. */
function cargoArgs(features) {
  return [
    "test", "--release", "--workspace", "--all-targets",
    "--features", features, "--no-run", "--message-format=json",
  ];
}

/** Every release test binary cargo builds, in a deterministic order. */
function testBinaries(features) {
  const stdout = execFileSync("cargo", cargoArgs(features), {
    encoding: "utf8",
    maxBuffer: 512 * 1024 * 1024,
    stdio: ["ignore", "pipe", "inherit"],
  });
  const executables = stdout
    .split("\n")
    .flatMap((line) => {
      if (!line.startsWith("{")) return [];
      const message = JSON.parse(line);
      return message.profile?.test && message.executable ? [message.executable] : [];
    });
  return [...new Set(executables)].sort();
}

/** The binaries belonging to `shard` of `shards`, dealt round-robin. */
export function slice(binaries, shard, shards) {
  return binaries.filter((_, index) => index % shards === shard - 1);
}

function parse(argv) {
  const read = (flag) => {
    const at = argv.indexOf(flag);
    return at === -1 ? undefined : Number(argv[at + 1]);
  };
  const readText = (flag) => {
    const at = argv.indexOf(flag);
    return at === -1 ? undefined : argv[at + 1];
  };
  return {
    list: argv.includes("--list"),
    shard: read("--shard"),
    shards: read("--of"),
    features: readText("--features"),
  };
}

/** Enumerates or runs one shard. Only reached on direct invocation, so
 *  the contract test can import [`slice`] without cargo running. */
function main() {
  const { list, shard, shards, features } = parse(argv.slice(2));
  if (!features) {
    console.error(
      "test-shards.mjs: --features is required. Pass the Makefile's " +
        "_TEST_FEATURES; guessing it here is how a test binary stops " +
        "being built without the shard noticing.",
    );
    exit(2);
  }
  const binaries = testBinaries(features);
  if (list) {
    for (const binary of binaries) console.log(binary);
    return;
  }
  if (!Number.isInteger(shard) || !Number.isInteger(shards) || shard < 1 || shard > shards) {
    console.error(
      "usage: test-shards.mjs --features <set> --shard <1-based> --of <count> | --list",
    );
    exit(2);
  }
  const mine = slice(binaries, shard, shards);
  announce(shard, shards, mine, binaries.length);
  const elapsed = [];
  // Fail-fast ([TEST-RULES]): the first failing binary ends the shard.
  for (const binary of mine) {
    console.log(`==> ${basename(binary)}`);
    const started = Date.now();
    execFileSync(binary, { stdio: "inherit" });
    elapsed.push([basename(binary), Date.now() - started]);
  }
  summarise(shard, shards, elapsed);
}

/** Prints the shard's manifest before a single test runs.
 *
 *  A shard that times out prints nothing about what it was carrying, so
 *  a run that dies at the cap gives no way to tell an imbalanced slice
 *  from a genuinely slow test. Naming the binaries up front means the
 *  log answers that even when the job is killed mid-flight. */
function announce(shard, shards, mine, total) {
  console.log(`shard ${shard}/${shards}: ${mine.length} of ${total} binaries`);
  for (const binary of mine) console.log(`    ${basename(binary)}`);
}

/** Prints each binary's wall time, slowest first, plus the shard total.
 *
 *  Rebalancing wants measured runtime, not a count: the slices are dealt
 *  round-robin over binaries of very different size, so a count says
 *  nothing about which shard sets the wall clock. */
function summarise(shard, shards, elapsed) {
  const total = elapsed.reduce((sum, [, ms]) => sum + ms, 0);
  console.log(`shard ${shard}/${shards} timings, slowest first:`);
  for (const [name, ms] of [...elapsed].sort(([, a], [, b]) => b - a)) {
    console.log(`    ${(ms / 1000).toFixed(1)}s  ${name}`);
  }
  console.log(`shard ${shard}/${shards} total: ${(total / 1000).toFixed(1)}s`);
}

if (argv[1] && resolve(argv[1]) === resolve(fileURLToPath(import.meta.url))) {
  main();
}
