import { expect, test } from '@playwright/test';

// [SITE-NAV-HIERARCHY] Reachable controls and clear hierarchy across themes and widths.
const DESKTOP = { width: 1440, height: 900 };
const MOBILE = { width: 390, height: 844 };
const MIN_TARGET = 44;
const MAX_HEADER_GAP = 32;

async function expectTargets(page, selector) {
  const targets = await page.locator(selector).evaluateAll(elements => elements.map(element => {
    const { width, height } = element.getBoundingClientRect();
    return { label: element.textContent.trim(), width, height };
  }));
  for (const target of targets) {
    expect(target.height, target.label).toBeGreaterThanOrEqual(MIN_TARGET);
    expect(target.width, target.label).toBeGreaterThanOrEqual(MIN_TARGET);
  }
}

async function expectDesktop(page) {
  await expectTargets(page, '.nav-link, .lang-link, .theme-toggle');
  const gap = await page.evaluate(() => {
    const links = document.querySelector('.nav-links').getBoundingClientRect();
    const actions = document.querySelector('.nav-actions').getBoundingClientRect();
    return actions.left - links.right;
  });
  expect(gap, 'navigation and utilities form one group').toBeLessThanOrEqual(MAX_HEADER_GAP);
}

test('desktop navigation has accessible targets and current-page states', async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  for (const theme of ['dark', 'light']) {
    await page.goto('/docs/');
    await page.evaluate(value => document.documentElement.dataset.theme = value, theme);
    await expectDesktop(page);
    await expect(page.locator('.docs-sidebar__link.is-active')).toHaveAttribute('aria-current', 'page');
    const guides = page.locator('[data-docs-group="guides"]');
    await guides.locator('summary').focus();
    await page.keyboard.press('Enter');
    await expect(guides).toHaveAttribute('open', '');
    await expectTargets(page, '.docs-sidebar__link:visible, .docs-nav-group__summary');
    await page.getByRole('button', { name: 'Collapse documentation sidebar' }).click();
    await expect(page.locator('.docs-sidebar__nav')).toBeHidden();
    await page.getByRole('button', { name: 'Expand documentation sidebar' }).click();
    await expect(guides.locator('a').first()).toBeVisible();
  }
});

test('mobile navigation fits and respects reduced motion', async ({ page }) => {
  await page.setViewportSize(MOBILE);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const route of ['/', '/docs/', '/zh/docs/']) {
    await page.goto(route);
    await expectTargets(page, '.lang-link, .theme-toggle, .mobile-menu-toggle');
    await page.locator('.mobile-menu-toggle').click();
    await expect(page.locator('body')).toHaveClass(/menu-open/);
    const drawer = page.locator(route === '/' ? '.nav-links' : '.docs-sidebar');
    await expect(drawer).toHaveCSS('transition-duration', '1e-05s');
    expect(await drawer.evaluate(element => element.getBoundingClientRect().right)).toBeLessThanOrEqual(MOBILE.width);
    await page.keyboard.press('Escape');
    await expect(page.locator('body')).not.toHaveClass(/menu-open/);
  }
});
