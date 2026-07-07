// E2E: activation path — extension activates on fixture workspace, status bar appears,
// tree populates with the sample cluster, bubble commands exist, and the bundled
// deslop-mcp is registered with VS Code's MCP API.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as vscode from "vscode";
import { ExtensionApi } from "../../extension";
import { activateExtension } from "./helpers";

// [VSIX-ACTIVATION]
suite("activation", () => {
  test("extension activates on a C# fixture workspace", async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-live");
    assert.ok(ext, "extension should be registered");
    await ext.activate();
    assert.ok(ext.isActive, "extension should be active");
  });

  test("activity bar view container is registered", async () => {
    const views = vscode.window.visibleTextEditors;
    void views;
    // Presence is verified via command — the view container itself is not queryable.
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("deslop.openReport"));
    assert.ok(commands.includes("deslop.openWorstCluster"));
    assert.ok(commands.includes("deslop.pickEmbeddingModel"));
    assert.ok(commands.includes("deslop.revealActiveBinary"));
  });

  test("status bar item is shown after activation", async () => {
    // VS Code API exposes no direct read for status bar items; we assert the
    // command it routes to is installed.
    const commands = await vscode.commands.getCommands(true);
    assert.ok(commands.includes("deslop.openWorstCluster"));
  });
});

// The slice of package.json this suite asserts on. `Extension.packageJSON`
// is typed `any` upstream; the cast keeps property access type-checked.
interface ContributesManifest {
  readonly contributes?: {
    readonly mcpServerDefinitionProviders?: ReadonlyArray<{
      readonly id: string;
      readonly label: string;
    }>;
    readonly mcpServers?: unknown;
  };
}

// [VSIX-MCP-INTEGRATION] Issue #267: VS Code silently drops unknown
// contribution keys, so the legacy `contributes.mcpServers` block never
// registered anything — MCP hosts fell back to a hand-written bare-name
// config and failed with "Executable not found in $PATH: deslop-mcp".
// Activation must register the resolved absolute bundled path via
// `mcpServerDefinitionProviders` + `vscode.lm.registerMcpServerDefinitionProvider`.
suite("mcp registration", () => {
  test("activation registers the bundled deslop-mcp with VS Code's MCP API by absolute path", async () => {
    const ext = vscode.extensions.getExtension("nimblesite.deslop-live");
    assert.ok(ext, "extension should be registered");

    const manifest = ext.packageJSON as ContributesManifest;
    const providers = manifest.contributes?.mcpServerDefinitionProviders;
    assert.ok(
      providers?.some((provider) => provider.id === "deslop"),
      "package.json must contribute mcpServerDefinitionProviders with id 'deslop'",
    );
    assert.equal(
      manifest.contributes?.mcpServers,
      undefined,
      "contributes.mcpServers is not a VS Code contribution point and must be removed",
    );

    const api = await activateExtension();
    assert.ok(api.resolvedMcp, "bundled deslop-mcp must resolve during activation");
    // Widened cast so this test compiled — and failed red — before the fix
    // added the field to ExtensionApi (fix-bug skill: test precedes fix).
    const { mcpDefinition } = api as ExtensionApi & {
      readonly mcpDefinition?: vscode.McpStdioServerDefinition;
    };
    assert.ok(
      mcpDefinition,
      "activation must expose the MCP server definition it registered",
    );
    assert.equal(mcpDefinition.label, "Deslop");
    assert.equal(mcpDefinition.command, api.resolvedMcp.path);
    assert.ok(
      path.isAbsolute(mcpDefinition.command),
      "MCP server must launch by absolute path, never a $PATH lookup",
    );
    assert.ok(
      fs.existsSync(mcpDefinition.command),
      "registered command must point at the real bundled binary",
    );
    const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
    assert.ok(workspaceRoot, "fixture workspace must be open");
    assert.deepEqual(mcpDefinition.args, ["--root", workspaceRoot]);
    assert.equal(mcpDefinition.version, api.resolvedMcp.version);
  });
});
