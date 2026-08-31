import { HelpAction } from "../components/HelpAction";
import { HelpBubble, HelpedText } from "../components/HelpBubble";
import { post } from "../store";
import { COLOR, FONT } from "../theme";
import type { ReportCluster, ReportOccurrence } from "../../../src/types/report";

const TWELVE_PIXEL_SIZE = "12px";
const ELEVEN_PIXEL_FONT_SIZE = "11px";
const LARGE_SPACING = "24px";
const SMALL_SPACING = "8px";
const FLEX_DISPLAY = "flex";
const GRID_DISPLAY = "grid";
const WRAP_LAYOUT = "wrap";
const END_ALIGNMENT = "flex-end";
const CENTER_ALIGNMENT = "center";
const ANYWHERE_WRAP = "anywhere";
const FULL_WIDTH = "100%";
const LABEL_CLASS = "label";
const WITH_HELP_CLASS = "with-help";
const OCCURRENCES_TOPIC = "occurrences";
const OCCURRENCE_LOCATION_TOPIC = "occurrence-location";
const OPEN_OCCURRENCE_MESSAGE = "open/occurrence";

interface OccurrenceListProps {
  cluster: ReportCluster;
  focusedIndex: number;
  accent: string;
}

export function OccurrenceList({ cluster, focusedIndex, accent }: OccurrenceListProps) {
  return (
    <section style={{ marginTop: LARGE_SPACING }}>
      <div
        class={LABEL_CLASS}
        style={{ color: COLOR.onSurfaceMuted, marginBottom: TWELVE_PIXEL_SIZE, fontFamily: FONT.mono }}
      >
        <HelpedText topic={OCCURRENCES_TOPIC}>OCCURRENCES</HelpedText>
      </div>
      {cluster.occurrences.map((occurrence, index) => (
        <article
          key={`${occurrence.path}-${occurrence.start_byte}`}
          title={occurrenceTitle(occurrence, index)}
          style={{
            background: index % 2 === 0 ? COLOR.surfaceContainerLow : COLOR.surface,
            padding: "14px 20px",
            display: GRID_DISPLAY,
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 220px), 1fr))",
            gap: "16px",
            alignItems: CENTER_ALIGNMENT,
            outline: index === focusedIndex ? `1px solid ${accent}` : "none",
          }}
        >
          <OccurrenceLocation occurrence={occurrence} />
          <OccurrenceActions cluster={cluster} occurrence={occurrence} index={index} />
        </article>
      ))}
    </section>
  );
}

function OccurrenceLocation({ occurrence }: { occurrence: ReportOccurrence }) {
  return (
    <div style={{ minWidth: 0 }}>
      <div
        class={WITH_HELP_CLASS}
        style={{ fontFamily: FONT.mono, fontSize: TWELVE_PIXEL_SIZE, maxWidth: FULL_WIDTH }}
        title={locationTitle(occurrence)}
      >
        <button
          class="text-action"
          onClick={() => openOccurrence(occurrence)}
          title={openTitle(occurrence)}
          aria-label={openTitle(occurrence)}
          style={{ maxWidth: FULL_WIDTH, overflowWrap: ANYWHERE_WRAP }}
        >
          {occurrence.displayLocation?.label ?? occurrence.path}
        </button>
        <HelpBubble topic={OCCURRENCE_LOCATION_TOPIC} />
      </div>
      <div
        style={{
          fontFamily: FONT.mono,
          color: COLOR.onSurfaceMuted,
          fontSize: ELEVEN_PIXEL_FONT_SIZE,
          marginTop: "2px",
        }}
        title={locationDescriptionTitle(occurrence)}
      >
        <HelpedText
          topic={occurrence.hidden ? "hidden-occurrence" : OCCURRENCE_LOCATION_TOPIC}
          title={locationDescriptionTitle(occurrence)}
        >
          {occurrence.displayLocation?.description ??
            "line and column unavailable until the file is loaded"}
          {occurrence.hidden ? " · hidden" : ""}
        </HelpedText>
      </div>
    </div>
  );
}

function OccurrenceActions({
  cluster,
  occurrence,
  index,
}: {
  cluster: ReportCluster;
  occurrence: ReportOccurrence;
  index: number;
}) {
  return (
    <div
      style={{
        display: FLEX_DISPLAY,
        gap: SMALL_SPACING,
        flexWrap: WRAP_LAYOUT,
        justifyContent: END_ALIGNMENT,
      }}
    >
      <HelpAction topic="open-action">
        <button
          onClick={() => openOccurrence(occurrence)}
          title={openTitle(occurrence)}
          aria-label={openTitle(occurrence)}
        >
          Open
        </button>
      </HelpAction>
      <HelpAction topic="compare-action">
        <button
          class={index === 0 ? "" : "primary"}
          onClick={() => compareWithCanonical(cluster.id, index)}
          aria-disabled={index === 0}
          style={index === 0 ? { opacity: 0.3 } : { color: "inherit" }}
          title={compareTitle(index)}
          aria-label={compareTitle(index)}
        >
          Compare
        </button>
      </HelpAction>
    </div>
  );
}

function openOccurrence(occurrence: ReportOccurrence): void {
  post({ kind: OPEN_OCCURRENCE_MESSAGE, occurrence });
}

function compareWithCanonical(clusterId: string, index: number): void {
  if (index !== 0) post({ kind: "compare/canonical", clusterId });
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
