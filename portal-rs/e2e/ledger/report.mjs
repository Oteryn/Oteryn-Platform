import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';

const inventory = JSON.parse(readFileSync(new URL('../inventory.json', import.meta.url)));
const forms = JSON.parse(readFileSync(new URL('./forms.json', import.meta.url)));
const hitDirectory = new URL('../test-results/ledger/', import.meta.url);
const refusals = new Set([400, 401, 403, 404, 422]);
const rows = new Map();
const ids = new Set();
const key = (...parts) => JSON.stringify(parts);

const evidenceRules = 2;
const resultsFile = new URL('../test-results/results.json', import.meta.url);
const results = JSON.parse(readFileSync(resultsFile));
const configDirectory = dirname(results.config.configFile);
const passing = new Set();
function collectPassing(suite, parents = []) {
  for (const spec of suite.specs ?? []) {
    const title = [...parents, spec.title].join(' › ');
    const file = relative(configDirectory, resolve(results.config.rootDir, spec.file));
    for (const test of spec.tests) {
      for (const result of test.results) {
        if (result.status === 'passed' && test.expectedStatus === 'passed') {
          passing.add(key(file, title, test.projectName, result.workerIndex, result.retry));
        }
      }
    }
  }
  for (const child of suite.suites ?? []) collectPassing(child, [...parents, child.title]);
}
for (const suite of results.suites) collectPassing(suite);

function row(kind, crate, parts, properties) {
  const index = key(kind, ...parts);
  if (!rows.has(index)) {
    const number = createHash('sha256').update(index).digest().readUIntBE(0, 6);
    const id = `${kind}-${crate}-${number}`;
    if (ids.has(id)) throw new Error(`Duplicate ledger row ID ${id}`);
    ids.add(id);
    rows.set(index, { id, crate, ...properties, evidence: new Map() });
  }
  return rows.get(index);
}

for (const { crate, route, methods } of inventory.routes) {
  for (const method of methods) row('R', crate, [route, method], { method, route });
}

for (const form of forms) {
  for (const button of form.buttons) {
    const destination = row('F', form.crate, [form.action, form.method, button], {
      method: form.method, route: form.route_pattern, action: form.action, button, templates: [],
    });
    const location = `${form.template}:${form.line}`;
    if (!destination.templates.includes(location)) destination.templates.push(location);
  }
}

function observe(destination, hit) {
  if (!destination) return;
  const index = key(hit.actor, hit.credential, hit.status);
  if (!destination.evidence.has(index)) {
    destination.evidence.set(index, { actor: hit.actor, credential: hit.credential, status: hit.status, projects: new Set(), cases: new Map() });
  }
  const evidence = destination.evidence.get(index);
  evidence.projects.add(hit.project);
  const provenance = { spec: hit.spec, test: hit.test, project: hit.project };
  evidence.cases.set(key(hit.spec, hit.test, hit.project), provenance);
}

if (existsSync(hitDirectory)) {
  for (const file of readdirSync(hitDirectory).filter(name => name.endsWith('.jsonl')).sort()) {
    for (const line of readFileSync(new URL(file, hitDirectory), 'utf8').split('\n').filter(Boolean)) {
      const hit = JSON.parse(line);
      if (hit.recorder_errors) throw new Error('Ledger capture failed. See the Playwright test failure.');
      if (!Object.hasOwn(hit, 'credential')) throw new Error('Ledger capture uses outdated evidence rules. Rerun Playwright.');
      if (hit.status === null || !passing.has(key(hit.spec, hit.test, hit.project, hit.worker, hit.retry))) continue;
      observe(rows.get(key('R', hit.route_pattern, hit.method)), hit);
      if (hit.form_action) {
        for (const button of hit.form_action.buttons) {
          observe(rows.get(key('F', hit.form_action.action, hit.method, button)), hit);
        }
      }
    }
  }
}

const compare = (a, b) => a.localeCompare(b, 'en', { numeric: true });
const result = [...rows.values()].sort((a, b) =>
  compare(a.crate, b.crate) || compare(a.id, b.id)).map(({ evidence, ...entry }) => ({
  ...entry,
  evidence: [...evidence.values()].sort((a, b) =>
    compare(key(a.actor, a.credential), key(b.actor, b.credential)) || a.status - b.status)
    .map(({ cases, projects, ...hit }) => ({
      ...hit,
      projects: [...projects].sort(),
      cases: [...cases.entries()].sort(([a], [b]) => compare(a, b)).map(([, evidence]) => evidence),
    })),
  hit: evidence.size > 0,
  success: [...evidence.values()].some(hit => hit.status >= 200 && hit.status < 400),
  refusal: [...evidence.values()].some(hit => refusals.has(hit.status)),
}));
const unhit = result.filter(entry => !entry.hit);
const totals = {
  rows: result.length,
  hit: result.length - unhit.length,
  unhit: unhit.length,
  unhit_by_crate: Object.fromEntries([...new Set(result.map(entry => entry.crate))]
    .map(crate => [crate, unhit.filter(entry => entry.crate === crate).length])),
  forms_never_submitted: unhit.filter(entry => entry.id.startsWith('F-')).length,
};

