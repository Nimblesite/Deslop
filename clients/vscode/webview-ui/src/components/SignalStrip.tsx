import { COLOR, FONT } from "../theme";
import type { ReportSignals } from "../../../src/types/report";

interface Props {
  signals: ReportSignals;
}

const SIGNAL_CONTEXT = {
  structural:
    "Structural score: how much the parsed AST shape matches. High means the code is organized almost the same way even if names or literals changed.",
  jaccard:
    "Jaccard score: normalized token overlap after Deslop removes trivia. High means the code text is very similar after formatting and naming noise are ignored.",
  embedding:
    "Embedding score: semantic similarity from the local embedding model. High means the code appears to do the same job even when the syntax differs.",
  fused:
    "Fused score: Deslop's combined clone signal. A pair generally becomes a reportable cluster when this reaches the configured threshold.",
} as const;

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
        <div key={label} title={signalTitle(label, value)}>
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
    </div>
  );
}

function signalTitle(label: string, value: number): string {
  const context = SIGNAL_CONTEXT[label as keyof typeof SIGNAL_CONTEXT];
  return `${context} Current value: ${value.toFixed(2)}.`;
}
