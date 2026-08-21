import { expect, test } from "@playwright/test";

const representativeRoutes = [
  "/",
  "/zh/",
  "/docs/",
  "/zh/docs/",
  "/blog/",
  "/zh/blog/",
  "/blog/live-mcp-lsp-duplicate-code-prevention/",
  "/zh/blog/live-mcp-lsp-duplicate-code-prevention/",
  "/releases/",
  "/zh/releases/",
];

const isInScope = (pathname) => !pathname.startsWith("/issues/") && pathname !== "/issues/";

test("declares the social image with its actual MIME type", async ({ page }) => {
  await page.goto("/blog/towards-100-percent-accuracy/");
  const socialImageType = await page.locator('meta[property="og:image:type"]').getAttribute("content");
  expect(socialImageType).toBe("image/png");
});

test("publishes complete, parseable metadata on every ordinary page", async ({ page }) => {
  const sitemapResponse = await page.request.get("/sitemap.xml");
  expect(sitemapResponse.ok()).toBeTruthy();
  const sitemap = await sitemapResponse.text();
  const routes = await page.evaluate((xml) => {
    const doc = new DOMParser().parseFromString(xml, "application/xml");
    if (doc.querySelector("parsererror")) throw new Error("Invalid sitemap XML");
    return [...doc.querySelectorAll("loc")].map((node) => new URL(node.textContent).pathname);
  }, sitemap);

  expect(routes.some((route) => route.includes("/tags/"))).toBeFalsy();
  expect(routes.some((route) => route.includes("/categories/"))).toBeFalsy();

  for (const route of routes.filter(isInScope)) {
    const response = await page.goto(route);
    expect(response?.ok(), route).toBeTruthy();
    const metadata = await page.evaluate(() => ({
      title: document.title,
      description: document.querySelector('meta[name="description"]')?.content,
      canonical: document.querySelector('link[rel="canonical"]')?.href,
      ogTitle: document.querySelector('meta[property="og:title"]')?.content,
      ogDescription: document.querySelector('meta[property="og:description"]')?.content,
      ogImage: document.querySelector('meta[property="og:image"]')?.content,
      ogImageWidth: document.querySelector('meta[property="og:image:width"]')?.content,
      ogImageHeight: document.querySelector('meta[property="og:image:height"]')?.content,
      twitterCard: document.querySelector('meta[name="twitter:card"]')?.content,
      twitterImage: document.querySelector('meta[name="twitter:image"]')?.content,
      h1Count: document.querySelectorAll("h1").length,
      alternates: [...document.querySelectorAll('link[rel="alternate"][hreflang]')].map((node) => ({
        lang: node.hreflang,
        href: node.href,
      })),
      jsonLdRaw: [...document.querySelectorAll('script[type="application/ld+json"]')].map((node) => node.textContent),
      jsonLd: [...document.querySelectorAll('script[type="application/ld+json"]')].map((node) => JSON.parse(node.textContent)),
    }));

    expect(metadata.title, route).toBeTruthy();
    expect(metadata.title.length, route).toBeLessThanOrEqual(70);
    expect(metadata.description, route).toBeTruthy();
    expect(metadata.description.length, route).toBeLessThanOrEqual(180);
    expect(metadata.canonical, route).toBe(`https://deslop.live${route}`);
    expect(metadata.ogTitle, route).toBe(metadata.title);
    expect(metadata.ogDescription, route).toBe(metadata.description);
    expect(metadata.ogImage, route).toMatch(/^https:\/\/deslop\.live\/assets\/img\//);
    expect(metadata.ogImageWidth, route).toBe("1200");
    expect(metadata.ogImageHeight, route).toBe("630");
    expect(metadata.twitterCard, route).toBe("summary_large_image");
    expect(metadata.twitterImage, route).toBe(metadata.ogImage);
    expect(metadata.h1Count, route).toBe(1);
    expect(metadata.jsonLd.length, route).toBeGreaterThanOrEqual(1);
    expect(metadata.jsonLdRaw.some((value) => value.includes("&#")), route).toBeFalsy();
    expect(metadata.alternates.map(({ lang }) => lang).sort(), route).toEqual(["en", "x-default", "zh"]);

    const entities = metadata.jsonLd.flatMap((value) => value["@graph"] || [value]);
    const pageEntities = entities.filter((entity) => ["WebPage", "TechArticle", "BlogPosting"].includes(entity["@type"]));
    const isBlogPost = /^\/(?:zh\/)?blog\/[^/]+\/$/.test(route);
    const isDocsPage = /^\/(?:zh\/)?docs\//.test(route);
    const expectedPageType = isBlogPost ? "BlogPosting" : isDocsPage ? "TechArticle" : "WebPage";
    expect(pageEntities, route).toHaveLength(1);
    expect(pageEntities[0]["@type"], route).toBe(expectedPageType);
    expect(pageEntities[0].inLanguage, route).toBe(route.startsWith("/zh/") ? "zh" : "en");
    expect(Boolean(pageEntities[0].datePublished), route).toBe(isBlogPost);
    expect(entities.filter((entity) => entity["@type"] === "WebSite"), route).toHaveLength(1);
    expect(entities.filter((entity) => entity["@id"] === "https://deslop.live/#organization"), route).toHaveLength(1);
    expect(entities.filter((entity) => entity["@type"] === "BreadcrumbList"), route).toHaveLength(route === "/" || route === "/zh/" ? 0 : 1);

    const socialImage = await page.request.get(new URL(metadata.ogImage).pathname);
    expect(socialImage.ok(), metadata.ogImage).toBeTruthy();
    if (isBlogPost) expect(metadata.ogImage.endsWith("/og-card.png"), route).toBeFalsy();
  }
});

test("keeps feed, robots, and archive cleanup crawler-safe", async ({ page }) => {
  const feedResponse = await page.request.get("/feed.xml");
  expect(feedResponse.ok()).toBeTruthy();
  const feed = await feedResponse.text();
  const parsedFeed = await page.evaluate((xml) => {
    const doc = new DOMParser().parseFromString(xml, "application/xml");
    return {
      invalid: Boolean(doc.querySelector("parsererror")),
      entries: doc.querySelectorAll("entry").length,
      summaries: [...doc.querySelectorAll("summary")].map((node) => node.textContent),
    };
  }, feed);
  expect(parsedFeed.invalid).toBeFalsy();
  expect(parsedFeed.entries).toBe(8);
  expect(parsedFeed.summaries.every((summary) => !summary.includes("%3C"))).toBeTruthy();

  const robots = await (await page.request.get("/robots.txt")).text();
  expect(robots).toContain("User-agent: *");
  expect(robots).toContain("Allow: /");
  expect(robots).toContain("Sitemap: https://deslop.live/sitemap.xml");

  await expect((await page.request.get("/blog/tags/")).status()).toBe(404);
  await expect((await page.request.get("/zh/blog/categories/")).status()).toBe(404);
});

for (const viewport of [
  { width: 320, height: 720 },
  { width: 390, height: 844 },
  { width: 768, height: 1024 },
  { width: 1024, height: 768 },
]) {
  test(`has no horizontal overflow at ${viewport.width}px`, async ({ page }) => {
    await page.setViewportSize(viewport);
    for (const route of representativeRoutes) {
      await page.goto(route);
      const dimensions = await page.evaluate(() => ({
        scrollWidth: document.documentElement.scrollWidth,
        clientWidth: document.documentElement.clientWidth,
      }));
      expect(dimensions.scrollWidth, route).toBeLessThanOrEqual(dimensions.clientWidth);
    }
  });
}

test("mobile navigation and touch targets remain usable", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/docs/");
  await page.locator(".mobile-menu-toggle").click();
  await expect(page.locator(".docs-sidebar")).toHaveClass(/open/);

  const tooSmall = await page.locator(".docs-sidebar a:visible, .docs-sidebar summary:visible").evaluateAll((elements) =>
    elements
      .map((element) => ({ text: element.textContent.trim(), height: element.getBoundingClientRect().height }))
      .filter((item) => item.height < 44),
  );
  expect(tooSmall).toEqual([]);

  await page.keyboard.press("Escape");
  await expect(page.locator("body")).not.toHaveClass(/menu-open/);

  await page.goto("/blog/");
  const blogTargets = await page.locator(".post-card__title, .post-card__more").evaluateAll((elements) =>
    elements.map((element) => element.getBoundingClientRect().height),
  );
  expect(blogTargets.every((height) => height >= 44)).toBeTruthy();
});

