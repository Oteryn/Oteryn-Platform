import assert from 'node:assert/strict';
import { loadRepositoryInputs, validateBackendFrontendCompleteness } from './validate-backend-frontend-completeness.mjs';

function clone(value) {
  return structuredClone(value);
}

function record(inputs, id) {
  const found = inputs.frontendLedger.capabilities.find((capability) => capability.id === id);
  assert.ok(found, `Missing fixture capability ${id}`);
  return found;
}

function expectError(inputs, mutation, expectedMarker) {
  const candidate = clone(inputs);
  mutation(candidate);
  const report = validateBackendFrontendCompleteness(candidate);
  assert.ok(report.errors.some((error) => error.includes(expectedMarker)), `Expected error containing ${JSON.stringify(expectedMarker)}, received:\n${report.errors.join('\n')}`);
}

const inputs = loadRepositoryInputs();
const baseline = validateBackendFrontendCompleteness(inputs);
assert.deepEqual(baseline.errors, [], `Repository backend/frontend ledger is invalid:\n${baseline.errors.join('\n')}`);
assert.equal(baseline.product_capability_count, 43);
assert.equal(baseline.backend_frontend_capability_count, 43);

expectError(inputs, (candidate) => {
  record(candidate, 'account.password-recovery').frontend_status = 'missing';
}, 'account.password-recovery is product implemented but frontend_status is missing');

expectError(inputs, (candidate) => {
  record(candidate, 'account.password-recovery').surface_ids = ['unknown.frontend.surface'];
}, 'references unknown portal surface unknown.frontend.surface');

expectError(inputs, (candidate) => {
  delete record(candidate, 'support.notifications').exception_reason;
}, 'support.notifications requires a bounded exception_reason');

expectError(inputs, (candidate) => {
  const product = candidate.productLedger.capabilities.find((capability) => capability.id === 'commerce.coin-purchase-balance-delivery');
  product.delivery_status = 'implemented';
}, 'commerce.coin-purchase-balance-delivery is product implemented but backend_status is partial');

expectError(inputs, (candidate) => {
  candidate.frontendLedger.capabilities = candidate.frontendLedger.capabilities.filter((capability) => capability.id !== 'public.highscores');
}, 'Canonical product capability has no backend/frontend record: public.highscores');

process.stdout.write(`${JSON.stringify({
  baseline_capabilities: baseline.backend_frontend_capability_count,
  negative_fixtures: 5,
  result: 'PASS',
}, null, 2)}\n`);
