import { mkdirSync, writeFileSync } from "node:fs";
import { resolve } from "node:path";
import { spawnSync } from "node:child_process";

const label = process.argv[2] ?? "run";
const repetitions = process.argv[3] ?? "5";
const baselineLabel = process.argv[4];
const artifactDirectory = resolve("target/perf-artifacts");
const artifact = resolve(artifactDirectory, `cluster-signals-${label}.json`);
const baseline = baselineLabel
  ? resolve(artifactDirectory, `cluster-signals-${baselineLabel}.json`)
  : "";

mkdirSync(artifactDirectory, { recursive: true });
const result = spawnSync(
  "cargo",
  [
    "bench",
    "--quiet",
    "-p",
    "deslop-core",
    "--bench",
    "cluster_signals",
    "--features",
    "benchmark",
    "--",
    label,
    repetitions,
    baseline,
  ],
  { encoding: "utf8", maxBuffer: 20 * 1024 * 1024 },
);

if (result.status !== 0) {
  process.stderr.write(result.stderr);
  process.exit(result.status ?? 1);
}

JSON.parse(result.stdout);
writeFileSync(artifact, result.stdout);
process.stdout.write(result.stdout);
