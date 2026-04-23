import { COLOR, FONT } from "../theme";
import { HelpedText, helpCopy, type HelpTopic } from "./HelpBubble";
import type { ReportSignals } from "../../../src/types/report";

interface Props {
  signals: ReportSignals;
}

export function SignalStrip({ signals }: Props) {
  const values: [HelpTopic, string, number][] = [
    ["structural", "structural", signals.structural],
    ["jaccard", "jaccard", signals.token_jaccard],
    ["embedding", "embedding", signals.embedding_cos],
    ["fused", "fused", signals.fused],
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
      {values.map(([topic, label, value]) => (
        <div key={label} title={signalTitle(topic, value)}>
          <div
            class="label"
            style={{
              color: COLOR.onSurfaceMuted,
              marginBottom: "6px",
              fontFamily: FONT.mono,
            }}
          >
            <HelpedText topic={topic} title={signalTitle(topic, value)}>{label}</HelpedText>
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

function signalTitle(topic: HelpTopic, value: number): string {
  return `${helpCopy(topic)} Current value: ${value.toFixed(2)}.`;
}
