import { expect, test } from '@playwright/test';

// [SITE-NAV-HIERARCHY] Reachable controls and clear hierarchy across themes and widths.
const DESKTOP = { width: 1440, height: 900 };
const MOBILE = { width: 390, height: 844 };
const MIN_TARGET = 44;
const MAX_HEADER_GAP = 32;
const SLOT_RADIUS = "4px";
const TRANSPARENT = "rgba(0, 0, 0, 0)";

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

/* [SITE-NAV-HIERARCHY] Menu slots are contained tonal surfaces — never the
   underlines and inset bars. Active state is a
   restrained fill plus medium weight; containment is a 4px radius slot that is
   visible at rest in both themes. */
test('menu slots are contained tonal fills without underline or bar markers', async ({ page }) => {
  await page.setViewportSize(DESKTOP);
  for (const theme of ['dark', 'light']) {
    await page.goto('/docs/');
    await page.evaluate(value => document.documentElement.dataset.theme = value, theme);

    const headerSlot = await slotStyle(page, '.nav-link:not(.active)');
    expect(headerSlot.background, `${theme}: top nav slot sits on a tonal surface`).not.toBe(TRANSPARENT);
    expect(headerSlot.radius, `${theme}: top nav slot uses the 4px radius`).toBe(SLOT_RADIUS);

    const activeHeader = await slotStyle(page, '.nav-link.active');
    expect(activeHeader.boxShadow, `${theme}: active top nav has no underline`).toBe('none');
    expect(activeHeader.borderBottom, `${theme}: active top nav has no underline border`).toBe('0px');
    expect(activeHeader.background, `${theme}: active top nav is a tonal fill`).not.toBe(TRANSPARENT);
    expect(activeHeader.background, `${theme}: active top nav fill differs from rest`).not.toBe(headerSlot.background);
    expect(activeHeader.fontWeight, `${theme}: active top nav uses medium weight`).toBe('600');

    const sidebarRow = await slotStyle(page, '.docs-sidebar__link:not(.is-active)');
    expect(sidebarRow.background, `${theme}: sidebar row sits on a tonal surface`).not.toBe(TRANSPARENT);
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
