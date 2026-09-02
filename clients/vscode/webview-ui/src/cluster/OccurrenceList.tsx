import { HelpAction } from "../components/HelpAction";
import { HelpBubble, HelpedText } from "../components/HelpBubble";
import { clearCompareEndpoints, compareLeft, compareRight, pickCompareEndpoint, post, sameEndpoint } from "../store";
import { COLOR, FONT } from "../theme";
import type { CompareEndpoint } from "../store";
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
// [VSIX-PAIR-COMPARE] Pair evidence renders only after the user selects two
// explicit endpoints; nothing here compares against an implicit canonical.
const COMPARE_PAIR_MESSAGE = "compare/pair";
const SELECT_FOR_COMPARISON = "Select for comparison";
const COMPARE_SELECTED = "Compare selected occurrences";

export function endpointOf(occurrence: ReportOccurrence): CompareEndpoint {
  return { path: occurrence.path, start_byte: occurrence.start_byte, end_byte: occurrence.end_byte };
}

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
        style={{ color: COLOR.onSurfaceMuted, marginBottom: TWELVE_PIXEL_SIZE, fontFamily: FONT.mono, display: FLEX_DISPLAY, alignItems: CENTER_ALIGNMENT, gap: SMALL_SPACING }}
      >
        <HelpedText topic={OCCURRENCES_TOPIC}>OCCURRENCES</HelpedText>
        <CompareSelectedButton />
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
          <OccurrenceActions cluster={cluster} occurrence={occurrence} />
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
}: {
  cluster: ReportCluster;
  occurrence: ReportOccurrence;
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
          aria-pressed={isSelected(cluster, occurrence)}
          onClick={() => pickCompareEndpoint(endpointOf(occurrence))}
          title={selectTitle(cluster, occurrence)}
          aria-label={SELECT_FOR_COMPARISON}
        >
          {SELECT_FOR_COMPARISON}
        </button>
      </HelpAction>
    </div>
  );
}

function openOccurrence(occurrence: ReportOccurrence): void {
  post({ kind: OPEN_OCCURRENCE_MESSAGE, occurrence });
}

function CompareSelectedButton() {
  const left = compareLeft.value;
  const right = compareRight.value;
  const ready = Boolean(left && right);
  return (
    <button
      disabled={!ready}
      aria-label={COMPARE_SELECTED}
      style={!ready ? { opacity: 0.3 } : { color: "inherit" }}
      onClick={() => {
        if (left && right) {
          post({ kind: COMPARE_PAIR_MESSAGE, left, right });
          clearCompareEndpoints();
        }
      }}
      title={
        ready
          ? "Open a diff of the two selected occurrences in VS Code's diff editor using Deslop's occurrence-range virtual documents."
          : "Select two occurrences to enable compare. Pair evidence exists only for the pair you choose."
      }
    >
      {COMPARE_SELECTED}
    </button>
  );
}

function isSelected(cluster: ReportCluster, occurrence: ReportOccurrence): boolean {
  const endpoint = endpointOf(occurrence);
  const left = compareLeft.value;
  const right = compareRight.value;
  return (
    cluster.occurrences.some((candidate) => sameEndpoint(endpointOf(candidate), endpoint)) &&
    Boolean((left && sameEndpoint(left, endpoint)) || (right && sameEndpoint(right, endpoint)))
  );
}

function selectTitle(cluster: ReportCluster, occurrence: ReportOccurrence): string {
  const endpoint = endpointOf(occurrence);
  const left = compareLeft.value;
  const right = compareRight.value;
  if (left && sameEndpoint(left, endpoint)) return "Selected as the left side of the pair compare.";
  if (right && sameEndpoint(right, endpoint)) return "Selected as the right side of the pair compare.";
  return left && right
    ? "Replace the right side of the pair compare with this occurrence."
    : left
      ? `Select this occurrence as the right side of the pair compare within ${cluster.id}.`
      : `Select this occurrence as the left side of the pair compare within ${cluster.id}.`;
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
