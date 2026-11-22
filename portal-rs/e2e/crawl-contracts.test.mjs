import assert from 'node:assert/strict';
import {test} from 'node:test';
import {coldView, emptyGameFixture, protectedDestination} from './crawl-contracts.mjs';

const url = pathname => new URL(pathname, 'https://localhost:18443');

test('the empty-game fallback is restricted to its fixture, actor, and exact URL', () => {
  const env = {PORTAL_E2E_CRAWL_FIXTURE: 'empty-game'};
  assert.equal(emptyGameFixture(env), true);
  const wiki = url('/en/wiki/creatures');
  assert.equal(coldView(wiki, 'anonymous', true).text, 'World data is currently unavailable.');
  for (const game of [{OTERYN_WORLD_CATALOG_FILE: 'pinned.json'}, {OTERYN_WORLD_CATALOG_URL: 'https://catalog.test/'}]) {
    assert.equal(coldView(wiki, 'anonymous', emptyGameFixture({...env, ...game})), undefined);
  }
  assert.equal(coldView(wiki, 'anonymous', emptyGameFixture({})), undefined);
  assert.equal(coldView(url('/en/wiki/calculators'), 'player', true), undefined);
  assert.equal(coldView(url('/en/wiki?unknown=1'), 'player', true), undefined);
  assert.equal(coldView(url('/en/account/bazaar'), 'anonymous', true), undefined);
  assert.equal(coldView(url('/en/account/bazaar'), 'player', true).selector, 'p');
});

test('protected links have an actor-specific final path and status', () => {
  const destination = (pathname, actor, emptyGame = false) =>
    protectedDestination(url(pathname), 'en', actor, emptyGame);
  assert.deepEqual(destination('/account', 'player'), {path: '/account', status: 200});
  assert.deepEqual(destination('/account', 'anonymous'), {path: '/en/login', status: 200});
  assert.deepEqual(destination('/pl/account/security', 'player'), {path: '/pl/password/change', status: 200});
  assert.deepEqual(destination('/admin', 'platform_admin'), {path: '/admin', status: 200});
  assert.deepEqual(destination('/admin', 'player'), {path: '/admin', status: 403});
  assert.deepEqual(destination('/en/account/bazaar', 'player', true), {path: '/en/account/bazaar', status: 503});
  assert.deepEqual(destination('/en/account/bazaar', 'player'), {path: '/en/account/bazaar', status: 200});
});
