import { COLOR, FONT } from "../theme";
import type { ReportSignals } from "../../../src/types/report";

interface Props {
  signals: ReportSignals;
}

const LABELS = ["structural", "jaccard", "embedding", "fused"] as const;

export function SignalStrip({ signals }: Props) {
  const values: [string, number][] = [
    ["structural", signals.structural],
    ["jaccard", signals.token_jaccard],
    ["embedding", signals.embedding_cos],
    ["fused", signals.fused],
  ];
  return (
    <div
      style={{
        display: "grid",
        gridTemplateColumns: "repeat(4, 1fr)",
        gap: "16px",
        padding: "16px 0",
      }}
    >
      {values.map(([label, value]) => (
        <div key={label}>
          <div
            class="label"
            style={{
              color: COLOR.onSurfaceMuted,
              marginBottom: "6px",
              fontFamily: FONT.mono,
            }}
          >
            {label}
          </div>
          <div
            style={{
              height: "6px",
              background: COLOR.surfaceContainerLowest,
              overflow: "hidden",
              borderRadius: "2px",
            }}
          >
            <div
              style={{
                width: `${Math.round(Math.max(0, Math.min(1, value)) * 100)}%`,
                height: "100%",
                background: `linear-gradient(90deg, ${COLOR.primary} 0%, ${COLOR.primaryContainer} 100%)`,
              }}
            />
          </div>
          <div
            style={{
              marginTop: "4px",
              fontFamily: FONT.mono,
              fontSize: "12px",
            }}
          >
            {value.toFixed(2)}
          </div>
        </div>
      ))}
      {/* LABELS unused re-export to silence tree-shake if styling later needs it */}
      <span style={{ display: "none" }}>{LABELS.join(",")}</span>
    </div>
  );
}
