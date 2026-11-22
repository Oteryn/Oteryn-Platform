import {createHash, randomBytes, randomUUID} from 'node:crypto';
import {uuid7} from '../game-sim-helpers.mjs';
import {test, expect} from '../fixtures.mjs';
import {login, register, submit} from '../helpers.mjs';
import {isolatedAdmin, query} from '../staff-rbac-helpers.mjs';

test('staff submits filters on each analytics page', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    for (const section of ['', '/activity', '/creatures', '/health', '/levels']) {
      await test.step(section || 'overview', async () => {
        const path = `/admin/analytics${section}`;
        expect((await page.goto(path)).status()).toBe(200);
        const form = page.locator('main form[method=get]:has([name=world])');
        const world = uuid7();
        await form.locator('[name=world]').fill(world);
        const [response] = await Promise.all([
          page.waitForResponse(r => r.request().isNavigationRequest() && new URL(r.url()).pathname === path),
          form.getByRole('button').click(),
        ]);
        expect(response.status()).toBe(200);
        expect(response.headers()['cache-control']).toBe('no-store, private');
        expect(new URL(response.url()).searchParams.get('world')).toBe(world);
        await expect(page.locator('main [data-state=unavailable]')).toBeVisible();
      });
    }
  } finally {
    await context.close();
  }
});

test('staff issues one gift code through the browser form', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    const path = `/${info.project.metadata.locale}/admin/store/codes`;
    expect((await page.goto(path)).status()).toBe(200);
    const note = `Ledger issuance ${randomUUID()}`;
    const now = Math.floor(Date.now() / 1000);
    const response = page.waitForResponse(r => r.request().isNavigationRequest() &&
      r.request().method() === 'POST' && new URL(r.url()).pathname === path);
    await submit(page, 'main form', {
      amount: '37', count: '1', valid_from: String(now - 60), valid_until: String(now + 3600), note,
    });
    expect((await response).status()).toBe(200);
    const rows = await query(`
      SELECT count(*)::int AS count FROM gift_codes c
      JOIN code_batches b ON b.id=c.batch_id WHERE b.note=$1
    `, [note]);
    expect(rows).toEqual([{count: 1}]);
  } finally {
    await context.close();
  }
});

test('staff approval requires an existing control preview', async ({browser}, info) => {
  test.setTimeout(300_000);
  const {context, page} = await isolatedAdmin(browser, info);
  try {
    expect((await page.goto('/admin/control/step-up')).status()).toBe(200);
    const csrf = await page.locator('main form [name=_csrf]').inputValue();
    const id = uuid7();
    const denied = await page.request.post(`/admin/control/approvals/${id}`, {
      form: {_csrf: csrf},
      headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
    });
    expect(denied.status()).toBe(403);
    expect(await query('SELECT id FROM admin_control_approvals WHERE preview_id=$1', [id])).toEqual([]);
  } finally {
    await context.close();
  }
});

test('ledger keeps bearer credentials separate from an unrelated staff session', async ({page, request, browser}, info) => {
  test.setTimeout(300_000);
  const lang = info.project.metadata.locale;
  await login(page, lang, await register(page, lang));
  const verifier = randomBytes(32).toString('base64url');
  const state = randomUUID();
  const origin = new URL(page.url()).origin;
  const redirect = `${origin}/__oauth/callback`;
  const params = new URLSearchParams({
    response_type: 'code', client_id: 'e2e-local', redirect_uri: redirect,
    scope: 'game:ticket', state, code_challenge: createHash('sha256').update(verifier).digest('base64url'),
    code_challenge_method: 'S256',
  });
  expect((await page.goto(`/oauth/authorize?${params}`)).status()).toBe(200);
  const csrf = await page.locator('main form [name=_csrf]').inputValue();
  const consent = await page.request.post('/oauth/authorize', {
    form: {...Object.fromEntries(params), _csrf: csrf, decision: 'approve'}, headers: {origin}, maxRedirects: 0,
  });
  expect(consent.status()).toBe(302);
  const callback = new URL(consent.headers().location);
  expect(callback.origin + callback.pathname).toBe(redirect);
  expect(callback.searchParams.get('state')).toBe(state);
  const code = callback.searchParams.get('code');
  expect(Boolean(code)).toBe(true);
  const token = await request.post('/oauth/token', {
    form: {grant_type: 'authorization_code', client_id: 'e2e-local', redirect_uri: redirect, code, code_verifier: verifier},
  });
  expect(token.status()).toBe(200);
  const {access_token} = await token.json();
  expect(Boolean(access_token)).toBe(true);

  const staff = await isolatedAdmin(browser, info);
  try {
    expect((await staff.page.request.get('/admin/roles', {maxRedirects: 0})).status()).toBe(200);
    const headers = {authorization: `Bearer ${access_token}`};
    for (const client of [request, staff.page.request]) {
      const response = await client.get('/api/v1/worlds', {headers, maxRedirects: 0});
      expect(response.status()).toBe(200);
    }
  } finally {
    await staff.context.close();
  }
});

// Unhit rows requiring a connected Admin Agent advertising each operation.
// The console only renders POST controls after checking live agent ownership,
// freshness and capability. A configured identity alone cannot enqueue work.
// Dashboard: F-admin-2051149960876, R-admin-260508168334342.
// Database: F-admin-38702896297808, R-admin-1919642460343.
// Worlds: F-admin-138082345336189, R-admin-136010976080442.
// Nodes: F-admin-253054411857215, R-admin-122795670899350.
// Player search: F-admin-17456964450888. Its POST form needs player.search.

// F-world-89263600590152, F-world-86048466149919, R-world-71870839689412:
// inspection and mutation controls require a selectable WorldStore version.
// Local catalogue qualification delivers an inactive candidate with no
// world_versions entry, so there is no version to inspect, edit or publish.
