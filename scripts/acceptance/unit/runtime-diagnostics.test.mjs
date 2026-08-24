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

const expectedHttpStatuses = [403, 404, 409, 419, 422, 429, 500, 503];

for (const status of expectedHttpStatuses) {
  test(`expected HTTP ${status} consumes the exact response and browser load error`, () => {
    const pathname = `/expected-${status}`;
    const state = diagnostics({
      httpErrors: [{ status, url: `http://127.0.0.1:8080${pathname}` }],
      serverErrors: status >= 500 ? [{ status, url: `http://127.0.0.1:8080${pathname}` }] : [],
      consoleErrors: [{
        text: `Failed to load resource: the server responded with a status of ${status}`,
        url: `http://127.0.0.1:8080${pathname}`,
      }],
    });

    helpers.allowExpectedHttpFailure(state, { status, pathname });
    assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(state));
  });
}

test('expected HTTP allowance enforces exact pathname, status and bounded count', () => {
  const state = diagnostics({
    httpErrors: [
      { status: 404, url: 'http://127.0.0.1:8080/missing' },
      { status: 404, url: 'http://127.0.0.1:8080/missing' },
    ],
    consoleErrors: [
      { text: 'Failed to load resource: the server responded with a status of 404', url: 'http://127.0.0.1:8080/missing' },
      { text: 'Failed to load resource: the server responded with a status of 404', url: 'http://127.0.0.1:8080/missing' },
    ],
  });
  helpers.allowExpectedHttpFailure(state, { status: 404, pathname: '/missing', count: 2 });
  assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(state));

  state.httpErrors.push({ status: 404, url: 'http://127.0.0.1:8080/missing' });
  state.consoleErrors.push({ text: 'Failed to load resource: the server responded with a status of 404', url: 'http://127.0.0.1:8080/missing' });
  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(state), /Unexpected browser\/runtime failures/u);
});

test('expected HTTP allowance does not consume the wrong status or pathname', () => {
  for (const httpError of [
    { status: 403, url: 'http://127.0.0.1:8080/other' },
    { status: 404, url: 'http://127.0.0.1:8080/expected' },
  ]) {
    const state = diagnostics({ httpErrors: [httpError] });
    helpers.allowExpectedHttpFailure(state, { status: 403, pathname: '/expected' });
    assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(state), /Unexpected browser\/runtime failures/u);
  }
});

for (const failure of ['net::ERR_ABORTED', 'NS_BINDING_ABORTED', 'Load request cancelled']) {
  test(`navigation cancellation ${failure} is non-fatal`, () => {
    assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(diagnostics({
      failedRequests: [{ method: 'GET', url: 'http://127.0.0.1:8080/next', failure }],
    })));
  });
}

test('ordinary request failures and CSP violations remain fatal', () => {
  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(diagnostics({
    failedRequests: [{ method: 'GET', url: 'http://127.0.0.1:8080/app.js', failure: 'net::ERR_FAILED' }],
  })), /Unexpected browser\/runtime failures/u);

  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(diagnostics({
    consoleErrors: [{
      text: "Applying inline style violates the following Content Security Policy directive 'style-src 'self''.",
      url: 'http://127.0.0.1:8080/admin/audit',
    }],
  })), /Unexpected browser\/runtime failures/u);
});

test('unregistered 4xx response without a console error is non-fatal', () => {
  assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(diagnostics({
    httpErrors: [{ status: 404, url: 'http://127.0.0.1:8080/unrelated-missing' }],
  })));
});

test('declared expected HTTP failure must actually occur', () => {
  const state = diagnostics();
  helpers.allowExpectedHttpFailure(state, { status: 404, pathname: '/missing' });
  assert.throws(
    () => helpers.assertNoUnexpectedRuntimeFailures(state),
    /Unexpected browser\/runtime failures/u,
  );
});

test('Wiki stable-key pattern remains valid under HTML pattern v-mode semantics', () => {
  const pattern = new RegExp('^(?:[a-z0-9]+([._\\-][a-z0-9]+)*)$', 'v');
  for (const value of ['guide', 'game.system', 'game_system', 'game-system']) {
    assert.equal(pattern.test(value), true, value);
  }
  for (const value of ['Game', '-guide', 'guide-', 'guide--system']) {
    assert.equal(pattern.test(value), false, value);
  }
});