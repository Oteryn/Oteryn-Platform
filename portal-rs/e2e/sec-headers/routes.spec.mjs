import { readFileSync } from 'node:fs';
import { test, expect } from '@playwright/test';
import { seedPasswordRange, waitForWikiBudget } from '../helpers.mjs';
import { cookie, expectSession, seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { additionalRoutes } from './additional-routes.mjs';
import { publishProfileWatermark } from './ownership.mjs';
import { checkHeaders } from './policy.mjs';
import { challengeForm, signIn, submitForm } from './session-actions.mjs';

const inventory = JSON.parse(readFileSync(new URL('../inventory.json', import.meta.url), 'utf8')).routes;
const expected = JSON.parse(readFileSync(new URL('../route-expectations.json', import.meta.url), 'utf8'))
  .filter(row => row.method === 'GET');
const rows = [...expected, ...additionalRoutes];
const ownerProfile = row => row.route.endsWith('/account/characters/{name}/profile');
const key = ({ crate, route }) => crate + ' ' + route;
const redirectStatuses = [301, 302, 303, 307, 308];

// Same known and missing references as specs/routes.spec.mjs. References that
// require an accepted Game producer retain the inventory's explicit 503.
function sample(route, locale) {
  return route.replaceAll('{lang}', locale)
    .replaceAll('{kind}', route.includes('stats/rankings') ? 'yesterday' : route.includes('rankings') ? 'experience' : 'items')
    .replaceAll('{tab}', 'experience')
    .replaceAll('{slug}', route.startsWith('/admin/pages/') ? 'rules' : 'missing-e2e')
    .replace(/\{(?:floor|zoom|x|y)\}/g, '0')
    .replace(/\{(?:name|key|family|channel|filename|\*target|\*rest)\}/g, 'missing-e2e')
    .replace(/\{[^}]+\}/g, '019a541b-a000-7000-8000-000000000001');
}

function cacheFor(path, status) {
  if (status !== 200) return undefined;
  // The Hunts module owns its stylesheet and explicitly uses private, no-store.
  if (path === '/favicon.ico') return 'public, max-age=86400';
  return undefined;
}

async function checkRoute(request, route, origin, locale) {
  let target = new URL(sample(route.route, locale), origin);
  const visited = new Set();
  for (;;) {
    expect(visited.has(target.href), key(route) + ' redirect cycle').toBe(false);
    expect(visited.size, key(route) + ' redirect limit').toBeLessThan(10);
    visited.add(target.href);
    await waitForWikiBudget(target.href);
    const response = await request.get(target.href, {
      maxRedirects: 0,
      headers: { accept: 'text/html', 'accept-language': locale },
    });
    try {
      const status = response.status();
      await checkHeaders(response, key(route) + ' HTTP ' + status, {
        cache: cacheFor(target.pathname, status), check: expect.soft,
      });
      if (!redirectStatuses.includes(status)) {
        expect.soft(status, key(route) + ' ' + route.reason).toBe(route.status);
        return;
      }
      const location = response.headers().location;
      expect(Boolean(location), key(route) + ' redirect location').toBe(true);
      target = new URL(location, target);
      expect(target.origin, key(route) + ' same-origin redirect').toBe(origin);
    } finally {
      await response.dispose();
    }
  }
}

test('GET expectations account for every route inventory entry', () => {
  expect(expected.map(key).sort()).toEqual(inventory.filter(entry => entry.methods.includes('GET')).map(key).sort());
  expect(new Set(rows.map(key)).size, 'unique GET matrix rows').toBe(rows.length);
  expect(rows.every(row => ['player', 'staff'].includes(row.actor))).toBe(true);
});

for (const actor of ['player', 'staff']) {
  test(actor + ' inventory has security headers on every GET and redirect hop', async ({ page, baseURL }, info) => {
    test.setTimeout(180_000);
    const locale = info.project.metadata.locale;
    const account = await seedAccount(locale);
    const root = '/' + locale + '/';
    let mfa;
    if (actor === 'staff') {
      const assigned = await query(`INSERT INTO account_roles(account_id, role_id)
        SELECT $1, id FROM roles WHERE key = 'platform_admin' AND built_in RETURNING role_id`, [account.id]);
      expect(assigned.length, 'built-in Platform Admin role assigned').toBe(1);
      mfa = await seedEntity('account-mfa', { accountId: account.id, password: account.password });
    } else {
      // Initialize the local registration corpus and policy with the shared helper.
      await seedPasswordRange(account.password);
    }
    expect((await page.goto(root + 'login')).status(), actor + ' login page').toBe(200);
    let response = await signIn(page, account);
    if (mfa) {
      expect(response.status(), 'staff password step').toBe(302);
      expect(response.headers().location, 'staff password step requires MFA').toBe(root + 'mfa/challenge');
      expect((await page.goto(root + 'mfa/challenge')).status(), 'staff challenge page').toBe(200);
      response = await submitForm(page, challengeForm, { code: mfa.recoveryCodes[0] });
    }
    expect(response.status(), actor + ' sign-in redirect').toBe(302);
    expect(response.headers().location, actor + ' sign-in destination').toBe(root);
    await expectSession(await cookie(page.context()), account.id);

    const origin = new URL(baseURL).origin;
    const routes = rows.filter(entry => entry.actor === actor);
    for (const row of routes.filter(row => !ownerProfile(row))) {
      await test.step(key(row), () => checkRoute(page.request, row, origin, locale));
    }
    if (actor === 'player') {
      await publishProfileWatermark();
      for (const row of routes.filter(ownerProfile)) {
        await test.step(key(row), () => checkRoute(page.request, row, origin, locale));
      }
    }
  });
}
