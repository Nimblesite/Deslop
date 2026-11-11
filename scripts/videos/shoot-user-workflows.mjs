// [DOC-FILMS-LIVE] Real user workflows, following SharpLsp's walkthroughs.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { typeText } from '../../../Videos/factory/film/browser.mjs';
import { cueLog } from '../../../SharpLsp/src/website/tools/video/kit/camera.mjs';
import { typeAt } from '../../../SharpLsp/src/website/tools/video/kit/workbench.mjs';
import { closeTerminal as closeEditorTerminal, compareToCanonical, filmEditor, finishShoot, focusWorkbench as focusEditor, hold, occurrenceRows, openFile, openFinding, point as pointOn, runCommand, tap as tapOn, take as takeShot, terminal as runInTerminal } from './film-workbench.mjs';
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
const editor = await filmEditor(work, workspace, saved && process.env.DESLOP_TUTORIAL_CDP);
const { page, cdp } = editor;
const cues = cueLog(clips);
const rows = () => occurrenceRows(page);
const tap = (locator, label) => tapOn(page, cues, locator, label);
const focusWorkbench = () => focusEditor(page);
async function take(name, action) {
  const requested = process.argv[2];
  if (requested === 'editor' && name.startsWith('action-')) return;
  if (requested === 'final' && ['action-create', 'action-version'].includes(name)) return;
  const commands = ['action-limit', 'action-changes', 'config-override', 'config-limit', 'osprey-config'];
  if (requested === 'commands' && !commands.includes(name)) return;
  if (requested && requested !== 'commands' && requested !== 'editor' && requested !== 'final' && !name.startsWith(requested)) return;
  await takeShot({ page, cdp, cues, frames: path.join(work, 'frames'), clips }, name, action);
}
const point = (x, y) => pointOn(page, x, y);
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
const finding = () => openFinding(page, cues);
async function compare() {
  await finding();
  await compareToCanonical(page, cues, 'Compare the two copies');
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
const terminal = (command, expected) => runInTerminal(page, command, expected);
const closeTerminal = () => closeEditorTerminal(page);
const workflow = version => `name: Deslop\non: [push, pull_request]\n\njobs:\n  repeated-code:\n    runs-on: ubuntu-latest\n    steps:\n      - uses: actions/checkout@v4\n      - uses: Nimblesite/Deslop@${version}\n        with:\n          no-fail-over: "true"\n          upload-artifact: "false"\n`;
if (['action-limit', 'commands'].includes(process.argv[2])) fs.writeFileSync(path.join(workspace, '.github/workflows/deslop.yml'), workflow('v0.41.0'));
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
  await take('review-edited-compare', async () => {
    await runCommand(page, 'View: Close All Editors');
    await compare();
  });
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
    await compareToCanonical(page, cues, 'Compare the Osprey files');
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
  await finishShoot(cdp, work);
}
process.exit(0);
