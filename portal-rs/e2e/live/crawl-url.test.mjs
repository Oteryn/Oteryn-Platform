import assert from 'node:assert/strict';
import { test } from 'node:test';
import { diagnosticURL, publicURL, redactURLs } from './crawl-url.mjs';

const base = 'https://portal.example';
const origins = new Set([base]);

test('rejects credential links and encoded equivalents before fetch', () => {
  for (const path of [
    '/en/reset-password/sample-secret', '/pl/password-reset/sample-secret',
    '/en/email-change/revert/sample-secret', '/%65mail-change%2Fconfirm%2Fsample-secret',
    '/en/%2565mail-change/sample-secret', '/en/news?refresh_token=sample-secret',
  ]) {
    assert.equal(publicURL(new URL(path, base), origins), false);
  }
  assert.equal(publicURL(new URL('/en/news?page=2', base), origins), true);
});

test('diagnostics omit credential values and preserve route context', () => {
  assert.equal(diagnosticURL(`${base}/en/email-change/confirm/sample-secret?token=sample-secret`),
    `${base}/en/email-change/[redacted]`);
  const evidence = redactURLs(`Location=/en/reset-password/sample-secret Referrer=${base}/en/news?token=sample-secret`);
  assert(!evidence.includes('sample-secret'));
  assert(evidence.includes('/en/reset-password/[redacted]'));
  assert(evidence.includes(`${base}/en/news`));
  assert.equal(redactURLs('Location=?action=sample-secret&%74oken=sample-secret&page=2'),
    'Location=?action=[redacted]&%74oken=[redacted]&page=2');
});
