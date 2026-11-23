import assert from 'node:assert/strict';
import { test } from 'node:test';
import { isGameUnavailable } from './game-unavailable.mjs';

function response(lang, heading, refusal, overrides = {}) {
  return {
    status: 503,
    headers: new Headers({
      'content-type': 'text/html; charset=utf-8',
      'cache-control': 'private, no-store',
      'x-oteryn-game-data': 'unavailable',
    }),
    body: Buffer.from(`<html lang="${lang}"><main id="main"><h1>${heading}</h1>${refusal}</main></html>`),
    ...overrides,
  };
}

const online = new URL('https://oteryn.com/en/online');
const unavailable = () => response('en', 'Online characters', '<p role="status">Temporarily unavailable.</p>');

test('matches the route-specific unavailable view in each locale', () => {
  assert.equal(isGameUnavailable(online, unavailable()), true);
  assert.equal(isGameUnavailable(new URL('https://oteryn.com/pl/wiki/creatures'), response(
    'pl', 'Dane świata są obecnie niedostępne.', '',
  )), true);
});

test('a different route, status, or locale remains a hard failure', () => {
  assert.equal(isGameUnavailable(new URL('https://oteryn.com/en/news'), unavailable()), false);
  assert.equal(isGameUnavailable(online, { ...unavailable(), status: 500 }), false);
  assert.equal(isGameUnavailable(online, { ...unavailable(), status: 404 }), false);
  assert.equal(isGameUnavailable(new URL('https://oteryn.com/pl/online'), unavailable()), false);
});

test('requires HTML, a no-store directive, and refusal markup in the main content', () => {
  for (const [header, value] of [
    ['content-type', 'text/plain'],
    ['cache-control', 'private, no-store="example"'],
    ['content-language', 'pl'],
  ]) {
    const result = unavailable();
    result.headers.set(header, value);
    assert.equal(isGameUnavailable(online, result), false);
  }
  assert.equal(isGameUnavailable(online, response('en', 'Service unavailable', '')), false);
  assert.equal(isGameUnavailable(online, response('en', 'Online characters', '')), false);
  assert.equal(isGameUnavailable(online, response(
    'en', 'Online characters', '<!-- <p role="status">Temporarily unavailable.</p> -->',
  )), false);
});

test('identical database-failure HTML is a hard smoke finding without the server signal', () => {
  const result = unavailable();
  assert.equal(isGameUnavailable(online, result), true);
  result.headers.delete('x-oteryn-game-data');
  assert.equal(isGameUnavailable(online, result), false);
});

test('a marker inside non-content markup exposed by cleanup remains a hard failure', () => {
  const marker = '<p role="status">Temporarily unavailable.</p>';
  for (const wrapper of [
    `<scr<!-- gap -->ipt>${marker}</script>`,
    `<!<!-- gap -->-- ${marker} -->`,
    `<tem<!-- gap -->plate>${marker}</template>`,
  ]) {
    assert.equal(isGameUnavailable(online, response('en', 'Online characters', wrapper)), false);
  }
});
