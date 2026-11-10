// [DOC-FILMS-LIVE] A real pi session, filmed through SharpLsp's camera.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { runCommand, sleep, typeText } from '../../../Videos/factory/film/browser.mjs';
import { cueLog, record } from '../../../SharpLsp/src/website/tools/video/kit/camera.mjs';
import { pointAt, ringClick } from '../../../SharpLsp/src/website/tools/video/kit/overlay.mjs';

const root = path.resolve(import.meta.dirname, '../..');
const require = createRequire(path.resolve(root, '../Videos/package.json'));
const { chromium } = require('playwright');
const workspace = process.env.DESLOP_FILM_WORKSPACE;
assert.ok(workspace && process.env.DESLOP_FILM_CDP, 'Supply the recorded editor and workspace');
const browser = await chromium.connectOverCDP(process.env.DESLOP_FILM_CDP);
const page = browser.contexts().flatMap(context => context.pages())[0];
const cdp = await page.context().newCDPSession(page);
const out = path.join(root, 'target/docs-live');
const clips = path.resolve(root, '../Videos/projects/deslop/captures/docs-live');
const frames = path.join(out, 'agent-frames');
const sessions = path.join(out, 'pi-session');
fs.mkdirSync(sessions, { recursive: true });
const cues = cueLog(clips);
const model = 'zai/glm-5.3-flash';
const prompts = [
  ['pi-duplicates', 'Use Deslop MCP duplicates with limit 1 and detail summary. Show the top finding and occurrence count. Do not write code.'],
  ['pi-find-similar', 'Before writing another helper, read Eta.cs and call Deslop find-similar with that complete file, language csharp, limit 2. Show the matching source paths.'],
];

function responses() {
  return fs.readdirSync(sessions).filter(file => file.endsWith('.jsonl'))
    .flatMap(file => fs.readFileSync(path.join(sessions, file), 'utf8').split('\n'))
    .filter(Boolean).map(line => JSON.parse(line));
}

async function waitForResult(tool, before) {
  const deadline = Date.now() + 90000;
  while (Date.now() < deadline) {
    const messages = responses().slice(before);
    const result = messages.find(row => row.message?.role === 'toolResult'
      && row.message.toolName === tool && !row.message.isError);
    if (result) return result;
    await sleep(500);
  }
  throw new Error(`pi did not return a successful ${tool} result`);
}

async function take(name, prompt) {
  process.stdout.write(`Recording ${name}\n`);
  const before = responses().length;
  const tool = name === 'pi-duplicates' ? 'duplicates' : 'find-similar';
  await record({ page, cdp, cues, frames, clips }, name, async () => {
    await ringClick(page, cues, page.locator('.terminal-wrapper').last(), 'Ask pi to consult Deslop MCP');
    await pointAt(page, { x: 600, y: 440 });
    await typeText(page, prompt, 18);
    await page.keyboard.press('Enter');
    await pointAt(page, { x: 850, y: 210 });
    const result = await waitForResult(`mcp__deslop__${tool.replaceAll('-', '_')}`, before);
    const data = JSON.parse(result.message.content[0].text);
    assert.ok(data.clusters.length > 0, 'The demonstration must show real matches');
    await pointAt(page, { x: 850, y: 360 });
    await sleep(8500);
  });
}

try {
  await runCommand(page, 'Terminal: Create New Terminal');
  const panel = await page.locator('.part.panel').boundingBox();
  if (panel.height < 600) await runCommand(page, 'View: Toggle Maximized Panel');
  await page.locator('.terminal-wrapper').last().click();
  const command = `pi -a --model ${model} --thinking low --session-dir '${sessions}' --no-skills --no-context-files --no-extensions -e builtin:mcp --tools 'read,mcp__deslop__*'`;
  await typeText(page, command, 1);
  await page.keyboard.press('Enter');
  await sleep(9000);
  for (const [name, prompt] of prompts.filter(([name]) => !process.argv[2] || name === process.argv[2])) await take(name, prompt);
  await page.keyboard.press('Control+C');
  await page.keyboard.press('Control+C');
} finally {
  await cdp.detach();
  await browser.close();
  fs.rmSync(frames, { recursive: true, force: true });
}
