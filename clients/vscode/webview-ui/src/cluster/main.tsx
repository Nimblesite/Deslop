import { render } from "preact";
import { useEffect } from "preact/hooks";

import {
  analysisState,
  clusters,
  post,
  selectedCluster,
  selectedClusterId,
  severityByClusterId,
  wireMessagePump,
} from "../store";
import { COLOR, FONT, GLOBAL_CSS, SEVERITY_COLOR } from "../theme";
import { SignalStrip } from "../components/SignalStrip";
import { SeverityBadge } from "../components/SeverityBadge";
import { bucketLabels, resolveBucket } from "../../../src/types/report";

function ClusterApp() {
  const cluster = selectedCluster.value;
  const list = clusters.value;
  const rank = cluster ? list.findIndex((c) => c.id === cluster.id) + 1 : 0;
  const severity = cluster ? severityByClusterId.value.get(cluster.id) ?? "faint" : "faint";

  useEffect(() => {
    const handler = (event: KeyboardEvent) => {
      if (event.key === "n" && list.length > 0) {
        const idx = (rank === 0 ? 0 : rank) % list.length;
        const next = list[idx];
        if (next) selectedClusterId.value = next.id;
      }
      if (event.key === "p" && list.length > 0) {
        const idx = rank <= 1 ? list.length - 1 : rank - 2;
        const next = list[idx];
        if (next) selectedClusterId.value = next.id;
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [rank, list]);

  if (!cluster) {
    return (
      <main style={{ padding: "24px" }}>
        <p>No cluster selected.</p>
      </main>
    );
  }

  const canonical = cluster.occurrences[0];
  const bucketInfo = bucketLabels(resolveBucket(cluster));

  return (
    <main
      style={{
        padding: "24px 32px",
        opacity: analysisState.value === "errored" ? 0.5 : 1,
        transition: "opacity 120ms",
      }}
    >
      <header
        style={{
          display: "grid",
          gridTemplateColumns: "minmax(0, 1fr) auto",
          gap: "24px",
          alignItems: "start",
          paddingBottom: "24px",
        }}
      >
        <div>
          <div
            class="label"
            style={{
              color: COLOR.onSurfaceMuted,
              marginBottom: "8px",
              fontFamily: FONT.mono,
              display: "flex",
              alignItems: "center",
              gap: "8px",
            }}
          >
            <span>CLUSTER · {cluster.id}</span>
            {bucketInfo.aiMatch ? (
              <span
                style={{
                  background: COLOR.secondaryContainer ?? COLOR.surfaceContainerLow,
                  color: COLOR.onSurface,
                  padding: "2px 6px",
                  borderRadius: "3px",
                  fontSize: "10px",
                  letterSpacing: "0.1em",
                  fontWeight: 700,
                }}
                title="Detected by the AI embedding pass — semantically equivalent, syntactically different."
              >
                AI MATCH
              </span>
            ) : null}
          </div>
          <h1
            style={{
              margin: 0,
              fontFamily: FONT.ui,
              fontSize: "2.25rem",
              fontWeight: 700,
              letterSpacing: "-0.02em",
            }}
          >
            {bucketInfo.plainTitle}
          </h1>
          <p
            style={{
              margin: "12px 0 0",
              color: COLOR.onSurfaceMuted,
              fontFamily: FONT.ui,
              fontSize: "15px",
            }}
          >
            {bucketInfo.actionSentence}
          </p>
        </div>
        <div style={{ textAlign: "right" }}>
          <SeverityBadge severity={severity} label={`#${rank || "?"}`} />
          <div
            style={{
              fontFamily: FONT.mono,
              color: COLOR.onSurfaceMuted,
              marginTop: "12px",
              fontSize: "12px",
            }}
          >
            weight {cluster.weight.toFixed(2)} · size {cluster.size} · × {cluster.occurrences.length}
          </div>
          {canonical ? (
            <div style={{ fontFamily: FONT.mono, fontSize: "12px", marginTop: "4px" }}>
              canonical: {canonical.path}
            </div>
          ) : null}
        </div>
      </header>

      <section style={{ background: COLOR.surfaceContainerLow, padding: "16px 24px" }}>
        <SignalStrip signals={cluster.signals} />
      </section>

      <section style={{ marginTop: "24px" }}>
        <div
          class="label"
          style={{ color: COLOR.onSurfaceMuted, marginBottom: "12px", fontFamily: FONT.mono }}
        >
          OCCURRENCES
        </div>
        {cluster.occurrences.map((o, i) => (
          <article
            key={`${o.path}-${o.start_byte}`}
            style={{
              background: i % 2 === 0 ? COLOR.surfaceContainerLow : COLOR.surface,
              padding: "14px 20px",
              display: "grid",
              gridTemplateColumns: "minmax(0, 1fr) auto",
              gap: "16px",
              alignItems: "center",
            }}
          >
            <div>
              <div style={{ fontFamily: FONT.mono, fontSize: "12px" }}>
                {o.displayLocation?.label ?? o.path}
              </div>
              <div
                style={{
                  fontFamily: FONT.mono,
                  color: COLOR.onSurfaceMuted,
                  fontSize: "11px",
                  marginTop: "2px",
                }}
              >
                {o.displayLocation?.description ??
                  "line and column unavailable until the file is loaded"}
                {o.hidden ? " · hidden" : ""}
              </div>
            </div>
            <div style={{ display: "flex", gap: "8px" }}>
              <button onClick={() => post({ kind: "open/occurrence", occurrence: o })}>
                Open
              </button>
              <button
                class={i === 0 ? "" : "primary"}
                onClick={() =>
                  post({
                    kind: "compare/canonical",
                    clusterId: cluster.id,
                  })
                }
                disabled={i === 0}
                style={i === 0 ? { opacity: 0.3 } : { color: "inherit" }}
              >
                Compare
              </button>
            </div>
          </article>
        ))}
      </section>

      <footer
        style={{
          marginTop: "24px",
          display: "flex",
          gap: "12px",
          justifyContent: "flex-end",
        }}
      >
        <button onClick={() => post({ kind: "navigate/prev" })}>← prev cluster (p)</button>
        <button onClick={() => post({ kind: "navigate/next" })}>next cluster (n) →</button>
      </footer>
      <HotkeyHelp accent={SEVERITY_COLOR[severity]} />
    </main>
  );
}

function HotkeyHelp({ accent }: { accent: string }) {
  return (
    <div
      class="mono"
      style={{
        marginTop: "32px",
        fontSize: "11px",
        color: COLOR.onSurfaceMuted,
      }}
    >
      <span style={{ color: accent }}>j/k</span> next/prev occurrence · <span style={{ color: accent }}>n/p</span> next/prev cluster · <span style={{ color: accent }}>Enter</span> open · <span style={{ color: accent }}>?</span> help
    </div>
  );
}

wireMessagePump();
const style = document.createElement("style");
style.textContent = GLOBAL_CSS;
document.head.appendChild(style);
const root = document.getElementById("root");
if (root) render(<ClusterApp />, root);
