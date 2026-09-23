import { HelpedText } from "../components/HelpBubble";
import { isPicked, pickedOccurrence, post, tapOccurrenceRow } from "../store";
import type { ReportCluster, ReportOccurrence } from "../../../src/types/report";

const OCCURRENCES_TOPIC = "occurrences";
const OPEN_OCCURRENCE_MESSAGE = "open/occurrence";
const COMPARE_CANONICAL_MESSAGE = "compare/canonical";
const CANONICAL_OCCURRENCE_INDEX = 0;
const ROW_TAP_HINT = "Tap this row to pick it, then tap a second row to compare the two.";
const PICKED_ROW_HINT = "Picked for comparison. Tap another row to compare it with this one, or tap this row again to unpick it.";
const ROW_INTERACTIVE_SELECTOR = "button, a";

interface OccurrenceListProps {
  cluster: ReportCluster;
  focusedIndex: number;
  accent: string;
}

interface OccurrenceActionsProps {
  cluster: ReportCluster;
  occurrence: ReportOccurrence;
  index: number;
}

// [VSIX-PAIR-COMPARE] Explicit controls and row taps share one selection.
export function OccurrenceList({ cluster, focusedIndex, accent }: OccurrenceListProps) {
  return <section class="occurrence-list">
    <div class="label occurrence-heading"><HelpedText topic={OCCURRENCES_TOPIC}>OCCURRENCES</HelpedText></div>
    {cluster.occurrences.map((occurrence, index) => <article
      key={`${occurrence.path}-${occurrence.start_byte}`} class="occurrence-row"
      title={occurrenceTitle(occurrence, index, isPicked(occurrence))} data-picked={isPicked(occurrence)}
      onClick={(event) => tapRow(event, cluster, occurrence)}
      style={{ outline: rowOutline(isPicked(occurrence), index === focusedIndex, accent) }}>
      <OccurrenceLocation occurrence={occurrence} />
      <OccurrenceActions cluster={cluster} occurrence={occurrence} index={index} />
    </article>)}
  </section>;
}

function OccurrenceLocation({ occurrence }: { occurrence: ReportOccurrence }) {
  return <div class="occurrence-location">
    <button class="text-action" onClick={() => openOccurrence(occurrence)}
      title={openTitle(occurrence)} aria-label={openTitle(occurrence)}>
      {occurrence.displayLocation?.label ?? occurrence.path}
    </button>
    <div class="occurrence-position" title={locationDescriptionTitle(occurrence)}>
      {occurrence.displayLocation?.description ?? "line and column unavailable until the file is loaded"}
      {occurrence.hidden ? " · hidden" : ""}
    </div>
  </div>;
}

function OccurrenceActions({ cluster, occurrence, index }: OccurrenceActionsProps) {
  return <div class="occurrence-actions">
    {index === CANONICAL_OCCURRENCE_INDEX ? <span class="canonical-label">Canonical</span> : null}
    <button disabled={index === CANONICAL_OCCURRENCE_INDEX}
      onClick={() => compareWithCanonical(cluster.id, occurrence, index)}
      title={compareTitle(index)} aria-label="Compare To Canonical">Compare To Canonical</button>
    <button class="secondary" onClick={() => tapOccurrenceRow(cluster, occurrence)}
      title={selectionTitle(occurrence)}
      aria-label={selectionLabel(occurrence)} aria-pressed={isPicked(occurrence)}>
      {selectionLabel(occurrence)}
    </button>
  </div>;
}

function selectionLabel(occurrence: ReportOccurrence): string {
  if (isPicked(occurrence)) return "Clear Selection";
  return pickedOccurrence.value ? "Compare with Selected" : "Select for Compare";
}

function selectionTitle(occurrence: ReportOccurrence): string {
  if (isPicked(occurrence)) return "Clear this selection to choose a different left side.";
  return pickedOccurrence.value
    ? "Open the selected occurrence on the left and this occurrence on the right."
    : "Choose this occurrence as the left side, then use Compare with Selected on another row.";
}

function openOccurrence(occurrence: ReportOccurrence): void {
  post({ kind: OPEN_OCCURRENCE_MESSAGE, occurrence });
}

function tapRow(event: MouseEvent, cluster: ReportCluster, occurrence: ReportOccurrence): void {
  if (event.target instanceof Element && event.target.closest(ROW_INTERACTIVE_SELECTOR)) return;
  tapOccurrenceRow(cluster, occurrence);
}

function rowOutline(picked: boolean, focused: boolean, accent: string): string {
  if (picked) return `2px solid ${accent}`;
  return focused ? `1px solid ${accent}` : "none";
}

function compareWithCanonical(clusterId: string, occurrence: ReportOccurrence, index: number): void {
  if (index !== CANONICAL_OCCURRENCE_INDEX) post({ kind: COMPARE_CANONICAL_MESSAGE, clusterId, occurrence });
}

function compareTitle(index: number): string {
  return index === CANONICAL_OCCURRENCE_INDEX
    ? "Compare is disabled on the canonical occurrence because it would compare the same range with itself."
    : "Compare this occurrence with the canonical occurrence in VS Code's diff editor.";
}

function occurrenceTitle(occurrence: ReportOccurrence, index: number, picked: boolean): string {
  const role = index === CANONICAL_OCCURRENCE_INDEX ? "Canonical occurrence" : `Occurrence ${index + 1}`;
  const hidden = occurrence.hidden ? " This occurrence is hidden by report_hide configuration." : "";
  return `${role}: ${occurrence.displayLocation?.label ?? occurrence.path}. ${locationDescriptionTitle(occurrence)}${hidden} ${picked ? PICKED_ROW_HINT : ROW_TAP_HINT}`;
}

function locationDescriptionTitle(occurrence: ReportOccurrence): string {
  const hidden = occurrence.hidden ? " Hidden means this path matched report_hide configuration." : "";
  return `${occurrence.displayLocation?.description ?? "Line and column unavailable because the source file could not be read by the extension host."}${hidden}`;
}

function openTitle(occurrence: ReportOccurrence): string {
  return `Open this occurrence in VS Code at ${occurrence.displayLocation?.label ?? occurrence.path}. The editor selection will cover the clone range.`;
}
