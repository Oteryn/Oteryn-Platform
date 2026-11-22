const htmlEntities = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };

export const escapeHtml = value => value.replace(/[&<>"']/g, character => htmlEntities[character]);

function failureReason(test) {
  if (test.status === 'expected') return 'Passed';
  if (test.status === 'skipped') return 'Case skipped';
  const failed = test.results.filter(result => result.status !== 'passed');
  if (failed.some(result => result.status === 'timedOut')) return 'Test timed out';
  if (failed.some(result => result.status === 'interrupted')) return 'Test interrupted';

  // Reduce errors to fixed descriptions. Do not publish messages, call logs,
  // request URLs, expected/received values, or attachments.
  const messages = failed.flatMap(result => result.errors ?? []).map(error => error.message ?? '');
  for (const message of messages) {
    const network = /\bnet::(ERR_ABORTED|ERR_CONNECTION_REFUSED|ERR_CONNECTION_RESET|ERR_FAILED|ERR_TIMED_OUT)\b/.exec(message);
    if (network) return `Browser network error (${network[1]})`;
    if (/Timeout \d+ms exceeded/.test(message)) return 'Browser operation timed out';
    if (/expect\([^\n]*\)\.(?:not\.)?to[A-Z][A-Za-z]+/.test(message)) return 'Test assertion failed';
    if (/page\.goto:/.test(message)) return 'Browser navigation failed';
  }
  return 'Test execution failed';
}

export function reportCases(suites, routeTitles) {
  const cases = [];
  function visit(suite) {
    for (const spec of suite.specs ?? []) {
      for (const test of spec.tests) {
        if (!/^[\w./-]+$/.test(spec.file) || !Number.isInteger(spec.line)) throw new Error('Invalid report source location');
        if (!/^[a-f0-9]{20}-[a-f0-9]{20}$/.test(spec.id)) throw new Error('Invalid Playwright case ID');
        if (!['expected', 'unexpected', 'flaky', 'skipped'].includes(test.status)) throw new Error('Invalid report test outcome');
        const route = spec.file === 'specs/routes.spec.mjs' && routeTitles.has(spec.title) ? spec.title : '';
        cases.push({
          id: spec.id,
          route,
          source: `${spec.file}:${spec.line}`,
          status: test.status,
          reason: failureReason(test),
        });
      }
    }
    for (const child of suite.suites ?? []) visit(child);
  }
  suites.forEach(visit);
  return cases;
}

export function reportRow({ id, route, source, status, reason }) {
  const values = [id, route, source, status, reason];
  return `<tr>${values.map(value => `<td>${escapeHtml(value)}</td>`).join('')}</tr>`;
}
