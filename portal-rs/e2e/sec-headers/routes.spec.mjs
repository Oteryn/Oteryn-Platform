import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test, expect } from '@playwright/test';
import { seedPasswordRange, waitForWikiBudget } from '../helpers.mjs';
import { cookie, expectSession, seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { additionalRoutes } from './additional-routes.mjs';
import { publishProfileWatermark } from './ownership.mjs';
import { checkHeaders } from './policy.mjs';
import { projectPath, publicRegistrations, registrationFor, routeShape } from './registrations.mjs';
import { registeredRedirect } from './redirects.mjs';
import { challengeForm, getFromPortal, signIn, submitForm } from './session-actions.mjs';

const expectations = JSON.parse(readFileSync(new URL('../route-expectations.json', import.meta.url), 'utf8'))
  .filter(row => row.method === 'GET');
const contracts = new Map([...expectations, ...additionalRoutes].map(row => [routeShape(row.route), row]));
const ownerProfile = row => row.route.endsWith('/account/characters/{name}/profile');

test.use({ trace: 'off', screenshot: 'off', video: 'off' });

// Same known and missing references as specs/routes.spec.mjs. References that
// require an accepted Game producer retain the inventory's explicit 503.
function sample(route, locale) {
  if (route === '/static/{*asset}') return '/static/css/tokens.css';
  return route.replaceAll('{lang}', locale).replaceAll('{locale}', locale)
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
  if (path === '/static/css/tokens.css') return 'public, max-age=300, must-revalidate';
  return undefined;
}

function uncoveredGetPaths(registrations, locale, responseContracts) {
  return registrations
    .filter(row => row.get && projectPath(row.path, locale) && !responseContracts.has(routeShape(row.path)))
    .map(row => row.path);
}

function matrix(locale) {
  const { registrations, legacy } = publicRegistrations();
  const get = registrations.filter(row => row.get && projectPath(row.path, locale));
  expect(uncoveredGetPaths(registrations, locale, contracts),
    'composed public GET registrations without a security-header scenario').toEqual([]);

  const routes = get.map(({ path }) => {
    const contract = contracts.get(routeShape(path));
    const target = sample(contract.route, locale);
    expect(registrationFor(registrations, target)?.path, 'concrete GET exercises ' + path).toBe(path);
    return { ...contract, target };
  });
  const aliases = legacy.flatMap(([old, destination]) => {
    const target = sample(old, locale);
    if (routes.some(row => row.target === target)) return [];
    const matched = registrationFor(registrations, target);
    if (matched?.get) {
      const contract = contracts.get(routeShape(matched.path));
      expect(Boolean(contract), 'shadowing GET has a contract for ' + old).toBe(true);
      // The entity router claims this legacy article shape. The alias sample
      // names an unknown entity kind, unlike the real items projection probe.
      if (old === '/wiki/{slug}' && matched.path === '/wiki/{kind}') {
        return [{ ...contract, route: old, target, status: 404, reason: 'unknown Wiki entity kind' }];
      }
      return [{ ...contract, route: old, target }];
    }
    if (matched) {
      return [{ actor: 'player', route: old, target, status: 405, reason: 'method-specific registration shadows the legacy GET' }];
    }
    const parts = old.split('/');
    const parameters = new Map(parts.map((part, index) => [part, target.split('/')[index]]));
    const location = '/pl' + destination.replace(/\{[^}]+\}/g, parameter => parameters.get(parameter));
    return [{ actor: 'player', route: old, target, status: 301, location, reason: 'legacy fallback' }];
  });
  const targets = new Set([...routes, ...aliases].map(row => row.target));
  // Preserve the inventory's explicit refusals and any existing fixture paths
  // even when the active composition has no matching GET handler.
  const probes = expectations.filter(row => !targets.has(sample(row.route, locale)))
    .map(row => ({ ...row, target: sample(row.route, locale) }));
  return [...routes, ...aliases, ...probes];
}

async function checkRoute(request, route, origin, locale) {
  let target = new URL(route.target, origin);
  const visited = new Set();
  for (;;) {
    expect(visited.has(target.href), route.route + ' redirect cycle').toBe(false);
    expect(visited.size, route.route + ' redirect limit').toBeLessThan(10);
    visited.add(target.href);
    await waitForWikiBudget(target.href);
    const response = await getFromPortal(request, target.href, {
      maxRedirects: 0,
      headers: { accept: 'text/html', 'accept-language': locale },
    });
    try {
      const status = response.status();
      await checkHeaders(response, target.pathname + ' HTTP ' + status, {
        cache: cacheFor(target.pathname, status), check: expect.soft,
      });
      const [expectedStatus, expectedLocation] = route.location
        ? [route.status, route.location]
        : registeredRedirect(target, locale) || [route.status, undefined];
      expect.soft(status, target.pathname + ' ' + route.reason).toBe(expectedStatus);
      expect.soft(response.headers().location, target.pathname + ' Location').toBe(expectedLocation);
      if (route.location || !expectedLocation) return;
      target = new URL(expectedLocation, target);
      expect(target.origin, target.pathname + ' same-origin redirect').toBe(origin);
    } finally {
      await response.dispose();
    }
  }
}

test('every composed public GET and legacy alias has a scenario', ({}, info) => {
  const locale = info.project.metadata.locale;
  const scenarios = matrix(locale);
  expect(scenarios.every(row => ['player', 'staff'].includes(row.actor))).toBe(true);
  const withoutForgotPassword = new Map(contracts);
  withoutForgotPassword.delete('/forgot-password');
  expect(uncoveredGetPaths(publicRegistrations().registrations, locale, withoutForgotPassword))
    .toEqual(['/forgot-password']);
});

async function checkExistingAnnouncement(page) {
  const title = 'Security headers draft ' + randomUUID();
  expect((await page.goto('/admin/announcements/create')).status()).toBe(200);
  const created = await submitForm(page, 'main form', {
    publication_state: 'draft', severity: 'info', starts_at: new Date().toISOString(),
    title, body: 'Disposable security headers fixture.',
  });
  expect(created.status()).toBe(303);
  const [draft] = await query('SELECT id FROM announcements WHERE title=$1', [title]);
  expect(Boolean(draft), 'draft was stored by the staff form').toBe(true);
  const path = '/admin/announcements/' + draft.id + '/edit';
  expect(created.headers().location).toBe(path);
  await checkHeaders(created, 'announcement create redirect');
  const response = await page.goto(path);
  expect(response.status()).toBe(200);
  await checkHeaders(response, 'existing announcement editor');
  await expect(page.locator('main form [name=title]')).toHaveValue(title);
}

for (const actor of ['player', 'staff']) {
  test(actor + ' composed routes have security headers on every GET and redirect hop', async ({ page, baseURL }, info) => {
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
    const routes = matrix(locale).filter(entry => entry.actor === actor);
    if (actor === 'staff') await checkExistingAnnouncement(page);
    for (const row of routes.filter(row => !ownerProfile(row))) {
      await test.step(row.target, () => checkRoute(page.request, row, origin, locale));
    }
    if (actor === 'player') {
      await publishProfileWatermark();
      for (const row of routes.filter(ownerProfile)) {
        await test.step(row.target, () => checkRoute(page.request, row, origin, locale));
      }
    }
  });
}
