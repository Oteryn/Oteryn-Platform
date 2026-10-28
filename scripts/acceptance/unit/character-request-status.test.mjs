import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import vm from 'node:vm';

const script = readFileSync(new URL('../../../public/js/character-request-status.js', import.meta.url), 'utf8');

async function run(outcomes, { pending = true, origin = 'https://oteryn.test', advanceClockBy = 0 } = {}) {
  const timers = [];
  let clock = 0;
  const reads = [];
  const navigations = [];
  const url = 'https://oteryn.test/account/characters/requests/operation?locale=pl';
  vm.runInNewContext(script, {
    URL,
    Date: { now: () => { const current = clock; clock += advanceClockBy; return current; } },
    AbortSignal: { timeout: (ms) => { assert.equal(ms, 5000); return { timeout: ms }; } },
    document: {
      querySelector: (selector) => selector.includes('data-outcome')
        ? (pending ? {} : null)
        : { href: url },
    },
    window: {
      location: { href: url, origin, replace: (target) => navigations.push(target) },
      setTimeout: (callback, delay) => { assert.equal(delay, 5000); timers.push(callback); },
    },
    fetch: async (target, options) => {
      reads.push({ target, options });
      const outcome = outcomes[Math.min(reads.length - 1, outcomes.length - 1)];
      if (outcome instanceof Error) throw outcome;
      return { ok: outcome !== 429, status: outcome === 429 ? 429 : 200, redirected: outcome === 'login', text: async () => outcome };
    },
    DOMParser: class {
      parseFromString(outcome) { return { querySelector: () => ({ dataset: { outcome } }) }; }
    },
  });
  while (timers.length) await timers.shift()();
  return { reads, navigations, url };
}

test('pending request rechecks the same localized operation and opens its committed result', async () => {
  const { reads, navigations, url } = await run(['pending', 'created']);
  assert.equal(reads.length, 2);
  assert.deepEqual(navigations, [url]);
  for (const read of reads) {
    assert.equal(read.target, url);
    assert.equal(read.options.method, undefined);
    assert.equal(read.options.credentials, 'same-origin');
    assert.equal(read.options.cache, 'no-store');
  }
});

test('rejected result also ends polling', async () => {
  const { reads, navigations, url } = await run(['rejected']);
  assert.equal(reads.length, 1);
  assert.deepEqual(navigations, [url]);
});

test('timeouts never create a character or invent an outcome and stop after five minutes', async () => {
  const { reads, navigations } = await run([new Error('offline')]);
  assert.equal(reads.length, 60);
  assert.deepEqual(navigations, []);
});

test('rate limiting and session redirects stop automatic requests', async () => {
  for (const response of [429, 'login']) {
    const { reads, navigations } = await run([response]);
    assert.equal(reads.length, 1);
    assert.deepEqual(navigations, []);
  }
});

test('terminal pages and a different origin do not start polling', async () => {
  assert.equal((await run(['pending'], { pending: false })).reads.length, 0);
  assert.equal((await run(['pending'], { origin: 'https://other.test' })).reads.length, 0);
});

test('slow reads consume the wall-clock budget instead of extending polling indefinitely', async () => {
  const { reads, navigations } = await run(['pending'], { advanceClockBy: 100000 });
  assert.equal(reads.length, 1);
  assert.deepEqual(navigations, []);
});
