import { chromium } from '@playwright/test';
import { writeFile } from 'node:fs/promises';

// [SITE-KEYWORDS] Capture observed search interest, never infer search counts.
const terms = ['duplicate code', 'code duplication', 'code clone', 'duplicate code detector'];
const url = `https://trends.google.com/trends/explore?date=today%205-y&q=${encodeURIComponent(terms.join(','))}&hl=en`;
const browser = await chromium.launch({ headless: true });
try {
  const page = await browser.newPage();
  await page.goto('https://trends.google.com/trends/', { waitUntil: 'domcontentloaded' });
  const response = page.waitForResponse(r => r.url().includes('/widgetdata/multiline'), { timeout: 45000 });
  await page.goto(url, { waitUntil: 'domcontentloaded' });
  let evidence;
  try {
    const result = await response;
    const body = await result.text();
    const data = JSON.parse(body.slice(body.indexOf('{')));
    const points = data.default?.timelineData ?? [];
    evidence = points.length ? terms.map((term, index) => {
      const present = points.filter(p => p.hasData?.[index] && p.value[index] > 0);
      return `| ${term} | ${present.length} | ${points.length} |`;
    }).join('\n') : `No timeline returned (HTTP ${result.status()}).`;
  } catch (error) {
    evidence = `Timeline unavailable: ${error.message.split('\n')[0]}\n\n${(await page.locator('body').innerText()).slice(0, 2500)}`;
  }
  await writeFile('../reports/research/keyword-interest.md', `# Keyword interest capture\n\nCaptured mechanically at ${new Date().toISOString()} using Playwright. Worldwide, past five years, all categories.\n\nSource: ${url}\n\n| Query | Timeline points with nonzero observed interest | Total points |\n| --- | ---: | ---: |\n${evidence}\n\nInterest is normalized, not search volume. Zero or unavailable data does not prove no searches. Broad queries can include unrelated intent; these results do not establish C-specific demand.\n`);
  console.log(evidence);
} finally {
  await browser.close();
}
