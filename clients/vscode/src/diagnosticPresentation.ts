import * as vscode from "vscode";
import { ReportStore } from "./reportStore";
import { CLUSTER_KINDS, SEVERITIES, type Severity, type SeverityOverrides } from "./types/report";

const SEVERITY_SETTING = "deslop.diagnostics.severityByKind";

/** [SEVERITY-MODEL] Facets stay available when diagnostic publication is disabled. */
export function watchDiagnosticPresentation(store: ReportStore): vscode.Disposable {
  const refresh = (): void => store.setSeverityOverrides(readSeverityOverrides());
  refresh();
  return vscode.workspace.onDidChangeConfiguration((event) => {
    if (event.affectsConfiguration(SEVERITY_SETTING)) refresh();
  });
}

function readSeverityOverrides(): SeverityOverrides {
  const values = vscode.workspace.getConfiguration("deslop").get<Record<string, unknown>>("diagnostics.severityByKind", {});
  const overrides: SeverityOverrides = {};
  for (const kind of CLUSTER_KINDS) {
    const level = values[kind];
    if (SEVERITIES.some((severity) => severity === level)) overrides[kind] = level as Severity;
  }
  return overrides;
}
