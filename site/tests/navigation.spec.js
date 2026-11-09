import { expect, test } from '@playwright/test';

// [SITE-NAV-HIERARCHY] Reachable controls and clear hierarchy across themes and widths.
const DESKTOP = { width: 1440, height: 900 };
const MOBILE = { width: 390, height: 844 };
const MIN_TARGET = 44;
const CENTER_TOLERANCE = 1;
const SLOT_RADIUS = "4px";
const TRANSPARENT = "rgba(0, 0, 0, 0)";

async function expectTargets(page, selector) {
  // Measure settled geometry: a running reveal transform multiplies the rect
  // through a float matrix and shaves fractions of a pixel off exact targets.
  await page.evaluate(() => Promise.all(document.getAnimations()
    .filter(animation => Number.isFinite(animation.effect.getComputedTiming().endTime))
    .map(animation => animation.finished.catch(() => undefined))));
  const targets = await page.locator(selector).evaluateAll(elements => elements.map(element => {
    const { width, height } = element.getBoundingClientRect();
    return { label: element.textContent.trim(), width, height };
  }));
  for (const target of targets) {
    expect(target.height, target.label).toBeGreaterThanOrEqual(MIN_TARGET);
    expect(target.width, target.label).toBeGreaterThanOrEqual(MIN_TARGET);
  }
}

async function slotStyle(page, selector) {
  return page.locator(selector).first().evaluate(element => {
    const style = getComputedStyle(element);
    return {
      background: style.backgroundColor,
      radius: style.borderRadius,
      boxShadow: style.boxShadow,
      textDecoration: style.textDecorationLine,
      borderLeft: style.borderLeftWidth,
      borderBottom: style.borderBottomWidth,
      fontWeight: style.fontWeight,
    };
  });
}

async function expectDesktop(page) {
  await expectTargets(page, '.nav-link, .lang-link, .theme-toggle');
  const centerOffset = await page.locator('.nav-links').evaluate(element => {
    const { left, width } = element.getBoundingClientRect();
    return Math.abs(left + width / 2 - innerWidth / 2);
  });
  expect(centerOffset, 'primary navigation is centered independently of utilities')
    .toBeLessThanOrEqual(CENTER_TOLERANCE);
}

test('desktop navigation has accessible targets and current-page states', async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  for (const theme of ['dark', 'light']) {
    await page.goto('/docs/');
    await page.evaluate(value => document.documentElement.dataset.theme = value, theme);
    await expectDesktop(page);
    await expect(page.locator('.nav-link.active')).toHaveAttribute('aria-current', 'page');
    await expect(page.locator('.docs-sidebar__link.is-active')).toHaveAttribute('aria-current', 'page');
    const guides = page.locator('[data-docs-group="guides"]');
    for (const group of await page.locator('.docs-nav-group').all()) {
      await expect(group).toHaveAttribute('open', '');
    }
    await guides.locator('summary').focus();
    await page.keyboard.press('Enter');
    await expect(guides).not.toHaveAttribute('open', '');
    await page.keyboard.press('Enter');
    await expect(guides).toHaveAttribute('open', '');
    await expectTargets(page, '.docs-sidebar__link:visible, .docs-nav-group__summary');
    await page.getByRole('button', { name: 'Collapse documentation sidebar' }).click();
    await expect(page.locator('.docs-sidebar__nav')).toBeHidden();
    await page.getByRole('button', { name: 'Expand documentation sidebar' }).click();
    await expect(guides.locator('a').first()).toBeVisible();
  }
});

