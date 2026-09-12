import { type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

import { anchorForClusterId, clusterPanelFeed } from "../src/clusterSelection";
import {
  CANONICAL_OCCURRENCE_INDEX,
  FIRST_PEER_INDEX,
  reportWithoutSignalSource,
  sampleReport,
  screenshotDir,
  type ViewKind,
  webviewHtml,
  withCanonicalUnsaved,
} from "./playwright-webview-fixture";

// `test`/`expect` come from the coverage fixture so this same suite records the
// webview V8 coverage when WEBVIEW_COVERAGE=1 (no separate rendering harness).
import { expect, test } from "./webview-coverage-fixture";


interface ViewportCase {
  readonly name: string;
  readonly width: number;
  readonly height: number;
}

const PAIR_EVIDENCE_HEADING = "PAIR EVIDENCE";
const PAIR_CONJOINED_SEPARATOR = "↔";
const PAIR_EVIDENCE_UNAVAILABLE = "PAIR EVIDENCE UNAVAILABLE";
const CONTENT_EVIDENCE_HEADING = "CONTENT EVIDENCE";
const CONTENT_EVIDENCE_VERDICT = "Its content evidence is 0.05 shared content";
const CONTENT_EVIDENCE_LABELS = ["AGREEMENT", "RENAME", "LITERAL"] as const;
// [CLONE-KIND-LABELS] The sample report carries one cluster per kind the
// smoke drives, so each surface is checked to title clusters by their kind.
const IDENTICAL_TITLE = "Identical code";
const NEARLY_IDENTICAL_TITLE = "Nearly identical code";
const STRUCTURAL_ONLY_TITLE = "Same shape, different content";
const RETIRED_NEUTRAL_TITLE = "Duplicate code";
const MASS_LABEL = "mass";
const WEIGHT_LABEL = "weight";
const CANONICAL_COMPARE_LABEL = "Compare is disabled on the canonical occurrence because it would compare the same range with itself.";
const PEER_COMPARE_LABEL = "Compare this occurrence with the canonical occurrence in VS Code's diff editor.";
const CANONICAL_COMPARE_MESSAGE = "compare/canonical";
// [VSIX-PAIR-COMPARE] Two row taps compare exactly those two occurrences;
// the retired per-row and gated buttons never render.
const PAIR_COMPARE_MESSAGE = "compare/pair";
const OCCURRENCE_ROW = "article";
const PICKED_ATTRIBUTE = "data-picked";
const ROW_TAP_POSITION = { x: 4, y: 4 };
const RETIRED_SELECT_LABEL = "Select for comparison";
const RETIRED_COMPARE_SELECTED_LABEL = "Compare selected occurrences";
const NEXT_CLUSTER_LABEL = "Next cluster";
const FIRST_CLUSTER_INDEX = 0;
const SECOND_CLUSTER_INDEX = 1;
const SECOND_PEER_INDEX = 2;
// [VSIX-PAIR-COMPARE] The row role the panel prints for the canonical member.
const CANONICAL_ROW_PREFIX = "Canonical occurrence:";
const CANONICAL_ROW_COUNT = 1;
const NAVIGATED_ROW_COUNT = 3;
const NAVIGATED_PEER_COUNT = 2;
const NEXT_CLUSTER_KEY = "n";
const PREVIOUS_CLUSTER_KEY = "p";
const INFORMATIONAL_FINDING = "Informational — not a clone.";
const SHAPE_CLUSTER_INDEX = 2;
const DESKTOP_VIEWPORT_INDEX = 0;

const viewports: readonly ViewportCase[] = [
  { name: "desktop", width: 1280, height: 900 },
  { name: "narrow", width: 390, height: 844 },
];

test.describe("VSIX webview bundles", () => {
  test("shape-only findings show information without duplication claims", async ({ page }) => {
    const viewport = viewports[DESKTOP_VIEWPORT_INDEX];
    if (!viewport) throw new Error("Missing desktop viewport");
    await loadView(page, "report", viewport);
    await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });
    const shapeRow = page.locator("li").filter({ hasText: STRUCTURAL_ONLY_TITLE });
    await expect(shapeRow.getByText(INFORMATIONAL_FINDING)).toBeVisible();
    await expect(shapeRow).not.toContainText("mass");
    await expect(shapeRow).not.toContainText("×");
    await loadView(page, "cluster", viewport);
    await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });
    await postHostMessage(page, { kind: "select/cluster", id: sampleReport.clusters[SHAPE_CLUSTER_INDEX].id });
    await expect(page.getByText(INFORMATIONAL_FINDING, { exact: true })).toBeVisible();
    await expect(page.locator("header")).not.toContainText("mass");
    await expect(page.locator("header")).not.toContainText("×");
    await expect(page.locator("header")).not.toContainText("Rank");
  });
  for (const viewport of viewports) {
    test(`report view renders and posts commands on ${viewport.name}`, async ({ page }) => {
      const errors = await loadView(page, "report", viewport);

      await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });

      await expect(page.getByText("DESLOP").first()).toBeVisible();
      await expect(page.getByRole("heading", { name: /18\.4%/ })).toBeVisible();
      await expect(page.getByText(IDENTICAL_TITLE, { exact: true })).toBeVisible();
      await expect(page.getByText(NEARLY_IDENTICAL_TITLE, { exact: true })).toBeVisible();
      await expect(page.getByText(STRUCTURAL_ONLY_TITLE, { exact: true })).toBeVisible();
      await expect(page.getByText(RETIRED_NEUTRAL_TITLE, { exact: true })).toHaveCount(0);

      await clearPostedMessages(page);
      await page.getByRole("button", { name: "Refresh" }).click();
      await expectPosted(page, "refresh");

      await clearPostedMessages(page);
      await page.getByText(IDENTICAL_TITLE, { exact: true }).first().click();
      await expectPosted(page, "open/cluster");

      await expectHealthyRender(page, errors, `report-${viewport.name}`);
    });

    test(`cluster view renders, navigates, and posts commands on ${viewport.name}`, async ({ page }) => {
      const errors = await loadView(page, "cluster", viewport);

      await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });
      await postHostMessage(page, { kind: "select/cluster", id: sampleReport.clusters[0].id });

      await expect(page.getByText("CLUSTER").first()).toBeVisible();
      await expect(page.getByRole("heading", { name: IDENTICAL_TITLE })).toBeVisible();
      await expect(page.getByText(MASS_LABEL, { exact: true })).toBeVisible();
      await expect(page.getByText(WEIGHT_LABEL, { exact: true })).toHaveCount(0);
      await expect(page.getByText(RETIRED_NEUTRAL_TITLE, { exact: true })).toHaveCount(0);
      // [FUSED-PAIR-SIGNALS] The admission signals are pair measurements and
      // never touch the cluster. The cluster card renders no pair-evidence
      // panel, no pair source, and no content metrics.
      await expect(page.getByText(CONTENT_EVIDENCE_HEADING, { exact: true })).toHaveCount(0);
      for (const label of CONTENT_EVIDENCE_LABELS) {
        await expect(page.getByText(label, { exact: true })).toHaveCount(0);
      }
      await expect(page.getByText(CONTENT_EVIDENCE_VERDICT, { exact: false })).toHaveCount(0);
      await expect(page.getByText(PAIR_EVIDENCE_HEADING, { exact: false })).toHaveCount(0);
      await expect(page.getByText(PAIR_EVIDENCE_UNAVAILABLE, { exact: false })).toHaveCount(0);
      // The occurrence list shows single editor locations (cluster membership
      // facts); only a pair-evidence line joins two of them with the arrow.
      await expect(page.getByText(PAIR_CONJOINED_SEPARATOR, { exact: false })).toHaveCount(0);

      await page.keyboard.press("n");
      await expect(page.getByRole("heading", { name: NEARLY_IDENTICAL_TITLE })).toBeVisible();
      await page.keyboard.press("p");
      await expect(page.getByRole("heading", { name: IDENTICAL_TITLE })).toBeVisible();

      await clearPostedMessages(page);
      await page.locator("button", { hasText: "Open" }).first().click();
      await expectPosted(page, "open/occurrence");

      await clearPostedMessages(page);
      // [VSIX-PAIR-COMPARE] A peer opens the canonical diff in one click.
      await expect(page.getByRole("button", { name: CANONICAL_COMPARE_LABEL })).toBeDisabled();
      const comparePeer = page.getByRole("button", { name: PEER_COMPARE_LABEL });
      await expect(comparePeer).toBeEnabled();
      await comparePeer.click();
      await expectPostedCanonical(page, FIRST_CLUSTER_INDEX, FIRST_PEER_INDEX);
      await page.getByRole("button", { name: NEXT_CLUSTER_LABEL, exact: true }).click();
      await clearPostedMessages(page);
      await page.getByRole("button", { name: PEER_COMPARE_LABEL }).nth(FIRST_PEER_INDEX).click();
      await expectPostedCanonical(page, SECOND_CLUSTER_INDEX, SECOND_PEER_INDEX);
      await expect(page.getByRole("button", { name: CANONICAL_COMPARE_LABEL })).toBeDisabled();

      // [VSIX-PAIR-COMPARE] Tapping one row picks it, tapping a second row
      // hands both endpoints to the host, and the pick is released.
      await clearPostedMessages(page);
      const rows = page.locator(OCCURRENCE_ROW);
      await rows.nth(FIRST_PEER_INDEX).click({ position: ROW_TAP_POSITION });
      await expect(rows.nth(FIRST_PEER_INDEX)).toHaveAttribute(PICKED_ATTRIBUTE, "true");
      await expect(rows.nth(SECOND_PEER_INDEX)).toHaveAttribute(PICKED_ATTRIBUTE, "false");
      await rows.nth(SECOND_PEER_INDEX).click({ position: ROW_TAP_POSITION });
      await expectPostedPair(page, SECOND_CLUSTER_INDEX, FIRST_PEER_INDEX, SECOND_PEER_INDEX);
      await expect(rows.nth(FIRST_PEER_INDEX)).toHaveAttribute(PICKED_ATTRIBUTE, "false");
      // Tapping the picked row again lets it go without posting anything.
      await clearPostedMessages(page);
      await rows.nth(FIRST_CLUSTER_INDEX).click({ position: ROW_TAP_POSITION });
      await expect(rows.nth(FIRST_CLUSTER_INDEX)).toHaveAttribute(PICKED_ATTRIBUTE, "true");
      await rows.nth(FIRST_CLUSTER_INDEX).click({ position: ROW_TAP_POSITION });
      await expect(rows.nth(FIRST_CLUSTER_INDEX)).toHaveAttribute(PICKED_ATTRIBUTE, "false");
      await expectNothingPosted(page);
      await expect(page.getByRole("button", { name: RETIRED_SELECT_LABEL })).toHaveCount(0);
      await expect(page.getByText(RETIRED_COMPARE_SELECTED_LABEL, { exact: true })).toHaveCount(0);

      await expectHealthyRender(page, errors, `cluster-${viewport.name}`);
    });

    test(`duplication view renders file rollup on ${viewport.name}`, async ({ page }) => {
      const errors = await loadView(page, "duplication", viewport);

      await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });

      await expect(page.getByText("DESLOP").first()).toBeVisible();
      await expect(page.getByRole("heading", { name: /18\.4%/ })).toBeVisible();
      await expect(page.getByText("alpha.dart")).toBeVisible();
      await expect(page.getByText("parser_beta.dart")).toBeVisible();

      await expectHealthyRender(page, errors, `duplication-${viewport.name}`);
    });
  }

  test("selecting a cluster renders its detail, never the empty state (#254)", async ({ page }) => {
    // Regression #254: `severityOf` was imported type-only, so esbuild erased
    // it from the bundle; the `severityByClusterId` computed threw on the first
    // selected-cluster render, Preact aborted the update, and every cluster
    // panel froze on "No cluster selected." Drive the real bundle exactly as
    // the host does and require the selected cluster's detail to render.
    const errors = await loadView(page, "cluster", viewports[0]);

    await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });
    await postHostMessage(page, { kind: "select/cluster", id: sampleReport.clusters[0].id });

    await expect(page.getByRole("heading", { name: IDENTICAL_TITLE })).toBeVisible();
    await expect(page.getByText("CLUSTER").first()).toBeVisible();
    await expect(page.getByText("No cluster selected.")).toHaveCount(0);
    expect(errors, errors.join("\n")).toEqual([]);
  });

  test("navigating to a cluster whose canonical is unsaved keeps every peer comparable", async ({ page }) => {
    // [VSIX-PAIR-COMPARE] `n` moves the selection inside the webview, so a
    // cluster the user never opened still becomes the detail. This drives the
    // real host feed builder over a real dirty projection: with the second
    // cluster's canonical occurrence in an unsaved buffer, the projection on
    // its own leaves the first clean peer in the canonical slot — labelled
    // canonical, its own Compare suppressed as a self-comparison, and the peer
    // below it diffed against the wrong range.
    const errors = await loadView(page, "cluster", viewports[DESKTOP_VIEWPORT_INDEX]);
    const opened = sampleReport.clusters[FIRST_CLUSTER_INDEX];
    const navigated = sampleReport.clusters[SECOND_CLUSTER_INDEX];
    const feed = clusterPanelFeed(
      sampleReport,
      withCanonicalUnsaved(sampleReport, SECOND_CLUSTER_INDEX),
      anchorForClusterId(sampleReport, opened.id),
    );

    await postHostMessage(page, { kind: "report/snapshot", report: feed.report });
    await postHostMessage(page, { kind: "select/cluster", id: feed.selectedId });
    await expect(page.getByRole("heading", { name: IDENTICAL_TITLE })).toBeVisible();

    await page.keyboard.press(NEXT_CLUSTER_KEY);
    await expect(page.getByRole("heading", { name: NEARLY_IDENTICAL_TITLE })).toBeVisible();

    // Exactly one row is canonical, and it is the engine's canonical — not the
    // first member that happens to be saved.
    const canonicalRows = await rowTitlesStartingWith(page, CANONICAL_ROW_PREFIX);
    expect(canonicalRows).toHaveLength(CANONICAL_ROW_COUNT);
    expect(canonicalRows[CANONICAL_OCCURRENCE_INDEX]).toContain(
      navigated.occurrences[CANONICAL_OCCURRENCE_INDEX].path,
    );
    const canonicalCompare = page.getByRole("button", { name: CANONICAL_COMPARE_LABEL });
    await expect(canonicalCompare).toHaveCount(CANONICAL_ROW_COUNT);
    await expect(canonicalCompare).toBeDisabled();
    await expect(page.locator(OCCURRENCE_ROW)).toHaveCount(NAVIGATED_ROW_COUNT);

    // Every surviving peer keeps its Compare action ...
    const peers = page.getByRole("button", { name: PEER_COMPARE_LABEL });
    await expect(peers).toHaveCount(NAVIGATED_PEER_COUNT);
    for (const peer of await peers.all()) {
      await expect(peer).toBeEnabled();
    }

    // ... and clicking one names that exact peer, against the engine's canonical.
    await clearPostedMessages(page);
    await peers.first().click();
    await expectPostedCanonical(page, SECOND_CLUSTER_INDEX, FIRST_PEER_INDEX);

    await page.keyboard.press(PREVIOUS_CLUSTER_KEY);
    await expect(page.getByRole("heading", { name: IDENTICAL_TITLE })).toBeVisible();
    expect(errors, errors.join("\n")).toEqual([]);
  });

  test("a cluster renders no pair scores with or without a signal source", async ({ page }) => {
    const errors = await loadView(page, "cluster", viewports[0]);

    await postHostMessage(page, { kind: "report/snapshot", report: reportWithoutSignalSource });
    await postHostMessage(page, { kind: "select/cluster", id: sampleReport.clusters[0].id });

    // [FUSED-PAIR-SIGNALS] No cluster surface renders pair evidence; an
    // absent source changes nothing on the card.
    await expect(page.getByText(PAIR_EVIDENCE_UNAVAILABLE, { exact: false })).toHaveCount(0);
    await expect(page.getByText(PAIR_EVIDENCE_HEADING, { exact: false })).toHaveCount(0);
    await expect(page.getByText("0.91", { exact: true })).toHaveCount(0);
    expect(errors, errors.join("\n")).toEqual([]);
  });
});

