import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createWriteStream } from 'node:fs';
import { mkdtemp, readFile, readdir, writeFile } from 'node:fs/promises';
import { get } from 'node:https';
import { tmpdir } from 'node:os';
import { basename, join, relative } from 'node:path';
import { createInterface } from 'node:readline';
import { setTimeout as pollInterval } from 'node:timers/promises';
import { fileURLToPath } from 'node:url';
import pg from 'pg';

const cwd = fileURLToPath(new URL('../', import.meta.url));
const [tier, project, shard] = process.argv.slice(2);
const manifest = JSON.parse(await readFile(new URL('smoke.json', import.meta.url)));
const escape = value => value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
let selection;
const smokeProjects = project ? [project] : manifest.projects;
if (tier === 'smoke' && !shard && smokeProjects.every(name => manifest.projects.includes(name))) {
  selection = [
    ...smokeProjects.map(name => `--project=${name}`),
    '--grep', ` (?:${manifest.titles.map(escape).join('|')})$`,
  ];
} else if (tier === 'nightly' && project && ['1', '2', '3'].includes(shard)) {
  const specs = (await readdir(join(cwd, 'specs'), { recursive: true, withFileTypes: true }))
    .filter(file => file.isFile() && /\.(spec|test)\.[cm]?[jt]sx?$/.test(file.name))
    .map(file => relative(cwd, join(file.parentPath, file.name)));
  specs.push('tiers/smoke.spec.mjs');
  specs.sort();
  // Keep the staff bootstrap with consumers of its browser storage state.
  const usesStaffState = file => /^(?:admin\.|content\.|game-sim-bazaar\.|routes\.|staff-)/.test(basename(file));
  const files = shard === '1'
    ? specs.filter(usesStaffState)
    : specs.filter(file => !usesStaffState(file)).filter((_, index) => index % 2 === Number(shard) - 2);
  if (!files.length) throw new Error('Empty nightly shard');
  selection = [`--project=${project}`, ...files.map(file => `/${escape(file)}$`)];
} else {
  throw new Error('Usage: node tiers/run.mjs smoke [PROJECT] | nightly PROJECT SHARD');
}

const scratch = await mkdtemp(join(tmpdir(), 'portal-ci-e2e-'));
const reportFile = join(scratch, 'results.json');
const env = {
  ...process.env,
  PORTAL_E2E_SCRATCH: scratch,
  PORTAL_E2E_AUTH_DIRECTORY: scratch,
  PLAYWRIGHT_JSON_OUTPUT_FILE: reportFile,
};
const started = performance.now();
const probe = () => new Promise(resolve => {
  const request = get('https://localhost:18443/en/', {
    family: 4, rejectUnauthorized: false, timeout: 2_000,
  }, response => {
    response.resume();
    resolve(response.statusCode === 200);
  });
  request.on('timeout', () => request.destroy());
  request.on('error', () => resolve(false));
});

function launch(script, args, log) {
  const output = createWriteStream(join(scratch, log), { mode: 0o600 });
  const child = spawn(process.execPath, [script, ...args], { cwd, env, stdio: ['ignore', 'pipe', 'pipe'] });
  child.stdout.pipe(output, { end: false });
  child.stderr.pipe(output, { end: false });
  child.on('close', () => output.end());
  return { child, done: once(child, 'close').then(([code]) => code) };
}

