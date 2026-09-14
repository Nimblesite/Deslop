import { expect, test } from "@playwright/test";

// Everything the homepage claims about Kevin Moore's public post, written out
// here independently of _data/testimonials.js. Quotes are verbatim; the LOC
// figures are the ones he stated beside each pull request. If the page ever
// paraphrases a quote, drops a pull request, or edits a number, these fail.
const AUTHOR = "Kevin Moore";
const AUTHOR_URL = "https://kevmoo.com/";
const AUTHOR_HANDLE = "@kevmoo";
const AUTHOR_ROLE = {
  en: "Product Manager for Dart & Flutter, Google",
  zh: "Google Dart 与 Flutter 产品经理",
};
const QUOTE = "Holy cow dude! My initial analysis of this tool, I am blown away and super excited.";
const QUOTE_SOURCE_URL = "https://x.com/kevmoo/status/2084838485262008358";
const QUOTE_SOURCE_DATE = "2026-08-05";
const REACTION = "AMAZING! SUPER useful! wow!";
const REACTION_SOURCE_URL = "https://x.com/kevmoo/status/2084863688184578520";

const MINUS = "−";
const LOC_LABEL = { en: "LOC", zh: "行" };
const MERGED_PULL_REQUESTS = [
  { repo: "peanut.dart", url: "https://github.com/kevmoo/peanut.dart/pull/220", linesRemoved: 16 },
  { repo: "mysql.dart", url: "https://github.com/kevmoo/mysql.dart/pull/29", linesRemoved: 22 },
  { repo: "pubviz", url: "https://github.com/kevmoo/pubviz/pull/179", linesRemoved: 55 },
  { repo: "scripts.dart", url: "https://github.com/kevmoo/scripts.dart/pull/54", linesRemoved: 20 },
  { repo: "dtt", url: "https://github.com/kevmoo/dtt/pull/25", linesRemoved: 13 },
];

const HOME_ROUTE = { en: "/", zh: "/zh/" };
const SECTION_ORDER = ["hero", "showcase", "testimonials", "editors", "community"];
const SECTION_HEADING = { en: "Findings that ship.", zh: "可以直接合并的发现。" };
const SECTION_EYEBROW = { en: "Field report", zh: "实地反馈" };

const chipText = (pullRequest, lang) =>
  `${pullRequest.repo} ${MINUS}${pullRequest.linesRemoved} ${LOC_LABEL[lang]}`;

for (const lang of ["en", "zh"]) {
  test(`quotes ${AUTHOR} verbatim and links the post it came from (${lang})`, async ({ page }) => {
    await page.goto(HOME_ROUTE[lang]);
    const testimonial = page.locator(".testimonial").first();

    const quote = testimonial.locator(".testimonial__quote");
    await expect(quote, "the quote must be reproduced word for word, untranslated").toHaveText(QUOTE);
    await expect(quote).toHaveAttribute("cite", QUOTE_SOURCE_URL);

    await expect(testimonial.locator(".testimonial__author")).toHaveText(AUTHOR);
    await expect(testimonial.locator(".testimonial__author")).toHaveAttribute("href", AUTHOR_URL);
    await expect(
      testimonial.locator(".testimonial__role"),
      "the role is the only part of the attribution that is translated",
    ).toHaveText(AUTHOR_ROLE[lang]);

    const source = testimonial.locator(".testimonial__source");
    await expect(source).toHaveAttribute("href", QUOTE_SOURCE_URL);
    await expect(source).toContainText(AUTHOR_HANDLE);
    await expect(source, "a reader must be able to date the quote").toContainText(QUOTE_SOURCE_DATE);

    const reaction = testimonial.locator(".testimonial__reaction");
    await expect(reaction).toHaveText(REACTION);
    await expect(reaction).toHaveAttribute("href", REACTION_SOURCE_URL);
  });

  test(`lists every merged pull request with the lines it removed (${lang})`, async ({ page }) => {
    await page.goto(HOME_ROUTE[lang]);
    const chips = page.locator(".testimonial .pr-chip");

    await expect(chips).toHaveCount(MERGED_PULL_REQUESTS.length);
    await expect(
      chips,
      "each chip names the package and the lines that pull request removed",
    ).toHaveText(MERGED_PULL_REQUESTS.map((pullRequest) => chipText(pullRequest, lang)));

    for (const [index, pullRequest] of MERGED_PULL_REQUESTS.entries()) {
      await expect(
        chips.nth(index),
        `${pullRequest.repo} must link to the pull request the figure came from`,
      ).toHaveAttribute("href", pullRequest.url);
    }
  });

  test(`sits between the product showcase and the install band (${lang})`, async ({ page }) => {
    await page.goto(HOME_ROUTE[lang]);

    const sections = await page
      .locator(".home > section, .home > div > section")
      .evaluateAll((elements) => elements.map((element) => element.className.split(" ")[0]));
    expect(sections, "social proof belongs after the demo and before the install CTA").toEqual(
      SECTION_ORDER,
    );

    await expect(page.locator("#testimonials-title")).toHaveText(SECTION_HEADING[lang]);
    await expect(page.locator(".testimonials .eyebrow-chip")).toHaveText(SECTION_EYEBROW[lang]);
    await expect(
      page.locator(".testimonials"),
      "the section is labelled by its own heading for screen readers",
    ).toHaveAttribute("aria-labelledby", "testimonials-title");
  });

  test(`opens every cited source safely in a new tab (${lang})`, async ({ page }) => {
    await page.goto(HOME_ROUTE[lang]);
    const links = page.locator(".testimonials a");

    const count = await links.count();
    expect(count, "the band cites the author, both posts, and every pull request").toBe(
      MERGED_PULL_REQUESTS.length + 3,
    );
    for (let index = 0; index < count; index += 1) {
      const link = links.nth(index);
      const href = await link.getAttribute("href");
      expect(href, "every citation points off-site").toMatch(/^https:\/\//);
      await expect(link, href).toHaveAttribute("target", "_blank");
      await expect(link, href).toHaveAttribute("rel", "noopener noreferrer");
    }
  });
}
