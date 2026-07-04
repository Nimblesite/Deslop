// Centralised Preact Signals store for every Deslop webview.
// Per [VSIX-STATE] + [VSIX-WEBVIEW-REACTIVITY]: one store, no parallel caches,
// no stale UI. The extension process posts messages; this is the only writer.

import { signal, computed, batch } from "@preact/signals";
import { severityOf } from "../../src/types/report";
import type {
  AnalysisState,
  Report,
  ReportCluster,
  Severity,
} from "../../src/types/report";

export type Filters = {
  language: string | null;
  severity: Severity | null;
  pathGlob: string;
};

export const report = signal<Report | null>(null);
export const selectedClusterId = signal<string | null>(null);
export const analysisState = signal<AnalysisState>({ state: "idle" });
export const filters = signal<Filters>({ language: null, severity: null, pathGlob: "" });
export const lastUpdatedAt = signal<number>(0);

export const clusters = computed<ReportCluster[]>(() => report.value?.clusters ?? []);

export const severityByClusterId = computed<Map<string, Severity>>(() => {
  const list = clusters.value;
  const total = list.length;
  const out = new Map<string, Severity>();
  list.forEach((cluster, i) => {
    const rank = i + 1;
    const pct = total <= 1 ? 0 : 1 - (rank - 1) / (total - 1);
    out.set(cluster.id, severityOf(pct));
  });
  return out;
});

export const selectedCluster = computed<ReportCluster | null>(() => {
  const id = selectedClusterId.value;
  if (!id) return null;
  return clusters.value.find((c) => c.id === id) ?? null;
});

export const filteredClusters = computed<ReportCluster[]>(() => {
  const { language, severity, pathGlob } = filters.value;
  const byId = severityByClusterId.value;
  const glob = pathGlob.trim().toLowerCase();
  return clusters.value.filter((cluster) => {
    if (language && !cluster.occurrences.some((o) => o.path.toLowerCase().endsWith(language))) {
      return false;
    }
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
  | { kind: "filter/set"; filters: Filters };

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
