// Publishes the action's result outputs from the rendered JSON report.
// [ACTION-GATE].
//
// Usage: node scripts/action-read-outputs.mjs <reportPrefix> <exitCode> [onlyChanged]

import { existsSync, readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { writeOutputs } from "./action-github-output.mjs";

// Exit 0 (clean) and exit 3 (threshold breached) both render a full report.
// Exit 1 (runtime error) and exit 2 (usage error) abort before rendering.
// Owned by docs/specs/pipeline.md [EXIT-CODES].
const RENDERS_REPORT = new Set([0, 3]);

/**
 * Names the population the gated percentage is measured over.
 *
 * `--only-changed` reroutes the mechanical gate to `metrics.diff` — duplicated
 * added lines over added lines ([METRICS-DIFF-SCOPE]) — so the breach message
 * must say "added lines" rather than claim the repo-wide figure failed. The
 * report itself records that rerouting: the CLI stamps
 * `metrics.diff.threshold.source` with the resolved source (`cli`/`config`)
 * only when the diff gate governed, and leaves `none` otherwise — so a live
 * source names the diff scope even if the input echo is lost. `--diff` alone
 * never moves the gate, so tagging a run must not rename its scope either.
 *
 * @param {{duplication_percent?: number, threshold?: {percent?: number, source?: string}} | undefined} diffMetrics
 *   `metrics.diff`, absent without `--diff`
 * @param {boolean} onlyChanged whether the run passed `--only-changed`
 * @param {string} jsonPath the report the metrics were read from, for the error
 * @returns {{scope: string, percent: string, ceiling: string} | undefined}
 */
function diffGate(diffMetrics, onlyChanged, jsonPath) {
  const source = diffMetrics?.threshold?.source;
  const reroutedByReport = typeof source === "string" && source !== "none";
  if (!onlyChanged && !reroutedByReport) return undefined;
  if (typeof diffMetrics?.duplication_percent !== "number") {
    throw new Error(
      `only-changed gated this run but ${jsonPath} carries no metrics.diff duplication_percent, ` +
        "so the percentage the gate measured cannot be named",
    );
  }
  return {
    scope: "added-lines",
    percent: String(diffMetrics.duplication_percent),
    ceiling: String(diffMetrics.threshold?.percent ?? 0),
  };
}

/**
 * Reads the JSON report and returns the outputs to publish.
 *
 * @param {string} reportPrefix the `--output` prefix, without extension
 * @param {number} exitCode the CLI's exit status
 * @param {boolean} [onlyChanged] whether the run passed `--only-changed`
 * @returns {Record<string, string>}
 */
export function readOutputs(reportPrefix, exitCode, onlyChanged = false) {
  const jsonPath = `${reportPrefix}.json`;
  if (!existsSync(jsonPath)) {
    if (RENDERS_REPORT.has(exitCode)) {
      throw new Error(`deslop exited ${exitCode} but rendered no JSON report at ${jsonPath}`);
    }
    return { "exit-code": String(exitCode) };
  }
  const metrics = JSON.parse(readFileSync(jsonPath, "utf8")).metrics ?? {};
  const duplicationPercent = String(metrics.duplication_percent ?? 0);
  const thresholdPercent = String(metrics.threshold?.percent ?? 0);
  const gate = diffGate(metrics.diff, onlyChanged, jsonPath) ?? {
    scope: "repository",
    percent: duplicationPercent,
    ceiling: thresholdPercent,
  };
  return {
    "exit-code": String(exitCode),
    "duplication-percent": duplicationPercent,
    "cluster-count": String(metrics.clusters_total ?? 0),
    "threshold-percent": thresholdPercent,
    "gate-scope": gate.scope,
    "gate-percent": gate.percent,
    "gate-threshold-percent": gate.ceiling,
    "report-json": jsonPath,
    "report-text": `${reportPrefix}.txt`,
    "report-html": `${reportPrefix}.html`,
  };
}

function main(argv) {
  const [reportPrefix, exitCode, onlyChanged] = argv;
  if (!reportPrefix || exitCode === undefined) {
    throw new Error("usage: action-read-outputs.mjs <reportPrefix> <exitCode> [onlyChanged]");
  }
  const outputs = readOutputs(reportPrefix, Number.parseInt(exitCode, 10), onlyChanged === "true");
  writeOutputs(outputs);
  console.log(
    `deslop measured ${outputs["duplication-percent"] ?? "no"} percent duplication ` +
      `across ${outputs["cluster-count"] ?? "0"} clusters`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
