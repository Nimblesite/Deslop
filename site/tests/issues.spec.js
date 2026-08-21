import { expect, test } from "@playwright/test";

test("keeps the issue map in documentation navigation, not the main nav", async ({ page }) => {
  await page.goto("/");
  await expect(
    page.locator(".site-header .nav-links").getByRole("link", {
      name: "Issue map",
      exact: true,
    }),
  ).toHaveCount(0);

  await page.goto("/docs/");
  await expect(
    page.locator(".docs-sidebar__nav").getByRole("link", {
      name: "Issue map",
      exact: true,
    }),
  ).toHaveAttribute("href", "/issues/");
});

test.describe("issue atlas", () => {
  test.beforeEach(async ({ page }) => {
    await page.goto("/issues/");
    await expect(page.locator("[data-visible-count]")).toContainText("Showing");
  });

  test("explains the backlog and renders the relationship graph", async ({ page }) => {
    await expect(page.getByRole("heading", { name: /The work, made legible/ })).toBeVisible();
    await expect(page.getByText("“fixed-on-main” is a verification state, not done.")).toBeVisible();
    await expect(page.locator(".graph-node")).not.toHaveCount(0);
    await expect(page.locator(".graph-halo")).toHaveCount(8);
    await expect(page.locator("[data-summary=open]")).not.toHaveText("—");
  });

  test("zooms the graph and opens useful issue detail", async ({ page }) => {
    const viewport = page.locator(".graph-viewport");
    await expect(viewport).toHaveAttribute("data-zoom", "1");
    await page.getByRole("button", { name: "Zoom in" }).click();
    await expect(viewport).not.toHaveAttribute("data-zoom", "1");
    await page.locator(".graph-node").first().click();
    await expect(page.locator("[data-issue-drawer]")).toHaveAttribute("aria-hidden", "false");
    await expect(page.getByRole("link", { name: /Open the full issue on GitHub/ })).toBeVisible();
    await page.getByRole("button", { name: "Close issue details" }).click();
    await expect(page.locator("[data-issue-drawer]")).toHaveAttribute("aria-hidden", "true");
  });

  test("filters fixed-on-main into the release verification queue", async ({ page }) => {
    await page.selectOption('select[name="label"]', "fixed-on-main");
    await expect(page.locator("[data-visible-count]")).toContainText(/Showing \d+ of \d+ issues/);
    const nodes = page.locator(".graph-node");
    expect(await nodes.count()).toBeGreaterThan(0);
    await nodes.first().click();
    await expect(page.locator(".drawer-kicker")).toContainText("verify next release");
    await expect(page.locator(".drawer-priority")).toContainText("Verify next release");
  });

  test("switches between runway, priority, and queue views", async ({ page }) => {
    await page.getByRole("tab", { name: "Runway" }).click();
    await expect(page.locator("[data-view-panel=runway] .runway-bar").first()).toBeVisible();
    await page.getByRole("tab", { name: "Priority" }).click();
    await expect(page.locator("[data-view-panel=board] .board-lane").first()).toBeVisible();
    await page.getByRole("tab", { name: "Queue" }).click();
    await expect(page.locator("[data-view-panel=queue] tbody tr").first()).toBeVisible();
    await expect(page).toHaveURL(/view=queue/);
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
    await expect(page.locator(".atlas-hero h1")).toBeVisible();
    await expect(page.locator(".network-svg")).toBeVisible();
    const viewportWidth = await page.evaluate(() => document.documentElement.clientWidth);
    const bodyWidth = await page.evaluate(() => document.body.scrollWidth);
    expect(bodyWidth).toBeLessThanOrEqual(viewportWidth);
  });
});
