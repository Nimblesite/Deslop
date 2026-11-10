import {
  BASE_LAYOUT_OVERRIDE,
  BLOG_POST_LAYOUT_OVERRIDE,
  DOCS_LAYOUT_OVERRIDE,
  blogIndexOverride,
} from "./layouts.js";
import {
  DISABLED_ARCHIVE_TEMPLATE,
  FEED_OVERRIDE,
  LLMS_TXT_OVERRIDE,
  ROBOTS_TXT_OVERRIDE,
  SITEMAP_OVERRIDE,
} from "./feeds.js";

const OVERRIDES = {
  "blog/index.njk": blogIndexOverride("en"),
  "zh/blog/index.njk": blogIndexOverride("zh"),
  "blog/tags.njk": DISABLED_ARCHIVE_TEMPLATE,
  "blog/tags-pages.njk": DISABLED_ARCHIVE_TEMPLATE,
  "blog/categories.njk": DISABLED_ARCHIVE_TEMPLATE,
  "blog/categories-pages.njk": DISABLED_ARCHIVE_TEMPLATE,
  "zh/blog/tags.njk": DISABLED_ARCHIVE_TEMPLATE,
  "zh/blog/tags-pages.njk": DISABLED_ARCHIVE_TEMPLATE,
  "zh/blog/categories.njk": DISABLED_ARCHIVE_TEMPLATE,
  "zh/blog/categories-pages.njk": DISABLED_ARCHIVE_TEMPLATE,
  "_includes/layouts/base.njk": BASE_LAYOUT_OVERRIDE,
  "_includes/layouts/docs.njk": DOCS_LAYOUT_OVERRIDE,
  "_includes/layouts/blog.njk": BLOG_POST_LAYOUT_OVERRIDE,
  "feed.njk": FEED_OVERRIDE,
  "sitemap.njk": SITEMAP_OVERRIDE,
  "llms.txt.njk": LLMS_TXT_OVERRIDE,
  "robots.txt.njk": ROBOTS_TXT_OVERRIDE,
};

/**
 * Replaces plugin-registered virtual templates with project-specific ones.
 * Runs as its own plugin so `virtualTemplates` is populated by the time it
 * executes — plugin registration is queued, not immediate.
 */
export function overrideVirtualTemplates(eleventyConfig) {
  const vt = eleventyConfig.virtualTemplates;
  if (!vt) return;
  for (const [key, content] of Object.entries(OVERRIDES)) {
    if (vt[key]) vt[key].content = content;
  }
}
