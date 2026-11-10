// [DOC-FILMS-LIVE] Record the packaged extension in an isolated VS Code.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { launchEditor, openFile, runCommand, sleep, typeText } from '../../../Videos/factory/film/browser.mjs';
import { cueLog, record } from '../../../SharpLsp/src/website/tools/video/kit/camera.mjs';
import { pointAt, ringClick } from '../../../SharpLsp/src/website/tools/video/kit/overlay.mjs';

const root = path.resolve(import.meta.dirname, '../..');
const out = path.join(root, 'target/docs-live');
const clips = path.resolve(root, '../Videos/projects/deslop/captures/docs-live');
const frames = path.join(out, 'frames-' + (process.env.DESLOP_FILM_CONTEXT ?? 'review'));
const fixture = path.resolve(root, '../Videos/projects/deslop/videos/live-demo/workspace');
const scratch = fs.mkdtempSync('/tmp/deslop-film-');
const workspace = path.join(scratch, 'duplicate-review');
fs.cpSync(fixture, workspace, { recursive: true });
const fixtures = path.join(root, 'crates/deslop/tests/fixtures');
const mode = process.env.DESLOP_FILM_CONTEXT ?? 'all';
const ospreyFixtures = ['osprey-identical', 'osprey-python-marker', 'osprey-python-project'];
for (const name of ['commands', 'files'].includes(mode) ? ospreyFixtures : []) {
  fs.cpSync(path.join(fixtures, name), path.join(workspace, name), { recursive: true });
}
fs.copyFileSync(path.join(root, 'action.yml'), path.join(workspace, 'action.yml'));
fs.writeFileSync(path.join(workspace, '.deslop.toml'),
  '[defaults]\nexclude = ["vendor/**"]\nreport_hide = ["generated/**"]\n\n[threshold]\nmax_duplication_percent = 5.0\n');
if (['all', 'mcp-config'].includes(mode)) {
  const install = '/home/demo/.vscode/extensions/nimblesite.deslop-live-<version>-linux-x64/bin/linux-x64/deslop-mcp';
  fs.writeFileSync(path.join(workspace, 'mcp-client.example.json'), JSON.stringify({
    mcpServers: { deslop: { command: install, args: ['--root', '/work/duplicate-review'] } },
  }, null, 2));
}
fs.mkdirSync(clips, { recursive: true });
const vsix = path.resolve(process.argv[2] ?? 'clients/vscode/deslop-live-darwin-arm64.vsix');
const { page, cdp, root: profile } = await launchEditor({
  workspace, extensions: [vsix], scale: 2,
  settings: { 'workbench.activityBar.location': 'default', 'workbench.statusBar.visible': true,
    'editor.codeLens': true, 'editor.fontSize': 20, 'editor.lineHeight': 28, 'editor.wordWrap': 'on' },
});
const cues = cueLog(clips);
async function take(name, act) {
  try {
    await record({ page, cdp, cues, frames, clips }, name, act);
  } catch (error) {
    for (const suffix of ['.mp4', '.track.json', '.cues.json']) {
      fs.rmSync(path.join(clips, name + suffix), { force: true });
    }
    throw error;
  }
}
const tap = (locator, label) => ringClick(page, cues, locator, label);
const diff = page.locator('.monaco-diff-editor').first();
const webview = page.frameLocator('iframe.webview').frameLocator('#active-frame');
const openCluster = () => runCommand(page, 'Deslop: Open Worst Cluster');

async function show(locator, label) {
  await locator.waitFor({ state: 'visible' });
  await tap(locator, label);
}

async function recordFiles(name, paths) {
  for (const file of paths) await openFile(page, file);
  await openFile(page, 'Eta.cs');
  await take(name, async () => {
    await pointAt(page, { x: 410, y: 180 });
    for (const file of paths) {
      const tab = page.getByRole('tab').filter({ hasText: path.basename(file) }).last();
      await show(tab, file);
      await pointAt(page, { x: 565, y: 195 });
      await sleep(3200);
    }
  });
}

async function recordContext() {
  for (const name of ospreyFixtures) {
    fs.cpSync(path.join(fixtures, name), path.join(workspace, name), { recursive: true });
  }
  await recordFiles('policy', ['.deslop.toml']);
  await recordFiles('action', ['action.yml']);
  await recordFiles('osprey-native', ['osprey-identical/alpha.osp']);
  await recordFiles('osprey-python', ['osprey-python-marker/beta.py']);
  await recordFiles('osprey-manifest', ['osprey-python-project/osprey.toml']);
}

