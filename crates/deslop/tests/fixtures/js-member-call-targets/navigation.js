export async function inspectNavigation(nav, expect) {
  await expect(nav.locator('a[href="/issues/"]')).toContainText("Issue graph");
  await expect(nav.locator('a[href="/issues/planner/"]')).toContainText("Issue planner");
}
