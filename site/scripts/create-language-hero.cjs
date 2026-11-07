// Generate a language-spotlight article's artwork from the shared language
// icon; no downloads. Usage:
//   node create-language-hero.cjs <language-id> <file-stem> <line-1> <line-2> <subtitle>
// e.g. node create-language-hero.cjs osprey osprey-crown "Osprey gets" "the crown." "Native duplicate detection for .osp"
const fs = require("node:fs/promises");
const path = require("node:path");
const {
  chromium,
} = require("../../clients/vscode/node_modules/@playwright/test");
const {
  languageIconSvg,
} = require("../../clients/vscode/out/types/languageIcons.js");
const HEIGHT = 630;
const WIDTH = 1200;
const HERO_WIDTH = 1600;
const SMALL_WIDTH = 800;
const WIDTHS = [HERO_WIDTH, SMALL_WIDTH, WIDTH];
const ARGUMENT_COUNT = 5;

function escapeXml(text) {
  return text.replace(
    /[<>&"]/g,
    (char) => ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", '"': "&quot;" })[char],
  );
}

function artwork(language, lineOne, lineTwo, subtitle) {
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630"><rect width="1200" height="630" fill="#11151d"/><circle cx="1110" cy="40" r="400" fill="#252319"/><g transform="translate(76 68) scale(7)">${languageIconSvg(language)}</g><g font-family="Arial, sans-serif"><text x="80" y="300" fill="#f5c451" font-size="22" letter-spacing="5">DESLOP · LANGUAGE SPOTLIGHT</text><text x="76" y="388" fill="#f5f1e8" font-size="76" font-weight="700">${escapeXml(lineOne)}</text><text x="76" y="472" fill="#f5c451" font-size="76" font-weight="700">${escapeXml(lineTwo)}</text><text x="80" y="554" fill="#c7cbd2" font-size="28">${escapeXml(subtitle)}</text></g></svg>`;
}

// The social card is a JPEG (`<stem>-og.jpg`); the hero and its 800px
// variant are WebP.
async function encode(page, svg, width, mime) {
  return page.evaluate(
    async ({ svg, width, height, mime }) => {
      const picture = new Image();
      picture.src = `data:image/svg+xml,${encodeURIComponent(svg)}`;
      await picture.decode();
      const canvas = document.createElement("canvas");
      canvas.width = width;
      canvas.height = height;
      canvas.getContext("2d").drawImage(picture, 0, 0, width, height);
      return canvas.toDataURL(mime, 0.94).split(",")[1];
    },
    { svg, width, height: (HEIGHT * width) / WIDTH, mime },
  );
}

async function main() {
  const [language, stem, lineOne, lineTwo, subtitle] = process.argv.slice(2);
  if (process.argv.length - 2 !== ARGUMENT_COUNT) {
    throw new Error(
      "usage: create-language-hero.cjs <language-id> <file-stem> <line-1> <line-2> <subtitle>",
    );
  }
  const svg = artwork(language, lineOne, lineTwo, subtitle);
  // `PLAYWRIGHT_CHANNEL=chrome` reuses an installed Google Chrome when the
  // bundled Chromium is missing.
  const browser = await chromium.launch({
    channel: process.env.PLAYWRIGHT_CHANNEL,
  });
  try {
    const page = await browser.newPage();
    for (const width of WIDTHS) {
      const social = width === WIDTH;
      const suffix = social
        ? "-og.jpg"
        : width === SMALL_WIDTH
          ? "-800.webp"
          : ".webp";
      const mime = social ? "image/jpeg" : "image/webp";
      const file = path.join(
        __dirname,
        `../src/assets/img/blog/${stem}${suffix}`,
      );
      await fs.writeFile(
        file,
        Buffer.from(await encode(page, svg, width, mime), "base64"),
      );
    }
  } finally {
    await browser.close();
  }
}
main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
