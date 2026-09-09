import assert from 'node:assert/strict';
import test from 'node:test';

const helpers = await import('../runtime-diagnostics.mjs');
const REQUEST_IDENTITY_A = 'a'.repeat(64);
const REQUEST_IDENTITY_B = 'b'.repeat(64);

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

test('attachDiagnostics records the exact Playwright browser before enforcing the gate', async () => {
  const state = diagnostics();
  const testInfo = {
    project: { use: { browserName: 'firefox' } },
    attach: async () => {},
  };

  await helpers.attachRuntimeDiagnostics(testInfo, state, 'test-sha');
  assert.equal(state.browserName, 'firefox');
});

test('installDiagnostics exposes exact browser identity to direct assertion paths', async () => {
  const acceptanceHelpers = await import('../tests/helpers.mjs');
  const pageFor = (browserName) => ({
    context: () => ({
      browser: () => browserName === null
        ? null
        : { browserType: () => ({ name: () => browserName }) },
    }),
    on: () => {},
  });
  const addExpectedDuplicate = (state) => {
    state.httpErrors.push({
      status: 404,
      method: 'GET',
      url: 'http://127.0.0.1:8080/missing',
      requestIdentity: REQUEST_IDENTITY_A,
    });
    state.failedRequests.push({
      method: 'GET',
      url: 'http://127.0.0.1:8080/missing',
      requestIdentity: REQUEST_IDENTITY_A,
      failure: '<unknown error>',
    });
    acceptanceHelpers.allowExpectedHttpFailure(state, { status: 404, pathname: '/missing' });
  };

  const firefox = acceptanceHelpers.installDiagnostics(pageFor('firefox'));
  assert.equal(firefox.browserName, 'firefox');
  addExpectedDuplicate(firefox);
  assert.doesNotThrow(() => acceptanceHelpers.assertNoUnexpectedRuntimeFailures(firefox));

  firefox.failedRequests.push({
    method: 'GET',
    url: 'http://127.0.0.1:8080/missing',
    requestIdentity: REQUEST_IDENTITY_A,
    failure: '<unknown error>',
  });
  assert.throws(
    () => acceptanceHelpers.assertNoUnexpectedRuntimeFailures(firefox),
    /Unexpected browser\/runtime failures/u,
  );

  for (const browserName of ['chromium', 'webkit', null]) {
    const state = acceptanceHelpers.installDiagnostics(pageFor(browserName));
    assert.equal(state.browserName, browserName);
    addExpectedDuplicate(state);
    assert.throws(
      () => acceptanceHelpers.assertNoUnexpectedRuntimeFailures(state),
      /Unexpected browser\/runtime failures/u,
    );
  }
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

test('Firefox duplicate request failure consumes only the exact matched response identity', () => {
  const state = diagnostics({
    browserName: 'firefox',
    httpErrors: [{
      status: 404,
      method: 'GET',
      url: 'http://127.0.0.1:8080/missing',
      requestIdentity: REQUEST_IDENTITY_A,
    }],
    failedRequests: [{
      method: 'GET',
      url: 'http://127.0.0.1:8080/missing',
      requestIdentity: REQUEST_IDENTITY_A,
      failure: '<unknown error>',
    }],
  });
  helpers.allowExpectedHttpFailure(state, { status: 404, pathname: '/missing' });
  assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(state));

  state.failedRequests.push({
    method: 'GET',
    url: 'http://127.0.0.1:8080/missing',
    requestIdentity: REQUEST_IDENTITY_A,
    failure: '<unknown error>',
  });
  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(state), /Unexpected browser\/runtime failures/u);
});

test('Firefox duplicate suppression rejects a different origin with the same pathname', () => {
  const state = diagnostics({
    browserName: 'firefox',
    httpErrors: [{
      status: 404,
      method: 'GET',
      url: 'http://127.0.0.1:8080/missing',
      requestIdentity: REQUEST_IDENTITY_A,
    }],
    failedRequests: [{
      method: 'GET',
      url: 'https://cdn.example.test/missing',
      requestIdentity: REQUEST_IDENTITY_B,
      failure: '<unknown error>',
    }],
  });
  helpers.allowExpectedHttpFailure(state, { status: 404, pathname: '/missing' });
  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(state), /Unexpected browser\/runtime failures/u);
});

