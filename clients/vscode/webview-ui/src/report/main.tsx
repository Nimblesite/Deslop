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
import { FilterSelect } from "../components/FilterSelect";
import { MetricHeading } from "../components/MetricHeading";
import { SeverityBadge } from "../components/SeverityBadge";
import {
  BUCKETS,
  bucketLabels,
  CATEGORIES,
  categoryLabels,
  clusterSlug,
  occurrenceCount,
  resolveBucket,
  SEVERITIES,
  severityLabel,
  type Bucket,
  type Category,
  type Severity,
} from "../../../src/types/report";
import { LANGUAGES, languageDisplayName } from "../../../src/types/languages";

const GRID_DISPLAY = "grid";
const TWELVE_PIXEL_SIZE = "12px";
const LARGE_SPACING = "24px";
const MEDIUM_SPACING = "16px";
const RIGHT_ALIGNMENT = "right";
const MONOSPACE_CLASS = "mono";

// [FACET-REPORT-WEBVIEW] Every option list derives from the shared
// registries (the #170/#198 anti-drift rule): languages from the
// language registry, severities from SEVERITIES, buckets/categories
// from the canonical mirrors. `null` = no filter on that axis.
const LANG_OPTIONS = [
  { label: "Any language", value: null as string | null },
  ...LANGUAGES.map((id) => ({ label: languageDisplayName(id), value: id as string | null })),
];

const SEVERITY_OPTIONS = [
  { label: "All severities", value: null as Severity | null },
  ...SEVERITIES.map((severity) => ({
    label: severityLabel(severity),
    value: severity as Severity | null,
  })),
];

const BUCKET_OPTIONS = [
  { label: "All clone types", value: null as Bucket | null },
  ...BUCKETS.map((bucket) => ({
    label: bucketLabels(bucket).plainTitle,
    value: bucket as Bucket | null,
  })),
];

const CATEGORY_OPTIONS = [
  { label: "All categories", value: null as Category | null },
  ...CATEGORIES.map((category) => ({
    label: categoryLabels(category).groupTitle,
    value: category as Category | null,
  })),
];

function ReportApp() {
  const snapshot = report.value;
  const rows = filteredClusters.value;
  const state = analysisState.value;

  if (!snapshot) {
    return (
      <main style={{ padding: LARGE_SPACING }}>
        <p>Deslop is warming up…</p>
      </main>
    );
  }

  return (
    <main style={{ padding: "24px clamp(16px, 6vw, 32px)" }}>
      <header style={{ display: GRID_DISPLAY, gap: MEDIUM_SPACING, paddingBottom: LARGE_SPACING }}>
        <div
          class="label"
          style={{ fontFamily: FONT.mono, color: COLOR.onSurfaceMuted }}
        >
          DESLOP · REPORT · {snapshot.tool_version}
        </div>
        <div
          style={{
            display: GRID_DISPLAY,
            gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 240px), 1fr))",
            alignItems: "baseline",
            gap: LARGE_SPACING,
          }}
        >
          <MetricHeading value={snapshot.metrics.duplication_percent} label="duplicated" />
          <div
            style={{
              textAlign: RIGHT_ALIGNMENT,
              fontFamily: FONT.mono,
              fontSize: TWELVE_PIXEL_SIZE,
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
          display: GRID_DISPLAY,
          gridTemplateColumns: "repeat(auto-fit, minmax(min(100%, 150px), 1fr))",
          gap: TWELVE_PIXEL_SIZE,
          paddingBottom: MEDIUM_SPACING,
        }}
      >
        <FilterSelect
          options={LANG_OPTIONS}
          value={filters.value.language}
          onChange={(language) => (filters.value = { ...filters.value, language })}
        />
        <FilterSelect
          options={SEVERITY_OPTIONS}
          value={filters.value.severity}
          onChange={(severity) => (filters.value = { ...filters.value, severity })}
        />
        <FilterSelect
          options={BUCKET_OPTIONS}
          value={filters.value.bucket}
          onChange={(bucket) => (filters.value = { ...filters.value, bucket })}
        />
        <FilterSelect
          options={CATEGORY_OPTIONS}
          value={filters.value.category}
          onChange={(category) => (filters.value = { ...filters.value, category })}
        />
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
                display: GRID_DISPLAY,
                gridTemplateColumns: "auto minmax(0,1fr) auto auto",
                gap: MEDIUM_SPACING,
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
                  class={MONOSPACE_CLASS}
                  style={{ color: COLOR.onSurfaceMuted, fontSize: "11px", marginTop: "2px" }}
                >
                  {cluster.occurrences[0]?.path ?? "?"}
                </div>
              </div>
              <div class={MONOSPACE_CLASS} style={{ fontSize: TWELVE_PIXEL_SIZE, textAlign: RIGHT_ALIGNMENT }}>
                × {occurrenceCount(cluster)}
              </div>
              <div class={MONOSPACE_CLASS} style={{ fontSize: TWELVE_PIXEL_SIZE, textAlign: RIGHT_ALIGNMENT }}>
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
