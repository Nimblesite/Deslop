import type { JSX } from "preact";
import { COLOR, FONT } from "../theme";
import { HelpedText } from "./HelpBubble";
import {
  confidenceRows,
  contentEvidenceVerdict,
  evidenceRows,
  formatSignal,
  signalTitle,
  type SignalRow,
} from "../../../src/types/signals";
import type { ReportSignals } from "../../../src/types/report";

interface Props {
  signals: ReportSignals;
}

// [FUSION-CONTENT-GATE] Four confidence scores, the three pieces of measured
// content evidence behind them, and one plain-English reading of the two
// together. Drawing the confidence alone is what made a corroborated rename
// and an anchor-poor scaffolding family look identical here: both render
// structural 1.00 / jaccard 1.00, and only the evidence tells them apart.
export function SignalStrip({ signals }: Props) {
  return (
    <div>
      <SignalGrid rows={confidenceRows(signals)} />
      <div class="label" style={EVIDENCE_HEADING}>
        <HelpedText topic="content-evidence">CONTENT EVIDENCE</HelpedText>
      </div>
      <SignalGrid rows={evidenceRows(signals)} />
      <p style={VERDICT}>{contentEvidenceVerdict(signals)}</p>
    </div>
  );
}

function SignalGrid({ rows }: { rows: SignalRow[] }) {
  return (
    <div style={GRID}>
      {rows.map((row) => (
        <SignalCell key={row.label} row={row} />
      ))}
    </div>
  );
}

function SignalCell({ row }: { row: SignalRow }) {
  const title = signalTitle(row);
  return (
    <div title={title}>
      <div class="label" style={CELL_LABEL}>
        <HelpedText topic={row.topic} title={title}>{row.label}</HelpedText>
      </div>
      <div style={BAR_TRACK}>
        <div style={{ ...BAR_FILL, width: barWidth(row.value) }} />
      </div>
      <div style={CELL_VALUE}>{formatSignal(row.value)}</div>
    </div>
  );
}

function barWidth(value: number): string {
  return `${Math.round(Math.max(0, Math.min(1, value)) * 100)}%`;
}

const GRID: JSX.CSSProperties = {
  display: "grid",
  gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 120px), 1fr))",
  gap: "16px",
  padding: "16px 0",
};

const EVIDENCE_HEADING: JSX.CSSProperties = {
  color: COLOR.onSurfaceMuted,
  fontFamily: FONT.mono,
  borderTop: `1px solid ${COLOR.ghostBorder}`,
  paddingTop: "12px",
};

const CELL_LABEL: JSX.CSSProperties = {
  color: COLOR.onSurfaceMuted,
  marginBottom: "6px",
  fontFamily: FONT.mono,
};

const BAR_TRACK: JSX.CSSProperties = {
  height: "6px",
  background: COLOR.surfaceContainerLowest,
  overflow: "hidden",
  borderRadius: "2px",
};

const BAR_FILL: JSX.CSSProperties = {
  height: "100%",
  background: `linear-gradient(90deg, ${COLOR.primary} 0%, ${COLOR.primaryContainer} 100%)`,
};

const CELL_VALUE: JSX.CSSProperties = {
  marginTop: "4px",
  fontFamily: FONT.mono,
  fontSize: "12px",
};

const VERDICT: JSX.CSSProperties = {
  margin: "4px 0 0",
  color: COLOR.onSurface,
  fontSize: "12px",
  lineHeight: 1.5,
};
