import techdoc from "eleventy-plugin-techdoc";
import syntaxHighlight from "@11ty/eleventy-plugin-syntaxhighlight";
import { overrideVirtualTemplates } from "./lib/overrides/index.js";

// The MIME type each social-card extension actually serves. `og:image:type`
// used to be the literal string "image/png" for every page, so seven of the
// eight blog posts — whose cards are JPEGs — declared a type their own file
// does not have. Scrapers that trust the declaration over sniffing render a
// broken card. The map is the whole vocabulary of image formats this site is
// allowed to ship a social card in; an extension outside it is a build-time
// error rather than a guess, because a silently wrong MIME type is exactly
// the defect this replaced.
const SOCIAL_IMAGE_MIME_TYPES = {
  ".png": "image/png",
  ".jpg": "image/jpeg",
  ".jpeg": "image/jpeg",
  ".webp": "image/webp",
};

export default function (eleventyConfig) {
  eleventyConfig.addFilter("toImageMimeType", (imagePath) => {
    const extension = String(imagePath ?? "")
      .slice(String(imagePath ?? "").lastIndexOf("."))
      .toLowerCase();
    const mimeType = SOCIAL_IMAGE_MIME_TYPES[extension];
    if (!mimeType) {
      throw new Error(
        `og:image "${imagePath}" has extension "${extension}", which has no declared MIME type. ` +
          `Add it to SOCIAL_IMAGE_MIME_TYPES or ship the card in one of: ${Object.keys(SOCIAL_IMAGE_MIME_TYPES).join(", ")}.`
      );
    }
    return mimeType;
  });

  eleventyConfig.addFilter("jsonValue", (value) =>
    JSON.stringify(value ?? "")
      .replaceAll("<", "\\u003c")
      .replaceAll(">", "\\u003e")
      .replaceAll("&", "\\u0026")
  );

  eleventyConfig.addFilter("videoDuration", (time) => {
    const [minutes, seconds] = time.split(":").map(Number);
    return `PT${minutes * 60 + seconds}S`;
  });

  eleventyConfig.addPlugin(techdoc, {
    site: {
      name: "Deslop",
      url: "https://deslop.live",
      description:
        "Find duplicate code in nine languages. Deslop ranks what to remove first and tells your coding agent when similar code already exists — live in VS Code.",
      ogImage: "/assets/img/og-card.png",
      ogImageWidth: "1200",
      ogImageHeight: "630",
    },
    features: {
      blog: true,
      docs: true,
      darkMode: true,
      i18n: false,
    },
    // Ship English + Mandarin. Registering both here drives the plugin's
    // language-prefixed collections (zhposts, zhDocs) and the /zh/blog/ index,
    // tags, and categories pages, and emits the en + zh + x-default hreflang
    // cluster from the base layout. The /zh/ content tree lives in src/zh/.
    i18n: {
      defaultLanguage: "en",
      languages: ["en", "zh"],
    },
  });

  eleventyConfig.addPlugin(syntaxHighlight);
  eleventyConfig.addPlugin(overrideVirtualTemplates);
  eleventyConfig.addPassthroughCopy("src/assets");
  eleventyConfig.addPassthroughCopy({
    "../docs/designs/logo.png": "assets/img/logo.png",
    "../docs/designs/logo.svg": "assets/img/logo.svg",
  });

  return {
    dir: { input: "src", output: "_site" },
    markdownTemplateEngine: "njk",
  };
}
