import { render } from "preact";
import { useEffect } from "preact/hooks";
import {
  analysisState, clusters, focusedOccurrenceIndex, pickedOccurrence, post,
  selectedCluster, selectedClusterId, severityByClusterId, shortcutHelpExpanded, wireMessagePump,
} from "../store";
import { GLOBAL_CSS, KIND_COLOR } from "../theme";
import { ClusterBadge } from "../components/ClusterBadge";
import { DocTextLink, HelpBubble } from "../components/HelpBubble";
import { clusterSlug, kindTitle, occurrenceCount, isClone, INFORMATIONAL_FINDING } from "../../../src/types/report";
import { formatMass } from "../../../src/types/format";
import type { ReportCluster } from "../../../src/types/report";
import { OccurrenceList } from "./OccurrenceList";

const CLUSTER_ID_TOPIC = "cluster-id";
const OPEN_OCCURRENCE_MESSAGE = "open/occurrence";
const KEYDOWN_EVENT = "keydown";
const FAINT_SEVERITY = "hint";
const UNKNOWN_RANK = "unknown";
const INITIAL_INDEX = 0;
const NEXT_OFFSET = 1;
const PREVIOUS_OFFSET = -1;

// [VSIX-WEBVIEW-ACTIONS-CONTEXT] A compact summary; technical detail is opt-in.
function ClusterApp() {
  const cluster = selectedCluster.value;
  useEffect(() => {
    focusedOccurrenceIndex.value = INITIAL_INDEX;
    pickedOccurrence.value = null;
  }, [cluster?.id]);
  useEffect(() => {
    window.addEventListener(KEYDOWN_EVENT, handleShortcut);
    return () => window.removeEventListener(KEYDOWN_EVENT, handleShortcut);
  }, []);
  if (!cluster) return <main class="cluster-panel"><p>No cluster selected.</p></main>;
  return <main class="cluster-panel" style={{ opacity: analysisState.value.state === "errored" ? 0.5 : 1 }}>
    <ClusterHeader cluster={cluster} />
    <OccurrenceList cluster={cluster} focusedIndex={focusedIndexFor(cluster)} accent={KIND_COLOR[cluster.kind]} />
    <ClusterNavigation /><HotkeyHelp />
  </main>;
}

function ClusterHeader({ cluster }: { cluster: ReportCluster }) {
  const slug = clusterSlug(cluster);
  const severity = severityByClusterId.value.get(cluster.id) ?? FAINT_SEVERITY;
  return <header class="cluster-header">
    <div class="cluster-eyebrow"><span class="label">CLUSTER</span>
      <ClusterBadge kind={cluster.kind} severity={severity} label={`${slug}`} title={clusterIdTitle(cluster)} />
    </div>
    <div class="cluster-heading"><h1>{kindTitle(cluster.kind)}</h1><HelpBubble topic="clone-kind" /></div>
    <p class="cluster-summary">{isClone(cluster) ? `${occurrenceCount(cluster)} locations with repeated code` : INFORMATIONAL_FINDING}</p>
    {isClone(cluster) ? <div class="cluster-weight"><span>Duplicated <span>mass</span> <strong>{formatMass(cluster.mass)}</strong></span><HelpBubble topic="mass" /></div> : null}
    <details class="cluster-details"><summary>Technical details</summary>
      <div>Cluster <DocTextLink topic={CLUSTER_ID_TOPIC} title={clusterIdTitle(cluster)}>{cluster.id}</DocTextLink></div>
      {isClone(cluster) ? <div>Rank {cluster.rank || UNKNOWN_RANK} · Canonical syntax nodes: {cluster.canonical_node_count}</div> : null}
    </details>
  </header>;
}

function ClusterNavigation() {
  const list = clusters.value;
  const rank = list.findIndex((cluster) => cluster.id === selectedClusterId.value) + NEXT_OFFSET;
  return <footer class="cluster-navigation">
    <button onClick={() => selectPreviousCluster(list, rank)} title="Previous cluster (p)" aria-label="Previous cluster">← Previous</button>
    <span class="cluster-position">{rank} of {list.length}</span>
    <button onClick={() => selectNextCluster(list, rank)} title="Next cluster (n)" aria-label="Next cluster">Next →</button>
  </footer>;
}

