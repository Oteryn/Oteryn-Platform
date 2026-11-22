import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const inventory = JSON.parse(readFileSync(new URL('../inventory.json', import.meta.url)));
const forms = JSON.parse(readFileSync(new URL('./forms.json', import.meta.url)));
const hitDirectory = process.env.PORTAL_E2E_LEDGER_HITS ??
  fileURLToPath(new URL('../test-results/ledger/', import.meta.url));
const reportDirectory = process.env.PORTAL_E2E_LEDGER_REPORT_DIR ??
  fileURLToPath(new URL('./', import.meta.url));
const refusals = new Set([400, 401, 403, 404, 422]);
const rows = new Map();
const ids = new Set();
const key = (...parts) => JSON.stringify(parts);

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
  const index = key(hit.actor, hit.status);
  if (!destination.evidence.has(index)) {
    destination.evidence.set(index, { actor: hit.actor, status: hit.status, projects: new Set() });
  }
  destination.evidence.get(index).projects.add(hit.project);
}

const hitFiles = existsSync(hitDirectory)
  ? readdirSync(hitDirectory).filter(name => name.endsWith('.jsonl')).sort()
  : [];
if (!hitFiles.length) throw new Error(`No Playwright ledger evidence in ${hitDirectory}`);
for (const file of hitFiles) {
  for (const line of readFileSync(join(hitDirectory, file), 'utf8').split('\n').filter(Boolean)) {
    const hit = JSON.parse(line);
    if (hit.status === null) continue;
    observe(rows.get(key('R', hit.route_pattern, hit.method)), hit);
    if (hit.form_action) {
      for (const button of hit.form_action.buttons) {
        observe(rows.get(key('F', hit.form_action.action, hit.method, button)), hit);
      }
    }
  }
}

const compare = (a, b) => a.localeCompare(b, 'en', { numeric: true });
const result = [...rows.values()].sort((a, b) =>
  compare(a.crate, b.crate) || compare(a.id, b.id)).map(({ evidence, ...entry }) => ({
  ...entry,
  evidence: [...evidence.values()].sort((a, b) =>
    compare(key(a.actor), key(b.actor)) || a.status - b.status)
    .map(hit => ({ ...hit, projects: [...hit.projects].sort() })),
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
    const evidence = entry.evidence.map(({ actor, status }) =>
      `${Array.isArray(actor) ? actor.join(' + ') : actor ?? 'unknown'} ${status}`).join(', ');
    const submitter = entry.button ? code(`${entry.button.name}=${entry.button.value}`)
      : entry.action ? 'Unnamed' : '—';
    return `| ${entry.id} | ${entry.method} | ${code(entry.action ?? entry.route)} | ${submitter} | ${evidence || '—'} | ${yesNo(entry.success)} | ${yesNo(entry.refusal)} |`;
  }),
  '',
];

writeFileSync(join(reportDirectory, 'ledger.json'), JSON.stringify({ totals, rows: result }, null, 2) + '\n');
writeFileSync(join(reportDirectory, 'LEDGER.md'), markdown.join('\n'));

if (process.argv.includes('--gate') && unhit.length) {
  for (const entry of unhit) {
    const button = entry.button ? ` ${entry.button.name}=${entry.button.value}` : '';
    console.error(`${entry.id} ${entry.method} ${entry.action ?? entry.route}${button}`);
  }
  process.exitCode = 1;
}
