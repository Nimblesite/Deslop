import type { JSX } from "preact";

import type { ReportCluster, ReportOccurrence, ReportSignals } from "../../../src/types/report";
import { COLOR, FONT } from "../theme";
import { HelpedText } from "./HelpBubble";
import { SignalStrip } from "./SignalStrip";

const ELECTED_PAIR_HEADING = "ELECTED PAIR EVIDENCE";
const UNAVAILABLE_HEADING = "PAIR EVIDENCE UNAVAILABLE";
const UNAVAILABLE_MESSAGE = "Deslop did not name two source occurrences, so no pair scores are shown.";
const PAIR_SEPARATOR = " ↔ ";

interface ElectedPairEvidence {
  pair: readonly [ReportOccurrence, ReportOccurrence];
  signals: ReportSignals;
  verdict: string;
}

interface Props {
  evidence: ElectedPairEvidence | null;
}

// [FUSED-CLUSTER-SIGNALS] A score is rendered only when the wire names the
// exact admitted pair that produced it. Unscoped cluster scores are illegal.
export function PairEvidence({ evidence }: Props) {
  return evidence ? <MeasuredPair evidence={evidence} /> : <UnavailablePair />;
}

function MeasuredPair({ evidence }: { evidence: ElectedPairEvidence }) {
  return (
    <section style={PANEL_STYLE}>
      <PairHeading text={ELECTED_PAIR_HEADING} />
      <p style={SOURCE_STYLE}>{pairLabel(evidence.pair)}</p>
      <SignalStrip signals={evidence.signals} verdict={evidence.verdict} />
    </section>
  );
}

function UnavailablePair() {
  return (
    <section style={PANEL_STYLE}>
      <PairHeading text={UNAVAILABLE_HEADING} />
      <p style={UNAVAILABLE_STYLE}>{UNAVAILABLE_MESSAGE}</p>
    </section>
  );
}

function PairHeading({ text }: { text: string }) {
  return (
    <div class="label" style={HEADING_STYLE}>
      <HelpedText topic="signals">{text}</HelpedText>
    </div>
  );
}

function pairLabel(pair: readonly [ReportOccurrence, ReportOccurrence]): string {
  return `${occurrenceLabel(pair[0])}${PAIR_SEPARATOR}${occurrenceLabel(pair[1])}`;
}

function occurrenceLabel(occurrence: ReportOccurrence): string {
  return occurrence.displayLocation?.label ?? occurrence.path;
}

export function electedPairEvidence(cluster: ReportCluster): ElectedPairEvidence | null {
  const source = cluster.signal_source;
  if (!source || source.left === source.right) return null;
  const left = cluster.occurrences[source.left];
  const right = cluster.occurrences[source.right];
  if (!left || !right) return null;
  return { pair: [left, right], signals: cluster.signals, verdict: cluster.evidence_verdict };
}

const PANEL_STYLE: JSX.CSSProperties = {
  background: COLOR.surfaceContainerLow,
  padding: "16px 24px",
};

const HEADING_STYLE: JSX.CSSProperties = {
  marginBottom: "8px",
  fontFamily: FONT.mono,
};

const SOURCE_STYLE: JSX.CSSProperties = {
  color: COLOR.onSurface,
  fontFamily: FONT.mono,
  fontSize: "12px",
  margin: 0,
  overflowWrap: "anywhere",
};

const UNAVAILABLE_STYLE: JSX.CSSProperties = {
  color: COLOR.onSurfaceMuted,
  fontFamily: FONT.ui,
  fontSize: "12px",
  margin: 0,
};