test('installed diagnostics preserve secret-safe query identity for Firefox duplicate matching', async () => {
  const acceptanceHelpers = await import('../tests/helpers.mjs');
  const install = () => {
    const handlers = new Map();
    const page = {
      context: () => ({ browser: () => ({ browserType: () => ({ name: () => 'firefox' }) }) }),
      on: (event, handler) => handlers.set(event, handler),
    };
    return { state: acceptanceHelpers.installDiagnostics(page), handlers };
  };
  const emitResponse = (handlers, rawUrl, method = 'GET') => handlers.get('response')({
    status: () => 404,
    url: () => rawUrl,
    request: () => ({ method: () => method }),
  });
  const emitFailure = (handlers, rawUrl) => handlers.get('requestfailed')({
    method: () => 'GET',
    url: () => rawUrl,
    failure: () => ({ errorText: '<unknown error>' }),
  });

  const mismatched = install();
  emitResponse(mismatched.handlers, 'http://127.0.0.1:8080/missing?id=1');
  emitFailure(mismatched.handlers, 'http://127.0.0.1:8080/missing?id=2');
  acceptanceHelpers.allowExpectedHttpFailure(mismatched.state, { status: 404, pathname: '/missing' });
  assert.equal(mismatched.state.httpErrors[0].url, 'http://127.0.0.1:8080/missing');
  assert.equal(mismatched.state.failedRequests[0].url, 'http://127.0.0.1:8080/missing');
  assert.notEqual(
    mismatched.state.httpErrors[0].requestIdentity,
    mismatched.state.failedRequests[0].requestIdentity,
  );
  assert.throws(
    () => acceptanceHelpers.assertNoUnexpectedRuntimeFailures(mismatched.state),
    /Unexpected browser\/runtime failures/u,
  );

  const exact = install();
  emitResponse(exact.handlers, 'http://127.0.0.1:8080/missing?id=1');
  emitFailure(exact.handlers, 'http://127.0.0.1:8080/missing?id=1');
  acceptanceHelpers.allowExpectedHttpFailure(exact.state, { status: 404, pathname: '/missing' });
  assert.equal(exact.state.httpErrors[0].method, 'GET');
  assert.equal(exact.state.httpErrors[0].requestIdentity, exact.state.failedRequests[0].requestIdentity);
  assert.doesNotThrow(() => acceptanceHelpers.assertNoUnexpectedRuntimeFailures(exact.state));

  const wrongResponseMethod = install();
  emitResponse(wrongResponseMethod.handlers, 'http://127.0.0.1:8080/missing?id=1', 'POST');
  emitFailure(wrongResponseMethod.handlers, 'http://127.0.0.1:8080/missing?id=1');
  acceptanceHelpers.allowExpectedHttpFailure(wrongResponseMethod.state, { status: 404, pathname: '/missing' });
  assert.equal(wrongResponseMethod.state.httpErrors[0].method, 'POST');
  assert.throws(
    () => acceptanceHelpers.assertNoUnexpectedRuntimeFailures(wrongResponseMethod.state),
    /Unexpected browser\/runtime failures/u,
  );
});

test('Firefox duplicate suppression remains fail-closed for method, failure and missing identity', () => {
  for (const failedRequest of [
    { method: 'POST', failure: '<unknown error>', requestIdentity: REQUEST_IDENTITY_A },
    { method: 'GET', failure: 'net::ERR_FAILED', requestIdentity: REQUEST_IDENTITY_A },
    { method: 'GET', failure: '<unknown error>' },
  ]) {
    const state = diagnostics({
      browserName: 'firefox',
      httpErrors: [{
        status: 404,
        method: 'GET',
        url: 'http://127.0.0.1:8080/missing',
        requestIdentity: REQUEST_IDENTITY_A,
      }],
      failedRequests: [{
        url: 'http://127.0.0.1:8080/missing',
        ...failedRequest,
      }],
    });
    helpers.allowExpectedHttpFailure(state, { status: 404, pathname: '/missing' });
    assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(state), /Unexpected browser\/runtime failures/u);
  }
});

test('non-Firefox unknown request failures remain fatal even after an expected response', () => {
  const state = diagnostics({
    browserName: 'chromium',
    httpErrors: [{
      status: 404,
      method: 'GET',
      url: 'http://127.0.0.1:8080/missing',
      requestIdentity: REQUEST_IDENTITY_A,
    }],
    failedRequests: [{
      method: 'GET',
      url: 'http://127.0.0.1:8080/missing',
      requestIdentity: REQUEST_IDENTITY_A,
      failure: '<unknown error>',
    }],
  });
  helpers.allowExpectedHttpFailure(state, { status: 404, pathname: '/missing' });
  assert.throws(() => helpers.assertNoUnexpectedRuntimeFailures(state), /Unexpected browser\/runtime failures/u);
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

test('separate identical expected HTTP allowances each consume one response', () => {
  const state = diagnostics({
    httpErrors: [
      { status: 403, url: 'http://127.0.0.1:8080/admin/media' },
      { status: 403, url: 'http://127.0.0.1:8080/admin/media' },
    ],
    consoleErrors: [
      { text: 'Failed to load resource: the server responded with a status of 403', url: 'http://127.0.0.1:8080/admin/media' },
      { text: 'Failed to load resource: the server responded with a status of 403', url: 'http://127.0.0.1:8080/admin/media' },
    ],
  });
  helpers.allowExpectedHttpFailure(state, { status: 403, pathname: '/admin/media' });
  helpers.allowExpectedHttpFailure(state, { status: 403, pathname: '/admin/media' });
  assert.doesNotThrow(() => helpers.assertNoUnexpectedRuntimeFailures(state));
});


test('WebKit evidence capture avoids Playwright screenshot CSP mutation', async () => {
  const acceptanceHelpers = await import('../tests/helpers.mjs');
  const fs = await import('node:fs');
  const path = await import('node:path');
  let screenshotCalls = 0;
  const marker = path.join(acceptanceHelpers.repoRoot, 'artifacts', 'acceptance', 'screenshots', 'unit-webkit-csp.json');
  fs.rmSync(marker, { force: true });

  const page = {
    context: () => ({ browser: () => ({ browserType: () => ({ name: () => 'webkit' }) }) }),
    screenshot: async () => { screenshotCalls += 1; },
  };

  await acceptanceHelpers.evidenceScreenshot(page, 'unit-webkit-csp');
  assert.equal(screenshotCalls, 0);
  const evidence = JSON.parse(fs.readFileSync(marker, 'utf8'));
  assert.equal(evidence.browser, 'webkit');
  assert.equal(evidence.screenshot, 'skipped-playwright-csp-mutation');
  fs.rmSync(marker, { force: true });
});
