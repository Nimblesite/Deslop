import { KIND_COLOR, SEVERITY_DOT, FONT } from "../theme";
import { kindTitle, type ClusterKind, type Severity } from "../../../src/types/report";

// [CLONE-KIND-COLOR] The badge is coloured by the cluster's clone kind and
// carries the mass rank band as its glyph — the two channels every cluster
// surface shows, never mixed.
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
  switch (severity) {
    case "worst":
      return "Worst severity: this cluster is at the very top of the current report by duplicated mass.";
    case "top10":
      return "High severity: this cluster is in the top tenth of the current report by duplicated mass.";
    case "mid":
      return "Medium severity: this cluster is in the upper half of the current report by duplicated mass.";
    case "faint":
      return "Low severity: this cluster is below the upper half of the current report but still matched Deslop's clone threshold.";
  }
}
