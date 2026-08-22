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

const CARGO_ARGS = [
  "test", "--release", "--workspace", "--all-targets",
  "--features", "deslop-core/live", "--no-run", "--message-format=json",
];

/** Every release test binary cargo builds, in a deterministic order. */
function testBinaries() {
  const stdout = execFileSync("cargo", CARGO_ARGS, {
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
  return { list: argv.includes("--list"), shard: read("--shard"), shards: read("--of") };
}

/** Enumerates or runs one shard. Only reached on direct invocation, so
 *  the contract test can import [`slice`] without cargo running. */
function main() {
  const { list, shard, shards } = parse(argv.slice(2));
  const binaries = testBinaries();
  if (list) {
    for (const binary of binaries) console.log(binary);
    return;
  }
  if (!Number.isInteger(shard) || !Number.isInteger(shards) || shard < 1 || shard > shards) {
    console.error("usage: test-shards.mjs --shard <1-based> --of <count> | --list");
    exit(2);
  }
  const mine = slice(binaries, shard, shards);
  console.log(`shard ${shard}/${shards}: ${mine.length} of ${binaries.length} binaries`);
  // Fail-fast ([TEST-RULES]): the first failing binary ends the shard.
  for (const binary of mine) {
    console.log(`==> ${basename(binary)}`);
    execFileSync(binary, { stdio: "inherit" });
  }
}

if (argv[1] && resolve(argv[1]) === resolve(fileURLToPath(import.meta.url))) {
  main();
}
