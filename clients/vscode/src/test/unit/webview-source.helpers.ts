// Shared TS-AST parsing and walking for source-level pins. The Preact
// webviews cannot run under vscode-test (no DOM, separate esbuild bundle),
// so unit tests inspect the parsed source tree instead of executing it, and
// extension-host render paths are pinned the same way. One parser and one
// walker, reused by every source test — no per-file copies.

import * as fs from "node:fs";
import * as path from "node:path";
import * as ts from "typescript";

/** Absolute path to a file under clients/vscode/webview-ui/src. */
export function webviewUiPath(relativePath: string): string {
  return path.resolve(__dirname, "../../../webview-ui/src", relativePath);
}

/** Absolute path to a file under clients/vscode. */
export function extensionPath(relativePath: string): string {
  return path.resolve(__dirname, "../../..", relativePath);
}

/** Parses any TypeScript or TSX file into a TypeScript AST. */
export function parseTypeScriptSource(sourcePath: string, scriptKind: ts.ScriptKind): ts.SourceFile {
  const source = fs.readFileSync(sourcePath, "utf8");
  return ts.createSourceFile(sourcePath, source, ts.ScriptTarget.Latest, true, scriptKind);
}

/** Parses a webview-ui source file into a TypeScript AST. */
export function parseWebviewSource(
  relativePath: string,
  scriptKind: ts.ScriptKind = ts.ScriptKind.TSX,
): ts.SourceFile {
  return parseTypeScriptSource(webviewUiPath(relativePath), scriptKind);
}

/** Every node under `root` (inclusive) that satisfies `predicate`, in
 * source order. */
export function descendants(root: ts.Node, predicate: (node: ts.Node) => boolean): ts.Node[] {
  const matches: ts.Node[] = [];
  function visit(node: ts.Node): void {
    if (predicate(node)) matches.push(node);
    node.forEachChild(visit);
  }
  visit(root);
  return matches;
}

/** Whether any node under `node` (inclusive) satisfies `predicate`. */
export function hasDescendant(node: ts.Node, predicate: (node: ts.Node) => boolean): boolean {
  if (predicate(node)) return true;
  let found = false;
  node.forEachChild((child) => {
    if (!found) found = hasDescendant(child, predicate);
  });
  return found;
}
