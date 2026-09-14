export async function inspectStatus(page, expect) {
  await expect(page.locator(".graph-legend")).toContainText("Blocks →");
  await expect(page.locator(".statistics-source")).toContainText("No AI enrichment");
}
