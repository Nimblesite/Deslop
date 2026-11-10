// [DOC-FILMS-LIVE] Real user workflows, following SharpLsp's walkthroughs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { createRequire } from 'node:module';
import { launchEditor, sleep, typeText } from '../../../Videos/factory/film/browser.mjs';
import { cueLog, record } from '../../../SharpLsp/src/website/tools/video/kit/camera.mjs';
import { runCommand as factoryRunCommand } from '../../../SharpLsp/src/website/tools/video/kit/editor.mjs';
import { typeAt } from '../../../SharpLsp/src/website/tools/video/kit/workbench.mjs';
import { pointAt, ringClick } from '../../../SharpLsp/src/website/tools/video/kit/overlay.mjs';
const root = path.resolve(import.meta.dirname, '../..');
const videos = path.resolve(root, '../Videos');
const work = path.join(root, 'target/docs-tutorial');
const clips = path.join(videos, 'projects/deslop/captures/docs-tutorial');
const saved = process.env.DESLOP_TUTORIAL_CDP ? JSON.parse(fs.readFileSync(path.join(work, 'session.json'), 'utf8')) : null;
const scratch = saved ? path.dirname(saved.workspace) : fs.mkdtempSync('/tmp/deslop-tutorial-');
const workspace = saved?.workspace ?? path.join(scratch, 'duplicate-review');
fs.mkdirSync(work, { recursive: true });
if (!saved) fs.cpSync(path.join(videos, 'projects/deslop/videos/live-demo/workspace'), workspace, { recursive: true });
fs.rmSync(path.join(workspace, '.mcp.json'), { force: true });
fs.mkdirSync(path.join(workspace, '.github/workflows'), { recursive: true });
for (const file of saved ? [] : ['.deslop.toml', '.github/workflows/deslop.yml']) fs.writeFileSync(path.join(workspace, file), '');
const originals = Object.fromEntries(['Eta.cs', 'Zeta.cs'].map(file => [file, fs.readFileSync(path.join(videos, 'projects/deslop/videos/live-demo/workspace', file), 'utf8')]));
for (const args of saved ? [] : [['init', '-q'], ['add', '.'], ['-c', 'user.name=Demo', '-c', 'user.email=demo@example.test', 'commit', '-qm', 'Start with the example project']]) execFileSync('git', args, { cwd: workspace });
const require = createRequire(path.join(videos, 'package.json'));
const connection = saved ? await require('playwright').chromium.connectOverCDP(process.env.DESLOP_TUTORIAL_CDP) : null;
const connectedPage = connection?.contexts().flatMap(context => context.pages())[0];
const editor = saved ? { page: connectedPage, cdp: await connectedPage.context().newCDPSession(connectedPage), root: saved.profile } : await launchEditor({ workspace, extensions: [path.join(root, 'clients/vscode/deslop-live-darwin-arm64.vsix')],
  width: 1600, height: 900, scale: 2, settings: { 'workbench.activityBar.location': 'default',
    'workbench.statusBar.visible': true, 'window.commandCenter': true, 'editor.fontSize': 20, 'editor.lineHeight': 28,
    'editor.autoIndent': 'none', 'editor.autoClosingBrackets': 'never', 'editor.autoClosingQuotes': 'never',
    'editor.formatOnType': false, 'editor.wordWrap': 'on', 'terminal.integrated.fontSize': 21 } });
