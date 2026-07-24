// [VSIX-MCP-INTEGRATION] Registers the resolved deslop-mcp with VS Code's
// MCP API so MCP-aware agent hosts (Copilot Chat, Claude Code) launch the
// bundled binary by absolute path — never a bare-name $PATH lookup (#267).
// The provider id must match `contributes.mcpServerDefinitionProviders` in
// package.json; VS Code throws at registration otherwise, failing loudly.

import * as vscode from "vscode";
import { ResolvedBinary } from "./binary";
import { log } from "./logging";

function deslopServerDefinition(
  mcp: ResolvedBinary,
  workspaceRoot: string,
): vscode.McpStdioServerDefinition {
  return new vscode.McpStdioServerDefinition(
    "Deslop",
    mcp.path,
    ["--root", workspaceRoot],
    {},
    mcp.version,
  );
}

/// Wires the single "deslop" stdio MCP server against the given workspace
/// root — the same root the LSP analyses. Returns the registered definition
/// (exposed on the extension API for tests), or undefined when there is no
/// resolved binary or no open workspace to analyse.
export function wireMcpRegistration(
  context: vscode.ExtensionContext,
  mcp: ResolvedBinary | undefined,
  workspaceRoot: string | undefined,
): vscode.McpStdioServerDefinition | undefined {
  if (!mcp || !workspaceRoot) {
    log("mcp registration skipped", {
      binaryResolved: mcp !== undefined,
      workspaceOpen: Boolean(workspaceRoot),
    });
    return undefined;
  }
  const definition = deslopServerDefinition(mcp, workspaceRoot);
  context.subscriptions.push(
    vscode.lm.registerMcpServerDefinitionProvider("deslop", {
      provideMcpServerDefinitions: () => [definition],
    }),
  );
  log("mcp registered", { source: mcp.source, version: mcp.version });
  return definition;
}
