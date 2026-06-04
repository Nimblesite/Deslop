import { expect, test, type Page } from "@playwright/test";
import fs from "node:fs";
import path from "node:path";

type ViewKind = "cluster" | "duplication" | "report";

interface ViewportCase {
  readonly name: string;
  readonly width: number;
  readonly height: number;
}

interface PostedMessage {
  readonly kind?: string;
}

declare global {
  interface Window {
    __deslopPosts?: PostedMessage[];
    acquireVsCodeApi?: () => { postMessage: (data: PostedMessage) => void };
  }
}

const repoRoot = findRepoRoot(process.cwd());
const webviewDir = path.join(repoRoot, "clients", "vscode", "media", "webview");
const screenshotDir = path.join(repoRoot, "target", "playwright-webview");

const viewports: readonly ViewportCase[] = [
  { name: "desktop", width: 1280, height: 900 },
  { name: "narrow", width: 390, height: 844 },
];

test.describe("VSIX webview bundles", () => {
  for (const viewport of viewports) {
    test(`report view renders and posts commands on ${viewport.name}`, async ({ page }) => {
      const errors = await loadView(page, "report", viewport);

      await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });

      await expect(page.getByText("DESLOP").first()).toBeVisible();
      await expect(page.getByRole("heading", { name: /18\.4%/ })).toBeVisible();
      await expect(page.getByText("Same behavior, different code")).toBeVisible();

      await clearPostedMessages(page);
      await page.getByRole("button", { name: "Refresh" }).click();
      await expectPosted(page, "refresh");

      await clearPostedMessages(page);
      await page.getByText("Same behavior, different code").click();
      await expectPosted(page, "open/cluster");

      await expectHealthyRender(page, errors, `report-${viewport.name}`);
    });

    test(`cluster view renders, navigates, and posts commands on ${viewport.name}`, async ({ page }) => {
      const errors = await loadView(page, "cluster", viewport);

      await postHostMessage(page, { kind: "report/snapshot", report: sampleReport });
      await postHostMessage(page, { kind: "select/cluster", id: sampleReport.clusters[0].id });

      await expect(page.getByText("CLUSTER").first()).toBeVisible();
      await expect(page.getByRole("heading", { name: "Same behavior, different code" })).toBeVisible();
      await expect(page.getByText("SIGNALS")).toBeVisible();
      await expect(page.getByText("src/dart/alpha.dart:12:3")).toBeVisible();

      await page.keyboard.press("n");
      await expect(page.getByRole("heading", { name: "Nearly identical code" })).toBeVisible();
      await page.keyboard.press("p");
      await expect(page.getByRole("heading", { name: "Same behavior, different code" })).toBeVisible();

      await clearPostedMessages(page);
      await page.locator("button", { hasText: "Open" }).first().click();
      await expectPosted(page, "open/occurrence");

      await clearPostedMessages(page);
      await page.locator("button", { hasText: "Compare" }).nth(1).click();
      await expectPosted(page, "compare/canonical");

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

function webviewHtml(kind: ViewKind): string {
  const bundle = fs
    .readFileSync(path.join(webviewDir, `${kind}.js`), "utf8")
    .replaceAll("</script", "<\\/script");
  return `<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Deslop ${kind}</title>
    <style>body { margin: 0; }</style>
    <script>
      window.__deslopPosts = [];
      window.acquireVsCodeApi = function () {
        return {
          postMessage: function (data) {
            window.__deslopPosts.push(data);
          }
        };
      };
    </script>
  </head>
  <body>
    <div id="root"></div>
    <script type="module">${bundle}</script>
  </body>
</html>`;
}

function findRepoRoot(startDir: string): string {
  let current = startDir;
  while (true) {
    const marker = path.join(current, "clients", "vscode", "media", "webview", "report.js");
    if (fs.existsSync(marker)) return current;
    const parent = path.dirname(current);
    if (parent === current) {
      throw new Error(`Could not find repo root from ${startDir}`);
    }
    current = parent;
  }
}

const sampleReport = {
  tool_version: "playwright-smoke",
  min_nodes: 5,
  files_analysed: 4,
  clusters_hidden: 0,
  cache_stats: { hits: 7, misses: 2 },
  metrics: {
    analysed_loc: 520,
    duplicated_loc: 96,
    duplication_percent: 18.4,
    clusters_total: 3,
    duplicated_files: 3,
    threshold: { percent: 15, breached: true, source: "config" },
    per_file: [
      { path: "src/dart/alpha.dart", analysed_loc: 120, duplicated_loc: 42, duplication_percent: 35 },
      { path: "src/dart/parser_beta.dart", analysed_loc: 180, duplicated_loc: 38, duplication_percent: 21.1 },
      { path: "src/models/models.g.dart", analysed_loc: 220, duplicated_loc: 16, duplication_percent: 7.3 },
    ],
  },
  schema_doc: "playwright smoke schema",
  action_hints: [],
  boilerplate_hints: [],
  embedding_provenance: {
    provider_id: "ollama",
    model_id: "nomic-embed-text",
    model_version: "smoke",
    dimensions: 768,
    attempted_subtrees: 12,
    indexed_subtrees: 12,
    failed_subtrees: 0,
  },
  clusters: [
    {
      id: "abcdef1234567890",
      weight: 42.75,
      size: 2,
      canonical_node_count: 18,
      signals: { structural: 0.22, token_jaccard: 0.34, embedding_cos: 0.91, fused: 0.88 },
      bucket: "same_behavior",
      occurrences_total: 2,
      occurrences_truncated: false,
      summary: "Two Dart classes compute the same geometry values through different implementations.",
      interpretation: "Same behavior, different code.",
      occurrences: [
        occurrence("src/dart/alpha.dart", 120, 248, 12, 3),
        occurrence("src/dart/beta.dart", 420, 558, 31, 5),
      ],
    },
    {
      id: "bcdefa2345678901",
      weight: 26.5,
      size: 3,
      canonical_node_count: 14,
      signals: { structural: 0.99, token_jaccard: 0.96, embedding_cos: 0.7, fused: 0.86 },
      bucket: "nearly_identical",
      occurrences_total: 3,
      occurrences_truncated: false,
      summary: "Parser branches differ only by token names.",
      interpretation: "Review the locations; small differences may matter.",
      occurrences: [
        occurrence("src/dart/parser_alpha.dart", 210, 330, 44, 7),
        occurrence("src/dart/parser_beta.dart", 610, 742, 88, 9),
        occurrence("src/dart/parser_gamma.dart", 1000, 1130, 122, 11),
      ],
    },
    {
      id: "cdefab3456789012",
      weight: 11.2,
      size: 2,
      canonical_node_count: 9,
      signals: { structural: 1, token_jaccard: 1, embedding_cos: 0.82, fused: 0.97 },
      bucket: "identical",
      occurrences_total: 2,
      occurrences_truncated: false,
      summary: "Generated model serialization helpers match exactly.",
      interpretation: "Safe to extract; every copy is the same.",
      occurrences: [
        occurrence("src/models/models.g.dart", 80, 160, 15, 1),
        occurrence("src/models/serializers.g.dart", 180, 260, 27, 1, true),
      ],
    },
  ],
};

function occurrence(
  filePath: string,
  startByte: number,
  endByte: number,
  line: number,
  column: number,
  hidden = false,
): object {
  return {
    path: filePath,
    start_byte: startByte,
    end_byte: endByte,
    start_line: line,
    end_line: line + 4,
    hidden,
    displayLocation: {
      line,
      column,
      label: `${filePath}:${line}:${column}`,
      description: `line ${line}, column ${column}`,
      commandTitle: "Open occurrence",
    },
  };
}
