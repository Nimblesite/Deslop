import { mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { spawnSync } from "node:child_process";

const repos = [
  ["dart-path", "https://github.com/dart-lang/path.git"],
  ["dart-collection", "https://github.com/dart-lang/collection.git"],
];

const work = mkdtempSync(join(tmpdir(), "deslop-dart-real-"));

try {
  run("cargo", ["build", "-p", "deslop"]);
  for (const [name, url] of repos) {
    benchmarkRepo(name, url);
  }
} finally {
  rmSync(work, { recursive: true, force: true });
}

function benchmarkRepo(name, url) {
  const repo = join(work, name);
  run("git", ["clone", "--depth", "1", url, repo]);
  const out = join(work, `${name}-report`);
  run(resolve("target/debug/deslop"), [
    repo,
    "--output",
    out,
    "--notext",
    "--nohtml",
  ]);
  const report = JSON.parse(readFileSync(`${out}.json`, "utf8"));
  assertValidMetrics(name, report);
  assertNoGeneratedClusters(name, repo, report);
  console.log(
    `${name}: files=${report.files_analysed} loc=${report.metrics.analysed_loc} clusters=${report.clusters.length} hidden=${report.clusters_hidden}`,
  );
}

function assertValidMetrics(name, report) {
  if (report.files_analysed <= 0) throw new Error(`${name}: no Dart files analysed`);
  if (report.metrics.analysed_loc <= 0) throw new Error(`${name}: analysed_loc was zero`);
  if (!Array.isArray(report.clusters)) throw new Error(`${name}: clusters missing`);
}

function assertNoGeneratedClusters(name, repo, report) {
  for (const cluster of report.clusters) {
    const generated = cluster.occurrences.filter((occ) => isGenerated(repo, occ.path));
    if (generated.length === cluster.occurrences.length) {
      throw new Error(`${name}: generated-file cluster surfaced: ${cluster.summary}`);
    }
  }
}

function isGenerated(repo, relativePath) {
  if (!relativePath.endsWith(".dart")) return false;
  if (generatedSuffix(relativePath)) return true;
  return generatedHeader(readHead(join(repo, relativePath)));
}

function generatedSuffix(path) {
  return (
    path.endsWith(".g.dart") ||
    path.endsWith(".freezed.dart") ||
    path.endsWith(".gr.dart") ||
    path.endsWith("_bindings.dart")
  );
}

function generatedHeader(head) {
  const lower = head.toLowerCase();
  return lower.includes("generated") && lower.includes("do not edit");
}

function readHead(path) {
  try {
    return readFileSync(path, "utf8").slice(0, 1024);
  } catch {
    return "";
  }
}

function run(command, args) {
  const result = spawnSync(command, args, {
    encoding: "utf8",
    maxBuffer: 20 * 1024 * 1024,
    stdio: "pipe",
    timeout: 180_000,
  });
  if (result.status === 0) return;
  const rendered = [result.stdout, result.stderr].filter(Boolean).join("\n");
  throw new Error(`${command} ${args.join(" ")} failed\n${rendered}`);
}
