import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import {cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {test} from 'node:test';
import {inventoryDigest} from './capture.mjs';

test('coverage requires a response from the matching passing attempt', t => {
  const directory = mkdtempSync(join(tmpdir(), 'portal-ledger-report-'));
  t.after(() => rmSync(directory, {recursive: true}));
  mkdirSync(join(directory, 'ledger'));
  mkdirSync(join(directory, 'test-results/ledger'), {recursive: true});
  for (const file of ['inventory.json', 'ledger/capture.mjs', 'ledger/forms.json', 'ledger/report.mjs']) {
    cpSync(new URL(`../${file}`, import.meta.url), join(directory, file));
  }

  const result = (status, workerIndex, retry) => ({status, workerIndex, retry});
  const run = (projectName, expectedStatus, results) => ({projectName, expectedStatus, results});
  const spec = (title, tests) => ({file: 'specs/ledger.spec.mjs', title, tests});
  const suites = [{suites: [{title: 'search', specs: [
    spec('submits', [
      run('pl-1440', 'passed', [result('timedOut', 0, 0), result('passed', 1, 1)]),
      run('en-390', 'passed', [result('failed', 2, 0)]),
    ]),
    spec('expected failure', [run('pl-1440', 'failed', [result('failed', 1, 0)])]),
  ]}]}];
  const baselineCapture = {
    revision: 'a'.repeat(40), runId: '10000000-0000-4000-8000-000000000000', clean: true, inventory: inventoryDigest(),
  };
  const candidateCapture = {...baselineCapture, runId: '20000000-0000-4000-8000-000000000000'};
  const config = {configFile: join(directory, 'playwright.config.mjs'), rootDir: directory};
  const results = join(directory, 'test-results/results.json');
  const captureFile = join(directory, 'test-results/ledger/requests.jsonl');
  const report = (...args) => execFileSync(process.execPath, [join(directory, 'ledger/report.mjs'), ...args], {stdio: 'pipe'});
  const saveResults = ledger => writeFileSync(results, JSON.stringify({config: {...config, metadata: {ledger}}, suites}));
  const saveCaptures = (requests, capture) => writeFileSync(captureFile, requests.map(row =>
    JSON.stringify({...row, capture})).join('\n'));
  const rejects = (message, ...args) => assert.throws(() => report(...args),
    error => error.status === 1 && error.stderr.toString().includes(message));

  const hit = {
    route_pattern: '/search', method: 'GET', status: 200, actor: 'anonymous', credential: 'none',
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
  saveResults(baselineCapture);
  saveCaptures(requests, baselineCapture);
  report();

  const {rows, capture} = JSON.parse(readFileSync(join(directory, 'ledger/ledger.json')));
  assert.deepEqual(capture, baselineCapture);
  const search = rows.find(row => row.id === 'F-web-26667257346850');
  assert.deepEqual(search.evidence, [{
    actor: 'anonymous', credential: 'none', status: 200, projects: ['pl-1440'],
    cases: [{spec: hit.spec, test: hit.test, project: 'pl-1440'}],
  }]);
  assert.equal(rows.find(row => row.id === 'R-identity-145836777639757').hit, false);

  saveResults(candidateCapture);
  rejects('Ledger captures do not match the Playwright run.', '--delta');
  saveCaptures([{recorder_errors: ['session actor lookup']}], candidateCapture);
  rejects('Ledger capture failed');

  const loot = {...hit, route_pattern: '/{lang}/wiki/calculators', form_action: {
    action: '/{lang}/wiki/calculators', buttons: [null], operation: {name: 'calculator', value: 'loot'},
  }};
  saveCaptures([loot], candidateCapture);
  report('--delta'); // The report directory is outside any checkout.
  const delta = readFileSync(join(directory, 'ledger/DELTA.md'), 'utf8');
  assert.ok(delta.includes(`Tested revision \`${candidateCapture.revision}\`, run \`${candidateCapture.runId}\`. Baseline run \`${baselineCapture.runId}\`.`));
  const [gained, unhit] = delta.split('## Baseline gaps still unhit');
  assert.match(gained, /\| F-[^\n]+calculator=loot/);
  for (const operation of ['experience', 'stamina', 'training']) {
    assert.ok(!gained.includes(`calculator=${operation}`));
    assert.ok(unhit.includes(`calculator=${operation}`));
  }

  saveResults(baselineCapture);
  saveCaptures([loot], baselineCapture);
  rejects('The baseline must be a separate Playwright run of the same captured revision.', '--delta');

  const otherRevision = {...candidateCapture, revision: 'b'.repeat(40)};
  saveResults(otherRevision);
  saveCaptures([loot], otherRevision);
  rejects('The baseline must be a separate Playwright run of the same captured revision.', '--delta');

  const dirty = {...candidateCapture, clean: false};
  saveResults(dirty);
  saveCaptures([loot], dirty);
  rejects('Ledger evidence requires a clean checkout at capture time.');

  const otherInventory = {...candidateCapture, inventory: '0'.repeat(64)};
  saveResults(otherInventory);
  saveCaptures([loot], otherInventory);
  rejects('The route and form inventory does not match the Playwright capture.');
});
