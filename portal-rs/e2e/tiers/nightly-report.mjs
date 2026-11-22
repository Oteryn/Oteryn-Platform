import { playwrightCases } from './nightly-plan.mjs';

const htmlEntities = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };

export const escapeHtml = value => value.replace(/[&<>"']/g, character => htmlEntities[character]);

function failureReason(status, result) {
  if (status === 'passed') return 'Passed';
  if (status === 'skipped') return 'Case skipped';
  if (status === 'timedOut') return 'Test timed out';
  if (status === 'interrupted') return 'Test interrupted';
  if (status === 'notRun') return 'No result recorded';
  if (status === 'multiple') return 'More than one result recorded';

  // Reduce errors to fixed descriptions. Do not publish messages, call logs,
  // request URLs, expected/received values, or attachments.
  const messages = (result.errors ?? []).map(error => error.message ?? '');
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
  for (const { spec, test } of playwrightCases(suites)) {
    if (!/^[\w./-]+$/.test(spec.file) || !Number.isInteger(spec.line)) throw new Error('Invalid report source location');
    if (!/^[a-f0-9]{20}-[a-f0-9]{20}$/.test(spec.id)) throw new Error('Invalid Playwright case ID');
    if (!['expected', 'unexpected', 'flaky', 'skipped'].includes(test.status)) throw new Error('Invalid report test outcome');
    const [result] = test.results;
    const status = test.results.length === 1 ? result.status : result ? 'multiple' : 'notRun';
    if (!['passed', 'failed', 'timedOut', 'skipped', 'interrupted', 'multiple', 'notRun'].includes(status)) {
      throw new Error('Invalid Playwright result status');
    }
    const route = spec.file === 'specs/routes.spec.mjs' && routeTitles.has(spec.title) ? spec.title : '';
    cases.push({
      id: spec.id,
      route,
      source: `${spec.file}:${spec.line}`,
      status,
      reason: failureReason(status, result),
    });
  }
  return cases;
}

export function reportRow({ id, route, source, status, reason }) {
  const values = [id, route, source, status, reason];
  return `<tr>${values.map(value => `<td>${escapeHtml(value)}</td>`).join('')}</tr>`;
}
