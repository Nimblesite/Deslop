// Unit: [CLONE-KIND-COLOR] the extension host holds exactly one table that
// maps a cluster to a colour.
//
// gh #522: three tables were live at once. One was keyed by mass rank band and
// its own comment called using it to paint "a defect"; a second was keyed by an
// evidence level that was itself computed from the rank band and renamed; a
// third sat privately in the tree and painted from VS Code's chart palette
// rather than the Deslop tokens every other surface used. The same finding could
// therefore be one colour in the Top Offenders list and another in the editor,
// and no comment in the file described what actually shipped.
//
// The rule is not "these particular tables are gone" — a fourth could be added
// tomorrow. It is that the host declares one colour table, so this walks every
// source file's AST and fails on any second object literal whose values are
// colour tokens.

import * as assert from "node:assert/strict";
import * as fs from "node:fs";
import * as path from "node:path";
import * as ts from "typescript";

import { KIND_COLOR } from "../../design";
import { descendants, extensionPath, parseTypeScriptSource } from "./webview-source.helpers";

/** The extension-host sources this rule governs. */
const HOST_SOURCE_ROOT = "src";

/** The one file allowed to declare a colour table. */
const PAINT_TABLE_FILE = path.join("src", "design.ts");

/** The declaration that is the paint table. */
const PAINT_TABLE_NAME = "KIND_COLOR";

/** Directories holding no shipped host source. */
const EXCLUDED_DIRECTORIES = ["test"];

/** The token namespace every Deslop surface paints from. */
const COLOR_NAMESPACE = "COLOR";

/** The palette a Deslop surface may never paint a cluster from: VS Code's own
 * chart colours, which are a different set of colours for the same finding. */
const FOREIGN_PALETTE_PREFIX = "charts.";

/** Keys that mean "how big is this finding", never "how well does it match". */
const RANK_BAND_KEYS = ["worst", "top10", "mid", "faint"];

/** How many colour tables the host may declare. */
const ALLOWED_PAINT_TABLES = 1;

suite("[CLONE-KIND-COLOR] one paint table for the extension host", () => {
  test("no source outside design.ts declares a second colour table", () => {
    const offenders = hostSources()
      .filter((source) => !source.endsWith(PAINT_TABLE_FILE))
      .flatMap((source) => colourTablesIn(source));
    assert.deepEqual(
      offenders,
      [],
      `only ${PAINT_TABLE_FILE} may map a cluster to a colour; a second table lets ` +
        `the tree and the editor disagree about the same finding (gh #522). Found: ` +
        `${offenders.join(", ")}`,
    );
  });

  test("design.ts declares exactly one colour table, and it is the kind table", () => {
    const declared = colourTablesIn(extensionPath(PAINT_TABLE_FILE));
    assert.equal(
      declared.length,
      ALLOWED_PAINT_TABLES,
      `the host declares one colour table; found ${declared.join(", ")}`,
    );
    assert.ok(
      declared[0]?.endsWith(PAINT_TABLE_NAME),
      `the one colour table is ${PAINT_TABLE_NAME}; found ${declared[0]}`,
    );
  });

  test("no host source paints a cluster from the editor's chart palette", () => {
    const offenders = hostSources().filter((source) =>
      descendants(parse(source), (node) => isForeignPaletteLiteral(node)).length > 0,
    );
    assert.deepEqual(
      offenders,
      [],
      `every surface paints from the Deslop ${COLOR_NAMESPACE} tokens; painting from ` +
        `${FOREIGN_PALETTE_PREFIX}* gives the same finding two colours (gh #522)`,
    );
  });

  test("the one table is keyed by clone kind, never by mass rank band", () => {
    const keys = Object.keys(KIND_COLOR);
    for (const band of RANK_BAND_KEYS) {
      assert.ok(
        !keys.includes(band),
        `${PAINT_TABLE_NAME} is keyed by what the code is, not by where it sorts; ` +
          `"${band}" is a mass rank band (gh #521)`,
      );
    }
  });
});

/** Every shipped extension-host source file. */
function hostSources(): string[] {
  const root = extensionPath(HOST_SOURCE_ROOT);
  const found: string[] = [];
  const pending = [root];
  while (pending.length > 0) {
    const directory = pending.pop();
    if (directory === undefined) break;
    for (const entry of fs.readdirSync(directory, { withFileTypes: true })) {
      collect(path.join(directory, entry.name), entry.isDirectory(), pending, found);
    }
  }
  return found.sort();
}

/** Queues a directory or keeps a TypeScript source. */
function collect(entry: string, isDirectory: boolean, pending: string[], found: string[]): void {
  if (isDirectory) {
    if (!EXCLUDED_DIRECTORIES.includes(path.basename(entry))) pending.push(entry);
    return;
  }
  if (entry.endsWith(".ts") && !entry.endsWith(".d.ts")) found.push(entry);
}

/** Parses one host source. */
function parse(source: string): ts.SourceFile {
  return parseTypeScriptSource(source, ts.ScriptKind.TS);
}

/** The names of every declaration in `source` whose initialiser is an object
 * literal mapping keys to Deslop colour tokens. */
function colourTablesIn(source: string): string[] {
  return descendants(parse(source), isColourTable).map(
    (node) => `${path.basename(source)}:${nameOf(node)}`,
  );
}

/** The declared name of a variable declaration node. */
function nameOf(node: ts.Node): string {
  return ts.isVariableDeclaration(node) && ts.isIdentifier(node.name) ? node.name.text : "anonymous";
}

/** Whether `node` declares an object literal whose every value is a colour
 * token — the shape of a paint table, whatever it happens to be keyed by. */
function isColourTable(node: ts.Node): boolean {
  if (!ts.isVariableDeclaration(node) || node.initializer === undefined) return false;
  const literal = unwrap(node.initializer);
  if (!ts.isObjectLiteralExpression(literal) || literal.properties.length === 0) return false;
  return literal.properties.every(isColourAssignment);
}

/** Looks through an `as const` assertion to the literal beneath it. */
function unwrap(expression: ts.Expression): ts.Expression {
  return ts.isAsExpression(expression) ? unwrap(expression.expression) : expression;
}

/** Whether one property assigns a Deslop colour token. */
function isColourAssignment(property: ts.ObjectLiteralElementLike): boolean {
  return (
    ts.isPropertyAssignment(property) &&
    ts.isPropertyAccessExpression(property.initializer) &&
    ts.isIdentifier(property.initializer.expression) &&
    property.initializer.expression.text === COLOR_NAMESPACE
  );
}

/** Whether `node` is a string literal naming VS Code's chart palette. */
function isForeignPaletteLiteral(node: ts.Node): boolean {
  return ts.isStringLiteral(node) && node.text.startsWith(FOREIGN_PALETTE_PREFIX);
}
