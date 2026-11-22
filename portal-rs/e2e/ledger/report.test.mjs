import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {test} from 'node:test';

test('coverage requires a response from the matching passing attempt', t => {
  const directory = mkdtempSync(join(tmpdir(), 'portal-ledger-report-'));
  t.after(() => rmSync(directory, {recursive: true}));
  mkdirSync(join(directory, 'ledger'));
  mkdirSync(join(directory, 'test-results/ledger'), {recursive: true});
  for (const file of ['inventory.json', 'ledger/forms.json', 'ledger/report.mjs']) {
    cpSync(new URL(`../${file}`, import.meta.url), join(directory, file));
  }

  const result = (status, workerIndex, retry) => ({status, workerIndex, retry});
  const run = (projectName, expectedStatus, results) => ({projectName, expectedStatus, results});
  const spec = (title, tests) => ({file: 'ledger.spec.mjs', title, tests});
  const suites = [{suites: [{title: 'search', specs: [
    spec('submits', [
      run('pl-1440', 'passed', [result('timedOut', 0, 0), result('passed', 1, 1)]),
      run('en-390', 'passed', [result('failed', 2, 0)]),
    ]),
    spec('expected failure', [run('pl-1440', 'failed', [result('failed', 1, 0)])]),
  ]}]}];
  writeFileSync(join(directory, 'test-results/results.json'), JSON.stringify({suites}));

  const hit = {
    route_pattern: '/search', method: 'GET', status: 200, actor: 'anonymous',
    form_action: {action: '/search', buttons: [null]},
    spec: 'specs/ledger.spec.mjs', test: 'search › submits', project: 'pl-1440', worker: 1, retry: 1,
  };
  const requests = [
    hit,
    {...hit, route_pattern: '/account/recovery-key', method: 'POST', form_action: null, worker: 0, retry: 0},
    {...hit, project: 'en-390', worker: 2, retry: 0},
    {...hit, test: 'search › expected failure', retry: 0},
    {...hit, status: null},
  ];
  writeFileSync(join(directory, 'test-results/ledger/requests.jsonl'), requests.map(row => JSON.stringify(row)).join('\n'));
  execFileSync(process.execPath, [join(directory, 'ledger/report.mjs')]);

  const {rows} = JSON.parse(readFileSync(join(directory, 'ledger/ledger.json')));
  const search = rows.find(row => row.id === 'F-web-26667257346850');
  assert.deepEqual(search.evidence, [{
    actor: 'anonymous', status: 200, projects: ['pl-1440'],
    cases: [{spec: hit.spec, test: hit.test, project: 'pl-1440'}],
  }]);
  assert.equal(rows.find(row => row.id === 'R-identity-145836777639757').hit, false);
});
