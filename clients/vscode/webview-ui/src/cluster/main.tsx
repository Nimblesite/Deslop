import { render } from "preact";
import { useEffect } from "preact/hooks";
import { signal } from "@preact/signals";

import {
  analysisState,
  clusters,
  post,
  selectedCluster,
  selectedClusterId,
  severityByClusterId,
  wireMessagePump,
} from "../store";
import { COLOR, FONT, GLOBAL_CSS, SEVERITY_COLOR } from "../theme";
import { HelpAction } from "../components/HelpAction";
import { electedPairEvidence, PairEvidence } from "../components/PairEvidence";
import { SeverityBadge } from "../components/SeverityBadge";
import {
  DocTextLink,
  HelpBubble,
  HelpedText,
  helpCopy,
  type HelpTopic,
} from "../components/HelpBubble";
import {
  bucketLabels,
  clusterInterpretation,
  clusterSlug,
  occurrenceCount,
  resolveBucket,
} from "../../../src/types/report";
import { formatScore } from "../../../src/types/format";
import { helpValueTitle } from "../../../src/types/signals";
import type { ReportCluster, ReportOccurrence } from "../../../src/types/report";
import { OccurrenceList } from "./OccurrenceList";

const focusedOccurrenceIndex = signal(0);
const shortcutHelpExpanded = signal(false);
const TWELVE_PIXEL_SIZE = "12px";
const ELEVEN_PIXEL_FONT_SIZE = "11px";
const TEN_PIXEL_SIZE = "10px";
const LARGE_SPACING = "24px";
const SMALL_SPACING = "8px";
const FLEX_DISPLAY = "flex";
const SPACE_TEXT = " ";
const GRID_DISPLAY = "grid";
const WRAP_LAYOUT = "wrap";
const END_ALIGNMENT = "flex-end";
const CENTER_ALIGNMENT = "center";
const ANYWHERE_WRAP = "anywhere";
const OPEN_OCCURRENCE_MESSAGE = "open/occurrence";
const UNKNOWN_RANK = "unknown";
const FAINT_SEVERITY = "faint";
const KEYDOWN_EVENT = "keydown";
const LABEL_CLASS = "label";
const WITH_HELP_CLASS = "with-help";
const CLONE_BUCKET_TOPIC = "clone-bucket";
const CLUSTER_ID_TOPIC = "cluster-id";
const CLUSTER_NAVIGATION_TOPIC = "cluster-navigation";
const CANONICAL_TOPIC = "canonical";
const OCCURRENCES_TOPIC = "occurrences";
const WEIGHT_TOPIC = "weight";
const SIZE_TOPIC = "size";
const BADGE_PADDING = "2px 6px";
const BOLD_FONT_WEIGHT = 700;

