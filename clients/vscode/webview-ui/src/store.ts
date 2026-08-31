// Centralised Preact Signals store for every Deslop webview.
// Per [VSIX-STATE] + [VSIX-WEBVIEW-REACTIVITY]: one store, no parallel caches,
// no stale UI. The extension process posts messages; this is the only writer.

import { signal, computed, batch } from "@preact/signals";
import {
  applyFacetFilter,
  type AnalysisState,
  type FacetFilter,
  type Report,
  type ReportCluster,
  type Severity,
  clusterBand,
} from "../../src/types/report";

// [SEVERITY-CONFIG] Filters are the mass severity band and a path glob
// only. The language, bucket, and category axes are retired: the wire
// carries no similarity classification or parser stamp on the cluster,
// and a webview must never re-derive an axis the engine stopped sending.
export type Filters = {
  severity: Severity | null;
  pathGlob: string;
};

export const EMPTY_FILTERS: Filters = {
  severity: null,
  pathGlob: "",
};

export const report = signal<Report | null>(null);
export const selectedClusterId = signal<string | null>(null);
export const analysisState = signal<AnalysisState>({ state: "idle" });
export const filters = signal<Filters>(EMPTY_FILTERS);
// [FACET-TOP-OFFENDERS-FILTER] Workspace facet filter pushed by the
// extension host so this list agrees with the filtered tree.
export const facetFilter = signal<FacetFilter>({ severities: [] });
export const lastUpdatedAt = signal<number>(0);

export const clusters = computed<ReportCluster[]>(() => report.value?.clusters ?? []);

// [SEVERITY-BAND] The band is the engine's: it classifies the cluster's
// rank percentile, which is a calculation, so it is computed once in
// `report_weight::rank_band` and carried on the wire. This panel used to
// re-derive it from array position, which rebanded every cluster the
// moment the list it saw was filtered or projected.
export const severityByClusterId = computed<Map<string, Severity>>(() => {
  const out = new Map<string, Severity>();
  for (const cluster of clusters.value) out.set(cluster.id, clusterBand(cluster));
  return out;
});

export const selectedCluster = computed<ReportCluster | null>(() => {
  const id = selectedClusterId.value;
  if (!id) return null;
  return clusters.value.find((c) => c.id === id) ?? null;
});

export const filteredClusters = computed<ReportCluster[]>(() => {
  const { severity, pathGlob } = filters.value;
  const byId = severityByClusterId.value;
  const glob = pathGlob.trim().toLowerCase();
  // Base slice: the workspace facet filter, shared with the tree and
  // status bar; the webview's own selects refine it below.
  return applyFacetFilter(clusters.value, facetFilter.value).filter((cluster) => {
    if (severity && byId.get(cluster.id) !== severity) return false;
    if (glob && !cluster.occurrences.some((o) => o.path.toLowerCase().includes(glob))) {
      return false;
    }
    return true;
  });
});

// [VSIX-WEBVIEW-PROTOCOL] Host→webview message schema — the authoritative set
// the webview accepts (docs/specs/webview-runtime.md). The extension host is the
// only legitimate writer; any payload without a string `kind` is ignored.
export type HostMessage =
  | { kind: "report/snapshot"; report: Report }
  | { kind: "report/delta"; report: Report }
  | { kind: "analysis/state"; state: AnalysisState }
  | { kind: "select/cluster"; id: string | null }
  | { kind: "filter/set"; filters: Filters }
  | { kind: "facetFilter/set"; filter: FacetFilter };

// [VSIX-REACTIVITY-WEBVIEW] The sole batched writer of webview signals:
// the host posts messages, this folds them into the signal graph.
export function applyHostMessage(message: HostMessage): void {
  batch(() => {
    switch (message.kind) {
      case "report/snapshot":
      case "report/delta":
        report.value = message.report;
        lastUpdatedAt.value = Date.now();
        break;
      case "analysis/state":
        analysisState.value = message.state;
        break;
      case "select/cluster":
        selectedClusterId.value = message.id;
        break;
      case "filter/set":
        filters.value = message.filters;
        break;
      case "facetFilter/set":
        facetFilter.value = message.filter;
        break;
    }
  });
}

declare global {
  interface Window {
    acquireVsCodeApi?: () => { postMessage: (data: unknown) => void };
  }
}

const vsApi = typeof window !== "undefined" && window.acquireVsCodeApi ? window.acquireVsCodeApi() : null;

export function post(message: unknown): void {
  vsApi?.postMessage(message);
}

export function wireMessagePump(): void {
  window.addEventListener("message", (event) => {
    const data = event.data as HostMessage | undefined;
    if (!data || typeof (data as { kind?: unknown }).kind !== "string") return;
    applyHostMessage(data);
  });
  post({ kind: "ready" });
}
