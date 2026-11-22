import assert from 'node:assert/strict';
import { test } from 'node:test';
import { reportCases, reportRow } from './nightly-report.mjs';

test('generated route cases retain distinct IDs, safe labels and fixed failure reasons', () => {
  const route = 'route example /{lang}/<entry>';
  const privateValue = 'PRIVATE_TEST_VALUE';
  const specs = [
    { id: `${'a'.repeat(20)}-${'b'.repeat(20)}`, title: route },
    { id: `${'a'.repeat(20)}-${'c'.repeat(20)}`, title: privateValue },
  ].map(spec => ({
    ...spec,
    file: 'specs/routes.spec.mjs',
    line: 13,
    tests: [{
      status: 'unexpected',
      results: [{
        status: 'failed',
        errors: [{ message: `page.goto: net::ERR_ABORTED at https://localhost/puzzles?token=${privateValue}` }],
        attachments: [{ body: privateValue }],
      }],
    }],
  }));
  const cases = reportCases([{ suites: [{ specs }] }], new Set([route]));
  assert.equal(new Set(cases.map(entry => entry.id)).size, 2);
  assert.deepEqual(cases.map(entry => entry.route), [route, '']);
  assert.deepEqual(cases.map(entry => entry.reason), Array(2).fill('Browser network error (ERR_ABORTED)'));
  const html = cases.map(reportRow).join('\n');
  assert.match(html, /&lt;entry&gt;/);
  assert.ok(!html.includes(privateValue));
  assert.ok(!html.includes('https://localhost'));
});
