import { login } from '../helpers.mjs';
import { after, before, digest, expect, one, test } from './fixtures.mjs';
import { authorize, token, tokens } from './oauth.mjs';

async function invalidGrant(response) {
  expect(response.status()).toBe(400);
  expect(await response.json()).toEqual({
    error: 'invalid_grant', error_description: 'The provided authorization grant is invalid.',
  });
}

test('authorization codes expire before exchange and cannot create a token family', async ({ page, lang, db, seed }) => {
  const account = await seed();
  await login(page, lang, account);
  const valid = await authorize(page, lang);
  const expired = await authorize(page, lang);
  const state = hash => one(db, `SELECT used_at,revoked_at,extract(epoch FROM expires_at-created_at)::int AS lifetime
    FROM oauth_authorization_codes WHERE code_hash=$1 AND account_id=$2`, [hash, account.id]);
  const move = (hash, seconds) => one(db, `UPDATE oauth_authorization_codes SET expires_at=clock_timestamp()+make_interval(secs=>$3)
    WHERE code_hash=$1 AND account_id=$2 RETURNING used_at`, [hash, account.id, seconds]);
  for (const form of [valid, expired]) {
    expect(await state(digest(form.code))).toEqual({ used_at: null, revoked_at: null, lifetime: 300 });
  }

  await move(digest(valid.code), before);
  await tokens(page, valid);
  expect((await state(digest(valid.code))).used_at).toBeInstanceOf(Date);
  const families = () => one(db, 'SELECT count(*)::int AS count FROM oauth_token_families WHERE account_id=$1', [account.id]);
  expect(await families()).toEqual({ count: 1 });
  await move(digest(expired.code), after);
  await invalidGrant(await token(page, expired));
  expect(await state(digest(expired.code))).toMatchObject({ used_at: null, revoked_at: null });
  expect(await families()).toEqual({ count: 1 });
});

test('expired refresh family cannot rotate even an unused refresh token', async ({ page, lang, db, seed }) => {
  const account = await seed();
  await login(page, lang, account);
  const valid = await tokens(page, await authorize(page, lang));
  const expired = await tokens(page, await authorize(page, lang));
  const family = hash => one(db, `SELECT f.id,f.revoked_at,extract(epoch FROM f.expires_at-f.created_at)::int AS lifetime
    FROM oauth_token_families f JOIN oauth_tokens t ON t.family_id=f.id
    WHERE f.account_id=$1 AND t.refresh_hash=$2`, [account.id, hash]);
  const move = (id, seconds) => one(db, `UPDATE oauth_token_families SET expires_at=clock_timestamp()+make_interval(secs=>$3)
    WHERE id=$1 AND account_id=$2 RETURNING revoked_at`, [id, account.id, seconds]);
  const refresh = issued => ({ grant_type: 'refresh_token', client_id: 'e2e-local',
    refresh_token: issued.refresh_token, scope: 'game:ticket' });
  const liveFamily = await family(digest(valid.refresh_token));
  const expiredFamily = await family(digest(expired.refresh_token));
  expect(liveFamily.lifetime).toBe(3600);
  expect(expiredFamily.lifetime).toBe(3600);

  await move(liveFamily.id, before);
  const rotated = await tokens(page, refresh(valid));
  expect(rotated.refresh_token !== valid.refresh_token, 'refresh token rotated').toBe(true);

  await move(expiredFamily.id, after);
  await invalidGrant(await token(page, refresh(expired)));
  expect(await one(db, `SELECT f.revoked_at,count(t.id)::int AS tokens,count(t.used_at)::int AS used
    FROM oauth_token_families f JOIN oauth_tokens t ON t.family_id=f.id
    WHERE f.id=$1 GROUP BY f.id`, [expiredFamily.id]))
    .toEqual({ revoked_at: null, tokens: 1, used: 0 });
});
