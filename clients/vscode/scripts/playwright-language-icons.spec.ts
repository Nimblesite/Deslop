// [VSIX-LANGUAGE-ICONS] Every language appears across all webview surfaces.
import { createHash } from "node:crypto";
import { test, expect } from "@playwright/test";
import { loadView, postHostMessage, sampleReport, screenshotDir, type ViewKind } from "./playwright-webview-fixture";
import { LANGUAGES, SOURCE_EXTENSIONS, languageForPath, languageDisplayName } from "../src/types/languages";

const VIEWS: readonly ViewKind[] = ["cluster", "report", "duplication"];
const VIEWPORT = { width: 980, height: 760 };
const ICON_SELECTOR = ".language-icon";
const FIRST = 0;
const SVG_SIZE = 24;
const REPORT_MESSAGE = "report/snapshot";
const SELECT_MESSAGE = "select/cluster";
const OSPREY = "osprey";
const OSPREY_SHA256 = "ce94ff11f7041d412c4cb6b1d32f3ee5685506fb0a602b0734758c8bfdffb03e";
const PNG_PREFIX = "data:image/png;base64,";
const OSPREY_FLAVORS = ["osprey-ml", "osprey-python"];

for (const view of VIEWS) {
  test(`${view} renders accessible language artwork for every supported language`, async ({ page }) => {
    const errors = await loadView(page, view, VIEWPORT);
    for (const language of LANGUAGES) {
      const snapshot = reportForLanguage(language);
      await postHostMessage(page, { kind: REPORT_MESSAGE, report: snapshot });
      await postHostMessage(page, { kind: SELECT_MESSAGE, id: snapshot.clusters[FIRST].id });
      const icon = page.locator(ICON_SELECTOR).first();
      await expect(icon).toHaveAttribute("data-language", language);
      await expect(icon).toHaveAttribute("aria-label", languageDisplayName(language));
      await expect(icon.locator("svg")).toHaveAttribute("viewBox", `0 0 ${SVG_SIZE} ${SVG_SIZE}`);
      await expect(icon.locator("svg")).toBeVisible();
      if (language === OSPREY) {
        const href = await icon.locator("image").getAttribute("href");
        expect(href?.startsWith(PNG_PREFIX)).toBe(true);
        const digest = createHash("sha256").update(Buffer.from(href!.slice(PNG_PREFIX.length), "base64")).digest("hex");
        expect(digest).toBe(OSPREY_SHA256);
        await page.screenshot({ path: `${screenshotDir}/osprey-${view}.png` });
      } else {
        await expect(icon.locator("path")).toBeVisible();
      }
    }
    for (const flavor of OSPREY_FLAVORS) {
      const snapshot = reportForLanguage(OSPREY, flavor);
      await postHostMessage(page, { kind: REPORT_MESSAGE, report: snapshot });
      await postHostMessage(page, { kind: SELECT_MESSAGE, id: snapshot.clusters[FIRST].id });
      await expect(page.locator(ICON_SELECTOR).first()).toHaveAttribute("data-language", OSPREY);
      await expect(page.locator(ICON_SELECTOR).first()).toHaveAttribute("aria-label", languageDisplayName(OSPREY));
    }
    expect(errors).toEqual([]);
  });
}

function reportForLanguage(language: string, selected?: string) {
  const snapshot = structuredClone(sampleReport);
  const extension = SOURCE_EXTENSIONS.find((candidate) => languageForPath(`source.${candidate}`) === language);
  const path = `src/example.${selected === "osprey-python" ? "py" : selected === "osprey-ml" ? "ospml" : extension}`;
  for (const cluster of snapshot.clusters) {
    for (const occurrence of cluster.occurrences) {
      occurrence.path = path;
      occurrence.language = selected;
      if (occurrence.displayLocation) occurrence.displayLocation.label = path;
    }
  }
  for (const metric of snapshot.metrics.per_file) {
    metric.path = path;
    metric.language = selected;
  }
  return snapshot;
}