function ClusterApp() {
  const cluster = selectedCluster.value;
  const list = clusters.value;
  const rank = cluster ? list.findIndex((c) => c.id === cluster.id) + 1 : 0;
  const severity = cluster
    ? severityByClusterId.value.get(cluster.id) ?? FAINT_SEVERITY
    : FAINT_SEVERITY;
  const slug = cluster ? clusterSlug(cluster) : "";

  useEffect(() => {
    focusedOccurrenceIndex.value = 0;
  }, [cluster?.id]);

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (isEditableTarget(event.target)) return;
      const currentList = clusters.value;
      const currentCluster = selectedCluster.value;
      const currentRank = currentCluster
        ? currentList.findIndex((c) => c.id === currentCluster.id) + 1
        : 0;
      if (event.key === "n" && currentList.length > 0) {
        event.preventDefault();
        selectNextCluster(currentList, currentRank);
      }
      if (event.key === "p" && currentList.length > 0) {
        event.preventDefault();
        selectPreviousCluster(currentList, currentRank);
      }
      if (event.key === "j" && currentCluster) {
        event.preventDefault();
        moveFocusedOccurrence(currentCluster, 1);
      }
      if (event.key === "k" && currentCluster) {
        event.preventDefault();
        moveFocusedOccurrence(currentCluster, -1);
      }
      if (event.key === "Enter" && currentCluster) {
        event.preventDefault();
        openFocusedOccurrence(currentCluster);
      }
      if (event.key === "?") {
        event.preventDefault();
        shortcutHelpExpanded.value = !shortcutHelpExpanded.value;
      }
    };
    window.addEventListener(KEYDOWN_EVENT, handler);
    return () => window.removeEventListener(KEYDOWN_EVENT, handler);
  }, []);

  if (!cluster) {
    return (
      <main style={{ padding: "24px" }}>
        <p>No cluster selected.</p>
      </main>
    );
  }

  const canonical = cluster.occurrences[0];
  const bucketInfo = bucketLabels(resolveBucket(cluster));
  const evidence = clusterInterpretation(cluster);
  const focusedIndex = focusedIndexFor(cluster);

  return (
    <main
      style={{
        padding: "24px clamp(16px, 6vw, 32px)",
        minHeight: "100vh",
        display: FLEX_DISPLAY,
        flexDirection: "column",
        opacity: analysisState.value.state === "errored" ? 0.5 : 1,
        transition: "opacity 120ms",
      }}
    >
      <header
        style={{
          display: GRID_DISPLAY,
          gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
          gap: LARGE_SPACING,
          alignItems: "start",
          paddingBottom: LARGE_SPACING,
        }}
      >
        <div>
          <div
            class={LABEL_CLASS}
            style={{
              color: COLOR.onSurfaceMuted,
              marginBottom: SMALL_SPACING,
              fontFamily: FONT.mono,
              display: FLEX_DISPLAY,
              alignItems: CENTER_ALIGNMENT,
              gap: SMALL_SPACING,
              flexWrap: WRAP_LAYOUT,
              minWidth: 0,
            }}
          >
            <HelpedText topic={CLUSTER_ID_TOPIC} title={clusterIdTitle(cluster.id, rank, list.length)}>
              CLUSTER ·{SPACE_TEXT}
              <DocTextLink topic={CLUSTER_ID_TOPIC} title={clusterIdTitle(cluster.id, rank, list.length)}>
                {cluster.id}
              </DocTextLink>
            </HelpedText>
            {bucketInfo.aiMatch ? (
              <HelpedText topic="ai-match" title={aiMatchTitle()}>
                <span
                  style={{
                    background: COLOR.secondaryContainer ?? COLOR.surfaceContainerLow,
                    color: COLOR.onSurface,
                    padding: BADGE_PADDING,
                    borderRadius: "3px",
                    fontSize: TEN_PIXEL_SIZE,
                    letterSpacing: "0.1em",
                    fontWeight: BOLD_FONT_WEIGHT,
                  }}
                >
                  AI MATCH
                </span>
              </HelpedText>
            ) : null}
          </div>
          <h1
            style={{
              margin: 0,
              fontFamily: FONT.ui,
              fontSize: "2.25rem",
              fontWeight: BOLD_FONT_WEIGHT,
              letterSpacing: "-0.02em",
            }}
            title={`${bucketInfo.plainTitle}: ${evidence}`}
          >
            <HelpedText topic={CLONE_BUCKET_TOPIC} title={`${bucketInfo.plainTitle}: ${evidence}`}>
              <DocTextLink topic={CLONE_BUCKET_TOPIC}>{bucketInfo.plainTitle}</DocTextLink>
            </HelpedText>
          </h1>
          <p
            style={{
              margin: "12px 0 0",
              color: COLOR.onSurfaceMuted,
              fontFamily: FONT.ui,
              fontSize: "15px",
            }}
            title={`Engine interpretation for this bucket: ${evidence}`}
          >
            <HelpedText topic={CLONE_BUCKET_TOPIC}>{evidence}</HelpedText>
          </p>
        </div>
        <div style={{ textAlign: "right", minWidth: 0, overflowWrap: ANYWHERE_WRAP }}>
          <span class={WITH_HELP_CLASS} style={{ justifyContent: END_ALIGNMENT }}>
            <SeverityBadge
              severity={severity}
              label={`${slug}`}
              title={rankTitle(rank, list.length, severity)}
            />
            <HelpBubble topic="rank" />
          </span>
          <div
            style={{
              fontFamily: FONT.mono,
              color: COLOR.onSurfaceMuted,
              marginTop: TWELVE_PIXEL_SIZE,
              fontSize: TWELVE_PIXEL_SIZE,
              display: FLEX_DISPLAY,
              justifyContent: END_ALIGNMENT,
              gap: TEN_PIXEL_SIZE,
              flexWrap: WRAP_LAYOUT,
            }}
            title={clusterStatsTitle(cluster)}
          >
            <StatItem topic={WEIGHT_TOPIC} label={WEIGHT_TOPIC} value={formatScore(cluster.weight)} />
            <StatItem topic={SIZE_TOPIC} label={SIZE_TOPIC} value={String(cluster.size)} />
            <StatItem topic="occurrence-count" label={OCCURRENCES_TOPIC} value={`× ${occurrenceCount(cluster)}`} />
          </div>
          {canonical ? (
            <div
              style={{
                fontFamily: FONT.mono,
                fontSize: TWELVE_PIXEL_SIZE,
                marginTop: "4px",
                overflowWrap: ANYWHERE_WRAP,
              }}
              title={canonicalTitle(canonical)}
            >
              <HelpedText topic={CANONICAL_TOPIC} title={canonicalTitle(canonical)}>
                <DocTextLink topic={CANONICAL_TOPIC}>{CANONICAL_TOPIC}</DocTextLink>: {canonical.path}
              </HelpedText>
            </div>
          ) : null}
        </div>
      </header>

      <PairEvidence evidence={electedPairEvidence(cluster)} />

      <OccurrenceList
        cluster={cluster}
        focusedIndex={focusedIndex}
        accent={SEVERITY_COLOR[severity]}
      />

      <div style={{ marginTop: "auto", paddingTop: LARGE_SPACING }}>
        <footer
          style={{
            display: FLEX_DISPLAY,
            gap: "12px",
            justifyContent: END_ALIGNMENT,
            flexWrap: WRAP_LAYOUT,
          }}
        >
          <HelpAction topic={CLUSTER_NAVIGATION_TOPIC}>
            <button
              onClick={() => selectPreviousCluster(list, rank)}
              title="Previous cluster: move to the cluster ranked immediately before this one. Same behavior as the p keyboard shortcut."
              aria-label="Previous cluster"
            >
              ← prev cluster (p)
            </button>
          </HelpAction>
          <HelpAction topic={CLUSTER_NAVIGATION_TOPIC}>
            <button
              onClick={() => selectNextCluster(list, rank)}
              title="Next cluster: move to the cluster ranked immediately after this one. Same behavior as the n keyboard shortcut."
              aria-label="Next cluster"
            >
              next cluster (n) →
            </button>
          </HelpAction>
        </footer>
        <HotkeyHelp accent={SEVERITY_COLOR[severity]} />
      </div>
    </main>
  );
}