function HotkeyHelp() {
  return <div class="cluster-shortcuts">
    <button class="text-action" onClick={() => { shortcutHelpExpanded.value = !shortcutHelpExpanded.value; }}
      title="Show or hide keyboard shortcuts" aria-label="Toggle keyboard shortcut help">Keyboard shortcuts</button>
    <HelpBubble topic="keyboard-shortcuts" />
    {shortcutHelpExpanded.value ? <p title="Detailed keyboard help">
      <kbd>j</kbd> / <kbd>k</kbd> next / previous occurrence · <kbd>n</kbd> / <kbd>p</kbd> next / previous cluster · <kbd>Enter</kbd> open focused occurrence · <kbd>?</kbd> toggle help
    </p> : null}
  </div>;
}

function handleShortcut(event: KeyboardEvent): void {
  if (isEditableTarget(event.target)) return;
  const cluster = selectedCluster.value;
  const list = clusters.value;
  const rank = list.findIndex((item) => item.id === cluster?.id) + NEXT_OFFSET;
  const actions: Record<string, () => void> = {
    n: () => selectNextCluster(list, rank), p: () => selectPreviousCluster(list, rank),
    j: () => { if (cluster) moveFocusedOccurrence(cluster, NEXT_OFFSET); },
    k: () => { if (cluster) moveFocusedOccurrence(cluster, PREVIOUS_OFFSET); },
    Enter: () => { if (cluster) openFocusedOccurrence(cluster); },
    "?": () => { shortcutHelpExpanded.value = !shortcutHelpExpanded.value; },
  };
  const action = actions[event.key];
  if (action) { event.preventDefault(); action(); }
}

function selectNextCluster(list: ReportCluster[], rank: number): void {
  selectClusterByOffset(list, rank, NEXT_OFFSET);
}

function selectPreviousCluster(list: ReportCluster[], rank: number): void {
  selectClusterByOffset(list, rank, PREVIOUS_OFFSET);
}

function selectClusterByOffset(list: ReportCluster[], rank: number, offset: number): void {
  if (list.length === INITIAL_INDEX) return;
  const current = rank > INITIAL_INDEX ? rank - NEXT_OFFSET : INITIAL_INDEX;
  const next = (current + offset + list.length) % list.length;
  selectedClusterId.value = list[next]?.id ?? null;
  focusedOccurrenceIndex.value = INITIAL_INDEX;
  pickedOccurrence.value = null;
}

function moveFocusedOccurrence(cluster: ReportCluster, offset: number): void {
  const total = cluster.occurrences.length;
  if (total === INITIAL_INDEX) return;
  focusedOccurrenceIndex.value = (focusedIndexFor(cluster) + offset + total) % total;
}

function openFocusedOccurrence(cluster: ReportCluster): void {
  const occurrence = cluster.occurrences[focusedIndexFor(cluster)] ?? cluster.occurrences[INITIAL_INDEX];
  if (occurrence) post({ kind: OPEN_OCCURRENCE_MESSAGE, occurrence });
}

function focusedIndexFor(cluster: ReportCluster): number {
  const max = Math.max(INITIAL_INDEX, cluster.occurrences.length - NEXT_OFFSET);
  return Math.min(Math.max(INITIAL_INDEX, focusedOccurrenceIndex.value), max);
}

function isEditableTarget(target: EventTarget | null): boolean {
  return target instanceof HTMLElement &&
    ["BUTTON", "A", "INPUT", "SELECT", "TEXTAREA", "SUMMARY"].includes(target.tagName);
}

function clusterIdTitle(cluster: ReportCluster): string {
  return `Cluster ${cluster.id}. ${isClone(cluster) ? `Rank ${cluster.rank || UNKNOWN_RANK} by duplicated mass.` : INFORMATIONAL_FINDING}`;
}

wireMessagePump();
const style = document.createElement("style");
style.textContent = GLOBAL_CSS;
document.head.appendChild(style);
const root = document.getElementById("root");
if (root) render(<ClusterApp />, root);
