import { test, expect } from "@playwright/test";

test("sidebar sits below the header on the docs page", async ({ page }) => {
  await page.goto("/docs/");
  await page.reload();
  const headerBox = await page.locator(".site-header").boundingBox();
  const sidebarBox = await page.locator(".docs-sidebar").boundingBox();
  expect(sidebarBox.y).toBeGreaterThanOrEqual(headerBox.y + headerBox.height);
});

test("sidebar sits below the header on the issues page", async ({ page }) => {
  await page.goto("/issues/");
  await page.reload();
  const headerBox = await page.locator(".site-header").boundingBox();
  const sidebarBox = await page.locator(".docs-sidebar").boundingBox();
  expect(sidebarBox.y).toBeGreaterThanOrEqual(headerBox.y + headerBox.height);
});
