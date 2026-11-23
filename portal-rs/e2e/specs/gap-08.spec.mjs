import crypto from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, password, pool, submit } from '../helpers.mjs';

function seedAccount(locale) {
  const account = {
    email: `gap08-${crypto.randomUUID()}@example.invalid`,
    password: password(),
    locale,
  };
  const seed = spawnSync(process.env.PORTAL_E2E_BINARY, [
    'seed', 'account', JSON.stringify(account),
  ], { encoding: 'utf8' });
  expect(seed.status, 'seed a verified Platform account').toBe(0);
  return { ...account, id: JSON.parse(seed.stdout).accountId };
}

async function grant(page, scope) {
  const verifier = crypto.randomBytes(32).toString('base64url');
  const origin = new URL(page.url()).origin;
  const callback = `${origin}/__oauth/callback`;
  const state = crypto.randomBytes(24).toString('hex');
  const query = new URLSearchParams({
    response_type: 'code',
    client_id: 'e2e-local',
    redirect_uri: callback,
    scope,
    state,
    code_challenge: crypto.createHash('sha256').update(verifier).digest('base64url'),
    code_challenge_method: 'S256',
  });
  expect((await page.goto(`/oauth/authorize?${query}`)).status()).toBe(200);
  await expect(page.locator('main p').first()).toContainText(scope);
  const form = page.locator('main form');
  const fields = await form.evaluate(node => Object.fromEntries(new FormData(node)));
  await csrfRejected(page, '/oauth/authorize', { ...fields, decision: 'approve' });
  await form.locator('button[value=approve]').click();
  await expect(page).toHaveURL(/\/__oauth\/callback\?/);
  const result = new URL(page.url()).searchParams;
  expect(result.get('state')).toBe(state);
  const token = await page.request.post('/oauth/token', {
    form: {
      grant_type: 'authorization_code',
      client_id: 'e2e-local',
      redirect_uri: callback,
      code: result.get('code'),
      code_verifier: verifier,
    },
  });
  expect(token.status()).toBe(200);
  const issued = await token.json();
  expect(issued.scope).toBe(scope);
  return issued;
}

async function openAccountDestination(page, target) {
  const accountButton = page.locator('header [data-account-link]');
  if (await accountButton.isVisible()) {
    await accountButton.click();
  } else {
    const summary = page.locator('header .om-fallback-account > summary');
    if (!await summary.isVisible()) {
      await page.locator('header .om-fallback-menu > summary').click();
    }
    await summary.click();
  }
  const link = page.locator(`a[href="${target}"]:visible`).first();
  await expect(link).toBeVisible();
  const [result] = await Promise.all([
    page.waitForResponse(response => new URL(response.url()).pathname === target),
    link.click(),
  ]);
  return result;
}

test('approved account grant reads the owner and leaves journal data unavailable', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const owner = seedAccount(lang);
  const other = seedAccount(lang);
  const db = pool();
  try {
    for (const [account, subject, read] of [
      [owner, 'Owner unread', false],
      [owner, 'Owner read', true],
      [other, 'Other account', false],
    ]) {
      await db.query(
        `INSERT INTO notifications (account_id, category, subject, body, read_at)
         VALUES ($1, 'security', $2, $2, CASE WHEN $3 THEN now() END)`,
        [account.id, subject, read],
      );
    }
  } finally {
    await db.end();
  }

  await login(page, lang, owner);
  expect((await page.goto(`/${lang}/account/privacy`)).status()).toBe(200);
  const dashboard = await openAccountDestination(page, `/${lang}/account/dashboard`);
  expect(dashboard.status()).toBe(503);
  expect(dashboard.headers()['cache-control']).toContain('no-store');
  await expect(page.locator('header#portal-header')).toHaveCount(1);
  await expect(page.getByRole('status')).toHaveText(lang === 'pl'
    ? 'Dane dziennika są niedostępne.' : 'Journal data is unavailable.');
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);

  const summaryPath = '/api/v1/account/summary';
  expect((await page.request.get(summaryPath)).status()).toBe(401);
  await page.route('**/__oauth/callback?**', route => route.fulfill({
    status: 200, contentType: 'text/plain', body: 'Local OAuth callback',
  }));
  const ticket = await grant(page, 'game:ticket');
  const read = await grant(page, 'account:read');
  const get = (path, access = read.access_token) => page.request.get(path, {
    headers: { authorization: `Bearer ${access}` },
  });
  const summary = await get(summaryPath);
  expect(summary.status()).toBe(200);
  expect(summary.headers()['cache-control']).toContain('no-store');
  expect(await summary.json()).toEqual({ account_id: owner.id, locale: lang, coins: null });

  const feed = await get('/api/v1/feed?limit=100');
  expect(feed.status()).toBe(200);
  const subjects = (await feed.json()).items.map(item => item.subject);
  expect(subjects).toEqual(expect.arrayContaining(['Owner unread', 'Owner read']));
  expect(subjects).not.toContain('Other account');
  const filtered = await get('/api/v1/feed?limit=1&unread_only=true');
  expect(filtered.status()).toBe(200);
  const [item] = (await filtered.json()).items;
  expect(item).toMatchObject({ subject: 'Owner unread', read: false });
  expect((await get(summaryPath, ticket.access_token)).status()).toBe(401);
  expect((await get(summaryPath, read.refresh_token)).status()).toBe(401);
  expect((await get('/api/v1/feed', 'malformed token')).status()).toBe(401);
  expect((await get(`${summaryPath}?account_id=${other.id}`)).status()).toBe(400);
  expect((await get('/api/v1/feed?limit=101')).status()).toBe(400);
});

test('browser optional consent records and revokes an owner decision', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  await login(page, lang, seedAccount(lang));
  expect((await page.goto(`/${lang}/account/privacy`)).status()).toBe(200);
  const target = `/${lang}/account/consents`;
  expect((await openAccountDestination(page, target)).status()).toBe(200);
  const form = 'main form:has(input[name=kind][value=play_session])';
  const decision = page.locator(form).locator('#play_session');
  await expect(decision).not.toBeChecked();
  await csrfRejected(page, target, { kind: 'play_session', version: 'invalid', granted: 'true' });
  await decision.check();
  await submit(page, form);
  await expect(decision).toBeChecked();
  await expect(page.locator(form).locator('time')).toHaveCount(1);
  await decision.uncheck();
  await submit(page, form);
  await expect(decision).not.toBeChecked();
  await expect(page.locator('main #personalised_ads')).not.toBeChecked();
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});
