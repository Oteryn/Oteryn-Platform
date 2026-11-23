import { randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { test, expect } from '@playwright/test';
import { seedPasswordRange, waitForWikiBudget } from '../helpers.mjs';
import { cookie, expectSession, seedAccount, seedEntity } from '../security/auth-fixtures.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { additionalRoutes } from './additional-routes.mjs';
import { publishOwnedCharacter, publishProfileWatermark } from './ownership.mjs';
import { checkHeaders, sameOriginForm, shellPolicy, stylesForm, surface } from './policy.mjs';
import { routePolicy } from './response-contracts.mjs';
import { matchedRouteHeader, ordinaryLogin, projectPath, publicRegistrations, publicRoute, routeShape } from './registrations.mjs';
import { registeredRedirect } from './redirects.mjs';
import { challengeForm, getFromPortal, signIn, submitForm } from './session-actions.mjs';

const expectations = JSON.parse(readFileSync(new URL('../route-expectations.json', import.meta.url), 'utf8'))
  .filter(row => row.method === 'GET')
  .map(row => row.actor === 'game_master_senior' ? { ...row, actor: 'gm' } : row);
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
    .replace(/\{[^{}]+\}/g, '019a541b-a000-7000-8000-000000000001');
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
  const report = publicRegistrations();
  expect(report.mfaLoginRewrite, 'main sweep uses the MFA composition').toBe(true);
  expect(report.registrations.filter(row => row.get && ordinaryLogin(row.path)).map(row => row.path))
    .toEqual(['/en/login', '/login', '/pl/login']);
  const { registrations, legacy } = report;
  const get = registrations.filter(row => row.get && projectPath(row.path, locale));
  expect(uncoveredGetPaths(registrations, locale, contracts),
    'composed public GET registrations without a security-header scenario').toEqual([]);

  const routes = get.map(({ path }) => {
    const contract = contracts.get(routeShape(path));
    const target = sample(contract.route, locale);
    const expectedPattern = ordinaryLogin(path) ? path.replace(/\/login$/, '/mfa/login') : path;
    expect(publicRoute(report, target)?.path, 'concrete GET dispatch for ' + path).toBe(expectedPattern);
    return { ...contract, target };
  });
  const aliases = legacy.flatMap(([old, destination]) => {
    const target = sample(old, locale);
    if (routes.some(row => row.target === target)) return [];
    const matched = publicRoute(report, target);
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
    const location = '/pl' + destination.replace(/\{[^{}]+\}/g, parameter => parameters.get(parameter));
    return [{ actor: 'player', route: old, target, status: 301, location, reason: 'legacy fallback' }];
  });
  const targets = new Set([...routes, ...aliases].map(row => row.target));
  // Preserve the inventory's explicit refusals and any existing fixture paths
  // even when the active composition has no matching GET handler.
  const probes = expectations.filter(row => !targets.has(sample(row.route, locale)))
    .map(row => ({ ...row, target: sample(row.route, locale) }));
  return { report, routes: [...routes, ...aliases, ...probes] };
}

// These legacy destinations are intentionally unmounted with the local
// projection configuration. The exact fallback refusal remains part of the alias.
const absentAliasDestination = /^\/pl\/(?:start|download(?:\/missing-e2e)?)$/;

function destinationContract(report, path) {
  const matched = publicRoute(report, path);
  if (!matched && absentAliasDestination.test(path)) return { route: path, status: 404, reason: 'unmounted legacy destination', legacyFallback: true };
  const contract = matched?.get && contracts.get(routeShape(matched.path));
  expect(Boolean(contract), 'response contract for redirect destination ' + path).toBe(true);
  return contract;
}

async function checkRoute(request, route, origin, locale, report) {
  let target = new URL(route.target, origin);
  const visited = new Set();
  let contract = route;
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
      const matched = publicRoute(report, target.pathname);
      // Axum deliberately omits MatchedPath on the private nest_service tail.
      // The static scenario still requires the composed ServeDir registration.
      const reportedPattern = matched?.path === '/static/{*__private__axum_nest_tail_param}' ? undefined : matched?.path;
      expect.soft(response.headers()[matchedRouteHeader], target.pathname + ' selected Axum registration')
        .toBe(reportedPattern);
      const [expectedStatus, expectedLocation] = contract.location
        ? [contract.status, contract.location]
        : registeredRedirect(target, locale) || [contract.status, undefined];
      await checkHeaders(response, target.pathname + ' HTTP ' + status, {
        ...routePolicy(contract, expectedStatus, matched?.path),
        cache: cacheFor(target.pathname, expectedStatus), check: expect.soft,
      });
      expect.soft(status, target.pathname + ' ' + contract.reason).toBe(expectedStatus);
      expect.soft(response.headers().location, target.pathname + ' Location').toBe(expectedLocation);
      if (!expectedLocation) return;
      target = new URL(expectedLocation, target);
      expect(target.origin, target.pathname + ' same-origin redirect').toBe(origin);
      contract = destinationContract(report, target.pathname);
    } finally {
      await response.dispose();
    }
  }
}