let stack;
const stop = () => stack?.child.kill('SIGINT');
process.once('SIGINT', stop);
process.once('SIGTERM', stop);
try {
  const admin = new URL(env.PORTAL_E2E_ADMIN_DATABASE_URL);
  if (!['localhost', '127.0.0.1', '[::1]'].includes(admin.hostname)) throw new Error('Local PostgreSQL only');
  const databaseDeadline = performance.now() + 30_000;
  while (true) {
    const client = new pg.Client({ connectionString: admin.href, connectionTimeoutMillis: 1_000 });
    const connected = await client.connect().then(() => true, () => false);
    await client.end();
    if (connected) break;
    if (performance.now() > databaseDeadline) throw new Error('Local PostgreSQL readiness timed out');
    await pollInterval(250);
  }
  stack = launch('local.mjs', [], 'local.log');
  let launched = false;
  createInterface({ input: stack.child.stdout }).on('line', line => {
    if (line.startsWith('Services starting.')) launched = true;
  });
  const deadline = performance.now() + (env.PORTAL_E2E_BINARY ? 120_000 : 2_700_000);
  while (true) {
    if (stack.child.exitCode !== null || stack.child.signalCode) throw new Error('Local harness exited during startup');
    if (launched && await probe()) break;
    if (performance.now() > deadline) throw new Error('Local harness readiness timed out');
    await pollInterval(250);
  }

  const args = [join(scratch, 'env.json'), 'test', '--config=tiers/playwright.config.mjs', ...selection];
  const listing = await launch('run.mjs', [...args, '--list', '--reporter=list'], 'list.log').done;
  const list = await readFile(join(scratch, 'list.log'), 'utf8');
  const count = Number(/^Total: (\d+) tests? in /m.exec(list)?.[1]);
  if (listing !== 0 || !count) throw new Error('Playwright discovery failed or selected no tests');
  if (tier === 'smoke') {
    for (const name of smokeProjects) {
      for (const title of manifest.titles) {
        const matches = list.split('\n').filter(line => line.trimStart().startsWith(`[${name}] `) && line.endsWith(` › ${title}`));
        if (matches.length !== 1) throw new Error(`Expected one ${manifest.tag} case per project for "${title}"`);
      }
    }
    if (count !== smokeProjects.length * manifest.titles.length) throw new Error('Unexpected smoke test count');
  }

  const status = await launch('run.mjs', [...args, '--trace=off', '--reporter=json'], 'playwright.log').done;
  const { stats, suites } = JSON.parse(await readFile(reportFile, 'utf8'));
  const summary = `Listed ${count}, passed ${stats.expected}, failed ${stats.unexpected}, flaky ${stats.flaky}, skipped ${stats.skipped}, ${Math.round((performance.now() - started) / 1000)}s including harness startup`;
  console.log(summary);

  // Source locations and outcomes are safe to publish. Raw browser evidence
  // can contain form values, cookies, error payloads, and attachments.
  const rows = [];
  function collect(suite) {
    for (const spec of suite.specs ?? []) {
      for (const test of spec.tests) {
        if (!spec.file.match(/^[\w./-]+$/) || !Number.isInteger(spec.line)) throw new Error('Invalid report source location');
        if (!['expected', 'unexpected', 'flaky', 'skipped'].includes(test.status)) throw new Error('Invalid report test outcome');
        const source = `${spec.file}:${spec.line}`;
        rows.push(`<tr><td>${source}</td><td>${test.status}</td></tr>`);
        if (test.status !== 'expected') console.error(`${source} ${test.status}`);
      }
    }
    for (const child of suite.suites ?? []) collect(child);
  }
  suites.forEach(collect);
  if (tier === 'nightly') {
    await writeFile(join(cwd, 'tier-report.html'), `<!doctype html><html lang="en"><meta charset="utf-8"><title>Portal E2E</title><h1>Portal E2E</h1><p>${summary}</p><table><thead><tr><th>Source</th><th>Outcome</th></tr></thead><tbody>${rows.join('\n')}</tbody></table></html>\n`);
  }
  if (status !== 0 || stats.expected !== count || stats.unexpected || stats.flaky || stats.skipped) {
    throw new Error('Playwright did not pass every selected test');
  }
} catch (error) {
  console.error(error instanceof SyntaxError ? 'Invalid Playwright report' : error.message);
  if (!process.env.CI) console.error(`Private diagnostics: ${scratch}`);
  process.exitCode = 1;
} finally {
  stop();
  await stack?.done;
}
