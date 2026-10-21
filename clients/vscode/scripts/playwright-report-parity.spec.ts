// [VSIX-TOP-OFFENDERS-EXCLUDED] [OUTPUT-HUMAN-HTML] Both surfaces consume the real CLI report.
import fs from "node:fs";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { pathToFileURL } from "node:url";
import { test, expect } from "./webview-coverage-fixture";
import { loadView, postHostMessage } from "./playwright-webview-fixture";
import { CLUSTER_KINDS, kindTitle, type Report } from "../src/types/report";

const ROOT = path.resolve(__dirname, "../../..");
const ARTIFACTS = path.join(ROOT, "target", "pr588-report-parity");
const FIXTURE = path.join(ROOT, "clients/vscode/src/test/fixtures/csharp-small");
const VIEWPORT = { width: 1280, height: 900 };
const CONFIGS = [{ name: "included", body: "" }, { name: "empty-section", body: "[total]\n" }, { name: "excluded", body: '[total]\ninclude = []\n[threshold.kinds]\nnearly_identical = 0\n' }];

function measuredReport(name: string, config: string): { report: Report; html: string; exit: number | null } {
  fs.mkdirSync(ARTIFACTS, { recursive: true });
  const configPath = path.join(ARTIFACTS, `${name}.toml`);
  const output = path.join(ARTIFACTS, name);
  fs.writeFileSync(configPath, config);
  const result = spawnSync(path.join(ROOT, "target/release/deslop"), [FIXTURE, "--config", configPath, "--output", output, "--no-incremental", "--no-color"], { encoding: "utf8" });
  expect(result.error, result.stderr).toBeUndefined();
  return { report: JSON.parse(fs.readFileSync(`${output}.json`, "utf8")) as Report, html: `${output}.html`, exit: result.status };
}

for (const scenario of CONFIGS) {
  test(`HTML and VSIX agree on ${scenario.name} groups and verdicts`, async ({ page, context }) => {
    const measured = measuredReport(scenario.name, scenario.body);
    const { report } = measured;
    expect(report.metrics.total_includes).toEqual(scenario.name === "excluded" ? [] : ["identical", "nearly_identical"]);
    expect(report.clusters.map((cluster) => [cluster.id, cluster.kind, cluster.mass, cluster.rank, cluster.occurrence_count])).toEqual([
      ["a5b1c68838489f44", "nearly_identical", 89, 1, 2],
      ["59c1cca6d7a0739b", "identical", 83, 2, 2],
      ["6a6df03a736cad45", "nearly_identical", 76, 3, 2],
    ]);
    expect(report.clusters[0].occurrences.map((item) => item.path).sort()).toEqual(["Alpha.cs", "Beta.cs"]);
    const html = await context.newPage();
    await html.goto(pathToFileURL(measured.html).href);
    await loadView(page, "report", VIEWPORT);
    await postHostMessage(page, { kind: "report/snapshot", report });
    const kinds = CLUSTER_KINDS.filter((kind) => report.clusters.some((cluster) => cluster.kind === kind));
    for (const surface of [html, page]) {
      const groups = surface.locator("details.clone-group");
      await expect(groups).toHaveCount(kinds.length);
      for (const [index, kind] of kinds.entries()) {
        const group = groups.nth(index);
        const excluded = !report.metrics.total_includes.includes(kind);
        await expect(group.locator("summary")).toContainText(kindTitle(kind));
        expect(await group.evaluate((element) => (element as HTMLDetailsElement).open)).toBe(!excluded);
        expect(await group.evaluate((element) => element.classList.contains("clone-group--excluded"))).toBe(excluded);
        if (excluded) await group.locator("summary").click();
        const cards = group.locator(surface === html ? ".cluster-card" : ".report-cluster-row");
        for (const card of await cards.all()) await expect(card).toHaveCSS("opacity", excluded ? "0.55" : "1");
        for (const cluster of report.clusters.filter((item) => item.kind === kind)) await expect(group).toContainText(`mass ${cluster.mass}`);
      }
      await expect(surface.locator("body")).toContainText("total counts:");
      if (scenario.name === "excluded") await expect(surface.locator("body")).toContainText("Nearly identical code");
    }
    if (scenario.name === "excluded") {
      expect(measured.exit).toBe(3);
      await expect(html.locator(".metrics-banner--breached")).toBeVisible();
      await expect(page.locator(".threshold-verdict--breached")).toContainText("Nearly identical code");
    } else expect(measured.exit).toBe(0);
  });
}
