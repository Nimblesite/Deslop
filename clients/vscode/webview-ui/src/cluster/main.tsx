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
import { SignalStrip } from "../components/SignalStrip";
import { SeverityBadge } from "../components/SeverityBadge";
import {
  DocTextLink,
  HelpBubble,
  HelpedText,
  helpCopy,
  type HelpTopic,
} from "../components/HelpBubble";
import { bucketLabels, clusterSlug, occurrenceCount, resolveBucket } from "../../../src/types/report";
import type { ReportCluster, ReportOccurrence } from "../../../src/types/report";

const focusedOccurrenceIndex = signal(0);
const shortcutHelpExpanded = signal(false);

function ClusterApp() {
  const cluster = selectedCluster.value;
  const list = clusters.value;
  const rank = cluster ? list.findIndex((c) => c.id === cluster.id) + 1 : 0;
  const severity = cluster ? severityByClusterId.value.get(cluster.id) ?? "faint" : "faint";
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
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
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
  const focusedIndex = focusedIndexFor(cluster);

  return (
    <main
      style={{
        padding: "24px clamp(16px, 6vw, 32px)",
        minHeight: "100vh",
        display: "flex",
        flexDirection: "column",
        opacity: analysisState.value === "errored" ? 0.5 : 1,
        transition: "opacity 120ms",
      }}
    >
      <header
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 280px), 1fr))",
          gap: "24px",
          alignItems: "start",
          paddingBottom: "24px",
        }}
      >
        <div>
          <div
            class="label"
            style={{
              color: COLOR.onSurfaceMuted,
              marginBottom: "8px",
              fontFamily: FONT.mono,
              display: "flex",
              alignItems: "center",
              gap: "8px",
              flexWrap: "wrap",
              minWidth: 0,
            }}
          >
            <HelpedText topic="cluster-id" title={clusterIdTitle(cluster.id, rank, list.length)}>
              CLUSTER ·{" "}
              <DocTextLink topic="cluster-id" title={clusterIdTitle(cluster.id, rank, list.length)}>
                {cluster.id}
              </DocTextLink>
            </HelpedText>
            {bucketInfo.aiMatch ? (
              <HelpedText topic="ai-match" title={aiMatchTitle()}>
                <span
                  style={{
                    background: COLOR.secondaryContainer ?? COLOR.surfaceContainerLow,
                    color: COLOR.onSurface,
                    padding: "2px 6px",
                    borderRadius: "3px",
                    fontSize: "10px",
                    letterSpacing: "0.1em",
                    fontWeight: 700,
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
              fontWeight: 700,
              letterSpacing: "-0.02em",
            }}
            title={`${bucketInfo.plainTitle}: ${bucketInfo.actionSentence}`}
          >
            <HelpedText topic="clone-bucket" title={`${bucketInfo.plainTitle}: ${bucketInfo.actionSentence}`}>
              <DocTextLink topic="clone-bucket">{bucketInfo.plainTitle}</DocTextLink>
            </HelpedText>
          </h1>
          <p
            style={{
              margin: "12px 0 0",
              color: COLOR.onSurfaceMuted,
              fontFamily: FONT.ui,
              fontSize: "15px",
            }}
            title={`Recommended reading for this bucket: ${bucketInfo.actionSentence}`}
          >
            <HelpedText topic="clone-bucket">{bucketInfo.actionSentence}</HelpedText>
          </p>
        </div>
        <div style={{ textAlign: "right", minWidth: 0, overflowWrap: "anywhere" }}>
          <span class="with-help" style={{ justifyContent: "flex-end" }}>
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
              marginTop: "12px",
              fontSize: "12px",
              display: "flex",
              justifyContent: "flex-end",
              gap: "10px",
              flexWrap: "wrap",
            }}
            title={clusterStatsTitle(cluster)}
          >
            <StatItem topic="weight" label="weight" value={cluster.weight.toFixed(2)} />
            <StatItem topic="size" label="size" value={String(cluster.size)} />
            <StatItem topic="occurrence-count" label="occurrences" value={`× ${occurrenceCount(cluster)}`} />
          </div>
          {canonical ? (
            <div
              style={{
                fontFamily: FONT.mono,
                fontSize: "12px",
                marginTop: "4px",
                overflowWrap: "anywhere",
              }}
              title={canonicalTitle(canonical)}
            >
              <HelpedText topic="canonical" title={canonicalTitle(canonical)}>
                <DocTextLink topic="canonical">canonical</DocTextLink>: {canonical.path}
              </HelpedText>
            </div>
          ) : null}
        </div>
      </header>

      <section style={{ background: COLOR.surfaceContainerLow, padding: "16px 24px" }}>
        <div class="label" style={{ marginBottom: "8px", fontFamily: FONT.mono }}>
          <HelpedText topic="signals">SIGNALS</HelpedText>
        </div>
        <SignalStrip signals={cluster.signals} />
      </section>

      <section style={{ marginTop: "24px" }}>
        <div
          class="label"
          style={{ color: COLOR.onSurfaceMuted, marginBottom: "12px", fontFamily: FONT.mono }}
        >
          <HelpedText topic="occurrences">OCCURRENCES</HelpedText>
        </div>
        {cluster.occurrences.map((o, i) => (
          <article
            key={`${o.path}-${o.start_byte}`}
            title={occurrenceTitle(o, i)}
            style={{
              background: i % 2 === 0 ? COLOR.surfaceContainerLow : COLOR.surface,
              padding: "14px 20px",
              display: "grid",
              gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 220px), 1fr))",
              gap: "16px",
              alignItems: "center",
              outline: i === focusedIndex ? `1px solid ${SEVERITY_COLOR[severity]}` : "none",
            }}
          >
            <div style={{ minWidth: 0 }}>
              <div
                class="with-help"
                style={{ fontFamily: FONT.mono, fontSize: "12px", maxWidth: "100%" }}
                title={locationTitle(o)}
              >
                <button
                  class="text-action"
                  onClick={() => post({ kind: "open/occurrence", occurrence: o })}
                  title={openTitle(o)}
                  aria-label={openTitle(o)}
                  style={{ maxWidth: "100%", overflowWrap: "anywhere" }}
                >
                  {o.displayLocation?.label ?? o.path}
                </button>
                <HelpBubble topic="occurrence-location" />
              </div>
              <div
                style={{
                  fontFamily: FONT.mono,
                  color: COLOR.onSurfaceMuted,
                  fontSize: "11px",
                  marginTop: "2px",
                }}
                title={locationDescriptionTitle(o)}
              >
                <HelpedText
                  topic={o.hidden ? "hidden-occurrence" : "occurrence-location"}
                  title={locationDescriptionTitle(o)}
                >
                  {o.displayLocation?.description ??
                    "line and column unavailable until the file is loaded"}
                  {o.hidden ? " · hidden" : ""}
                </HelpedText>
              </div>
            </div>
            {/* [VSIX-WEBVIEW-ACTIONS-CONTEXT] Open / Compare per occurrence
                (prev/next cluster live in the footer); each posts a typed
                message the host acts on. */}
            <div style={{ display: "flex", gap: "8px", flexWrap: "wrap", justifyContent: "flex-end" }}>
              <span class="with-help">
                <button
                  onClick={() => post({ kind: "open/occurrence", occurrence: o })}
                  title={openTitle(o)}
                  aria-label={openTitle(o)}
                >
                  Open
                </button>
                <HelpBubble topic="open-action" />
              </span>
              <span class="with-help">
                <button
                  class={i === 0 ? "" : "primary"}
                  onClick={() => {
                    if (i === 0) return;
                    post({ kind: "compare/canonical", clusterId: cluster.id });
                  }}
                  aria-disabled={i === 0}
                  style={i === 0 ? { opacity: 0.3 } : { color: "inherit" }}
                  title={compareTitle(i)}
                  aria-label={compareTitle(i)}
                >
                  Compare
                </button>
                <HelpBubble topic="compare-action" />
              </span>
            </div>
          </article>
        ))}
      </section>

      <div style={{ marginTop: "auto", paddingTop: "24px" }}>
        <footer
          style={{
            display: "flex",
            gap: "12px",
            justifyContent: "flex-end",
            flexWrap: "wrap",
          }}
        >
          <span class="with-help">
            <button
              onClick={() => selectPreviousCluster(list, rank)}
              title="Previous cluster: move to the cluster ranked immediately before this one. Same behavior as the p keyboard shortcut."
              aria-label="Previous cluster"
            >
              ← prev cluster (p)
            </button>
            <HelpBubble topic="cluster-navigation" />
          </span>
          <span class="with-help">
            <button
              onClick={() => selectNextCluster(list, rank)}
              title="Next cluster: move to the cluster ranked immediately after this one. Same behavior as the n keyboard shortcut."
              aria-label="Next cluster"
            >
              next cluster (n) →
            </button>
            <HelpBubble topic="cluster-navigation" />
          </span>
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
        fontSize: "11px",
        color: COLOR.onSurfaceMuted,
      }}
      title="Keyboard help for this cluster panel. These shortcuts work while focus is in the webview but not inside a button or input."
    >
      <span style={{ color: accent }} title="j moves the focused occurrence down; k moves it up.">
        j/k
      </span>{" "}
      next/prev occurrence ·{" "}
      <span style={{ color: accent }} title="n moves to the next cluster; p moves to the previous cluster.">
        n/p
      </span>{" "}
      next/prev cluster ·{" "}
      <span style={{ color: accent }} title="Enter opens the currently focused occurrence in the editor.">
        Enter
      </span>{" "}
      open ·{" "}
      <button
        onClick={() => {
          shortcutHelpExpanded.value = !shortcutHelpExpanded.value;
        }}
        title="Show or hide detailed keyboard shortcut help for this cluster panel."
        aria-label="Toggle keyboard shortcut help"
        style={{ padding: "2px 6px", color: accent }}
      >
        ?
      </button>{" "}
      help <HelpBubble topic="keyboard-shortcuts" />
      {shortcutHelpExpanded.value ? (
        <div
          style={{ marginTop: "8px", maxWidth: "760px" }}
          title="Detailed keyboard help: occurrence movement changes the highlighted occurrence row; cluster movement changes the selected cluster; Enter opens the focused occurrence."
        >
          j/k changes the highlighted occurrence row. n/p changes the selected cluster. Enter opens the highlighted occurrence in VS Code. ? toggles this help text.
        </div>
      ) : null}
    </div>
  );
}

