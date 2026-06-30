import { COLOR, FONT } from "../theme";

interface MetricHeadingProps {
  readonly value: number;
  readonly label: string;
}

export function MetricHeading({ value, label }: MetricHeadingProps) {
  return (
    <h1
      style={{
        margin: 0,
        fontFamily: FONT.ui,
        fontSize: "3rem",
        fontWeight: 700,
        letterSpacing: "-0.03em",
      }}
    >
      {value.toFixed(1)}%
      <span
        style={{
          fontSize: "1rem",
          fontFamily: FONT.mono,
          color: COLOR.onSurfaceMuted,
          marginLeft: "16px",
        }}
      >
        {label}
      </span>
    </h1>
  );
}