test('every composed public GET and legacy alias has a scenario', ({}, info) => {
  const locale = info.project.metadata.locale;
  const { routes: scenarios } = matrix(locale);
  expect(scenarios.every(row => ['player', 'staff', 'gm'].includes(row.actor))).toBe(true);
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
  await checkHeaders(created, 'announcement create redirect', { csp: sameOriginForm });
  const response = await page.goto(path);
  expect(response.status()).toBe(200);
  await checkHeaders(response, 'existing announcement editor', { csp: shellPolicy(sameOriginForm) });
  await expect(page.locator('main form [name=title]')).toHaveValue(title);
  return { path, title };
}

async function checkReferralsMethod(request, root) {
  for (const accept of ['application/json', 'text/html']) {
    const response = await request.fetch(root + 'account/referrals', {
      method: 'OPTIONS', headers: { accept }, maxRedirects: 0,
    }).catch(() => { throw new Error('Portal OPTIONS failed for referrals'); });
    expect(response.status()).toBe(405);
    expect(response.headers().allow).toBe('GET,HEAD,POST');
    expect(response.headers()[matchedRouteHeader]).toBe('/{lang}/account/referrals');
    await checkHeaders(response, 'referrals OPTIONS with ' + accept, { csp: stylesForm, referrer: 'same-origin' });
  }
}

async function checkExistingResources(page, browser, info, baseURL, draft) {
  const locale = info.project.metadata.locale;
  const viewerContext = await browser.newContext({
    baseURL, locale, viewport: info.project.use.viewport,
  });
  try {
    const viewer = await viewerContext.newPage();
    const account = await seedAccount(locale);
    expect((await viewer.goto('/' + locale + '/login')).status()).toBe(200);
    const mfa = await seedEntity('account-mfa', { accountId: account.id, password: account.password });
    const passwordOnly = await signIn(viewer, account);
    const challenge = '/' + locale + '/mfa/challenge';
    expect(passwordOnly.status()).toBe(302);
    expect(passwordOnly.headers().location).toBe(challenge);
    expect((await viewer.goto(challenge)).status()).toBe(200);
    const completed = await submitForm(viewer, challengeForm, { code: mfa.recoveryCodes[0] });
    expect(completed.status()).toBe(302);
    expect(completed.headers().location).toBe('/' + locale + '/');
    await expectSession(await cookie(viewerContext), account.id);
    const response = await viewer.goto(draft.path);
    expect(response.status(), 'player cannot read the existing staff draft').toBe(403);
    await checkHeaders(response, 'existing staff draft denied', { csp: shellPolicy(sameOriginForm) });
    await expect(viewer.locator('main')).not.toContainText(draft.title);

    const [{ id: world }] = await query('SELECT id FROM world_status WHERE name=$1', ['Local Bazaar World']);
    const name = await publishOwnedCharacter(account.id, world);
    const path = '/' + locale + '/account/characters/' + encodeURIComponent(name) + '/profile';
    const owned = await viewer.goto(path);
    expect(owned.status(), 'character owner can read the existing profile editor').toBe(200);
    await checkHeaders(owned, 'owned character profile editor', { csp: surface });
    await expect(viewer.locator('main form [name=hidden]')).toBeVisible();

    const other = await seedAccount(locale);
    // The staff browser must also exercise this endpoint as a regular player.
    await page.context().clearCookies();
    expect((await page.goto('/' + locale + '/login')).status()).toBe(200);
    expect((await signIn(page, other)).status()).toBe(302);
    await expectSession(await cookie(page.context()), other.id);
    const denied = await page.goto(path);
    expect(denied.status(), 'another player cannot read the existing profile editor').toBe(404);
    await checkHeaders(denied, 'existing character editor denied', { csp: surface });
    await expect(page.locator('main form [name=hidden]')).toHaveCount(0);
  } finally {
    await viewerContext.close();
  }
}

for (const actor of ['player', 'staff', 'gm']) {
  test(actor + ' composed routes have security headers on every GET and redirect hop', async ({ page, baseURL, browser }, info) => {
    test.setTimeout(180_000);
    const locale = info.project.metadata.locale;
    const account = await seedAccount(locale);
    const root = '/' + locale + '/';
    let mfa;
    if (actor !== 'player') {
      const assigned = await query(`INSERT INTO account_roles(account_id, role_id)
        SELECT $1, id FROM roles WHERE key = $2 AND built_in RETURNING role_id`, [account.id, actor === 'gm' ? 'game_viewer' : 'platform_admin']);
      expect(assigned.length, 'the actor received the exact built-in role').toBe(1);
      mfa = await seedEntity('account-mfa', { accountId: account.id, password: account.password });
    } else {
      // Initialize the local registration corpus and policy with the shared helper.
      await seedPasswordRange();
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
    const { report, routes: allRoutes } = matrix(locale);
    const routes = allRoutes.filter(entry => entry.actor === actor);
    const draft = actor === 'staff' ? await checkExistingAnnouncement(page) : null;
    for (const row of routes.filter(row => !ownerProfile(row))) {
      await test.step(row.target, () => checkRoute(page.request, row, origin, locale, report));
    }
    if (draft) {
      await checkExistingResources(page, browser, info, baseURL, draft);
    } else if (actor === 'player') {
      await checkReferralsMethod(page.request, root);
      await publishProfileWatermark();
      for (const row of routes.filter(ownerProfile)) {
        await test.step(row.target, () => checkRoute(page.request, row, origin, locale, report));
      }
    }
  });
}
