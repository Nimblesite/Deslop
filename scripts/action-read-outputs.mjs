// Publishes the action's result outputs from the rendered JSON report.
// [ACTION-GATE].
//
// Usage: node scripts/action-read-outputs.mjs <reportPrefix> <exitCode>

import { existsSync, readFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { writeOutputs } from "./action-github-output.mjs";

// Exit 0 (clean) and exit 3 (threshold breached) both render a full report.
// Exit 1 (runtime error) and exit 2 (usage error) abort before rendering.
// Owned by docs/specs/pipeline.md [EXIT-CODES].
const RENDERS_REPORT = new Set([0, 3]);

/**
 * Reads the JSON report and returns the outputs to publish.
 *
 * @param {string} reportPrefix the `--output` prefix, without extension
 * @param {number} exitCode the CLI's exit status
 * @returns {Record<string, string>}
 */
export function readOutputs(reportPrefix, exitCode) {
  const jsonPath = `${reportPrefix}.json`;
  if (!existsSync(jsonPath)) {
    if (RENDERS_REPORT.has(exitCode)) {
      throw new Error(`deslop exited ${exitCode} but rendered no JSON report at ${jsonPath}`);
    }
    return { "exit-code": String(exitCode) };
  }
  const metrics = JSON.parse(readFileSync(jsonPath, "utf8")).metrics ?? {};
  return {
    "exit-code": String(exitCode),
    "duplication-percent": String(metrics.duplication_percent ?? 0),
    "cluster-count": String(metrics.clusters_total ?? 0),
    "threshold-percent": String(metrics.threshold?.percent ?? 0),
    "report-json": jsonPath,
    "report-text": `${reportPrefix}.txt`,
    "report-html": `${reportPrefix}.html`,
  };
}

function main(argv) {
  const [reportPrefix, exitCode] = argv;
  if (!reportPrefix || exitCode === undefined) {
    throw new Error("usage: action-read-outputs.mjs <reportPrefix> <exitCode>");
  }
  const outputs = readOutputs(reportPrefix, Number.parseInt(exitCode, 10));
  writeOutputs(outputs);
  console.log(
    `deslop measured ${outputs["duplication-percent"] ?? "no"} percent duplication ` +
      `across ${outputs["cluster-count"] ?? "0"} clusters`,
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main(process.argv.slice(2));
}
