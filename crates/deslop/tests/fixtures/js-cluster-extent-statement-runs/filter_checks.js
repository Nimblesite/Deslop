export const filterChecks = {
  async openGraphFilters(page, expect) {
    await page.locator(".graph-filter-panel summary").click();
    await page.getByLabel("Only regressions").check();
    await expect(page.locator(".graph-filter-panel[open]")).toHaveCount(1);
  },

  async checkPriorityChipContrast(page, expect) {
    const chips = page.locator(".priority-board .label-chip");
    expect(await chips.count()).toBeGreaterThan(0);
    await expect(chips.first()).toHaveAttribute("data-priority", "p0");
  },

  async checkTestimonialAuthor(page, expect) {
    const testimonial = page.locator(".testimonial").nth(2);
    await expect(testimonial.locator(".testimonial__author")).toHaveText("Dana Whitfield");
    await expect(testimonial.locator(".testimonial__role")).toHaveText("Staff Engineer");
    await expect(testimonial.locator("blockquote")).not.toBeEmpty();
  },

  async dismissCookieBanner(page, expect) {
    await page.locator(".mobile-menu-toggle").click();
    await page.getByRole("button", { name: "Accept analytics" }).click();
    await expect(page.locator(".cookie-banner")).toBeHidden();
  },

  async searchFromMenu(page, expect) {
    await page.locator(".site-header .search-trigger").click();
    await page.getByPlaceholder("Search the docs").fill("duplicate detection");
    await expect(page.locator(".search-results li")).toHaveCount(7);
  },
};
