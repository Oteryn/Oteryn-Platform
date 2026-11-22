import { spawnSync } from 'node:child_process';
import path from 'node:path';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, login, password, pool } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';
import { paidTopup, uuid7 } from '../staff-commerce-helpers.mjs';

test('season pass spends sandbox coins once and keeps Game claims closed', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const pass = `/${lang}/pass`;
  const season = uuid7();
  const reward = uuid7();
  const account = { email: `pass-${uuid7()}@example.invalid`, password: password() };
  const buy = `${pass}/${season}/buy`;
  const binary = process.env.PORTAL_E2E_BINARY || path.resolve(
    process.env.CARGO_TARGET_DIR || '../target', 'debug/oteryn-portal',
  );
  const db = pool();
  try {
    const seed = spawnSync(binary, ['seed', 'account', JSON.stringify({
      ...account, locale: lang,
    })], {
      env: { ...process.env, OTERYN_ENV: 'testing', OTERYN_DATABASE_URL: process.env.PORTAL_E2E_DATABASE_URL || process.env.OTERYN_DATABASE_URL },
      encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'],
    });
    expect(seed.status, 'seed CLI creates the isolated player').toBe(0);
    const seeded = JSON.parse(seed.stdout);
    expect(seeded.status).toBe('active');
    await db.query(`INSERT INTO pass_seasons(id,starts_at,ends_at,price)
      VALUES ($1,statement_timestamp()-interval '1 day',statement_timestamp()+interval '41 days',100)`, [season]);
    await db.query(`INSERT INTO pass_challenges(id,season_id,source,metric,target,xp)
      VALUES ($1,$2,'journal','synthetic.challenge',1,1)`, [uuid7(), season]);
    await db.query(`INSERT INTO pass_rewards(id,season_id,track,xp,grant_key,grant_revision)
      VALUES ($1,$2,'free',1,'synthetic.reward','1')`, [reward, season]);

    const anonymous = await page.request.get(pass, { maxRedirects: 0 });
    expect(anonymous.status()).toBe(303);
    expect(anonymous.headers().location).toBe(`/${lang}/login`);
    await login(page, lang, account);
    const sessionTimestamps = () => query(`SELECT last_seen_at::text, updated_at::text,
      idle_expires_at::text, absolute_expires_at::text FROM web_sessions WHERE account_id=$1`, [seeded.accountId]);
    const before = await sessionTimestamps();
    expect(before).toHaveLength(1);
    const overview = await page.goto(pass);
    expect(overview.status()).toBe(200);
    expect(overview.headers()['cache-control']).toBe('private, no-store');
    await expect(page.locator('html')).toHaveAttribute('lang', lang);
    await expect(page.locator('main')).toContainText('synthetic.reward');
    expect(await query('SELECT season_id FROM pass_progress WHERE account_id=$1 AND season_id=$2',
      [seeded.accountId, season])).toEqual([]);
    expect(await sessionTimestamps()).toEqual(before);
    const form = page.locator(`form[action="${buy}"]`);
    let operation = await form.locator('[name=operation]').inputValue();
    let csrf = await form.locator('[name=_csrf]').inputValue();
    const post = (target, payload) => page.request.post(target, {
      form: { _csrf: csrf, operation, ...payload },
      headers: { origin: new URL(page.url()).origin },
      maxRedirects: 0,
    });
    await csrfRejected(page, buy, { operation });
    expect((await post(buy, { operation: 'invalid' })).status()).toBe(422);
    expect((await post(buy, { price: '1' })).status()).toBe(422);
    expect((await post(buy, {})).status()).toBe(403);
    expect(await query('SELECT id FROM wallet_operations WHERE id=$1', [operation])).toEqual([]);

    const funding = await paidTopup(page, lang, db);
    expect(funding.coins).toBe('1234');
    expect((await page.goto(pass)).status()).toBe(200);
    operation = await form.locator('[name=operation]').inputValue();
    csrf = await form.locator('[name=_csrf]').inputValue();
    await expect(form).toHaveAttribute('method', 'post');
    const buyUrl = new URL(buy, page.url()).href;
    const [first] = await Promise.all([
      page.waitForResponse(response => response.url() === buyUrl && response.request().method() === 'POST'),
      form.getByRole('button', { name: lang === 'pl' ? 'Kup ścieżkę płatną' : 'Buy paid track', exact: true }).click(),
    ]);
    expect(first.status()).toBe(303);
    expect(first.headers().location).toBe(pass);
    await expect(page).toHaveURL(new URL(pass, buyUrl).href);
    expect((await post(buy, {})).status()).toBe(303);
    expect((await post(buy, { operation: uuid7() })).status()).toBe(409);
    expect((await post(`${pass}/${uuid7()}/buy`, {})).status()).toBe(409);
    expect((await post(`${pass}/${season}/claim`, { operation: uuid7(), reward })).status()).toBe(404);
    expect((await page.goto(pass)).status()).toBe(200);
    await expect(form).toHaveCount(0);
    await expect(page.locator('form[action$="/claim"]')).toHaveCount(0);

    expect(await query(`SELECT paid,purchase_key,xp FROM pass_progress
      WHERE account_id=$1 AND season_id=$2`, [seeded.accountId, season]))
      .toEqual([{ paid: true, purchase_key: operation, xp: '0' }]);
    expect(await query(`SELECT SUM(e.available_delta)::bigint AS ledger,
      COUNT(*) FILTER (WHERE o.kind='pass_purchase' AND o.id=$2)::int AS debits
      FROM wallet_entries e
      JOIN wallet_operations o ON o.id=e.operation_id
      WHERE e.account_id=$1`, [seeded.accountId, operation]))
      .toEqual([{ ledger: '1134', debits: 1 }]);
    expect(await query("SELECT target,after->>'available' AS available FROM audit_log WHERE action='pass.purchase.debited' AND request_id=$1", [operation]))
      .toEqual([{ target: seeded.accountId, available: '1134' }]);
    expect(await query('SELECT id FROM pass_claims WHERE season_id=$1', [season])).toEqual([]);
  } finally {
    await db.query('DELETE FROM pass_progress WHERE season_id=$1', [season]);
    await db.query('DELETE FROM pass_rewards WHERE season_id=$1', [season]);
    await db.query('DELETE FROM pass_challenges WHERE season_id=$1', [season]);
    await db.query('DELETE FROM pass_seasons WHERE id=$1', [season]);
    await db.end();
  }
});
