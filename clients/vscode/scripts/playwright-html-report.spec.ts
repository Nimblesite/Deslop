// E2E: the standalone HTML report (the artifact the "Open HTML Report" button
// shows in every IDE client, byte-identical to what `deslop --output` writes).
// Drives the real `deslop` CLI over a fixture repo, loads the rendered file in
// a real browser, and asserts the design-system CSS actually applies — dark
// theme, layout container, cluster-card accent border, and syntax colours — so
// a broken/missing stylesheet fails loudly instead of shipping an unstyled wall
// of text. [OUTPUT-HUMAN-HTML]

import { test, type Page } from "@playwright/test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL } from "node:url";

interface ViewportCase {
  readonly name: string;
  readonly width: number;
  readonly height: number;
}

const repoRoot = findRepoRoot(process.cwd());
const fixtureDir = path.join(repoRoot, "clients", "vscode", "src", "test", "fixtures", "csharp-small");
const screenshotDir = path.join(repoRoot, "target", "playwright-html-report");

const viewports: readonly ViewportCase[] = [
  { name: "desktop", width: 1280, height: 900 },
  { name: "narrow", width: 390, height: 844 },
];

let reportUrl = "";
let unstyledUrl = "";

test.beforeAll(() => {
  const realHtmlPath = generateReport();
  reportUrl = pathToFileURL(realHtmlPath).href;
  unstyledUrl = pathToFileURL(makeUnstyledTwin(realHtmlPath)).href;
});

test.describe("standalone HTML report", () => {
  for (const viewport of viewports) {
    test(`renders with the design-system CSS applied on ${viewport.name}`, async ({ page }) => {
      await page.setViewportSize({ width: viewport.width, height: viewport.height });
      await page.goto(reportUrl, { waitUntil: "load" });

      const styles = await readStyles(page);
      assertThemeAndStylesheet(styles);
      assertLayoutAndAccents(styles);
      assertNoHorizontalOverflow(styles, viewport);

      fs.mkdirSync(screenshotDir, { recursive: true });
      await page.screenshot({
        path: path.join(screenshotDir, `report-${viewport.name}.png`),
        fullPage: true,
      });
    });
  }

  // Negative control: prove the guard above has teeth. The unstyled twin
  // reproduces the exact pre-fix failure ([OUTPUT-HUMAN-HTML], #192) — a
  // <style> block carrying only the dangling `@import url("base.css")`
  // aggregator, so on a file:// report every design token collapses to the
  // browser default (transparent body, serif text, no accent borders). The
  // same assertions that pass on the real report MUST fail here, or they
  // could silently pass a zero-CSS report and never catch a regression.
  test("the design-system CSS guard rejects an unstyled report", async ({ page }) => {
    await page.goto(unstyledUrl, { waitUntil: "load" });
    const styles = await readStyles(page);
    assert.throws(
      () => {
        assertThemeAndStylesheet(styles);
        assertLayoutAndAccents(styles);
      },
      "the CSS guard must FAIL on a report whose design-system stylesheet did not load",
    );
  });
});

interface ReportStyles {
  readonly theme: string | null;
  readonly styleTagLength: number;
  readonly bodyBg: string;
  readonly bodyColor: string;
  readonly hasShell: boolean;
  readonly shellMaxWidth: string;
  readonly hasCard: boolean;
  readonly cardBorderLeftWidth: string;
  readonly cardBorderLeftStyle: string;
  readonly cardBorderLeftColor: string;
  readonly h1FontWeight: string;
  readonly h1FontSize: number;
  readonly hasKeyword: boolean;
  readonly keywordColor: string;
  readonly scrollWidth: number;
  readonly viewportWidth: number;
}

async function readStyles(page: Page): Promise<ReportStyles> {
  return page.evaluate(() => {
    const css = (el: Element | null): CSSStyleDeclaration | null => (el ? getComputedStyle(el) : null);
    const body = getComputedStyle(document.body);
    const shell = css(document.querySelector(".report-shell"));
    const card = css(document.querySelector(".cluster-card"));
    const h1 = css(document.querySelector(".report-shell h1"));
    const keyword = css(document.querySelector(".tok-keyword"));
    return {
      theme: document.documentElement.getAttribute("data-theme"),
      styleTagLength: (document.querySelector("style")?.textContent ?? "").length,
      bodyBg: body.backgroundColor,
      bodyColor: body.color,
      hasShell: shell !== null,
      shellMaxWidth: shell?.maxWidth ?? "",
      hasCard: card !== null,
      cardBorderLeftWidth: card?.borderLeftWidth ?? "",
      cardBorderLeftStyle: card?.borderLeftStyle ?? "",
      cardBorderLeftColor: card?.borderLeftColor ?? "",
      h1FontWeight: h1?.fontWeight ?? "",
      h1FontSize: Number.parseFloat(h1?.fontSize ?? "0"),
      hasKeyword: keyword !== null,
      keywordColor: keyword?.color ?? "",
      scrollWidth: Math.max(document.documentElement.scrollWidth, document.body.scrollWidth),
      viewportWidth: window.innerWidth,
    };
  });
}

// The dark theme + inlined stylesheet must be live: an unstyled document keeps
// the transparent default body background, so an opaque near-black background
// with light text is hard proof the design system loaded.
function assertThemeAndStylesheet(styles: ReportStyles): void {
  assert.equal(styles.theme, "dark", "report html must request the dark theme");
  assert.ok(styles.styleTagLength > 1000, `inlined <style> must carry the design system, got ${styles.styleTagLength} chars`);
  const bg = channels(styles.bodyBg);
  assert.ok(opaque(bg), `body background must be opaque (CSS applied), got ${styles.bodyBg}`);
  assert.ok(sum(bg) < 150, `dark theme body background must be near-black, got ${styles.bodyBg}`);
  assert.ok(sum(channels(styles.bodyColor)) > 450, `dark theme body text must be light, got ${styles.bodyColor}`);
}