const { page, cdp } = editor;
const extension = fs.readdirSync(path.join(editor.root, 'x')).find(name => name.startsWith('nimblesite.deslop-live-'));
const binaries = path.join(editor.root, 'x', extension, 'bin/darwin-arm64');
fs.writeFileSync(path.join(work, 'session.json'), JSON.stringify({ workspace, profile: editor.root, binaries }, null, 2));
const settingsFile = path.join(editor.root, 'u/User/settings.json');
const settings = JSON.parse(fs.readFileSync(settingsFile, 'utf8'));
settings['terminal.integrated.env.osx'] = { PATH: `${binaries}:${process.env.PATH}` };
fs.writeFileSync(settingsFile, JSON.stringify(settings, null, 2));
const cues = cueLog(clips);
const webview = page.frameLocator('.webview-overlay-content[style*="visibility: visible"] > iframe.webview').frameLocator('#active-frame');
const rows = () => webview.locator('.occurrence-row');
const tap = (locator, label) => ringClick(page, cues, locator, label);
const hold = (milliseconds = 2000) => sleep(milliseconds);
async function focusWorkbench() {
  const tabs = page.locator('.tabs-container .tab');
  if (await tabs.count()) await tabs.first().click();
  else await page.locator('.part.editor').click({ position: { x: 100, y: 100 } });
}
async function openFile(page, file) {
  await page.locator('.command-center .command-center-center').click();
  const input = page.locator('.quick-input-widget input');
  await input.waitFor({ state: 'visible' });
  await input.fill(file);
  await hold(500); await page.keyboard.press('Enter'); await hold(700);
}
async function runCommand(page, command) { await focusWorkbench(); await factoryRunCommand(page, command); }
async function take(name, action) {
  if (process.argv[2] && !name.startsWith(process.argv[2])) return;
  if (fs.existsSync(path.join(clips, name + '.mp4')) && !process.env.DESLOP_TUTORIAL_FORCE) return;
  process.stdout.write(`Recording ${name}\n`);
  try {
    await record({ page, cdp, cues, frames: path.join(work, 'frames'), clips }, name, action);
  } catch (error) {
    for (const suffix of ['.mp4', '.track.json', '.cues.json']) fs.rmSync(path.join(clips, name + suffix), { force: true });
    throw error;
  }
  process.stdout.write(`Recorded ${name}\n`);
}
async function point(x, y) { await pointAt(page, { x, y }); }
async function edit(file, text, typed = false) {
  await openFile(page, file);
  await page.locator('.editor-group-container.active .monaco-editor').first().click({ position: { x: 250, y: 75 } });
  await page.keyboard.press('Meta+A');
  await page.keyboard.press('Backspace');
  for (const [index, part] of text.split('\n').entries()) {
    if (index) {
      await page.keyboard.press('Escape');
      await page.keyboard.press('Enter');
      await page.keyboard.press('Home');
      await page.keyboard.press('Shift+End');
    }
    if (part) await typeText(page, part, 8);
    else await page.keyboard.press('Delete');
  }
  await page.keyboard.press('Meta+S');
  await point(760, 260);
  assert.equal(fs.readFileSync(path.join(workspace, file), 'utf8'), text, `Editor did not save the intended ${file}`);
}
async function finding() {
  await runCommand(page, 'Deslop: Open Worst Cluster');
  await rows().nth(1).waitFor({ timeout: 30000 });
}
async function compare() {
  await finding();
  await tap(rows().nth(1).getByRole('button', { name: 'Compare To Canonical' }), 'Compare the two copies');
  await page.locator('.monaco-diff-editor').first().waitFor();
  await point(960, 300); await hold(2400);
  await page.mouse.wheel(0, 150); await point(650, 430); await hold(2000);
}
async function sourceFiles() {
  await finding();
  await tap(rows().nth(0).locator('.text-action'), 'Open the first source file');
  await point(640, 270); await hold(2500);
  await finding();
  await tap(rows().nth(1).locator('.text-action'), 'Open the second source file');
  await point(820, 420); await hold(2500);
}
async function terminal(command, expected) {
  await runCommand(page, 'Terminal: Create New Terminal');
  await page.locator('.terminal-wrapper').last().click();
  await hold(400);
  await typeText(page, command, 25); await page.keyboard.press('Enter');
  await hold(2000); await point(760, 560);
  const deadline = Date.now() + 30000;
  while (Date.now() < deadline) {
    const text = execFileSync('tesseract', ['stdin', 'stdout', '--psm', '6'], { input: await page.locator('.terminal-wrapper').last().screenshot(), encoding: 'utf8', stdio: ['pipe', 'pipe', 'ignore'] });
    const normalized = value => value.toLowerCase().split(/\s+/).join('').replace(/exitstatus:[qo@]/g, 'exitstatus:0');
    if (normalized(text).includes(normalized(expected))) { await hold(3000); return; }
    await hold(500);
  }
  throw new Error(`The terminal did not visibly show ${expected}`);
}
async function closeTerminal() {
  if (await page.locator('.terminal-wrapper').count()) await runCommand(page, 'Terminal: Kill the Active Terminal Instance');
  if (await page.locator('.part.panel').isVisible()) await runCommand(page, 'View: Toggle Panel Visibility');
}
const workflow = version => `name: Deslop\non: [push, pull_request]\n\njobs:\n  repeated-code:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: Nimblesite/Deslop@${version}\n        with:\n          no-fail-over: "true"\n          upload-artifact: "false"\n`;
try {
  await focusWorkbench();
  while (await page.locator('.terminal-wrapper').count()) await closeTerminal();
  if (await page.locator('.part.panel').isVisible()) await runCommand(page, 'View: Toggle Panel Visibility');
  await runCommand(page, 'View: Close All Editors');
  await openFile(page, 'Eta.cs');
  await runCommand(page, 'Deslop: Focus on Top Offenders View');
  await runCommand(page, 'Notifications: Clear All Notifications');
  await take('action-create', async () => {
    await edit('.github/workflows/deslop.yml', workflow('v0.40.0'), true);
    await point(680, 190); await hold(2000); await point(850, 420); await hold(2500);
  });
  await take('action-version', async () => {
    await openFile(page, '.github/workflows/deslop.yml');
    await typeAt(page, { line: 9, text: '      - uses: Nimblesite/Deslop@v0.41.0' });
    await page.keyboard.press('Meta+S');
    await point(950, 345); await hold(3500); await point(740, 420); await hold(2500);
  });
  await take('action-limit', async () => {
    await openFile(page, '.github/workflows/deslop.yml');
    await typeAt(page, { line: 11, text: '          fail-over: "5.0"' });
    await page.keyboard.press('Meta+S');
    await hold(2200);
    await terminal('deslop . --fail-over 5; echo "exit status: $?"', 'exit status: 3');
  });
  await closeTerminal();
  await take('action-changes', async () => {
    await edit('.github/workflows/deslop.yml', workflow('v0.41.0').replace('no-fail-over: "true"', 'fail-over: "5.0"\n          diff: changes.patch\n          only-changed: "true"'));
    await terminal('cp Eta.cs Added.cs; git add -N Added.cs; git diff -- Added.cs > changes.patch\ndeslop . --diff changes.patch --only-changed --fail-over 5; echo "exit status: $?"', 'exit status: 3');
  });
  await closeTerminal();
  fs.rmSync(path.join(workspace, 'Added.cs'), { force: true });
  await take('start-extension', async () => {
    await runCommand(page, 'Extensions: Show Installed Extensions');
    const entry = page.locator('.monaco-list-row').filter({ hasText: 'Deslop' }).first();
    await entry.waitFor(); await tap(entry, 'Open the installed Deslop extension');
    await point(800, 230); await hold(3500);
    await openFile(page, 'Eta.cs'); await runCommand(page, 'Deslop: Focus on Top Offenders View'); await hold(2500);
  });
  await take('review-finding', async () => {
    await openFile(page, 'Eta.cs');
    const row = page.locator('.part.sidebar .monaco-list-row').filter({ hasText: 'Type II' }).first();
    await tap(row, 'Open the repeated-code finding'); await rows().nth(1).waitFor();
    await point(700, 470); await hold(2500);
    await tap(rows().nth(1).locator('.text-action'), 'Follow the file link'); await point(760, 270); await hold(2800);
  });
  await take('review-sources', sourceFiles);
  await take('review-compare', compare);
  await take('review-pair', async () => {
    await finding();
    await tap(rows().nth(1).getByRole('button', { name: 'Select for Compare' }), 'Select the first file');
    await tap(rows().nth(0).getByRole('button', { name: 'Compare with Selected' }), 'Compare with the selected file');
    await page.locator('.monaco-diff-editor').first().waitFor(); await point(980, 340); await hold(3200);
    await page.mouse.wheel(0, 130); await point(650, 460); await hold(2000);
  });
  await take('review-counts', async () => {
    await openFile(page, 'Eta.cs');
    const heading = page.locator('.part.sidebar').getByText('Duplication', { exact: true });
    await tap(heading, 'Collapse the file totals'); await tap(heading, 'Expand the file totals');
    await point(185, 380); await hold(3000);
    const file = page.locator('.part.sidebar .monaco-list-row').filter({ hasText: 'Zeta.cs' }).last();
    await tap(file, 'Open a file from the totals'); await point(650, 300); await hold(2500);
  });
  await take('review-refresh', async () => {
    await edit('Zeta.cs', originals['Zeta.cs'].replace('class Anchor', 'class SharedCounter'), true);
    await hold(2500); await finding(); await point(800, 490); await hold(2500);
  });
  await take('review-edit', async () => {
    await edit('Zeta.cs', originals['Zeta.cs'].replaceAll('bound', 'limit'), true);
    await hold(2300); await openFile(page, 'Eta.cs'); await point(750, 440); await hold(2500);
  });
  await take('review-edited-compare', compare);
  for (const [file, text] of Object.entries(originals)) fs.writeFileSync(path.join(workspace, file), text);
  await take('config-create', async () => {
    await edit('.deslop.toml', '[defaults]\nexclude = []\n\n[threshold]\nmax_duplication_percent = 5.0\n', true);
    await point(790, 250); await hold(3000); await point(170, 380); await hold(3000);
  });
  fs.mkdirSync(path.join(workspace, 'archive'), { recursive: true });
  fs.writeFileSync(path.join(workspace, 'archive/Imported.cs'), originals['Zeta.cs']);
  fs.mkdirSync(path.join(workspace, 'reference'), { recursive: true });
  fs.writeFileSync(path.join(workspace, 'reference/Reference.cs'), originals['Zeta.cs']);
  await hold(1500);
  await take('config-exclude', async () => {
    await edit('.deslop.toml', '[defaults]\nexclude = ["archive/**"]\nreport_hide = ["reference/**"]\n\n[threshold]\nmax_duplication_percent = 5.0\n', true);
    await hold(2000); await point(170, 380); await hold(3000); await point(790, 210); await hold(2200);
  });
  await take('config-override', () => terminal('deslop . --no-fail-over; echo "exit status: $?"', 'exit status: 0'));
  await closeTerminal();
  await take('config-limit', () => terminal('deslop . --fail-over 5; echo "exit status: $?"', 'exit status: 3'));
  await closeTerminal();
  for (const name of ['osprey-identical', 'osprey-python-marker', 'osprey-python-project']) {
    fs.cpSync(path.join(root, 'crates/deslop/tests/fixtures', name), path.join(workspace, name), { recursive: true });
  }
  await hold(3000);
  const ospreyFinding = async (file) => {
    await openFile(page, file);
    await runCommand(page, 'Deslop: Focus on Top Offenders View');
    const row = page.locator('.part.sidebar .monaco-list-row').filter({ hasText: path.basename(file) }).first();
    await row.waitFor({ timeout: 30000 });
    await tap(row, 'Open the Osprey finding');
    await rows().nth(1).waitFor();
  };
  await take('osprey-finding', async () => {
    await ospreyFinding('osprey-identical/alpha.osp');
    await tap(rows().nth(0).locator('.text-action'), 'Open the Osprey source');
    await point(750, 280); await hold(3200);
    await openFile(page, 'osprey-identical/beta.osp'); await point(740, 420); await hold(2800);
  });
  await take('osprey-compare', async () => {
    await ospreyFinding('osprey-identical/alpha.osp');
    await tap(rows().nth(1).getByRole('button', { name: 'Compare To Canonical' }), 'Compare the Osprey files');
    await page.locator('.monaco-diff-editor').first().waitFor();
    await point(700, 280); await hold(3000); await point(1080, 440); await hold(3000);
  });
  await take('osprey-python', async () => {
    await openFile(page, 'osprey-python-marker/alpha.py'); await point(750, 130); await hold(3500);
    await ospreyFinding('osprey-python-marker/alpha.py'); await point(750, 460); await hold(3000);
  });
  await take('osprey-config', async () => {
    await edit('osprey-python-project/osprey.toml', '[project]\nentry = "alpha.py"\npython_sources = ["alpha.py", "beta.py"]\n');
    await hold(2200);
    await terminal('deslop osprey-python-project --no-fail-over; echo "exit status: $?"', 'exit status: 0');
  });
  await closeTerminal();
  process.stdout.write('User workflows recorded\n');
} finally {
  await cdp.detach();
  fs.rmSync(path.join(work, 'frames'), { recursive: true, force: true });
}
process.exit(0);
