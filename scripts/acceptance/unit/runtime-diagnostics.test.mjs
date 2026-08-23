import assert from 'node:assert/strict';
import test from 'node:test';

const helpers = await import('../runtime-diagnostics.mjs');

function diagnostics(overrides = {}) {
  return {
    testedSha: 'test-sha',
    consoleErrors: [],
    pageErrors: [],
    failedRequests: [],
    serverErrors: [],
    ...overrides,
  };
}

test('unexpected runtime diagnostics fail the acceptance gate', () => {
  assert.equal(typeof helpers.assertNoUnexpectedRuntimeFailures, 'function');

  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(diagnostics({
    pageErrors: [{ message: 'boom' }],
  })), /Unexpected browser\/runtime failures/u);
});

test('expected HTTP failure allowance is bounded to one status and path', () => {
  assert.equal(typeof helpers.allowExpectedHttpFailure, 'function');

  const state = diagnostics({
    consoleErrors: [{
      text: 'Failed to load resource: the server responded with a status of 503 (Service Unavailable)',
      url: 'http://127.0.0.1:8080/online',
    }],
    serverErrors: [{ status: 503, url: 'http://127.0.0.1:8080/online' }],
  });

  helpers.allowExpectedHttpFailure(state, { status: 503, pathname: '/online' });
  assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(state));

  state.serverErrors.push({ status: 503, url: 'http://127.0.0.1:8080/servers' });
  assert.throws(
    () => helpers.assertNoUnexpectedRuntimeFailures(state),
    /Unexpected browser\/runtime failures/u,
  );
});

test('attachDiagnostics persists evidence before enforcing the runtime gate', async () => {
  const attachments = [];
  const testInfo = {
    attach: async (name) => attachments.push(name),
  };

  await assert.rejects(
    () => helpers.attachRuntimeDiagnostics(testInfo, diagnostics({
      failedRequests: [{
        method: 'GET',
        url: 'http://127.0.0.1:8080/assets/missing.js',
        failure: 'net::ERR_FAILED',
      }],
    })),
    /Unexpected browser\/runtime failures/u,
  );
  assert.deepEqual(attachments, ['exact-tested-sha', 'browser-diagnostics']);
});

test('navigation aborts are ignored but real request failures remain fatal', () => {
  assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(diagnostics({
    failedRequests: [{
      method: 'GET',
      url: 'http://127.0.0.1:8080/redirected',
      failure: 'net::ERR_ABORTED',
    }],
  })));

  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(diagnostics({
    failedRequests: [{
      method: 'GET',
      url: 'http://127.0.0.1:8080/assets/app.js',
      failure: 'net::ERR_FAILED',
    }],
  })), /Unexpected browser\/runtime failures/u);
});
