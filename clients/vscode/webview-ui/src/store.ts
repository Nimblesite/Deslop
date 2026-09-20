// Centralised Preact Signals store for every Deslop webview.
// Per [VSIX-STATE] + [VSIX-WEBVIEW-REACTIVITY]: one store, no parallel caches,
// no stale UI. The extension process posts messages; this is the only writer.

import { signal, computed, batch } from "@preact/signals";
import {
  applyFacetFilter,
  type AnalysisState,
  type ClusterKind,
  type FacetFilter,
  type Report,
  type ReportCluster,
  type ReportOccurrence,
  type Severity,
  clusterSeverity,
} from "../../src/types/report";

// [FACET-MODEL] Filters select reported severity, category and source path.
export type Filters = {
  severity: Severity | null;
  kind: ClusterKind | null;
  pathGlob: string;
};

export const EMPTY_FILTERS: Filters = {
  severity: null,
  kind: null,
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


// [SEVERITY-MODEL] Read diagnostic levels from the host's report projection.
export const severityByClusterId = computed<Map<string, Severity>>(() => {
  const out = new Map<string, Severity>();
  for (const cluster of clusters.value) out.set(cluster.id, clusterSeverity(cluster));
  return out;
});

export const selectedCluster = computed<ReportCluster | null>(() => {
  const id = selectedClusterId.value;
  if (!id) return null;
  return clusters.value.find((c) => c.id === id) ?? null;
});

// [VSIX-PAIR-COMPARE] The occurrence row one tap picked, waiting for a second
// tap to name the other endpoint. Cleared when the selected cluster changes.
export const pickedOccurrence = signal<ReportOccurrence | null>(null);
const COMPARE_PAIR_MESSAGE = "compare/pair";

function sameOccurrence(left: ReportOccurrence, right: ReportOccurrence): boolean {
  return left.path === right.path && left.start_byte === right.start_byte && left.end_byte === right.end_byte;
}

/** Whether this row is the one a tap picked. */
export function isPicked(occurrence: ReportOccurrence): boolean {
  const picked = pickedOccurrence.value;
  return picked !== null && sameOccurrence(picked, occurrence);
}

// [VSIX-PAIR-COMPARE] One tap picks a row. A second tap on another row of the
// same cluster posts both endpoints, first tap on the left, and clears the
// pick; tapping the picked row again unpicks it. A pick that is no longer a
// member of the cluster is replaced, never compared.
export function tapOccurrenceRow(cluster: ReportCluster, occurrence: ReportOccurrence): void {
  const picked = pickedOccurrence.value;
  const pickedIsMember = picked !== null && cluster.occurrences.some((member) => sameOccurrence(member, picked));
  if (picked === null || !pickedIsMember) {
    pickedOccurrence.value = occurrence;
    return;
  }
  if (sameOccurrence(picked, occurrence)) {
    pickedOccurrence.value = null;
    return;
  }
  post({ kind: COMPARE_PAIR_MESSAGE, left: picked, right: occurrence });
  pickedOccurrence.value = null;
}

export const filteredClusters = computed<ReportCluster[]>(() => {
  const { severity, kind, pathGlob } = filters.value;
  const byId = severityByClusterId.value;
  const glob = pathGlob.trim().toLowerCase();
  // Base slice: the workspace facet filter, shared with the tree and
  // status bar; the webview's own selects refine it below.
  return applyFacetFilter(clusters.value, facetFilter.value).filter((cluster) => {
    if (severity && byId.get(cluster.id) !== severity) return false;
    if (kind && cluster.kind !== kind) return false;
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
        pickedOccurrence.value = null;
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
