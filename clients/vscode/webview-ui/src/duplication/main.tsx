// [VSIX-METRICS-REPORT] Duplication report webview. Renders the headline
// duplication score plus a per-folder / per-file breakdown. Every figure
// is read verbatim off the wire (`metrics.folders` / `metrics.per_file`,
// computed by the engine's single `percent` function per [METRICS-REPO]);
// `buildFolderRollup` only nests the rows, so this table, the sidebar
// Duplication panel, and the CLI can never disagree.

import { render } from "preact";

import { report, wireMessagePump } from "../store";
import { COLOR, FONT, GLOBAL_CSS } from "../theme";
import { MetricHeading } from "../components/MetricHeading";
import { buildFolderRollup, type RollupChild } from "../../../src/tree/rollup";
import { thresholdStatus } from "../../../src/tree/threshold";
import { formatPercent } from "../../../src/types/format";

function percentColor(percent: number): string {
  if (percent >= 30) return "#e5534b";
  if (percent >= 10) return "#d4a72c";
  return COLOR.onSurfaceMuted;
}

function leafName(path: string): string {
  return path.split(/[/\\]/).filter(Boolean).pop() ?? path;
}

function Row({ child, depth }: { child: RollupChild; depth: number }) {
  const isFolder = child.kind === "folder";
  const name = isFolder ? child.folder.label : leafName(child.file.path);
  const detail = isFolder
    ? `${child.folder.duplicatedLoc}/${child.folder.analysedLoc} LOC`
    : `${child.file.duplicated_loc}/${child.file.analysed_loc} LOC`;
  return (
    <>
      <li
        style={{
          display: "grid",
          gridTemplateColumns: "minmax(0,1fr) auto auto",
          gap: "16px",
          alignItems: "center",
          padding: "6px 8px",
          paddingLeft: `${8 + depth * 18}px`,
          borderTop: `1px solid ${COLOR.surfaceContainerLow}`,
        }}
      >
        <span
          style={{
            fontFamily: isFolder ? FONT.ui : FONT.mono,
            fontWeight: isFolder ? 600 : 400,
            fontSize: "13px",
            whiteSpace: "nowrap",
            overflow: "hidden",
            textOverflow: "ellipsis",
          }}
          title={isFolder ? child.folder.path : child.file.path}
        >
          {isFolder ? "📁 " : ""}
          {name}
        </span>
        <span class="mono" style={{ fontSize: "11px", color: COLOR.onSurfaceMuted }}>
          {detail}
        </span>
        <span
          class="mono"
          style={{
            fontSize: "13px",
            fontWeight: 600,
            color: percentColor(child.percent),
            textAlign: "right",
            minWidth: "62px",
          }}
        >
          {formatPercent(child.percent)}
        </span>
      </li>
      {isFolder
        ? child.folder.children.map((grandchild, index) => (
            <Row key={index} child={grandchild} depth={depth + 1} />
          ))
        : null}
    </>
  );
}

function DuplicationApp() {
  const snapshot = report.value;
  if (!snapshot) {
    return (
      <main style={{ padding: "24px" }}>
        <p>Deslop is warming up…</p>
      </main>
    );
  }
  const metrics = snapshot.metrics;
  const rows = buildFolderRollup(metrics);
  const status = thresholdStatus(metrics.threshold);
  const gate = status.configured ? ` · ${status.label}` : "";
  return (
    <main style={{ padding: "24px 32px" }}>
      <header style={{ display: "grid", gap: "12px", paddingBottom: "20px" }}>
        <div class="label" style={{ fontFamily: FONT.mono, color: COLOR.onSurfaceMuted }}>
          DESLOP · DUPLICATION · {snapshot.tool_version}
        </div>
        <MetricHeading value={metrics.duplication_percent} label="duplicated" />
        <div class="mono" style={{ fontSize: "12px", color: COLOR.onSurfaceMuted }}>
          {metrics.duplicated_loc}/{metrics.analysed_loc} LOC · {metrics.clusters_total} clusters ·{" "}
          {metrics.duplicated_files} files{gate}
        </div>
      </header>
      {rows.length === 0 ? (
        <p style={{ color: COLOR.onSurfaceMuted }}>No duplication detected.</p>
      ) : (
        <ol style={{ margin: 0, padding: 0, listStyle: "none" }}>
          {rows.map((child, index) => (
            <Row key={index} child={child} depth={0} />
          ))}
        </ol>
      )}
    </main>
  );
}

wireMessagePump();
const style = document.createElement("style");
style.textContent = GLOBAL_CSS;
document.head.appendChild(style);
const root = document.getElementById("root");
if (root) render(<DuplicationApp />, root);
