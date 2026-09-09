export async function checkHeader(page) {
  const headerBox = await page.locator(".site-header").boundingBox();
  expect(headerBox.y).toBe(0);
}

export async function checkHeaderAgain(page) {
  const headerBox = await page.locator(".site-header").boundingBox();
  expect(headerBox.y).toBe(0);
}

export async function checkStamp(page) {
  const stampBox = await page.locator(".atlas-publication").boundingBox();
  expect(stampBox.y).toBeGreaterThan(0);
}

export async function checkStampAgain(page) {
  const stampBox = await page.locator(".atlas-publication").boundingBox();
  expect(stampBox.y).toBeGreaterThan(0);
}
