// Shared TS-AST parsing for webview-ui sources. The Preact webviews cannot run
// under vscode-test (no DOM, separate esbuild bundle), so unit tests inspect the
// parsed source tree instead of executing it. One parser, reused by every
// webview source test — no per-file copies.

import * as fs from "node:fs";
import * as path from "node:path";
import * as ts from "typescript";

/** Absolute path to a file under clients/vscode/webview-ui/src. */
export function webviewUiPath(relativePath: string): string {
  return path.resolve(__dirname, "../../../webview-ui/src", relativePath);
}

/** Parses a webview-ui source file into a TypeScript AST. */
export function parseWebviewSource(
  relativePath: string,
  scriptKind: ts.ScriptKind = ts.ScriptKind.TSX,
): ts.SourceFile {
  const sourcePath = webviewUiPath(relativePath);
  const source = fs.readFileSync(sourcePath, "utf8");
  return ts.createSourceFile(sourcePath, source, ts.ScriptTarget.Latest, true, scriptKind);
}
