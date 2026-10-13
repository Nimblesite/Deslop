import { expect } from "@playwright/test";

const CONTENT_TYPE_HEADER = "content-type";

/** [SITE-ASSET-TYPE] Verify the actual asset response against its declared type. */
export async function expectAssetType(page, assetPath, expectedType, contextLabel = assetPath) {
  const response = await page.request.get(assetPath);
  expect(response.ok(), contextLabel).toBeTruthy();
  expect(response.headers()[CONTENT_TYPE_HEADER], contextLabel).toContain(expectedType);
}