function HotkeyHelp({ accent }: { accent: string }) {
  return (
    <div
      class="mono"
      style={{
        marginTop: "32px",
        fontSize: ELEVEN_PIXEL_FONT_SIZE,
        color: COLOR.onSurfaceMuted,
      }}
      title="Keyboard help for this cluster panel. These shortcuts work while focus is in the webview but not inside a button or input."
    >
      <span style={{ color: accent }} title="j moves the focused occurrence down; k moves it up.">
        j/k
      </span>{SPACE_TEXT}
      next/prev occurrence ·{SPACE_TEXT}
      <span style={{ color: accent }} title="n moves to the next cluster; p moves to the previous cluster.">
        n/p
      </span>{SPACE_TEXT}
      next/prev cluster ·{SPACE_TEXT}
      <span style={{ color: accent }} title="Enter opens the currently focused occurrence in the editor.">
        Enter
      </span>{SPACE_TEXT}
      open ·{SPACE_TEXT}
      <button
        onClick={() => {
          shortcutHelpExpanded.value = !shortcutHelpExpanded.value;
        }}
        title="Show or hide detailed keyboard shortcut help for this cluster panel."
        aria-label="Toggle keyboard shortcut help"
        style={{ padding: BADGE_PADDING, color: accent }}
      >
        ?
      </button>{SPACE_TEXT}
      help <HelpBubble topic="keyboard-shortcuts" />
      {shortcutHelpExpanded.value ? (
        <div
          style={{ marginTop: SMALL_SPACING, maxWidth: "760px" }}
          title="Detailed keyboard help: occurrence movement changes the highlighted occurrence row; cluster movement changes the selected cluster; Enter opens the focused occurrence."
        >
          j/k changes the highlighted occurrence row. n/p changes the selected cluster. Enter opens the highlighted occurrence in VS Code. ? toggles this help text.
        </div>
      ) : null}
    </div>
  );
}

