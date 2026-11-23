import {test, expect} from '../fixtures.mjs';
import {csrfRejected, login, pool, register, submit} from '../helpers.mjs';

test('account security reviews and revokes a native remembered device', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  const account = await register(page, lang);
  await login(page, lang, account);
  const path = `/${lang}/account/security/native-devices`;
  const db = pool();
  try {
    const owner = (await db.query('SELECT id FROM accounts WHERE email=$1', [account.email])).rows[0].id;
    const foreign = (await db.query(
      "INSERT INTO accounts(email,password_hash,status,locale,email_verified_at) VALUES(uuidv7()::text || '@example.test','unused-fixture','active','en',now()) RETURNING id"
    )).rows[0].id;
    // Persist only Platform fixtures. The browser runner supplies no native client or Game authority.
    const seedFamily = async (accountId, persistence) => (await db.query(
      `INSERT INTO device_sessions
       (id,account_id,client_id,enrollment_token_id,game_generation,native_generation,
        absolute_expires_at,idle_expires_at,persistence_class,device_public_key,device_key_thumbprint)
       SELECT uuidv7(),id,uuidv7(),uuidv7()::text,game_auth_generation,native_security_generation,
              extract(epoch FROM now()+interval '1 day')::bigint,extract(epoch FROM now()+interval '1 hour')::bigint,
              $2::text,CASE WHEN $2='session' THEN decode(repeat('01',32),'hex') END,
              CASE WHEN $2='session' THEN uuidv7()::text END
       FROM accounts WHERE id=$1 RETURNING id`, [accountId, persistence]
    )).rows[0].id;
    const ownFamily = await seedFamily(owner, 'remembered');
    const foreignFamily = await seedFamily(foreign, 'remembered');
    const temporaryFamily = await seedFamily(owner, 'session');
    await db.query(
      `INSERT INTO game_login_tickets
       (ticket_hash,account_id,game_generation,native_generation,audience,expires_at,device_family_id)
       SELECT sha256(uuidv7()::text::bytea),id,game_auth_generation::text,native_security_generation::text,
              'oteryn-native-game-gateway',now()+interval '10 minutes',$2
       FROM accounts WHERE id=$1`, [owner, ownFamily]
    );

    await page.goto('/account');
    await page.locator(`main a[href="${path}"]`).click();
    await expect(page).toHaveURL(new RegExp(`${path}$`));
    await expect(page.locator('header#portal-header')).toHaveCount(1);
    await expect(page.getByRole('heading', {level: 1})).toHaveCount(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
    const device = page.locator(`[data-native-family="${ownFamily}"]`);
    await expect(device.locator('[data-native-state]')).toHaveAttribute('data-native-state', 'current');
    await expect(page.locator(`[data-native-family="${foreignFamily}"], [data-native-family="${temporaryFamily}"]`)).toHaveCount(0);
    await expect(device.locator('form')).toHaveCount(0);
    await page.locator('main a[href$="return_to=native-devices"]').click();
    await submit(page, 'main form', {current_password: account.password});
    await expect(page).toHaveURL(new RegExp(`${path}$`));

    await csrfRejected(page, path, {family: ownFamily});
    const csrf = await device.locator('input[name="_csrf"]').inputValue();
    for (const [family, status] of [[foreignFamily, 404], ['invalid', 400]]) {
      const refused = await page.request.post(path, {
        form: {_csrf: csrf, family}, headers: {origin: new URL(page.url()).origin}, maxRedirects: 0,
      });
      expect(refused.status()).toBe(status);
    }
    await submit(page, `[data-native-family="${ownFamily}"] form`);
    await expect(device.locator('[data-native-state]')).toHaveAttribute('data-native-state', 'revoked');
    await expect(device.locator('form')).toHaveCount(0);
    const state = (await db.query(
      `SELECT d.revoked_at IS NOT NULL AS revoked, t.expires_at <= clock_timestamp() AS expired
       FROM device_sessions d JOIN game_login_tickets t ON t.device_family_id=d.id WHERE d.id=$1`, [ownFamily]
    )).rows[0];
    expect(state).toEqual({revoked: true, expired: true});
    await expect(page.locator('header#portal-header')).toHaveCount(1);
    await expect(page.getByRole('heading', {level: 1})).toHaveCount(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  } finally {
    await db.end();
  }
});
