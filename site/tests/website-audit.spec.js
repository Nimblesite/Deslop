import { readdir, readFile } from "node:fs/promises";
import { extname } from "node:path";
import { expect, test } from "@playwright/test";
import { expectAssetType } from "./helpers/assets.js";

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
  "/python/",
  "/zh/python/",
];

// Every English blog post appears in the feed; the Chinese translations do not.
// Bump this when a post is added so the feed cannot silently lose an entry.
const ENGLISH_BLOG_POST_COUNT = 12;

const isInScope = (pathname) => !pathname.startsWith("/issues/") && pathname !== "/issues/";

// The MIME type each social-card extension actually serves, mirroring
// SOCIAL_IMAGE_MIME_TYPES in eleventy.config.js. Written out here rather than
// imported so the test states the expected mapping independently of the code
// that produces it.
const SOCIAL_IMAGE_MIME_TYPES = {
  png: "image/png",
  jpg: "image/jpeg",
  jpeg: "image/jpeg",
  webp: "image/webp",
};

const socialCardExtension = (imageUrl) => new URL(imageUrl).pathname.split(".").pop().toLowerCase();

test("declares the social image with its actual MIME type", async ({ page }) => {
  await page.goto("/blog/towards-100-percent-accuracy/");
  const socialImage = await page.locator('meta[property="og:image"]').getAttribute("content");
  const socialImageType = await page.locator('meta[property="og:image:type"]').getAttribute("content");
  expect(socialImage).toMatch(/towards-100-percent-accuracy-og\.png$/);
  expect(socialImageType).toBe("image/png");

  // The declaration used to be the literal "image/png" on every page, so it
  // was right here and wrong on the seven blog posts whose cards are JPEGs.
  // One page proves nothing; the pair has to agree everywhere.
  //
  // Fetched from the site under test by pathname, not from the absolute
  // production URL the tag carries: the assertion is about the card *this
  // build ships*. Against production it passed only for posts already
  // deployed and failed for every new one, which makes it a deployment
  // check wearing a build check's name.
  await expectAssetType(page, new URL(socialImage).pathname, socialImageType, socialImage);
});