async function recordNumbers() {
  await openFile(page, 'Eta.cs');
  const sidebar = page.locator('.part.sidebar');
  const heading = sidebar.getByText('Duplication', { exact: true });
  await take('numbers', async () => {
    await pointAt(page, { x: 580, y: 190 });
    await sleep(900);
    await show(heading, 'Collapse Duplication');
    await show(heading, 'Expand Duplication');
    await pointAt(page, { x: 155, y: 390 });
    await sleep(4200);
  });
}

async function recordTerminal(name, command) {
  const tab = page.locator('.terminal-wrapper').last();
  await runCommand(page, 'Terminal: Create New Terminal');
  await tab.waitFor();
  await page.keyboard.insertText(`export PATH="${path.join(root, 'target/release')}:$PATH"; clear`);
  await page.keyboard.press('Enter');
  await sleep(1000);
  await take(name, async () => {
    await pointAt(page, { x: 555, y: 380 });
    await show(tab, 'Focus the integrated terminal');
    await typeText(page, command, 43);
    await page.keyboard.press('Enter');
    await sleep(6500);
  });
  await runCommand(page, 'Terminal: Kill the Active Terminal Instance');
}

async function recordCommands() {
  await recordNumbers();
  await recordTerminal('gate', 'deslop . --fail-over 5 --nohtml --nojson; echo "exit status: $?"');
  await recordTerminal('osprey-scan', 'deslop osprey-identical --nohtml --nojson');
}

async function recordMcp() {
  fs.copyFileSync(path.join(import.meta.dirname, 'doc-film-mcp.mjs'), path.join(workspace, 'query.mjs'));
  await recordTerminal('mcp-status', 'node query.mjs duplicates');
  await recordTerminal('mcp-query', 'node query.mjs find-similar');
}

async function recordFinding(rows) {
  await openFile(page, 'Eta.cs');
  await take('find', async () => {
    await pointAt(page, { x: 610, y: 160 });
    const finding = page.locator('.part.sidebar .monaco-list-row').filter({ hasText: 'Type II' }).first();
    await show(finding, 'Open a finding from Top Offenders');
    await rows.nth(1).waitFor();
    await pointAt(page, { x: 500, y: 335 });
    await sleep(4800);
  });
}

async function recordReview() {
  await openCluster();
  const rows = webview.locator('.occurrence-row');
  await rows.nth(1).waitFor({ timeout: 30000 });
  const occurrence = rows.nth(1).locator('.text-action');
  if (mode === 'finding') return recordFinding(rows);
  await take('occurrence', async () => {
    await pointAt(page, { x: 660, y: 145 });
    await sleep(800);
    await show(occurrence, 'Open occurrence');
    await page.locator('.tabs-container .tab.active').filter({ hasText: 'Zeta.cs' }).first().waitFor();
    await pointAt(page, { x: 670, y: 285 });
    await sleep(3700);
  });
  await openCluster();
  await take('canonical', async () => {
    await pointAt(page, { x: 680, y: 140 });
    await sleep(800);
    await show(rows.nth(1).getByRole('button', { name: 'Compare To Canonical' }), 'Compare To Canonical');
    await diff.waitFor();
    await pointAt(page, { x: 820, y: 300 });
    await sleep(4100);
  });
  await openCluster();
  await take('pair', async () => {
    await pointAt(page, { x: 680, y: 140 });
    await sleep(800);
    await show(rows.nth(1).getByRole('button', { name: 'Select for Compare' }), 'Select for Compare');
    await rows.nth(1).getByRole('button', { name: 'Clear Selection' }).waitFor();
    await show(rows.nth(0).getByRole('button', { name: 'Compare with Selected' }), 'Compare with Selected');
    await diff.waitFor();
    await pointAt(page, { x: 810, y: 300 });
    await sleep(4100);
  });
  assert.ok(await diff.isVisible());
}

try {
  await runCommand(page, 'Notifications: Clear All Notifications');
  await openFile(page, 'Eta.cs');
  await runCommand(page, 'Deslop: Focus on Top Offenders View');
  if (mode === 'all') {
    await recordReview();
    await recordFinding(webview.locator('.occurrence-row'));
    await recordContext();
    await recordCommands();
    await recordMcp();
    await recordFiles('mcp-config', ['mcp-client.example.json']);
  } else if (mode === 'commands') await recordCommands();
  else if (mode === 'mcp-config') await recordFiles('mcp-config', ['mcp-client.example.json']);
  else if (mode === 'mcp') await recordMcp();
  else if (mode === 'files') await recordContext();
  else await recordReview();
} finally {
  // Leave VS Code running: repository policy forbids killing any editor process.
  await cdp.detach();
  process.stdout.write(`Recording profile: ${profile}\nRecording workspace: ${scratch}\n`);
  fs.rmSync(frames, { recursive: true, force: true });
}
process.exit(0);
