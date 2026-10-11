const NAVIGATION_CANCELLATION_FAILURES = new Set([
  'net::ERR_ABORTED',
  'NS_BINDING_ABORTED',
  'Load request cancelled',
]);

const EXPECTED_HTTP_DUPLICATE_FAILURES = new Set([
  '<unknown error>',
]);

// WebKit 2370 (Playwright 1.64) applies an internal inline style to every <select>
// and reports it as a style-src(-attr) violation under Platform's strict CSP. Platform
// ships no inline styles; Chromium and Firefox stay strict on the same pages. The
// allowance is limited to WebKit, this exact message, and a per-page budget equal to
// the number of <select> elements on that page, so extra violations still fail.
export const WEBKIT_SELECT_UA_STYLE_VIOLATION = "Refused to apply a stylesheet because its hash, its nonce, or 'unsafe-inline' does not appear in the style-src directive of the Content Security Policy.";

function webkitSelectViolationBudgets(diagnostics) {
  const budgets = new Map();
  if (diagnostics.browserName !== 'webkit') {
    return budgets;
  }
  for (const entry of diagnostics.consoleErrors ?? []) {
    if (entry.text === WEBKIT_SELECT_UA_STYLE_VIOLATION && Number.isInteger(entry.pageSelectCount)) {
      const key = diagnosticPath(entry.url);
      budgets.set(key, Math.max(budgets.get(key) ?? 0, entry.pageSelectCount));
    }
  }
  return budgets;
}

const REQUEST_IDENTITY_PATTERN = /^[0-9a-f]{64}$/u;

function diagnosticPath(rawUrl) {
  try {
    return new URL(rawUrl).pathname;
  } catch {
    return null;
  }
}

function requestIdentity(entry) {
  return typeof entry?.requestIdentity === 'string'
    && REQUEST_IDENTITY_PATTERN.test(entry.requestIdentity)
    ? entry.requestIdentity
    : null;
}

function matchesAllowance(allowance, entry) {
  return allowance.status === entry.status
    && allowance.pathname === diagnosticPath(entry.url);
}

function failedLoadStatus(entry) {
  const match = entry.text?.match(/Failed to load resource: the server responded with a status of (\d{3})/u);
  return match ? Number.parseInt(match[1], 10) : null;
}

export function allowExpectedHttpFailure(diagnostics, { status, pathname, count = 1 }) {
  if (!Number.isInteger(status) || status < 400 || status > 599) {
    throw new TypeError('Expected HTTP failure status must be an integer from 400 through 599.');
  }
  if (typeof pathname !== 'string' || !pathname.startsWith('/')) {
    throw new TypeError('Expected HTTP failure pathname must start with /.');
  }
  if (!Number.isInteger(count) || count < 1) {
    throw new TypeError('Expected HTTP failure count must be a positive integer.');
  }

  diagnostics.expectedHttpFailures ??= [];
  diagnostics.expectedHttpFailures.push({ status, pathname, count });
}

export function assertNoUnexpectedRuntimeFailures(diagnostics) {
  const allowances = (diagnostics.expectedHttpFailures ?? []).map((entry) => ({
    ...entry,
    responseRemaining: entry.count,
    consoleRemaining: entry.count,
    matchedResponses: 0,
    matchedResponseIdentities: [],
  }));
  const observedHttpErrors = Array.isArray(diagnostics.httpErrors)
    ? diagnostics.httpErrors
    : (diagnostics.serverErrors ?? []);
  const unexpectedHttpErrors = [];

  for (const entry of observedHttpErrors) {
    const pathname = diagnosticPath(entry.url);
    const matchingAllowance = allowances.find((candidate) => (
      matchesAllowance(candidate, entry) && candidate.responseRemaining > 0
    ));
    if (matchingAllowance) {
      matchingAllowance.responseRemaining -= 1;
      matchingAllowance.matchedResponses += 1;
      const identity = requestIdentity(entry);
      if (entry.method === 'GET' && identity !== null) {
        matchingAllowance.matchedResponseIdentities.push(identity);
      }
      continue;
    }

    const conflictsWithAllowance = allowances.some((candidate) => (
      candidate.status === entry.status || candidate.pathname === pathname
    ));
    if (entry.status >= 500 || matchingAllowance || conflictsWithAllowance) {
      unexpectedHttpErrors.push(entry);
    }
  }

  const unexpectedConsoleErrors = [];
  const selectBudgets = webkitSelectViolationBudgets(diagnostics);
  for (const entry of diagnostics.consoleErrors ?? []) {
    if (entry.text === WEBKIT_SELECT_UA_STYLE_VIOLATION) {
      const key = diagnosticPath(entry.url);
      const remaining = selectBudgets.get(key) ?? 0;
      if (remaining > 0) {
        selectBudgets.set(key, remaining - 1);
        continue;
      }
    }
    const status = failedLoadStatus(entry);
    const pathname = diagnosticPath(entry.url);
    const allowance = status === null ? null : allowances.find((candidate) => (
      candidate.status === status
        && candidate.pathname === pathname
        && candidate.matchedResponses > 0
        && candidate.consoleRemaining > 0
    ));

    if (allowance) {
      allowance.consoleRemaining -= 1;
    } else {
      unexpectedConsoleErrors.push(entry);
    }
  }

  const unexpectedFailedRequests = [];
  for (const entry of diagnostics.failedRequests ?? []) {
    if (NAVIGATION_CANCELLATION_FAILURES.has(entry.failure)) {
      continue;
    }

    let duplicate = null;
    const identity = requestIdentity(entry);
    if (
      diagnostics.browserName === 'firefox'
      && entry.method === 'GET'
      && EXPECTED_HTTP_DUPLICATE_FAILURES.has(entry.failure)
      && identity !== null
    ) {
      for (const allowance of allowances) {
        const matchIndex = allowance.matchedResponseIdentities.indexOf(identity);
        if (matchIndex >= 0) {
          duplicate = { allowance, matchIndex };
          break;
        }
      }
    }

    if (duplicate) {
      duplicate.allowance.matchedResponseIdentities.splice(duplicate.matchIndex, 1);
      continue;
    }

    unexpectedFailedRequests.push(entry);
  }

  const missingExpectedHttpFailures = allowances.filter((entry) => entry.responseRemaining > 0);
  const unexpected = Object.fromEntries(
    [
      ['consoleErrors', unexpectedConsoleErrors],
      ['pageErrors', diagnostics.pageErrors ?? []],
      ['failedRequests', unexpectedFailedRequests],
      ['httpErrors', unexpectedHttpErrors],
      ['missingExpectedHttpFailures', missingExpectedHttpFailures],
    ].filter(([, entries]) => entries.length > 0),
  );

  if (Object.keys(unexpected).length > 0) {
    throw new Error('Unexpected browser/runtime failures: ' + JSON.stringify(unexpected));
  }
}

export async function attachRuntimeDiagnostics(testInfo, diagnostics, testedSha) {
  const browserName = testInfo.project?.use?.browserName;
  diagnostics.browserName = typeof browserName === 'string' ? browserName : null;
  await Promise.allSettled(diagnostics.pendingChecks ?? []);

  await testInfo.attach('exact-tested-sha', {
    body: Buffer.from(`${testedSha}\n`, 'utf8'),
    contentType: 'text/plain',
  });
  await testInfo.attach('browser-diagnostics', {
    body: Buffer.from(JSON.stringify(diagnostics, null, 2), 'utf8'),
    contentType: 'application/json',
  });
  assertNoUnexpectedRuntimeFailures(diagnostics);
}
