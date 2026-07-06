// Unit: wireMcpRegistration — the [VSIX-MCP-INTEGRATION] activation glue
// (#267) that the E2E suite only reaches through the bundled dist/
// entrypoint, so it never lands in the instrumented out/ coverage. Exercised
// directly here (same pattern as extension-glue.unit.test.ts) against the
// real vscode.lm API in the test host.

import * as assert from "node:assert/strict";
import * as vscode from "vscode";
import { ResolvedBinary } from "../../binary";
import { wireMcpRegistration } from "../../mcpRegistration";

function resolvedMcpBinary(): ResolvedBinary {
  return {
    kind: "mcp",
    componentId: "deslop-mcp",
    source: "bundled",
    path: "/tmp/deslop-mcp",
    version: "1.0.0",
  };
}

function fakeContext(): {
  context: vscode.ExtensionContext;
  subscriptions: vscode.Disposable[];
} {
  const subscriptions: vscode.Disposable[] = [];
  const context = { subscriptions } as unknown as vscode.ExtensionContext;
  return { context, subscriptions };
}

suite("mcp registration glue", () => {
  test("wireMcpRegistration skips without a resolved binary", () => {
    const { context, subscriptions } = fakeContext();
    const definition = wireMcpRegistration(context, undefined, "/tmp/root");
    assert.equal(definition, undefined);
    assert.equal(subscriptions.length, 0, "nothing must be registered");
  });

  test("wireMcpRegistration skips without a workspace root", () => {
    const { context, subscriptions } = fakeContext();
    const definition = wireMcpRegistration(
      context,
      resolvedMcpBinary(),
      undefined,
    );
    assert.equal(definition, undefined);
    assert.equal(subscriptions.length, 0, "nothing must be registered");
  });

  test("wireMcpRegistration registers a stdio definition for the absolute binary path", () => {
    const { context, subscriptions } = fakeContext();
    const definition = wireMcpRegistration(
      context,
      resolvedMcpBinary(),
      "/tmp/fixture-root",
    );
    try {
      assert.ok(definition, "definition must be returned");
      assert.ok(definition instanceof vscode.McpStdioServerDefinition);
      assert.equal(definition.label, "Deslop");
      assert.equal(definition.command, "/tmp/deslop-mcp");
      assert.deepEqual(definition.args, ["--root", "/tmp/fixture-root"]);
      assert.deepEqual(definition.env, {});
      assert.equal(definition.version, "1.0.0");
      assert.equal(
        subscriptions.length,
        1,
        "provider disposable must be pushed onto the context",
      );
    } finally {
      // Unregister so the host is left exactly as the activated extension
      // configured it (its own provider stays live).
      for (const disposable of subscriptions) disposable.dispose();
    }
  });
});
