// Generate the article artwork from the shared language icon; no downloads.
const fs = require("node:fs/promises");
const path = require("node:path");
const { chromium } = require("../../clients/vscode/node_modules/@playwright/test");
const { languageIconSvg } = require("../../clients/vscode/out/types/languageIcons.js");
const HEIGHT = 630;
const WIDTH = 1200;
const HERO_WIDTH = 1600;
const SMALL_WIDTH = 800;
const WIDTHS = [HERO_WIDTH, SMALL_WIDTH, WIDTH];
const ARTWORK = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="630" viewBox="0 0 1200 630"><rect width="1200" height="630" fill="#11151d"/><circle cx="1110" cy="40" r="400" fill="#252319"/><g transform="translate(76 68) scale(7)">${languageIconSvg("osprey")}</g><g font-family="Arial, sans-serif"><text x="80" y="300" fill="#f5c451" font-size="22" letter-spacing="5">DESLOP · LANGUAGE SPOTLIGHT</text><text x="76" y="388" fill="#f5f1e8" font-size="76" font-weight="700">Osprey gets</text><text x="76" y="472" fill="#f5c451" font-size="76" font-weight="700">the crown.</text><text x="80" y="554" fill="#c7cbd2" font-size="28">Native duplicate detection for .osp</text></g></svg>`;

async function encode(page, width) {
  return page.evaluate(async ({ svg, width, height }) => {
    const picture = new Image();
    picture.src = `data:image/svg+xml,${encodeURIComponent(svg)}`;
    await picture.decode();
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    canvas.getContext("2d").drawImage(picture, 0, 0, width, height);
    return canvas.toDataURL("image/webp", 0.94).split(",")[1];
  }, { svg: ARTWORK, width, height: HEIGHT * width / WIDTH });
}

async function main() {
  const browser = await chromium.launch();
  try {
    const page = await browser.newPage();
    for (const width of WIDTHS) {
      const suffix = width === WIDTH ? "-og" : width === SMALL_WIDTH ? "-800" : "";
      const file = path.join(__dirname, `../src/assets/img/blog/osprey-crown${suffix}.webp`);
      await fs.writeFile(file, Buffer.from(await encode(page, width), "base64"));
    }
  } finally { await browser.close(); }
}
main().catch((error) => { console.error(error); process.exitCode = 1; });
