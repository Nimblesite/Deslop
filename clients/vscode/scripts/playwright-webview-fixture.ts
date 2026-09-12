// The smoke suite's fixture report and page bootstrap ([VSIX-WEBVIEW-COVERAGE]).
// Split out of `playwright-webview-smoke.spec.ts` so the spec stays a list of
// user journeys: the data a webview is fed, and the page it is fed into, are
// not assertions and do not belong beside them.

import fs from "node:fs";
import path from "node:path";

import type { Report, ReportCluster } from "../src/types/report";
import { FIXTURE_ROUTING } from "../src/test/cluster.helpers";

/** The three webview bundles the smoke drives. */
export type ViewKind = "cluster" | "duplication" | "report";

interface PostedMessage {
  readonly kind?: string;
  readonly clusterId?: string;
  readonly occurrence?: { readonly path?: string; readonly start_byte?: number; readonly end_byte?: number };
}

declare global {
  interface Window {
    __deslopPosts?: PostedMessage[];
    acquireVsCodeApi?: () => { postMessage: (data: PostedMessage) => void };
  }
}

const repoRoot = findRepoRoot(process.cwd());
const webviewDir = path.join(repoRoot, "clients", "vscode", "media", "webview");
export const screenshotDir = path.join(repoRoot, "target", "playwright-webview");

/** The canonical occurrence of a cluster is its first ([CLONE-KIND-FOLD]). */
export const CANONICAL_OCCURRENCE_INDEX = 0;
/** The member immediately after the canonical one. */
export const FIRST_PEER_INDEX = 1;

/**
 * The visible projection the store computes while the named cluster's canonical
 * occurrence sits in an unsaved buffer ([VSIX-STATE-DIRTY]): that occurrence is
 * elided and the counts become counts of the shorter view. Staged here rather
 * than imported because `projectVisible` lives behind the `vscode` module.
 */
export function withCanonicalUnsaved(report: Report, clusterIndex: number): Report {
  return {
    ...report,
    clusters: report.clusters.map((cluster, index) =>
      index === clusterIndex ? withoutCanonical(cluster) : cluster,
    ),
  };
}

function withoutCanonical(cluster: ReportCluster): ReportCluster {
  const kept = cluster.occurrences.slice(FIRST_PEER_INDEX);
  return {
    ...cluster,
    occurrences: kept,
    occurrence_count: kept.length,
    occurrences_total: kept.length,
  };
}

export function webviewHtml(kind: ViewKind): string {
  const bundle = fs
    .readFileSync(path.join(webviewDir, `${kind}.js`), "utf8")
    .replaceAll("</script", "<\\/script");
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Deslop ${kind}</title>
    <style>body { margin: 0; }</style>
    <script>
      window.__deslopPosts = [];
      window.acquireVsCodeApi = function () {
        return {
          postMessage: function (data) {
            window.__deslopPosts.push(data);
          }
        };
      };
    </script>
  </head>
  <body>
    <div id="root"></div>
    <script type="module">${bundle}</script>
  </body>
</html>`;
}

function findRepoRoot(startDir: string): string {
  let current = startDir;
  while (true) {
    const marker = path.join(current, "clients", "vscode", "media", "webview", "report.js");
    if (fs.existsSync(marker)) return current;
    const parent = path.dirname(current);
    if (parent === current) {
      throw new Error(`Could not find repo root from ${startDir}`);
    }
    current = parent;
  }
}

export const sampleReport = {
  tool_version: "playwright-smoke",
  min_nodes: 5,
  files_analysed: 4,
  clusters_hidden: 0,
  cache_stats: { hits: 7, misses: 2 },
  routing: FIXTURE_ROUTING,
  metrics: {
    analysed_loc: 520,
    duplicated_loc: 96,
    duplication_percent: 18.4,
    clusters_total: 3,
    duplicated_files: 3,
    threshold: { percent: 15, breached: true, source: "config" },
    per_file: [
      { path: "src/dart/alpha.dart", analysed_loc: 120, duplicated_loc: 42, duplication_percent: 35 },
      { path: "src/dart/parser_beta.dart", analysed_loc: 180, duplicated_loc: 38, duplication_percent: 21.1 },
      { path: "src/models/models.g.dart", analysed_loc: 220, duplicated_loc: 16, duplication_percent: 7.3 },
    ],
    // Engine-computed folder rows ([METRICS-REPO]) — the webview renders
    // these verbatim and performs no arithmetic of its own.
    folders: [
      { path: "src/dart", analysed_loc: 300, duplicated_loc: 80, duplication_percent: 26.7 },
      { path: "src", analysed_loc: 520, duplicated_loc: 96, duplication_percent: 18.5 },
      { path: "src/models", analysed_loc: 220, duplicated_loc: 16, duplication_percent: 7.3 },
    ],
  },
  schema_doc: "playwright smoke schema",
  action_hints: [],
  boilerplate_hints: [],
  embedding_provenance: {
    provider_id: "ollama",
    model_id: "nomic-embed-text",
    model_version: "smoke",
    dimensions: 768,
    attempted_subtrees: 12,
    succeeded_subtrees: 12,
    indexed_subtrees: 12,
    failed_subtrees: 0,
  },
  clusters: [
    {
      id: "abcdef1234567890",
      rank: 1,
      severity: "warning",
      kind: "identical",
      mass: 43,
      canonical_node_count: 18,
      occurrences_total: 2,
      occurrence_count: 2,
      occurrences_truncated: false,
      occurrences: [
        occurrence("src/dart/alpha.dart", 120, 248, 12, 3),
        occurrence("src/dart/beta.dart", 420, 558, 31, 5),
      ],
    },
    {
      id: "bcdefa2345678901",
      rank: 2,
      severity: "warning",
      kind: "nearly_identical",
      mass: 27,
      canonical_node_count: 14,
      occurrences_total: 3,
      occurrence_count: 3,
      occurrences_truncated: false,
      occurrences: [
        occurrence("src/dart/parser_alpha.dart", 210, 330, 44, 7),
        occurrence("src/dart/parser_beta.dart", 610, 742, 88, 9),
        occurrence("src/dart/parser_gamma.dart", 1000, 1130, 122, 11),
      ],
    },
    {
      id: "cdefab3456789012",
      rank: 0,
      severity: "none",
      kind: "structural_only",
      mass: 0,
      canonical_node_count: 9,
      occurrences_total: 2,
      occurrence_count: 2,
      occurrences_truncated: false,
      occurrences: [
        occurrence("src/models/models.g.dart", 80, 160, 15, 1),
        occurrence("src/models/serializers.g.dart", 180, 260, 27, 1, true),
      ],
    },
  ],
} satisfies Report;

export const reportWithoutSignalSource: Report = {
  ...sampleReport,
  clusters: sampleReport.clusters.map((cluster, index) =>
    index === 0 ? cluster : cluster),
};

function occurrence(
  filePath: string,
  startByte: number,
  endByte: number,
  line: number,
  column: number,
  hidden = false,
): object {
  return {
    path: filePath,
    start_byte: startByte,
    end_byte: endByte,
    start_line: line,
    end_line: line + 4,
    hidden,
    displayLocation: {
      line,
      column,
      label: `${filePath}:${line}:${column}`,
      description: `line ${line}, column ${column}`,
      commandTitle: "Open occurrence",
    },
  };
}
