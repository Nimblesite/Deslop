// [DOC-FILMS-LIVE] A real pi session, filmed through SharpLsp's camera.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { createRequire } from 'node:module';
import { sleep, typeText } from '../../../Videos/factory/film/browser.mjs';
import { runCommand } from '../../../SharpLsp/src/website/tools/video/kit/editor.mjs';
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
const clips = path.resolve(root, '../Videos/projects/deslop/captures/docs-tutorial');
const frames = path.join(out, 'agent-frames');
const sessions = path.join(out, 'pi-workflow-session');
fs.mkdirSync(sessions, { recursive: true });
const cues = cueLog(clips);
const model = 'zai/glm-5.3-flash';
const prompts = [
  ['agent-duplicates', 'Use Deslop duplicates to show one finding in this project. Limit 1. Show the file path and number of copies.', 'duplicates'],
  ['agent-details', 'Call cluster-by-id for that finding. Show every file path and source line range, and explain what I should review.', 'cluster_by_id'],
  ['agent-ask', 'Before adding another helper, read Eta.cs and use Deslop find-similar for this complete file. Limit 2. Show the matching files. Do not write code.', 'find_similar'],
  ['agent-search', 'Call Deslop find-similar again using the complete Eta.cs file. Then tell me which existing files I should inspect before writing another helper.', 'find_similar'],
  ['agent-results', 'In plain English, list the two matching files and the source lines I should inspect. Explain why I should review them before creating another copy.', null],
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

async function waitForAnswer(before) {
  const deadline = Date.now() + 60000;
  while (Date.now() < deadline) {
    const answer = responses().slice(before).find(row => row.message?.role === 'assistant' && row.message.stopReason === 'stop');
    if (answer) return answer;
    await sleep(500);
  }
  throw new Error('pi must finish its answer');
}

async function askAgent(prompt, tool, before) {
  await ringClick(page, cues, page.locator('.terminal-wrapper').last(), 'Ask pi to consult Deslop MCP');
  await pointAt(page, { x: 600, y: 440 });
  await typeText(page, prompt + ' Reply briefly in plain English: file names, line ranges, and what to review. Do not mention internal statistics or assume a refactor is safe.', 12);
  await page.keyboard.press('Enter');
  await pointAt(page, { x: 850, y: 210 });
  if (tool) {
    const result = await waitForResult(`mcp__deslop__${tool}`, before);
    const data = JSON.parse(result.message.content[0].text);
    assert.ok(!result.message.isError && Object.keys(data).length, 'The tool must return real results');
  }
  await waitForAnswer(before);
  await pointAt(page, { x: 850, y: 360 });
  await sleep(5000);
  await pointAt(page, { x: 700, y: 510 });
  await sleep(1800);
}

async function take(name, prompt, tool) {
  process.stdout.write(`Recording ${name}\n`);
  const before = responses().length;
  await record({ page, cdp, cues, frames, clips }, name, () => askAgent(prompt, tool, before));
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
  for (const [name, prompt, tool] of prompts.filter(([name]) => !process.argv[2] || name === process.argv[2])) await take(name, prompt, tool);
  await page.keyboard.press('Control+C');
  await page.keyboard.press('Control+C');
} finally {
  await cdp.detach();
  await browser.close();
  fs.rmSync(frames, { recursive: true, force: true });
}
