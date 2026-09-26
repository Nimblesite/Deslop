import { KIND_COLOR, SEVERITY_DOT, FONT } from "../theme";
import { kindChip, kindTaxonomy, kindTitle, severityLabel, type ClusterKind, type Severity } from "../../../src/types/report";

// [CLONE-KIND-COLOR] Colour shows the category; the glyph shows diagnostic severity.
// [VSIX-CLONE-TYPE-CHIP] The same compact taxonomy appears on every cluster badge.
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
      <span
        style={{
          border: `1px solid ${colour}`,
          borderRadius: "3px",
          padding: "1px 4px",
          whiteSpace: "nowrap",
        }}
      >
        {kindChip(kind)}
      </span>
      <span>{label ?? kindTitle(kind)}</span>
    </span>
  );
}

function badgeTitle(kind: ClusterKind, severity: Severity): string {
  return `${kindTitle(kind)} (${kindTaxonomy(kind)}). ${severityTitle(severity)}`;
}

function severityTitle(severity: Severity): string {
  return `Diagnostic: ${severityLabel(severity)}.`;
}
