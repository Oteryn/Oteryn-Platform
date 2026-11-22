import assert from 'node:assert/strict';
import { test } from 'node:test';
import { requireNightlyExecution, requireNightlyPlan } from './nightly-plan.mjs';

function manifest(files) {
  const project = 'pl-1440';
  return {
    project,
    cases: files.flat(),
    shards: files.map(cases => ({ filters: cases.map(id => JSON.parse(id)[0]), cases })),
  };
}

function reports(plan) {
  return plan.shards.map(({ cases }, index) => ({ project: plan.project, shard: index + 1, cases }));
}

const caseId = file => JSON.stringify([`specs/${file}.spec.mjs`, file, 'pl-1440']);
const [admin, alpha, beta, charlie, delta, earlier] =
  ['admin', 'a', 'b', 'c', 'd', '0-extra'].map(caseId);

test('requires every planned case in the correct project and shard', () => {
  const plan = manifest([[admin], [alpha, charlie], [beta, delta]]);
  const actual = reports(plan);
  requireNightlyExecution(plan, actual);
  assert.throws(() => requireNightlyExecution(plan, actual.slice(0, 2)), /Missing execution reports/);
  assert.throws(() => requireNightlyExecution(plan, [
    { ...actual[0], project: 'en-390' }, ...actual.slice(1),
  ]), /does not identify/);
  assert.throws(() => requireNightlyExecution(plan, [
    actual[0], { ...actual[1], cases: [alpha, alpha] }, actual[2],
  ]), /differs from the published plan/);
});

test('rejects an individually valid rerun whose discovery shifted shard assignments', () => {
  const published = manifest([[admin], [alpha, charlie], [beta, delta]]);
  const changed = manifest([[admin], [earlier, beta, delta], [alpha, charlie]]);
  requireNightlyPlan(published, published.project);
  requireNightlyPlan(changed, changed.project);
  const originalRun = reports(published);
  const rerun = reports(changed);
  assert.throws(() => requireNightlyExecution(published, [
    originalRun[0], rerun[1], originalRun[2],
  ]), /differs from the published plan .* shard 2/);
});
