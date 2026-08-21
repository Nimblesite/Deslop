import { expect, test } from "@playwright/test";

test.describe("documentation navigation", () => {
  test("groups every docs link and keeps the issue map out of the main nav", async ({ page }) => {
    await page.goto("/docs/vscode-cluster-panel/");

    await expect(
      page.locator(".site-header .nav-links").getByRole("link", {
        name: "Issue map",
        exact: true,
      }),
    ).toHaveCount(0);

    const docsNav = page.locator(".docs-sidebar__nav");
    await expect(docsNav.locator("details.docs-nav-group")).toHaveCount(4);
    await expect(docsNav.locator("a")).toHaveCount(11);
    await expect(
      docsNav.locator('[data-docs-group="guides"]'),
    ).toHaveAttribute("open", "");
    await expect(
      docsNav.locator('[data-docs-group="trust"]').getByRole("link", {
        name: "Issue map",
        exact: true,
      }),
    ).toHaveAttribute("href", "/issues/");
    await expect(
      docsNav.locator('[data-docs-group="trust"]').getByRole("link", {
        name: "Issue overview",
        exact: true,
      }),
    ).toHaveAttribute("href", "/issues/overview/");
  });

  test("localizes the grouped documentation navigation", async ({ page }) => {
    await page.goto("/zh/docs/");

    const docsNav = page.locator(".docs-sidebar__nav");
    await expect(docsNav.locator("a")).toHaveCount(11);
    await expect(docsNav.locator("summary")).toHaveText([
      "入门",
      "使用指南",
      "参考",
      "原理与透明度",
    ]);
    await expect(
      docsNav.getByRole("link", { name: "问题图谱", exact: true }),
    ).toHaveAttribute("href", "/issues/");
    await expect(
      docsNav.getByRole("link", { name: "问题概览", exact: true }),
    ).toHaveAttribute("href", "/issues/overview/");
  });

  test("expands grouped docs navigation without overflowing a phone", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/docs/vscode-cluster-panel/");
    await page.getByRole("button", { name: "Toggle menu" }).click();

    const sidebar = page.locator(".docs-sidebar");
    await expect(sidebar).toHaveClass(/open/);
    const trustGroup = sidebar.locator('[data-docs-group="trust"]');
    await expect(trustGroup).not.toHaveAttribute("open", "");
    await trustGroup.locator("summary").click();
    await expect(trustGroup).toHaveAttribute("open", "");

    const summaryHeight = await trustGroup
      .locator("summary")
      .evaluate((element) => element.getBoundingClientRect().height);
    expect(summaryHeight).toBeGreaterThanOrEqual(44);
    const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth);

    await trustGroup.getByRole("link", { name: "Issue map", exact: true }).click();
    await expect(page).toHaveURL(/\/issues\/$/);
  });
});

