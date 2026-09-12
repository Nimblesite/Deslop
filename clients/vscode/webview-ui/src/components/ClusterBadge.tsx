import { KIND_COLOR, SEVERITY_DOT, FONT } from "../theme";
import { kindTitle, severityLabel, type ClusterKind, type Severity } from "../../../src/types/report";

// [CLONE-KIND-COLOR] Colour shows the category; the glyph shows diagnostic severity.
export function ClusterBadge({
  kind,
  severity,
  label,
  title,
}: {
  kind: ClusterKind;
  severity: Severity;
  label?: string;
  title?: string;
}) {
  const colour = KIND_COLOR[kind];
  const dot = SEVERITY_DOT[severity];
  return (
    <span
      title={title ?? badgeTitle(kind, severity)}
      style={{
        display: "inline-flex",
        alignItems: "center",
        gap: "8px",
        padding: "2px 10px",
        fontFamily: FONT.mono,
        fontSize: "11px",
        letterSpacing: "0.06em",
        textTransform: "uppercase",
        color: colour,
        background: `${colour}1a`,
      }}
    >
      <span aria-hidden>{dot}</span>
      {label ?? kindTitle(kind)}
    </span>
  );
}

function badgeTitle(kind: ClusterKind, severity: Severity): string {
  return `${kindTitle(kind)}. ${severityTitle(severity)}`;
}

function severityTitle(severity: Severity): string {
  return `Diagnostic: ${severityLabel(severity)}.`;
}