async function loadView(
  page: Page,
  kind: ViewKind,
  viewport: ViewportCase,
): Promise<string[]> {
  const errors: string[] = [];
  page.on("console", (message) => {
    if (message.type() === "error") errors.push(message.text());
  });
  page.on("pageerror", (error) => {
    errors.push(error.message);
  });

  await page.setViewportSize({ width: viewport.width, height: viewport.height });
  await page.setContent(webviewHtml(kind), { waitUntil: "domcontentloaded" });
  await page.waitForFunction(() => window.__deslopPosts?.some((message) => message.kind === "ready"));
  return errors;
}

async function postHostMessage(page: Page, message: unknown): Promise<void> {
  await page.evaluate((payload) => {
    window.postMessage(payload, "*");
  }, message);
}

async function clearPostedMessages(page: Page): Promise<void> {
  await page.evaluate(() => {
    window.__deslopPosts = [];
  });
}

async function expectPosted(page: Page, kind: string): Promise<void> {
  await expect
    .poll(async () => {
      return await page.evaluate(() => window.__deslopPosts?.map((message) => message.kind) ?? []);
    })
    .toContain(kind);
}

async function expectPostedCanonical(page: Page, clusterIndex: number, occurrenceIndex: number): Promise<void> {
  const cluster = sampleReport.clusters[clusterIndex];
  await expect
    .poll(async () => {
      return await page.evaluate((kind) => window.__deslopPosts?.find((message) => message.kind === kind), CANONICAL_COMPARE_MESSAGE);
    })
    .toEqual({
      kind: CANONICAL_COMPARE_MESSAGE,
      clusterId: cluster.id,
      occurrence: cluster.occurrences[occurrenceIndex],
    });
}