test("keyboard focus is visible and responsive blog images are selected", async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.goto("/blog/live-mcp-lsp-duplicate-code-prevention/");
  await page.keyboard.press("Tab");
  const focus = await page.evaluate(() => {
    const element = document.activeElement;
    const style = getComputedStyle(element);
    return { tag: element.tagName, outlineWidth: style.outlineWidth, outlineStyle: style.outlineStyle };
  });
  expect(focus.tag).toBe("A");
  expect(focus.outlineStyle).not.toBe("none");
  expect(Number.parseFloat(focus.outlineWidth)).toBeGreaterThanOrEqual(2);

  const currentSrc = await page.locator(".prose-hero__img").evaluate((image) => image.currentSrc);
  expect(currentSrc).toContain("-800.webp");
});

test("internal page links and fragments resolve", async ({ page }) => {
  const checked = new Set();
  for (const route of representativeRoutes) {
    await page.goto(route);
    const links = await page.locator('a[href^="/"]').evaluateAll((anchors) =>
      anchors.map((anchor) => anchor.href),
    );
    for (const href of links) {
      const url = new URL(href);
      if (!isInScope(url.pathname) || checked.has(url.href)) continue;
      checked.add(url.href);
      const response = await page.request.get(`${url.pathname}${url.search}`);
      expect(response.status(), url.href).toBeLessThan(400);
      if (url.hash) {
        await page.goto(url.pathname);
        expect(await page.locator(url.hash).count(), url.href).toBeGreaterThan(0);
      }
    }
  }
});
