const NAVIGATION_CANCELLATION_FAILURES = new Set([
  'net::ERR_ABORTED',
  'NS_BINDING_ABORTED',
  'Load request cancelled',
]);

const EXPECTED_HTTP_DUPLICATE_FAILURES = new Set([
  '<unknown error>',
]);

function diagnosticPath(rawUrl) {
  try {
    return new URL(rawUrl).pathname;
  } catch {
    return null;
  }
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
    matchedResponseUrls: [],
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
      matchingAllowance.matchedResponseUrls.push(entry.url);
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
  for (const entry of diagnostics.consoleErrors ?? []) {
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
    if (
      diagnostics.browserName === 'firefox'
      && entry.method === 'GET'
      && EXPECTED_HTTP_DUPLICATE_FAILURES.has(entry.failure)
    ) {
      for (const allowance of allowances) {
        const matchIndex = allowance.matchedResponseUrls.indexOf(entry.url);
        if (matchIndex >= 0) {
          duplicate = { allowance, matchIndex };
          break;
        }
      }
    }

    if (duplicate) {
      duplicate.allowance.matchedResponseUrls.splice(duplicate.matchIndex, 1);
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
