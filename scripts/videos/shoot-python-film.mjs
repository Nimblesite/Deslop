// [DOC-FILMS-PYTHON] Deslop on pallets/click, the registered corpus commit: real Python clones, removed live.
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { execFileSync } from 'node:child_process';
import { cueLog } from '../../../SharpLsp/src/website/tools/video/kit/camera.mjs';
import { goTo } from '../../../SharpLsp/src/website/tools/video/kit/workbench.mjs';
import { closeTerminal, compareToCanonical, filmEditor, finishShoot, hold, occurrenceRows, openFile, openFinding, point, runCommand, tap, take, terminal } from './film-workbench.mjs';
const root = path.resolve(import.meta.dirname, '../..');
const work = path.join(root, 'target/python-film');
const clips = path.resolve(root, '../Videos/projects/deslop/captures/docs-tutorial');
const corpus = JSON.parse(fs.readFileSync(path.join(root, 'corpus/register/click.json'), 'utf8'));
const COMPLETION = 'src/click/shell_completion.py';
const BASH_METHOD_LINE = 432;
// Bottom copy first, so removing it leaves the upper copy's line number intact.
const COPIES = [{ shell: 'PowerShell', line: 530 }, { shell: 'Zsh', line: 454 }];
const METHOD_LINES = 11;
const ALIAS = '    get_completion_args = BashComplete.get_completion_args';
const TESTS = 'uv run -q --with pytest --with-editable . pytest -q tests/test_shell_completion.py';
const TESTS_PASSED = '58 passed';
const LIBRARY_ONLY = '[defaults]\nexclude = ["tests/**", "docs/**", "examples/**"]\n';
const git = (args, cwd) => execFileSync('git', args, { cwd, stdio: ['ignore', 'pipe', 'inherit'] });

function sourceCheckout() {
  const checkout = path.join(work, 'click-src');
  if (!fs.existsSync(path.join(checkout, '.git'))) {
    fs.mkdirSync(checkout, { recursive: true });
    git(['init', '-q'], checkout); git(['fetch', '-q', '--depth', '1', corpus.url, corpus.sha], checkout);
    git(['checkout', '-q', 'FETCH_HEAD'], checkout);
  }
  assert.equal(git(['rev-parse', 'HEAD'], checkout).toString().trim(), corpus.sha, 'click checkout is not the registered corpus commit');
  return checkout;
}

function freshWorkspace() {
  const workspace = path.join(fs.mkdtempSync('/tmp/deslop-python-'), 'click');
  fs.mkdirSync(workspace);
  execFileSync('sh', ['-c', `git -C "$0" archive HEAD | tar -x -C "$1"`, sourceCheckout(), workspace]);
  fs.writeFileSync(path.join(workspace, '.deslop.toml'), LIBRARY_ONLY);
  for (const args of [['init', '-q'], ['add', '.'], ['-c', 'user.name=Demo', '-c', 'user.email=demo@example.test', 'commit', '-qm', `click ${corpus.sha}`]]) git(args, workspace);
  execFileSync('sh', ['-c', TESTS], { cwd: workspace, stdio: 'ignore' });
  return workspace;
}

fs.mkdirSync(work, { recursive: true });
const resume = process.env.DESLOP_PYTHON_FILM_CDP;
const workspace = resume ? JSON.parse(fs.readFileSync(path.join(work, 'session.json'), 'utf8')).workspace : freshWorkspace();
const { page, cdp } = await filmEditor(work, workspace, resume);
const cues = cueLog(clips);
const shoot = { page, cdp, cues, frames: path.join(work, 'frames'), clips };
const rows = () => occurrenceRows(page);
const offender = file => page.locator('.part.sidebar .monaco-list-row').filter({ hasText: file }).first();
const showOffenders = () => runCommand(page, 'Deslop: Focus on Top Offenders View');

async function visitCopies(file, count) {
  for (let index = 0; index < count; index += 1) {
    if (index) { await showOffenders(); await openFinding(page, cues, offender(file), 'Back to the finding'); }
    await tap(page, cues, rows().nth(index).locator('.text-action'), `Open copy ${index + 1}`);
    await point(page, 760, 300 + index * 60); await hold(2200);
  }
}

async function removeCopy({ shell, line }) {
  await goTo(page, line);
  for (let step = 0; step < METHOD_LINES; step += 1) await page.keyboard.press('Shift+ArrowDown');
  await point(page, 700, 420); await hold(1200);
  await page.keyboard.type(ALIAS, { delay: 28 });
  // Escape first: the suggestion widget would otherwise take the Enter.
  for (const key of ['Escape', 'Enter']) await page.keyboard.press(key);
  await point(page, 640, 380); await hold(1400);
  process.stdout.write(`Replaced the ${shell} copy\n`);
}

try {
  await runCommand(page, 'View: Close All Editors');
  await openFile(page, COMPLETION); await goTo(page, BASH_METHOD_LINE);
  await showOffenders(); await offender('exceptions.py').waitFor({ timeout: 60000 });
  await runCommand(page, 'Notifications: Clear All Notifications');
  await take(shoot, 'python-open', async () => {
    await point(page, 760, 330); await hold(3500);
    await point(page, 190, 300); await hold(3000);
    await point(page, 190, 420); await hold(3000);
  });
  await take(shoot, 'python-compare', async () => {
    await openFinding(page, cues, offender('exceptions.py'), 'Open the exceptions finding');
    await point(page, 700, 420); await hold(2500);
    await compareToCanonical(page, cues, 'Compare the two classes');
    await point(page, 960, 320); await hold(3500);
    await page.mouse.wheel(0, 260); await point(page, 700, 460); await hold(3500);
  });
  await take(shoot, 'python-identical', async () => {
    await runCommand(page, 'View: Close All Editors'); await showOffenders();
    await openFinding(page, cues, offender('shell_completion.py'), 'Open the identical methods');
    await point(page, 700, 440); await hold(2500);
    await visitCopies('shell_completion.py', 3);
  });
  await take(shoot, 'python-dedupe', async () => {
    await runCommand(page, 'View: Close All Editors');
    await openFile(page, COMPLETION);
    for (const copy of COPIES) await removeCopy(copy);
    await page.keyboard.press('Meta+S'); await hold(2500);
    await showOffenders(); await point(page, 190, 360); await hold(3500);
    await terminal(page, TESTS, TESTS_PASSED);
  });
  await closeTerminal(page);
  const edited = fs.readFileSync(path.join(workspace, COMPLETION), 'utf8');
  assert.equal(edited.split(`${ALIAS}\n\n    def format_completion`).length - 1, COPIES.length, 'both copies became the alias, each followed by a blank line');
  process.stdout.write('Python film recorded\n');
} finally {
  await finishShoot(cdp, work);
}
process.exit(0);
