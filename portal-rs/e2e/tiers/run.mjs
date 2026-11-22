import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createWriteStream } from 'node:fs';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { get } from 'node:https';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { discoveredCases, planNightly, playwrightCases } from './nightly-plan.mjs';
import { escapeHtml, reportCases, reportRow } from './nightly-report.mjs';
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
  selection = [`--project=${project}`];
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

function launch(script, args, log, childEnv = env) {
  const output = createWriteStream(join(scratch, log), { mode: 0o600 });
  const child = spawn(process.execPath, [script, ...args], { cwd, env: childEnv, stdio: ['ignore', 'pipe', 'pipe'] });
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
  async function discover(filters, label) {
    const discovery = join(scratch, `discovery-${label}.json`);
    const status = await launch('run.mjs', [...args, ...filters, '--list', '--reporter=json'],
      `discovery-${label}.log`, { ...env, PLAYWRIGHT_JSON_OUTPUT_FILE: discovery }).done;
    if (status !== 0) throw new Error(`Playwright discovery failed for ${label}`);
    return JSON.parse(await readFile(discovery, 'utf8')).suites;
  }

  let plannedCases;
  if (tier === 'nightly') {
    const plans = await planNightly(async (filters, label) => discoveredCases(await discover(filters, label)));
    const plan = plans[Number(shard) - 1];
    args.push(...plan.filters);
    plannedCases = plan.cases;
  } else {
    const suites = await discover([], 'smoke');
    const listed = Array.from(playwrightCases(suites));
    for (const name of smokeProjects) {
      for (const title of manifest.titles) {
        const matches = listed.filter(({ spec, test }) => test.projectName === name && spec.title === title);
        if (matches.length !== 1) throw new Error(`Expected one ${manifest.tag} case per project for "${title}"`);
      }
    }
    plannedCases = discoveredCases(suites);
    const expected = smokeProjects.length * manifest.titles.length;
    if (!expected || plannedCases.length !== expected || new Set(plannedCases).size !== expected) {
      throw new Error('Unexpected smoke case identities or count');
    }
  }
  const count = plannedCases.length;

  const status = await launch('run.mjs', [...args, '--trace=off', '--reporter=json'], 'playwright.log').done;
  const { stats, suites } = JSON.parse(await readFile(reportFile, 'utf8'));
  const routes = JSON.parse(await readFile(join(cwd, 'route-expectations.json'), 'utf8'));
  const routeTitles = new Set(routes.map(row => `route ${row.crate} ${row.route}`));
  const cases = reportCases(suites, routeTitles);
  const passed = cases.filter(test => test.status === 'passed').length;
  const skipped = cases.filter(test => test.status === 'skipped').length;
  const other = cases.length - passed - skipped;
  const summary = `Listed ${count}, passed ${passed}, non-passing ${other}, flaky ${stats.flaky}, skipped ${skipped}, ${Math.round((performance.now() - started) / 1000)}s including harness startup`;
  console.log(summary);
  for (const test of cases) {
    if (test.status !== 'passed') console.error(`${test.source} ${test.id} ${test.status}: ${test.reason}`);
  }
  if (tier === 'nightly') {
    const rows = cases.map(reportRow);
    await writeFile(join(cwd, 'tier-report.html'), `<!doctype html><html lang="en"><meta charset="utf-8"><title>Portal E2E</title><h1>Portal E2E</h1><p>${escapeHtml(summary)}</p><table><thead><tr><th>Case ID</th><th>Route</th><th>Source</th><th>Outcome</th><th>Reason</th></tr></thead><tbody>${rows.join('\n')}</tbody></table></html>\n`);
  }
  if (JSON.stringify(discoveredCases(suites)) !== JSON.stringify(plannedCases)) {
    throw new Error('Playwright execution differs from discovery');
  }
  if (status !== 0 || passed !== count || stats.expected !== count || stats.unexpected || stats.flaky || stats.skipped) {
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