test.describe("issue atlas", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/issues/");
    await expect(page.locator("[data-visible-count]")).toContainText("Showing");
  });

  test("renders the relationship graph without dashboard content", async ({ page }) => {
    await expect(page.locator(".graph-node")).not.toHaveCount(0);
    await expect(page.locator(".graph-halo")).toHaveCount(8);
    await expect(page.locator(".atlas-hero")).toHaveCount(0);
    await expect(page.locator(".atlas-summary")).toHaveCount(0);
    await expect(page.locator(".verification-note")).toHaveCount(0);
    await expect(page.locator(".view-tabs")).toHaveCount(0);
    await expect(page.locator(".atlas-method")).toHaveCount(0);
  });

  test("zooms the graph and opens useful issue detail", async ({ page }) => {
    const viewport = page.locator(".graph-viewport");
    await expect(viewport).toHaveAttribute("data-zoom", "1");
    await page.getByRole("button", { name: "Zoom in" }).click();
    await expect(viewport).not.toHaveAttribute("data-zoom", "1");
    await page.getByRole("button", { name: "Reset graph position" }).click();
    await expect(viewport).toHaveAttribute("data-zoom", "1.00");
    const transform = await viewport.getAttribute("transform");
    const canvas = page.locator(".network-canvas");
    await canvas.scrollIntoViewIfNeeded();
    const box = await canvas.boundingBox();
    await page.mouse.move(box.x + box.width * 0.5, box.y + box.height * 0.5);
    await page.mouse.down();
    await page.mouse.move(box.x + box.width * 0.45, box.y + box.height * 0.45);
    await page.mouse.up();
    await expect(viewport).not.toHaveAttribute("transform", transform);
    await page.locator(".graph-node").first().click();
    await expect(page.locator("[data-issue-drawer]")).toHaveAttribute("aria-hidden", "false");
    await expect(page.getByRole("link", { name: /Open the full issue on GitHub/ })).toBeVisible();
    await page.getByRole("button", { name: "Close issue details" }).click();
    await expect(page.locator("[data-issue-drawer]")).toHaveAttribute("aria-hidden", "true");
  });

  test("glows every fixed-on-main node with its label color", async ({ page }) => {
    const fixedIssues = await page.evaluate(async () => {
      const response = await fetch("/assets/data/issues.json");
      const report = await response.json();
      return report.issues.flatMap((issue) => {
        const label = issue.labels.find((candidate) => candidate.name === "fixed-on-main");
        return label ? [{ number: issue.number, color: `#${label.color}` }] : [];
      });
    });

    const fixedNodes = page.locator(".graph-node--fixed-on-main");
    await expect(fixedNodes).toHaveCount(fixedIssues.length);
    for (const issue of fixedIssues) {
      const node = page.locator(`.graph-node[data-issue="${issue.number}"]`);
      await expect(node).toHaveClass(/graph-node--fixed-on-main/);
      expect(
        await node.evaluate((element) =>
          element.style.getPropertyValue("--fixed-on-main-color"),
        ),
      ).toBe(issue.color);
    }
    expect(
      await fixedNodes
        .first()
        .locator(".graph-node__dot")
        .evaluate((element) => getComputedStyle(element).filter),
    ).not.toBe("none");
  });

  test("filters fixed-on-main into the release verification queue", async ({ page }) => {
    await page.selectOption('select[name="label"]', "fixed-on-main");
    await expect(page.locator("[data-visible-count]")).toContainText(/Showing \d+ of \d+ issues/);
    const nodes = page.locator(".graph-node");
    expect(await nodes.count()).toBeGreaterThan(0);
    await expect(nodes.first()).toHaveClass(/graph-node--verify/);
    await nodes.first().click();
    await expect(page.locator(".drawer-kicker")).toContainText("verify next release");
    await expect(page.locator(".drawer-priority")).toContainText("Verify next release");
  });

  test("searches by issue number without losing context", async ({ page }) => {
    const issueNumber = await page.locator(".graph-node").first().getAttribute("data-issue");
    await page.locator('input[name="search"]').fill(`#${issueNumber}`);
    await expect(page.locator(".graph-node")).toHaveCount(1);
    await expect(page.locator("[data-visible-count]")).toContainText("Showing 1 of");
  });

  test("keeps the primary experience usable on a phone", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.reload();
    await expect(page.locator(".network-svg")).toBeVisible();
    await expect(page.locator(".atlas-hero")).toHaveCount(0);
    const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth);
  });
});

test.describe("issue overview", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/issues/overview/");
    await expect(page.locator("[data-visible-count]")) .toContainText("Showing");
  });

  test("holds the dashboard content away from the graph", async ({ page }) => {
    await expect(page.getByRole("heading", { name: /The work, made legible/ })).toBeVisible();
    await expect(page.getByText("“fixed-on-main” is a verification state, not done.")).toBeVisible();
    await expect(page.locator("[data-summary=open]")).not.toHaveText("—");
    await expect(page.locator(".summary-card")).toHaveCount(4);
    await expect(page.locator(".atlas-method")).toBeVisible();
    await expect(page.locator(".network-svg")).toHaveCount(0);
    await expect(page.getByRole("tab", { name: "Network" })).toHaveCount(0);
  });

  test("switches between runway, priority, and queue views", async ({ page }) => {
    await expect(page.locator("[data-view-panel=runway] .runway-bar").first()).toBeVisible();
    await page.getByRole("tab", { name: "Priority" }).click();
    await expect(page.locator("[data-view-panel=board] .board-lane").first()).toBeVisible();
    await page.getByRole("tab", { name: "Queue" }).click();
    await expect(page.locator("[data-view-panel=queue] tbody tr").first()).toBeVisible();
    await expect(page).toHaveURL(/view=queue/);
  });

  test("keeps the dashboard usable on a phone", async ({ page }) => {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.reload();
    await expect(page.locator(".atlas-hero h1")).toBeVisible();
    const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth);
  });
});
