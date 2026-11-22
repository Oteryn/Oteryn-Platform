import assert from 'node:assert/strict';
import { test } from 'node:test';
import { projectionPaths, unavailableView } from './contracts.mjs';

const url = path => new URL(path, 'https://localhost');

test('Game login history requires an explicit localized authority refusal for signed-in actors', () => {
  for (const actor of ['player', 'platform_admin']) {
    assert.ok(projectionPaths(actor).includes('account/game-logins'));
    assert.deepEqual(unavailableView(url('/en/account/game-logins'), actor, 'pl'), {
      selector: 'h1',
      text: 'Temporarily unavailable',
      reason: 'No accepted Game login ownership and baseline authority',
      filters: false,
    });
    assert.equal(unavailableView(url('/pl/account/game-logins'), actor, 'en').text, 'Usługa chwilowo niedostępna');
  }
  assert.equal(unavailableView(url('/en/account/game-logins'), 'anonymous', 'en'), undefined);
  assert.equal(unavailableView(url('/en/account/game-logins/other'), 'player', 'en'), undefined);
});

test('an unavailable route does not excuse errors on another route or request variant', () => {
  assert.ok(unavailableView(url('/en/atlas'), 'anonymous', 'en'));
  assert.equal(unavailableView(url('/en/atlas/typo'), 'anonymous', 'en'), undefined);
  assert.equal(unavailableView(url('/en/atlas?invalid=1'), 'anonymous', 'en'), undefined);
  assert.equal(unavailableView(url('/en/account/journal/reports?other=1'), 'player', 'en'), undefined);
});
