export const navigationChecks = {
  async openMobileDrawer(page, expect) {
    await page.setViewportSize({ width: 390, height: 844 });
    await page.goto("/docs/vscode-cluster-panel/");
    await page.getByRole("button", { name: "Toggle menu" }).click();
    const drawer = page.locator(".docs-sidebar");
    await expect(drawer).toHaveClass(/open/);
  },

  async checkGroupedSidebar(page, expect) {
    const sidebar = page.locator(".docs-sidebar");
    await expect(sidebar.locator('[data-docs-group="trust"]')).toHaveCount(4);
    await expect(sidebar.getByText(/Live duplicate-code analysis/)).toHaveCount(0);
  },

  async checkStatisticsHeading(page, expect) {
    await expect(page.getByRole("heading", { name: "Issue statistics" })).toBeVisible();
    await expect(page.locator(".issue-stats__total")).toHaveText("1,204");
  },

  async checkRunwayTab(page, expect) {
    await page.getByRole("tab", { name: "Runway" }).click();
    const runway = page.locator("#panel-runway");
    await expect(runway).toBeVisible();
    await expect(runway.locator(".runway-week")).toHaveCount(12);
    await expect(runway.getByText("Burn rate")).toBeVisible();
  },

  async checkLocalisedDocsLinks(page, expect) {
    const docsNav = page.locator("nav.docs-nav");
    await expect(docsNav.locator('a[href="/issues/"]')).toContainText("问题");
    await expect(docsNav.locator('a[href="/zh/docs/"]')).toContainText("文档");
  },
};
