// [DOC-FILMS-LIVE] Real clicks and keystrokes in the isolated VS Code a Deslop film is shot in.
import fs from 'node:fs';
import { execFileSync } from 'node:child_process';
import path from 'node:path';
import { createRequire } from 'node:module';
import { launchEditor, sleep, typeText } from '../../../Videos/factory/film/browser.mjs';
import { record } from '../../../SharpLsp/src/website/tools/video/kit/camera.mjs';
import { runCommand as factoryRunCommand } from '../../../SharpLsp/src/website/tools/video/kit/editor.mjs';
import { pointAt, ringClick } from '../../../SharpLsp/src/website/tools/video/kit/overlay.mjs';

const VSIX = path.resolve(import.meta.dirname, '../../clients/vscode/deslop-live-darwin-arm64.vsix');
const OCCURRENCE_TIMEOUT_MS = 30000;
const TERMINAL_TIMEOUT_MS = 60000;
const MIN_TERMINAL_HEIGHT = 600;
/** The isolated editor every Deslop film is shot in: the locally packaged VSIX, large type, no typing assists. */
const launchFilmEditor = workspace => launchEditor({ workspace, extensions: [VSIX],
  width: 1600, height: 900, scale: 2, settings: { 'workbench.activityBar.location': 'default',
    'workbench.statusBar.visible': true, 'window.commandCenter': true, 'editor.fontSize': 20, 'editor.lineHeight': 28,
    'editor.autoIndent': 'none', 'editor.autoClosingBrackets': 'never', 'editor.autoClosingQuotes': 'never',
    'editor.formatOnType': false, 'editor.wordWrap': 'on', 'terminal.integrated.fontSize': 21 } });

/**
 * Resume the editor at `cdpUrl` when given, otherwise launch a fresh one on `workspace`. Records the
 * session under `work` so a later run can resume, and puts the bundled Deslop CLI on the terminal PATH.
 */
export async function filmEditor(work, workspace, cdpUrl) {
  const require = createRequire(path.resolve(import.meta.dirname, '../../../Videos/package.json'));
  const saved = cdpUrl ? JSON.parse(fs.readFileSync(path.join(work, 'session.json'), 'utf8')) : null;
  const connected = saved && (await require('playwright').chromium.connectOverCDP(cdpUrl)).contexts().flatMap(context => context.pages())[0];
  const editor = connected ? { page: connected, cdp: await connected.context().newCDPSession(connected), root: saved.profile } : await launchFilmEditor(workspace);
  const extension = fs.readdirSync(path.join(editor.root, 'x')).find(name => name.startsWith('nimblesite.deslop-live-'));
  const binaries = path.join(editor.root, 'x', extension, 'bin/darwin-arm64');
  fs.writeFileSync(path.join(work, 'session.json'), JSON.stringify({ workspace, profile: editor.root, binaries }, null, 2));
  const settingsFile = path.join(editor.root, 'u/User/settings.json');
  const settings = JSON.parse(fs.readFileSync(settingsFile, 'utf8'));
  settings['terminal.integrated.env.osx'] = { PATH: `${binaries}:${process.env.PATH}` };
  fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
  return editor;
}

export const hold = (milliseconds = 2000) => sleep(milliseconds);
export const point = (page, x, y) => pointAt(page, { x, y });
export const tap = (page, cues, locator, label) => ringClick(page, cues, locator, label);
const webview = page => page.frameLocator('.webview-overlay-content[style*="visibility: visible"] > iframe.webview').frameLocator('#active-frame');
export const occurrenceRows = page => webview(page).locator('.occurrence-row');

export async function focusWorkbench(page) {
  await page.locator('.command-center .command-center-center').click();
  await page.keyboard.press('Escape');
}

export async function openFile(page, file) {
  await page.locator('.command-center .command-center-center').click();
  const input = page.locator('.quick-input-widget input');
  await input.waitFor({ state: 'visible' });
  await input.fill(file);
  await hold(500); await page.keyboard.press('Enter'); await hold(700);
}

export async function runCommand(page, command) { await focusWorkbench(page); await factoryRunCommand(page, command); }

/** Open the cluster panel on `row` of the Top Offenders view, or on the worst cluster when no row is given. */
export async function openFinding(page, cues, row, label) {
  if (row) { await row.waitFor({ timeout: OCCURRENCE_TIMEOUT_MS }); await tap(page, cues, row, label); }
  else await runCommand(page, 'Deslop: Open Worst Cluster');
  await occurrenceRows(page).nth(1).waitFor({ timeout: OCCURRENCE_TIMEOUT_MS });
}

/** Records one take unless its clip already exists; a failed take leaves no partial clip behind. */
export async function take(shoot, name, action) {
  if (fs.existsSync(path.join(shoot.clips, name + '.mp4')) && !process.env.DESLOP_TUTORIAL_FORCE) return;
  process.stdout.write(`Recording ${name}\n`);
  try {
    await record(shoot, name, action);
  } catch (error) {
    for (const suffix of ['.mp4', '.track.json', '.cues.json']) fs.rmSync(path.join(shoot.clips, name + suffix), { force: true });
    throw error;
  }
  process.stdout.write(`Recorded ${name}\n`);
}

/** Opens a new terminal, maximised so its output is readable on film, and focuses it. */
export async function openTerminal(page) {
  await runCommand(page, 'Terminal: Create New Terminal');
  const panel = await page.locator('.part.panel').boundingBox();
  if (panel.height < MIN_TERMINAL_HEIGHT) await runCommand(page, 'View: Toggle Maximized Panel');
  await page.locator('.terminal-wrapper').last().click();
}

/** Types `command` into a maximised terminal and waits until OCR reads `expected` on screen. */
export async function terminal(page, command, expected) {
  await openTerminal(page);
  await hold(400);
  await point(page, 650, 350);
  await typeText(page, command, 25); await page.keyboard.press('Enter');
  await hold(2000); await point(page, 850, 550);
  const deadline = Date.now() + TERMINAL_TIMEOUT_MS;
  const normalized = value => value.toLowerCase().split(/\s+/).join('').replace(/exitstatus:[qo@]/g, 'exitstatus:0');
  while (Date.now() < deadline) {
    const text = execFileSync('tesseract', ['stdin', 'stdout', '--psm', '6'], { input: await page.screenshot(), encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] });
    if (normalized(text).includes(normalized(expected))) { await hold(3000); return; }
    await hold(500);
  }
  throw new Error(`The terminal did not visibly show ${expected}`);
}

/** Opens the diff of the second occurrence against the canonical one, from an open finding. */
export async function compareToCanonical(page, cues, label) {
  await tap(page, cues, occurrenceRows(page).nth(1).getByRole('button', { name: 'Compare To Canonical' }), label);
  await page.locator('.monaco-diff-editor').first().waitFor();
}

/** Releases the recorded editor's debugging session and the shoot's scratch frames. */
export async function finishShoot(cdp, work) {
  await cdp.detach();
  fs.rmSync(path.join(work, 'frames'), { recursive: true, force: true });
}

export async function closeTerminal(page) {
  if (await page.locator('.terminal-wrapper').count()) await runCommand(page, 'Terminal: Kill the Active Terminal Instance');
  if (await page.locator('.part.panel').isVisible()) await runCommand(page, 'View: Toggle Panel Visibility');
}
