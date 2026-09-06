import { SEVERITY_COLOR, SEVERITY_DOT, FONT } from "../theme";
import type { Severity } from "../../../src/types/report";

export function SeverityBadge({
  severity,
  label,
  title,
}: {
  severity: Severity;
  label?: string;
  title?: string;
}) {
  const colour = SEVERITY_COLOR[severity];
  const dot = SEVERITY_DOT[severity];
  return (
    <span
      title={title ?? severityTitle(severity)}
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

function severityTitle(severity: Severity): string {
  switch (severity) {
    case "worst":
      return "Worst severity: this cluster is at the very top of the current report by duplication impact.";
    case "top10":
      return "High severity: this cluster is in the top tenth of the current report by duplication impact.";
    case "mid":
      return "Medium severity: this cluster is in the upper half of the current report by duplication impact.";
    case "faint":
      return "Low severity: this cluster is below the upper half of the current report but still matched Deslop's clone threshold.";
  }
}
