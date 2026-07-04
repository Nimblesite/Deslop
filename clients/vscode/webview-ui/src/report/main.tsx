import { render } from "preact";

import {
  analysisState,
  filteredClusters,
  filters,
  post,
  report,
  selectedClusterId,
  severityByClusterId,
  wireMessagePump,
} from "../store";
import { COLOR, FONT, GLOBAL_CSS } from "../theme";
import { MetricHeading } from "../components/MetricHeading";
import { SeverityBadge } from "../components/SeverityBadge";
import {
  bucketLabels,
  clusterSlug,
  occurrenceCount,
  resolveBucket,
  type Severity,
} from "../../../src/types/report";

const LANG_OPTIONS: { label: string; value: string | null }[] = [
  { label: "Any language", value: null },
  { label: "C#", value: ".cs" },
  { label: "Rust", value: ".rs" },
  { label: "Python", value: ".py" },
];

const SEVERITY_OPTIONS: { label: string; value: Severity | null }[] = [
  { label: "All severities", value: null },
  { label: "Worst 1%", value: "worst" },
  { label: "Top 10%", value: "top10" },
  { label: "Mid 40%", value: "mid" },
];

function ReportApp() {
  const snapshot = report.value;
  const rows = filteredClusters.value;
  const state = analysisState.value;

  if (!snapshot) {
    return (
      <main style={{ padding: "24px" }}>
        <p>Deslop is warming up…</p>
      </main>
    );
  }

  return (
    <main style={{ padding: "24px clamp(16px, 6vw, 32px)" }}>
      <header style={{ display: "grid", gap: "16px", paddingBottom: "24px" }}>
        <div
          class="label"
          style={{ fontFamily: FONT.mono, color: COLOR.onSurfaceMuted }}
        >
          DESLOP · REPORT · {snapshot.tool_version}
        </div>
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 240px), 1fr))",
            alignItems: "baseline",
            gap: "24px",
          }}
        >
          <MetricHeading value={snapshot.metrics.duplication_percent} label="duplicated" />
          <div
            style={{
              textAlign: "right",
              fontFamily: FONT.mono,
              fontSize: "12px",
              minWidth: 0,
            }}
          >
            <div>
              {snapshot.metrics.duplicated_loc}/{snapshot.metrics.analysed_loc} LOC
            </div>
            <div>{snapshot.clusters.length} clusters</div>
            <div>
              cache {snapshot.cache_stats.hits} hit · {snapshot.cache_stats.misses} miss
            </div>
            <div>analysis: {state.state}</div>
          </div>
        </div>
      </header>

      <section
        style={{
          display: "grid",
          gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 150px), 1fr))",
          gap: "12px",
          paddingBottom: "16px",
        }}
      >
        <select
          value={filters.value.language ?? ""}
          onChange={(event) => {
            const raw = (event.currentTarget as HTMLSelectElement).value;
            filters.value = { ...filters.value, language: raw === "" ? null : raw };
          }}
        >
          {LANG_OPTIONS.map((o) => (
            <option key={o.label} value={o.value ?? ""}>{o.label}</option>
          ))}
        </select>
        <select
          value={filters.value.severity ?? ""}
          onChange={(event) => {
            const raw = (event.currentTarget as HTMLSelectElement).value;
            filters.value = {
              ...filters.value,
              severity: raw === "" ? null : (raw as Severity),
            };
          }}
        >
          {SEVERITY_OPTIONS.map((o) => (
            <option key={o.label} value={o.value ?? ""}>{o.label}</option>
          ))}
        </select>
        <input
          type="text"
          placeholder="path glob (e.g. src/)"
          value={filters.value.pathGlob}
          onInput={(event) => {
            const raw = (event.currentTarget as HTMLInputElement).value;
            filters.value = { ...filters.value, pathGlob: raw };
          }}
        />
        <button onClick={() => post({ kind: "refresh" })} class="primary">
          Refresh
        </button>
      </section>

      <ol style={{ margin: 0, padding: 0, listStyle: "none" }}>
        {rows.map((cluster, i) => {
          const severity = severityByClusterId.value.get(cluster.id) ?? "faint";
          const slug = clusterSlug(cluster);
          return (
            <li
              key={cluster.id}
              onClick={() => {
                selectedClusterId.value = cluster.id;
                post({ kind: "open/cluster", id: cluster.id });
              }}
              style={{
                display: "grid",
                gridTemplateColumns: "auto minmax(0,1fr) auto auto",
                gap: "16px",
                alignItems: "center",
                padding: "12px 20px",
                background: i % 2 === 0 ? COLOR.surfaceContainerLow : COLOR.surface,
                cursor: "pointer",
              }}
              onMouseEnter={(event) =>
                ((event.currentTarget as HTMLElement).style.background =
                  COLOR.surfaceContainerHighest)
              }
              onMouseLeave={(event) =>
                ((event.currentTarget as HTMLElement).style.background =
                  i % 2 === 0 ? COLOR.surfaceContainerLow : COLOR.surface)
              }
            >
              <SeverityBadge severity={severity} label={`${slug}`} />
              <div style={{ minWidth: 0 }}>
                <div
                  style={{
                    fontFamily: FONT.ui,
                    fontSize: "14px",
                    display: "flex",
                    alignItems: "center",
                    gap: "8px",
                    flexWrap: "wrap",
                    minWidth: 0,
                  }}
                >
                  <span style={{ fontWeight: 600, overflowWrap: "anywhere" }}>
                    {bucketLabels(resolveBucket(cluster)).plainTitle}
                  </span>
                  {bucketLabels(resolveBucket(cluster)).aiMatch ? (
                    <span
                      style={{
                        background: COLOR.secondaryContainer ?? COLOR.surfaceContainerLow,
                        color: COLOR.onSurface,
                        padding: "1px 5px",
                        borderRadius: "3px",
                        fontSize: "9px",
                        letterSpacing: "0.1em",
                        fontWeight: 700,
                      }}
                      title="Detected by the AI embedding pass."
                    >
                      AI
                    </span>
                  ) : null}
                </div>
                <div
                  class="mono"
                  style={{ color: COLOR.onSurfaceMuted, fontSize: "11px", marginTop: "2px" }}
                >
                  {cluster.occurrences[0]?.path ?? "?"}
                </div>
              </div>
              <div class="mono" style={{ fontSize: "12px", textAlign: "right" }}>
                × {occurrenceCount(cluster)}
              </div>
              <div class="mono" style={{ fontSize: "12px", textAlign: "right" }}>
                w {cluster.weight.toFixed(1)}
              </div>
            </li>
          );
        })}
      </ol>
    </main>
  );
}

wireMessagePump();
const style = document.createElement("style");
style.textContent = GLOBAL_CSS;
document.head.appendChild(style);
const root = document.getElementById("root");
if (root) render(<ReportApp />, root);
