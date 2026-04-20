import { SEVERITY_COLOR, SEVERITY_DOT, FONT } from "../theme";
import type { Severity } from "../../../src/types/report";

export function SeverityBadge({ severity, label }: { severity: Severity; label?: string }) {
  const colour = SEVERITY_COLOR[severity];
  const dot = SEVERITY_DOT[severity];
  return (
    <span
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
      {label ?? severity}
    </span>
  );
}