// The report-only CSS layer must apply: the centred shell sets a px max-width,
// the heading is heavy and large, the worst-offender card carries the 4px solid
// accent border, and syntax highlighting paints keywords a distinct colour.
function assertLayoutAndAccents(styles: ReportStyles): void {
  assert.ok(styles.hasShell, "report must contain the .report-shell container");
  assert.ok(styles.shellMaxWidth.endsWith("px") && Number.parseFloat(styles.shellMaxWidth) > 0, `report-shell must have a px max-width, got ${styles.shellMaxWidth}`);
  assert.equal(styles.h1FontWeight, "800", "report heading must render at its 800 weight");
  assert.ok(styles.h1FontSize > 24, `report heading must be large, got ${styles.h1FontSize}px`);
  assert.ok(styles.hasCard, "report must contain at least one .cluster-card");
  assert.equal(styles.cardBorderLeftStyle, "solid", "cluster card must have a solid accent border");
  assert.equal(styles.cardBorderLeftWidth, "4px", "cluster card accent border must be 4px");
  assert.ok(opaque(channels(styles.cardBorderLeftColor)), `cluster card accent border must be a visible colour, got ${styles.cardBorderLeftColor}`);
  assert.ok(styles.hasKeyword, "report snippet must produce highlighted .tok-keyword spans");
  assert.notEqual(styles.keywordColor, styles.bodyColor, "syntax keywords must be coloured distinctly from body text");
}

function assertNoHorizontalOverflow(styles: ReportStyles, viewport: ViewportCase): void {
  assert.ok(
    styles.scrollWidth <= styles.viewportWidth + 2,
    `report must not overflow horizontally on ${viewport.name}: scrollWidth ${styles.scrollWidth} > viewport ${styles.viewportWidth}`,
  );
}

function generateReport(): string {
  const binary = resolveCliBinary();
  const outDir = fs.mkdtempSync(path.join(os.tmpdir(), "deslop-html-report-"));
  execFileSync(binary, [fixtureDir, "--output", path.join(outDir, "report"), "--nojson", "--notext"], {
    stdio: "pipe",
  });
  const htmlPath = path.join(outDir, "report.html");
  if (!fs.existsSync(htmlPath)) throw new Error(`deslop did not produce ${htmlPath}`);
  return htmlPath;
}

// The four `@import` statements the report's `<style>` carried before the
// design system was inlined (#192). Beside a file:// report with no sibling
// stylesheets they resolve to nothing, collapsing every design token.
const DANGLING_IMPORTS =
  '@import url("base.css");@import url("home.css");' +
  '@import url("prose.css");@import url("syntax.css");';

// Writes an unstyled twin of the real report — identical markup, but its
// inline `<style>` is replaced with the dangling-`@import` aggregator — into
// the same temp dir (where no `base.css` exists, so the imports 404 exactly as
// in the wild). Used as the negative control proving the CSS guard has teeth.
// A plain index splice, not a regex, keeps to the repo's no-regex rule.
function makeUnstyledTwin(realHtmlPath: string): string {
  const realHtml = fs.readFileSync(realHtmlPath, "utf8");
  const open = realHtml.indexOf("<style>");
  const close = realHtml.indexOf("</style>", open);
  if (open < 0 || close < 0) throw new Error(`no <style> block to break in ${realHtmlPath}`);
  const broken = `${realHtml.slice(0, open)}<style>${DANGLING_IMPORTS}${realHtml.slice(close)}`;
  const twinPath = path.join(path.dirname(realHtmlPath), "report-unstyled.html");
  fs.writeFileSync(twinPath, broken);
  return twinPath;
}

function resolveCliBinary(): string {
  const exe = process.platform === "win32" ? "deslop.exe" : "deslop";
  const candidates = [
    process.env["DESLOP_CLI"],
    path.join(repoRoot, "target", "release", exe),
    path.join(repoRoot, "target", "debug", exe),
  ].filter((candidate): candidate is string => Boolean(candidate));
  const found = candidates.find((candidate) => fs.existsSync(candidate));
  if (!found) {
    throw new Error(
      `deslop CLI not found. Build it with \`cargo build --release -p deslop\` (or set DESLOP_CLI). Tried: ${candidates.join(", ")}`,
    );
  }
  return found;
}

function channels(color: string): number[] {
  const inner = color.slice(color.indexOf("(") + 1, color.indexOf(")"));
  return inner.split(",").map((part) => Number.parseFloat(part.trim()));
}

function opaque(rgba: number[]): boolean {
  return rgba.length < 4 || rgba[3]! > 0;
}

function sum(rgba: number[]): number {
  return (rgba[0] ?? 0) + (rgba[1] ?? 0) + (rgba[2] ?? 0);
}

function findRepoRoot(startDir: string): string {
  let current = startDir;
  while (true) {
    if (
      fs.existsSync(path.join(current, "Cargo.toml")) &&
      fs.existsSync(path.join(current, "clients", "vscode"))
    ) {
      return current;
    }
    const parent = path.dirname(current);
    if (parent === current) throw new Error(`Could not find repo root from ${startDir}`);
    current = parent;
  }
}