const yesNo = value => value ? 'Yes' : 'No';
const code = value => `\`${String(value).replaceAll('|', '\\|').replaceAll('\`', '&#96;')}\``;
const actorLabel = actor => Array.isArray(actor) ? actor.join(' + ') : actor ?? 'unresolved';
const markdown = [
  '# Playwright coverage ledger',
  '',
  `${totals.rows} rows, ${totals.hit} hit, ${totals.unhit} unhit, ${totals.forms_never_submitted} form rows never submitted.`,
  '',
  '| Crate | Unhit |',
  '| --- | ---: |',
  ...Object.entries(totals.unhit_by_crate).map(([crate, count]) => `| ${crate} | ${count} |`),
  '',
  '| ID | Method | Route or form action | Submitter | Actors and statuses | Success | Refusal |',
  '| --- | --- | --- | --- | --- | --- | --- |',
  ...result.map(entry => {
    const evidence = entry.evidence.map(({ actor, credential, status }) =>
      `${actorLabel(actor)} [${credential}] ${status}`).join(', ');
    const submitter = entry.button ? code(`${entry.button.name}=${entry.button.value}`)
      : entry.action ? 'Unnamed' : '—';
    return `| ${entry.id} | ${entry.method} | ${code(entry.action ?? entry.route)} | ${submitter} | ${evidence || '—'} | ${yesNo(entry.success)} | ${yesNo(entry.refusal)} |`;
  }),
  '',
];

if (process.argv.includes('--delta')) {
  const baseline = JSON.parse(readFileSync(new URL('./ledger.json', import.meta.url)));
  if (baseline.evidence_rules !== evidenceRules) throw new Error('Regenerate the baseline with the current ledger rules before generating a delta.');
  const baselineIds = new Set(baseline.rows.filter(row => !row.hit).map(row => row.id));
  const gained = result.filter(row => row.hit && baselineIds.has(row.id));
  const stillUnhit = result.filter(row => !row.hit && baselineIds.has(row.id));
  const revision = execFileSync('git', ['rev-parse', 'HEAD'], {
    cwd: new URL('.', import.meta.url), encoding: 'utf8',
  }).trim();
  const lines = [
    '# Portal ledger coverage delta',
    '',
    `Tested revision \`${revision}\`.`,
    '',
    `${gained.length} of ${baselineIds.size} baseline gaps hit by passing tests, including ${gained.filter(row => row.action).length} form rows. ${stillUnhit.length} baseline gaps remain.`,
    '',
    '| Row | Method and path | Principal, credential, and status | Passing test and project |',
    '| --- | --- | --- | --- |',
  ];
  for (const row of gained) {
    for (const { actor, credential, status, cases } of row.evidence) {
      for (const { spec, test, project } of cases) {
        lines.push(`| ${row.id} | ${row.method} ${code(row.action ?? row.route)} | ${code(actorLabel(actor))} [${credential}] ${status} | ${code(spec)} · ${code(test)} · ${code(project)} |`);
      }
    }
  }
  lines.push('', '## Baseline gaps still unhit', '');
  for (const row of stillUnhit) lines.push(`- ${row.id} ${row.method} ${code(row.action ?? row.route)}`);
  writeFileSync(new URL('./DELTA.md', import.meta.url), lines.join('\n') + '\n');
} else {
  writeFileSync(new URL('./ledger.json', import.meta.url), JSON.stringify({ evidence_rules: evidenceRules, totals, rows: result }, null, 2) + '\n');
  writeFileSync(new URL('./LEDGER.md', import.meta.url), markdown.join('\n'));
}

if (process.argv.includes('--gate') && unhit.length) {
  for (const entry of unhit) {
    const button = entry.button ? ` ${entry.button.name}=${entry.button.value}` : '';
    console.error(`${entry.id} ${entry.method} ${entry.action ?? entry.route}${button}`);
  }
  process.exitCode = 1;
}