test("every page's declared social-image type matches the file it points at", async ({ page }) => {
  const sitemapResponse = await page.request.get("/sitemap.xml");
  expect(sitemapResponse.ok()).toBeTruthy();
  const routes = await page.evaluate((xml) => {
    const doc = new DOMParser().parseFromString(xml, "application/xml");
    if (doc.querySelector("parsererror")) throw new Error("Invalid sitemap XML");
    return [...doc.querySelectorAll("loc")].map((node) => new URL(node.textContent).pathname);
  }, await sitemapResponse.text());

  const inScope = routes.filter(isInScope);
  expect(inScope.length, "the sitemap must list pages to audit").toBeGreaterThan(0);

  const seenTypes = new Set();
  for (const route of inScope) {
    const response = await page.goto(route);
    expect(response?.ok(), route).toBeTruthy();
    const socialImage = await page.locator('meta[property="og:image"]').getAttribute("content");
    const socialImageType = await page
      .locator('meta[property="og:image:type"]')
      .getAttribute("content");
    expect(socialImage, `${route}: declares no og:image`).toBeTruthy();
    const extension = socialCardExtension(socialImage);
    expect(SOCIAL_IMAGE_MIME_TYPES[extension], `${route}: unmapped card extension .${extension}`).toBeTruthy();
    expect(socialImageType, `${route}: og:image ${socialImage}`).toBe(
      SOCIAL_IMAGE_MIME_TYPES[extension]
    );
    seenTypes.add(socialImageType);
  }

  // The site ships both JPEG and PNG cards. If every page suddenly agreed on
  // one type the loop above would still pass while the hardcoded-constant bug
  // was back, so pin that both are actually represented.
  expect([...seenTypes].sort(), "both card formats must still be in use").toEqual([
    "image/jpeg",
    "image/png",
  ]);
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
  expect(parsedFeed.entries).toBe(ENGLISH_BLOG_POST_COUNT);
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

// [SITE-ASSET-TYPE] Publish exactly the requested demo, ten documentation films and the Python film [DOC-FILMS-PYTHON].
const SITE_SOURCE = new URL("../src/", import.meta.url);
const ENCODED_MEDIA = new Set([".aac", ".avi", ".flac", ".m4a", ".m4v", ".mkv", ".mov", ".mp3", ".mp4", ".ogg", ".wav", ".webm"]);
const DOC_FILM_SLUGS = [
  "getting-started", "ai-integration", "vscode-cluster-panel", "github-action", "osprey",
  "configuration", "how-it-works", "research-background", "accuracy-transparency", "for-ai",
  "python-click",
];
const PUBLISHED_MEDIA = ["assets/video/deslop-demo.mp4", ...DOC_FILM_SLUGS.map(slug => `assets/video/docs/deslop-docs-${slug}.mp4`)];

test("the site publishes exactly the requested demo and documentation films", async () => {
  const files = await readdir(SITE_SOURCE, { recursive: true });
  expect(files.filter((file) => ENCODED_MEDIA.has(extname(file).toLowerCase())).sort()).toEqual([...PUBLISHED_MEDIA].sort());
});

// [SITE-DEMO-VIDEO] Native controls keep narration opt-in and captions accessible.
test("homepage demo plays on demand with captions on desktop and mobile", async ({ page }) => {
  for (const route of ["/", "/zh/"]) {
    for (const width of [1440, 390]) {
      await page.setViewportSize({ width, height: 900 });
      await page.goto(route);
      await page.locator('.hero__cta a[href="#demo"]').click();
      const video = page.locator('.hero video');
      await expect(video).toBeVisible();
      await expect(video).toHaveAttribute('controls', '');
      await expect(video).toHaveAttribute('preload', 'none');
      await expect(video).not.toHaveAttribute('autoplay');
      await expect(video.locator('track')).toHaveAttribute('srclang', 'en');
      const markup = await page.locator('.showcase__figure--demo script[type="application/ld+json"]').textContent();
      expect(JSON.parse(markup)).toMatchObject({
        '@type': 'VideoObject',
        contentUrl: 'https://deslop.live/assets/video/deslop-demo.mp4',
        thumbnailUrl: 'https://deslop.live/assets/img/deslop-demo-poster.webp',
        duration: 'PT34S',
        uploadDate: '2026-11-08',
        inLanguage: 'en',
      });
      await video.evaluate(element => { element.muted = true; return element.play(); });
      await expect.poll(() => video.evaluate(element => element.currentTime)).toBeGreaterThan(0);
      expect(await video.evaluate(element => element.videoWidth)).toBe(1600);
      expect(await video.evaluate(element => element.videoHeight)).toBe(1000);
      await video.evaluate(element => element.pause());
      expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    }
  }
});

test("English articles have distinct header artwork", async ({ page }) => {
  await page.goto('/blog/');
  const sources = await page.locator('main img').evaluateAll(images => images.map(image => image.getAttribute('src')).filter(src => src?.includes('/blog/')));
  expect(sources.length).toBeGreaterThan(5);
  expect(new Set(sources).size).toBe(sources.length);
  const contents = await Promise.all(sources.map(src => readFile(new URL(`../src${src}`, import.meta.url))));
  const unique = new Set(contents.map(bytes => bytes.toString('base64')));
  expect(unique.size, 'different filenames must not hide duplicate images').toBe(sources.length);
});

// [SITE-BLOG-ARTICLE] Search-result display limits and the minimum article
// length the site commits to for every English post.
const MIN_ENGLISH_POST_WORDS = 1000;
const MIN_TITLE_LENGTH = 30;
const MAX_TITLE_LENGTH = 60;
const MIN_DESCRIPTION_LENGTH = 120;
const MAX_DESCRIPTION_LENGTH = 160;
const MIN_IMAGE_ALT_LENGTH = 30;
const DEFAULT_SOCIAL_IMAGE_ALTS = [
  "Deslop duplicate-code analysis for editors and coding agents",
  "面向编辑器与编码智能体的 Deslop 重复代码分析",
];

test("every blog post publishes freshness, authorship, its own card alt and real substance", async ({ page }) => {
  const sitemap = await (await page.request.get("/sitemap.xml")).text();
  const posts = [...sitemap.matchAll(/<loc>https:\/\/deslop\.live(\/(?:zh\/)?blog\/[^/<]+\/)<\/loc>/g)].map(([, route]) => route);
  expect(posts.filter((route) => !route.startsWith("/zh/"))).toHaveLength(ENGLISH_BLOG_POST_COUNT);
  expect(posts.filter((route) => route.startsWith("/zh/"))).toHaveLength(ENGLISH_BLOG_POST_COUNT);

  for (const route of posts) {
    await page.goto(route);
    const article = await page.evaluate(() => {
      const meta = (property) => document.querySelector(`meta[property="${property}"]`)?.content;
      const posting = [...document.querySelectorAll('script[type="application/ld+json"]')]
        .flatMap((node) => JSON.parse(node.textContent)["@graph"] || [])
        .find((entity) => entity["@type"] === "BlogPosting");
      const bylineTimes = [...document.querySelectorAll(".post-article__meta time")].map((node) => node.dateTime);
      return {
        published: meta("article:published_time"),
        modified: meta("article:modified_time"),
        author: meta("article:author"),
        ogImageAlt: meta("og:image:alt"),
        twitterImageAlt: document.querySelector('meta[name="twitter:image:alt"]')?.content,
        posting,
        bylineTimes,
        bylineText: document.querySelector(".post-article__meta")?.textContent.replace(/\s+/g, " ").trim(),
        title: document.title,
        description: document.querySelector('meta[name="description"]')?.content,
        words: document.querySelector(".prose").innerText.split(/\s+/).filter(Boolean).length,
      };
    });

    expect(article.published, route).toBe(article.posting.datePublished);
    expect(article.modified, route).toBe(article.posting.dateModified);
    expect(article.author, route).toBe("Christian Findlay");
    expect(article.posting.author.name, route).toBe(article.author);
    expect(Date.parse(article.modified), route).toBeGreaterThanOrEqual(Date.parse(article.published));
    expect(DEFAULT_SOCIAL_IMAGE_ALTS, route).not.toContain(article.ogImageAlt);
    expect(article.twitterImageAlt, route).toBe(article.ogImageAlt);
    expect(article.bylineText, route).toContain("Christian Findlay");

    const isUpdated = article.modified.slice(0, 10) !== article.published.slice(0, 10);
    expect(article.bylineTimes, route).toEqual(
      isUpdated ? [article.published, article.modified] : [article.published]
    );
    if (isUpdated) expect(article.bylineText, route).toMatch(/Updated|更新于/);

    if (route.startsWith("/zh/")) continue;
    expect(article.words, route).toBeGreaterThanOrEqual(MIN_ENGLISH_POST_WORDS);
    expect(article.ogImageAlt.length, route).toBeGreaterThanOrEqual(MIN_IMAGE_ALT_LENGTH);
    expect(article.title.length, route).toBeGreaterThanOrEqual(MIN_TITLE_LENGTH);
    expect(article.title.length, route).toBeLessThanOrEqual(MAX_TITLE_LENGTH);
    expect(article.description.length, route).toBeGreaterThanOrEqual(MIN_DESCRIPTION_LENGTH);
    expect(article.description.length, route).toBeLessThanOrEqual(MAX_DESCRIPTION_LENGTH);
  }
});
