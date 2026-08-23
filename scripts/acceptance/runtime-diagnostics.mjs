function diagnosticPath(rawUrl) {
  try {
    return new URL(rawUrl).pathname;
  } catch {
    return null;
  }
}

export function allowExpectedHttpFailure(diagnostics, { status, pathname, count = 1 }) {
  if (!Number.isInteger(status) || status < 500 || status > 599) {
    throw new TypeError('Expected HTTP failure status must be an integer from 500 through 599.');
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
  const allowances = (diagnostics.expectedHttpFailures ?? []).map((entry) => ({ ...entry, remaining: entry.count }));
  const consumed = [];
  const unexpectedServerErrors = [];

  for (const entry of diagnostics.serverErrors ?? []) {
    const pathname = diagnosticPath(entry.url);
    const allowance = allowances.find((candidate) => (
      candidate.status === entry.status && candidate.pathname === pathname && candidate.remaining > 0
    ));
    if (!allowance) {
      unexpectedServerErrors.push(entry);
      continue;
    }
    allowance.remaining -= 1;
    consumed.push({ status: entry.status, pathname });
  }

  const expectedConsoleBudget = consumed.map((entry) => ({ ...entry, remaining: 1 }));
  const unexpectedConsoleErrors = [];
  for (const entry of diagnostics.consoleErrors ?? []) {
    const match = entry.text?.match(/Failed to load resource: the server responded with a status of (\d{3})/u);
    const pathname = diagnosticPath(entry.url);
    const budget = match ? expectedConsoleBudget.find((candidate) => (
      candidate.status === Number.parseInt(match[1], 10)
        && candidate.pathname === pathname
        && candidate.remaining > 0
    )) : null;
    if (budget) {
      budget.remaining -= 1;
    } else {
      unexpectedConsoleErrors.push(entry);
    }
  }

  const missingExpectedHttpFailures = allowances.filter((entry) => entry.remaining > 0);
  const unexpected = Object.fromEntries(
    [
      ['consoleErrors', unexpectedConsoleErrors],
      ['pageErrors', diagnostics.pageErrors ?? []],
      ['failedRequests', (diagnostics.failedRequests ?? []).filter((entry) => entry.failure !== 'net::ERR_ABORTED')],
      ['serverErrors', unexpectedServerErrors],
      ['missingExpectedHttpFailures', missingExpectedHttpFailures],
    ].filter(([, entries]) => entries.length > 0),
  );

  if (Object.keys(unexpected).length > 0) {
    throw new Error('Unexpected browser/runtime failures: ' + JSON.stringify(unexpected));
  }
}

export async function attachRuntimeDiagnostics(testInfo, diagnostics, testedSha) {
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