test('desktop destinations share one surface and drawer rows retain their own slots', async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  for (const theme of ['dark', 'light']) {
    await page.goto('/docs/');
    await page.evaluate(value => document.documentElement.dataset.theme = value, theme);

    const group = await slotStyle(page, '.nav-links');
    expect(group.background, `${theme}: desktop nav has a shared surface`).not.toBe(TRANSPARENT);
    const headerSlot = await slotStyle(page, '.nav-link:not(.active)');
    expect(headerSlot.background, `${theme}: inactive links show the shared surface`).toBe(TRANSPARENT);

    const activeHeader = await slotStyle(page, '.nav-link.active');
    expect(activeHeader.boxShadow, `${theme}: active top nav has no underline`).toBe('none');
    expect(activeHeader.borderBottom, `${theme}: active top nav has no underline border`).toBe('0px');
    expect(activeHeader.background, `${theme}: active top nav is a tonal fill`).not.toBe(TRANSPARENT);
    expect(activeHeader.background, `${theme}: active top nav fill differs from rest`).not.toBe(group.background);
    expect(activeHeader.fontWeight, `${theme}: active top nav uses medium weight`).toBe('600');

    const sidebarRow = await slotStyle(page, '.docs-sidebar__link:not(.is-active)');
    expect(sidebarRow.background, `${theme}: resting sidebar rows blend into the panel`).toBe(TRANSPARENT);
    expect(sidebarRow.radius, `${theme}: sidebar row uses the 4px radius`).toBe(SLOT_RADIUS);

    const activeRow = await slotStyle(page, '.docs-sidebar__link.is-active');
    expect(activeRow.boxShadow, `${theme}: active sidebar row has no left bar`).toBe('none');
    expect(activeRow.borderLeft, `${theme}: active sidebar row has no left border`).toBe('0px');
    expect(activeRow.background, `${theme}: active sidebar row is a tonal fill`).not.toBe(TRANSPARENT);
    expect(activeRow.background, `${theme}: active sidebar fill differs from rest`).not.toBe(sidebarRow.background);
    expect(activeRow.fontWeight, `${theme}: active sidebar row uses medium weight`).toBe('600');
  }

  await page.setViewportSize(MOBILE);
  await page.goto('/');
  await page.getByRole('button', { name: 'Toggle menu' }).click();
  const drawerSlot = await slotStyle(page, '.nav-link');
  expect(drawerSlot.background, 'mobile drawer rows sit on tonal surfaces').not.toBe(TRANSPARENT);
  expect(drawerSlot.radius, 'mobile drawer rows keep the 4px radius').toBe(SLOT_RADIUS);
  expect(drawerSlot.boxShadow, 'mobile drawer rows carry no markers').toBe('none');
  await page.keyboard.press('Escape');
});

test('mobile navigation fits and respects reduced motion', async ({ page }) => {
  await page.setViewportSize(MOBILE);
  await page.emulateMedia({ reducedMotion: 'reduce' });
  for (const route of ['/', '/docs/', '/docs/for-ai/', '/zh/docs/']) {
    await page.goto(route);
    await expectTargets(page, '.lang-link, .theme-toggle, .mobile-menu-toggle');
    await page.locator('.mobile-menu-toggle').click();
    await expect(page.locator('body')).toHaveClass(/menu-open/);
    const drawer = page.locator(route === '/' ? '.nav-links' : '.docs-sidebar');
    const siteLinks = drawer.locator('.nav-link');
    await expect(siteLinks).toHaveCount(3);
    await expect(page.locator('.nav-link')).toHaveCount(3);
    for (const group of await drawer.locator('.docs-nav-group').all()) {
      await expect(group).toHaveAttribute('open', '');
    }
    await expect(drawer).toHaveCSS('transition-duration', '1e-05s');
    await expect.poll(async () => (await drawer.boundingBox()).x).toBe(0);
    const bounds = await drawer.boundingBox();
    expect(bounds.x + bounds.width).toBeLessThanOrEqual(MOBILE.width);
    await expect(siteLinks.first()).toBeInViewport();
    await page.keyboard.press('Escape');
    await expect(page.locator('body')).not.toHaveClass(/menu-open/);
  }
  await page.setViewportSize(DESKTOP);
  await expect(page.locator('.site-header .nav-link')).toHaveCount(3);
  await expect(page.locator('.docs-sidebar .nav-link')).toHaveCount(0);
});