function StatItem({ topic, label, value }: { topic: HelpTopic; label: string; value: string }) {
  const title = helpValueTitle(helpCopy(topic), value);
  return (
    <span class={WITH_HELP_CLASS} title={title}>
      <span>
        <DocTextLink topic={topic} title={title}>{label}</DocTextLink> {value}
      </span>
      <HelpBubble topic={topic} />
    </span>
  );
}

function selectNextCluster(list: ReportCluster[], rank: number): void {
  selectClusterByOffset(list, rank, 1);
}

function selectPreviousCluster(list: ReportCluster[], rank: number): void {
  selectClusterByOffset(list, rank, -1);
}

function selectClusterByOffset(list: ReportCluster[], rank: number, offset: number): void {
  if (list.length === 0) return;
  const current = rank > 0 ? rank - 1 : 0;
  const next = (current + offset + list.length) % list.length;
  selectedClusterId.value = list[next]?.id ?? null;
  focusedOccurrenceIndex.value = 0;
}

function moveFocusedOccurrence(cluster: ReportCluster, offset: number): void {
  const total = cluster.occurrences.length;
  if (total === 0) return;
  focusedOccurrenceIndex.value = (focusedIndexFor(cluster) + offset + total) % total;
}

function openFocusedOccurrence(cluster: ReportCluster): void {
  const occurrence = cluster.occurrences[focusedIndexFor(cluster)] ?? cluster.occurrences[0];
  if (occurrence) post({ kind: OPEN_OCCURRENCE_MESSAGE, occurrence });
}

function focusedIndexFor(cluster: ReportCluster): number {
  const max = Math.max(0, cluster.occurrences.length - 1);
  return Math.min(Math.max(0, focusedOccurrenceIndex.value), max);
}

function isEditableTarget(target: EventTarget | null): boolean {
  return target instanceof HTMLElement &&
    ["BUTTON", "INPUT", "SELECT", "TEXTAREA"].includes(target.tagName);
}

function clusterIdTitle(id: string, rank: number, total: number): string {
  return `Cluster ${id}. Ranked ${rank || UNKNOWN_RANK} of ${total} by Deslop's worst-first duplication impact score.`;
}

function aiMatchTitle(): string {
  return "AI match: Deslop's embedding pass found semantic equivalence. Review both locations before merging.";
}

function rankTitle(rank: number, total: number, severity: string): string {
  return `Rank ${rank || UNKNOWN_RANK} of ${total}. Severity bucket ${severity} is based on this cluster's relative weight in the current report.`;
}

function clusterStatsTitle(cluster: ReportCluster): string {
  return `Weight is Deslop's duplication impact score. Size is the number of cloned AST members. Occurrences is the number of editor locations in this cluster: weight ${formatScore(cluster.weight)}, size ${cluster.size}, occurrences ${occurrenceCount(cluster)}.`;
}

function canonicalTitle(occurrence: ReportOccurrence): string {
  return `Canonical occurrence: Deslop uses this first occurrence as the comparison anchor for this cluster. Location: ${occurrence.displayLocation?.label ?? occurrence.path}.`;
}

wireMessagePump();
const style = document.createElement("style");
style.textContent = GLOBAL_CSS;
document.head.appendChild(style);
const root = document.getElementById("root");
if (root) render(<ClusterApp />, root);
