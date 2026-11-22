import fs from 'node:fs/promises';
import { login, submit } from '../helpers.mjs';
import { uuid7 } from '../staff-commerce-helpers.mjs';
import { action, enrollMfa } from '../staff-rbac-helpers.mjs';
import { after, before, digest, expect, one, test } from './fixtures.mjs';
import { authorize, tokens } from './oauth.mjs';

const nativeClient = '019a0000-0000-7000-8000-0000000c11e7';

async function nativeAccess(page, lang, seedEntity) {
  expect(await seedEntity('oauth-native-client', { clientId: nativeClient })).toMatchObject({
    clientId: nativeClient, enabled: true, firstParty: true, publicClient: true, nativeTicket: true,
  });
  return (await tokens(page, await authorize(page, lang, 'game:ticket', nativeClient))).access_token;
}

async function issue(page, bearer) {
  const response = await page.request.post('/api/v1/game-auth/tickets', {
    headers: { authorization: `Bearer ${bearer}` }, data: { protocol_version: 1 },
  });
  expect(response.headers()['cache-control']).toBe('no-store, private');
  return response;
}

test('a game ticket crosses the redemption gate only before expiry', async ({ page, lang, db, seed, seedEntity }) => {
  const account = await seed();
  await login(page, lang, account);
  const bearer = await nativeAccess(page, lang, seedEntity);
  const issued = await issue(page, bearer);
  expect(issued.status()).toBe(200);
  const ticket = await issued.json();
  expect(ticket.expires_in).toBe(60);
  expect(ticket.protocol_version).toBe(1);
  const hash = digest(ticket.ticket);
  const state = () => one(db, 'SELECT used_at,attempt_ref FROM game_login_tickets WHERE ticket_hash=$1 AND account_id=$2',
    [hash, account.id]);
  const move = seconds => one(db, `UPDATE game_login_tickets SET expires_at=clock_timestamp()+make_interval(secs=>$3)
    WHERE ticket_hash=$1 AND account_id=$2 RETURNING attempt_ref`, [hash, account.id, seconds]);
  const serviceToken = (await fs.readFile(process.env.PORTAL_E2E_EXPIRY_ADMISSION_BEARER_FILE, 'utf8')).trim();
  const attempt = uuid7();
  const character = uuid7();
  const request = () => page.request.post('http://127.0.0.1:8091/internal/v1/game-auth/native-admissions', {
    headers: { authorization: `Bearer ${serviceToken}` },
    data: {
      protocol_version: 2, game_login_ticket: ticket.ticket, attempt_ref: attempt, character_id: character,
      channel_id: null, offer: { client_build: 'expiry', client_platform: 'windows',
        transports: [{ protocol_major: 1, transport_profile: 1, alpn: 'oteryn-game/1' }] },
    },
  });
  await move(before);
  const live = await request();
  // The valid ticket reaches the live Game character-authority gate. No
  // authority is asserted for this seeded account and nothing commits.
  expect(live.status()).toBe(503);
  expect((await live.json()).error).toEqual({
    code: 'NATIVE_LOGIN_ROUTE_UNAVAILABLE', public_class: 'TEMPORARILY_UNAVAILABLE',
  });
  expect(await state()).toEqual({ used_at: null, attempt_ref: null });

  await move(after);
  const expired = await request();
  expect(expired.status()).toBe(401);
  expect(expired.headers()['cache-control']).toBe('no-store, private');
  expect(await expired.json()).toEqual({ protocol_version: 2, attempt_ref: attempt, error: {
    code: 'NATIVE_LOGIN_AUTHENTICATION_REQUIRED', public_class: 'AUTHENTICATION_REQUIRED',
  } });
  expect(await state()).toEqual({ used_at: null, attempt_ref: null });
  expect(await one(db, 'SELECT count(*)::int AS count FROM native_admissions WHERE account_id=$1', [account.id]))
    .toEqual({ count: 0 });
});

test('time-bound suspension blocks ticket issuance only while effective', async ({ page, browser, lang, db, seed, seedEntity }, info) => {
  const account = await seed();
  await login(page, lang, account);
  const bearer = await nativeAccess(page, lang, seedEntity);

  const staff = await seed();
  expect((await db.query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key='platform_admin'", [staff.id])).rowCount).toBe(1);
  const staffContext = await browser.newContext({
    baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  });
  try {
    const admin = await staffContext.newPage();
    await login(admin, lang, staff);
    await enrollMfa(admin, lang, staff);
    const firstRecoveryCode = admin.locator('textarea[readonly]').first();
    await expect(firstRecoveryCode).toBeVisible();
    const code = await firstRecoveryCode.inputValue();
    await login(admin, lang, staff);
    await expect(admin).toHaveURL(new RegExp(`/${lang}/mfa/challenge$`));
    await submit(admin, 'form[action$="/mfa/challenge"]', { code });
    expect((await admin.goto('/admin/moderation/enforcement/create')).status()).toBe(200);
    const start = Math.floor(Date.now() / 1000) - 60;
    await action(admin, 'form:has(input[value=sanction])', {
      target: account.id, category: 'suspension', status: 'active', public_reason: 'Expiry fixture',
      moderator_notes: 'Time-bound suspension qualification', effective_at: String(start),
      expires_at: String(start + 3600),
    });
    await expect(admin).toHaveURL(/enforcement\/[a-f0-9-]+$/);
  } finally {
    await staffContext.close();
  }

  const read = () => one(db, `SELECT id,status,extract(epoch FROM expires_at-effective_at)::int AS lifetime
    FROM sanctions WHERE account_id=$1 AND category='suspension'`, [account.id]);
  const sanction = await read();
  expect(sanction).toMatchObject({ status: 'active', lifetime: 3600 });
  const move = seconds => one(db, `UPDATE sanctions SET expires_at=clock_timestamp()+make_interval(secs=>$3)
    WHERE id=$1 AND account_id=$2 RETURNING status`, [sanction.id, account.id, seconds]);
  const tickets = () => one(db, 'SELECT count(*)::int AS count FROM game_login_tickets WHERE account_id=$1', [account.id]);

  await move(before);
  const blocked = await issue(page, bearer);
  expect(blocked.status()).toBe(401);
  expect(await blocked.json()).toEqual({ error: 'unauthenticated' });
  expect(await tickets()).toEqual({ count: 0 });

  await move(after);
  const allowed = await issue(page, bearer);
  expect(allowed.status()).toBe(200);
  expect((await allowed.json()).expires_in).toBe(60);
  expect(await tickets()).toEqual({ count: 1 });
  expect((await read()).status).toBe('active'); // The time predicate removes the block immediately.
});