async function expectPostedPair(
  page: Page,
  clusterIndex: number,
  leftIndex: number,
  rightIndex: number,
): Promise<void> {
  const cluster = sampleReport.clusters[clusterIndex];
  await expect
    .poll(async () => {
      return await page.evaluate((kind) => window.__deslopPosts?.find((message) => message.kind === kind), PAIR_COMPARE_MESSAGE);
    })
    .toEqual({
      kind: PAIR_COMPARE_MESSAGE,
      left: cluster.occurrences[leftIndex],
      right: cluster.occurrences[rightIndex],
    });
}

/** The `title` of every occurrence row whose role starts with `prefix`. */
async function rowTitlesStartingWith(page: Page, prefix: string): Promise<string[]> {
  const titles = await page
    .locator(OCCURRENCE_ROW)
    .evaluateAll((rows) => rows.map((row) => row.getAttribute("title") ?? ""));
  return titles.filter((title) => title.startsWith(prefix));
}

async function expectNothingPosted(page: Page): Promise<void> {
  await expect
    .poll(async () => {
      return await page.evaluate(() => window.__deslopPosts?.length ?? 0);
    })
    .toBe(0);
}

async function expectHealthyRender(
  page: Page,
  errors: readonly string[],
  screenshotName: string,
): Promise<void> {
  fs.mkdirSync(screenshotDir, { recursive: true });
  await page.screenshot({
    path: path.join(screenshotDir, `${screenshotName}.png`),
    fullPage: true,
  });

  const metrics = await page.evaluate(() => {
    const root = document.getElementById("root");
    const rootRect = root?.getBoundingClientRect();
    const viewportWidth = window.innerWidth;
    const scrollWidth = Math.max(document.documentElement.scrollWidth, document.body.scrollWidth);
    const offenders = Array.from(document.querySelectorAll<HTMLElement>("body *"))
      .map((element) => {
        const rect = element.getBoundingClientRect();
        return {
          tag: element.tagName.toLowerCase(),
          text: (element.textContent ?? "").trim().slice(0, 80),
          left: Math.round(rect.left),
          right: Math.round(rect.right),
          width: Math.round(rect.width),
        };
      })
      .filter((item) => item.right > viewportWidth + 2 || item.left < -2)
      .slice(0, 8);

    return {
      textLength: (document.body.textContent ?? "").trim().length,
      rootWidth: Math.round(rootRect?.width ?? 0),
      rootHeight: Math.round(rootRect?.height ?? 0),
      scrollWidth,
      viewportWidth,
      offenders,
    };
  });

  expect(metrics.textLength).toBeGreaterThan(40);
  expect(metrics.rootWidth).toBeGreaterThan(100);
  expect(metrics.rootHeight).toBeGreaterThan(100);
  expect(metrics.scrollWidth, JSON.stringify(metrics.offenders)).toBeLessThanOrEqual(
    metrics.viewportWidth + 2,
  );
  expect(metrics.offenders).toEqual([]);
  expect(errors).toEqual([]);
}