function StatItem({ topic, label, value }: { topic: HelpTopic; label: string; value: string }) {
  const title = `${helpCopy(topic)} Current value: ${value}.`;
  return (
    <span class="with-help" title={title}>
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
  if (occurrence) post({ kind: "open/occurrence", occurrence });
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
  return `Cluster ${id}. Ranked ${rank || "unknown"} of ${total} by Deslop's worst-first duplication impact score.`;
}

function aiMatchTitle(): string {
  return "AI match: Deslop's embedding pass found semantic equivalence. Review both locations before merging.";
}

function rankTitle(rank: number, total: number, severity: string): string {
  return `Rank ${rank || "unknown"} of ${total}. Severity bucket ${severity} is based on this cluster's relative weight in the current report.`;
}

function clusterStatsTitle(cluster: ReportCluster): string {
  return `Weight is Deslop's duplication impact score. Size is the number of cloned AST members. Occurrences is the number of editor locations in this cluster: weight ${cluster.weight.toFixed(2)}, size ${cluster.size}, occurrences ${occurrenceCount(cluster)}.`;
}

function canonicalTitle(occurrence: ReportOccurrence): string {
  return `Canonical occurrence: Deslop uses this first occurrence as the comparison anchor for this cluster. Location: ${occurrence.displayLocation?.label ?? occurrence.path}.`;
}

function occurrenceTitle(occurrence: ReportOccurrence, index: number): string {
  const role = index === 0 ? "Canonical occurrence" : `Occurrence ${index + 1}`;
  const hidden = occurrence.hidden
    ? " This occurrence is hidden by report_hide configuration but shown because the cluster also contains visible code."
    : "";
  return `${role}: ${occurrence.displayLocation?.label ?? occurrence.path}. ${occurrence.displayLocation?.description ?? "Line and column are unavailable until the file can be read."}${hidden}`;
}

function locationTitle(occurrence: ReportOccurrence): string {
  return `Editor target: ${occurrence.displayLocation?.label ?? occurrence.path}. This is the file and human line/column that Open will navigate to.`;
}

function locationDescriptionTitle(occurrence: ReportOccurrence): string {
  const hidden = occurrence.hidden ? " Hidden means this path matched report_hide configuration." : "";
  return `${occurrence.displayLocation?.description ?? "Line and column unavailable because the source file could not be read by the extension host."}${hidden}`;
}

function openTitle(occurrence: ReportOccurrence): string {
  return `Open this occurrence in VS Code at ${occurrence.displayLocation?.label ?? occurrence.path}. The editor selection will cover the clone range.`;
}

function compareTitle(index: number): string {
  if (index === 0) {
    return "Compare is disabled on the canonical occurrence because comparing the anchor to itself would not show a useful diff.";
  }
  return "Compare this cluster against its canonical occurrence in VS Code's diff editor using Deslop's occurrence-range virtual documents.";
}

wireMessagePump();
const style = document.createElement("style");
style.textContent = GLOBAL_CSS;
document.head.appendChild(style);
const root = document.getElementById("root");
if (root) render(<ClusterApp />, root);
