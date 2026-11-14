import fs from 'node:fs';
import {expect, test} from '@playwright/test';
// [SITE-PYTHON-LANDING] The Python landing page: a direct install, real Click clones, the real film.
const COPY = JSON.parse(fs.readFileSync(new URL('../src/_data/pythonLanding.json', import.meta.url), 'utf8'));
const INSTALL = 'vscode:extension/Nimblesite.deslop-live';
const MARKETPLACE = 'https://marketplace.visualstudio.com/items?itemName=Nimblesite.deslop-live';
const FILM = '/assets/video/docs/deslop-docs-python-click.mp4';
const PYLINT_DUPLICATE_CODE = 'https://pylint.readthedocs.io/en/stable/user_guide/messages/refactor/duplicate-code.html';
const INSTALL_BUTTONS = 2;
const CLONE_CARDS = 3;
const ROUTES = {en: '/python/', zh: '/zh/python/'};

for (const [lang, route] of Object.entries(ROUTES)) {
  test(`${lang}: every call to action installs Deslop straight into VS Code`, async ({page}) => {
    const copy = COPY[lang];
    await page.goto(route);
    await expect(page).toHaveTitle(copy.title);
    await expect(page.locator('meta[name="description"]')).toHaveAttribute('content', copy.description);
    await expect(page.locator('h1')).toHaveText(`${copy.headline} ${copy.headlineEm}`);
    const installs = page.locator('[data-cta="install"]');
    await expect(installs).toHaveCount(INSTALL_BUTTONS);
    for (const button of await installs.all()) {
      await expect(button).toHaveAttribute('href', INSTALL);
      await expect(button).toHaveText(copy.install);
      expect((await button.boundingBox()).height).toBeGreaterThanOrEqual(48);
    }
    await expect(page.locator('.py-hero').getByRole('link', {name: copy.marketplace})).toHaveAttribute('href', MARKETPLACE);
    await expect(page.locator('.py-hero__note').first()).toHaveText(copy.installNote);
    await expect(page.locator('meta[property="og:image"]')).toHaveAttribute('content', 'https://deslop.live/assets/img/python-og.png');
    expect((await page.request.get('/assets/img/python-og.png')).ok()).toBe(true);
    await expect(page.locator('.site-footer a[href$="/python/"]')).toHaveCount(1);
  });

  test(`${lang}: the real Click film is the first thing on the page`, async ({page}) => {
    const copy = COPY[lang];
    await page.goto(route);
    const film = page.locator('.py-hero #film video');
    await expect(film.locator('source')).toHaveAttribute('src', FILM);
    await expect(film).toBeInViewport({ratio: 0.9});
    await expect(page.locator('.py-hero video')).toHaveCount(1);
    await expect(page.locator('#py-film-title')).toHaveText(copy.filmTitle);
    expect((await page.request.get(FILM)).ok()).toBe(true);
    await film.evaluate(video => { video.muted = true; return video.play(); });
    await expect.poll(() => film.evaluate(video => video.currentTime)).toBeGreaterThan(0);
  });

  test(`${lang}: clone cards, steps and FAQ render the copy and its structured data`, async ({page}) => {
    const copy = COPY[lang];
    await page.goto(route);
    const cards = page.locator('.py-card');
    await expect(cards).toHaveCount(CLONE_CARDS);
    for (const [index, clone] of copy.clones.entries()) {
      await expect(cards.nth(index).locator('h3')).toHaveText(clone.title);
      await expect(cards.nth(index).locator('.py-card__where')).toHaveText(clone.where);
      await expect(cards.nth(index).locator('.py-card__code code.language-python')).toHaveText(clone.code);
      expect(await cards.nth(index).locator('.py-card__code .token.keyword').count()).toBeGreaterThan(0);
    }
    await expect(page.locator('.py-step h3')).toHaveText(copy.steps.map(step => step.title));
    const pylint = page.locator('.py-lint a');
    await expect(pylint).toHaveAttribute('href', PYLINT_DUPLICATE_CODE);
    const faq = page.locator('.py-faq__item');
    await expect(faq.locator('summary')).toHaveText(copy.faq.map(item => item.q));
    await expect(faq.first()).toHaveAttribute('open', '');
    await faq.nth(1).locator('summary').click();
    await expect(faq.nth(1).locator('p')).toBeVisible();
    await expect(faq.nth(1).locator('p')).toHaveText(copy.faq[1].a);
    const blocks = await page.locator('script[type="application/ld+json"]').allTextContents();
    const faqPage = blocks.map(block => JSON.parse(block)).find(block => block['@type'] === 'FAQPage');
    expect(faqPage.mainEntity.map(entity => entity.name)).toEqual(copy.faq.map(item => item.q));
    expect(faqPage.mainEntity.map(entity => entity.acceptedAnswer.text)).toEqual(copy.faq.map(item => item.a));
  });
}

test('motion runs by default and stops entirely under reduced motion', async ({page}) => {
  await page.goto(ROUTES.en);
  expect(await page.evaluate(() => document.getAnimations().length)).toBeGreaterThan(0);
  await page.emulateMedia({reducedMotion: 'reduce'});
  for (const colorScheme of ['dark', 'light']) {
    await page.emulateMedia({colorScheme});
    await page.goto(ROUTES.en);
    expect(await page.evaluate(() => document.getAnimations().length)).toBe(0);
    await expect(page.locator('h1')).toBeVisible();
    await expect(page.locator('[data-cta="install"]').first()).toBeVisible();
  }
});
