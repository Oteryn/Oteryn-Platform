import {expect, test} from '../../fixtures.mjs';
import {createHash, randomBytes} from 'node:crypto';
import {pool} from '../../helpers.mjs';
import {unique} from '../../staff-rbac-helpers.mjs';

const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
const visit = (page, url) => page.goto(url, {waitUntil: 'domcontentloaded'});
const digest = text => createHash('sha256').update(text).digest();

// Seed durable browser state in the disposable database using the same proof
// fields as admin/src/test_staff.rs. Requests still pass the real route guards.
async function panelActor(context, lang, role = null) {
  const session = randomBytes(32).toString('hex');
  const csrf = randomBytes(32).toString('hex');
  const grant = randomBytes(32).toString('hex');
  const db = pool();
  const connection = await db.connect();
  try {
    await connection.query('BEGIN');
    const {rows: [account]} = await connection.query(
      `INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
       VALUES ($1,'panel-test-only','active',now(),$2)
       RETURNING id,password_hash`,
      [`${unique()}@example.test`, lang],
    );
    if (role) {
      const membership = await connection.query(
        "INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2",
        [account.id, role],
      );
      expect(membership.rowCount).toBe(1);
      const {rows: [{generation}]} = await connection.query(
        'INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) RETURNING generation',
        [account.id, randomBytes(32)],
      );
      await connection.query(
        `INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
         VALUES($1,$2,$3,$4,'grant',clock_timestamp()+interval '15 minutes')`,
        [digest(grant), account.id, generation, digest(account.password_hash)],
      );
    }
    await connection.query(
      `INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation)
       SELECT $1,id,now()+interval '15 minutes',now()+interval '1 hour','panel test',$3,$4,native_security_generation
       FROM accounts WHERE id=$2`,
      [digest(session), account.id, csrf, JSON.stringify(role ? {mfa_grant: grant} : {})],
    );
    await connection.query('COMMIT');
    await context.addCookies([{
      name: '__Host-oteryn_session', value: session, url: origin, secure: true, httpOnly: true,
    }]);
    return account.id;
  } catch (error) {
    await connection.query('ROLLBACK');
    throw error;
  } finally {
    connection.release();
    await db.end();
  }
}

const pages = [
  ['/admin/operations/statistics', 'Platform statistics', 'Statystyki platformy', '?days=0'],
  ['/admin/operations/rate-limits', 'Rate limits', 'Limity żądań', '?account=other'],
  ['/admin/operations/flags', 'Effective feature flags', 'Aktywne ustawienia funkcji', '?name=OTERYN_PUBLIC_ORIGIN'],
  ['/admin/operations/health', 'System health', 'Stan systemu', '?account=other'],
  ['/admin/analytics', 'Game analytics', 'Analityka gry', '?world=invalid'],
];
const csv = '/admin/operations/rate-limits.csv';

test('platform operations are discoverable and enforce staff-only, read-only access', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  await panelActor(page.context(), lang, 'support');
  expect((await visit(page, '/admin')).status()).toBe(200);
  for (const [path] of pages) {
    await expect(page.locator(`nav[data-panel="admin"] a[href="${path}"]`)).toHaveCount(0);
  }
  for (const [path] of pages) {
    expect((await page.request.get(path)).status(), path).toBe(403);
  }
  expect((await page.request.get(csv)).status()).toBe(403);

  const staff = await browser.newContext({
    baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport,
  });
  try {
    await panelActor(staff, lang, 'platform_admin');
    const admin = await staff.newPage();
    expect((await visit(admin, '/admin')).status()).toBe(200);
    for (const [path, en, pl] of pages) {
      await expect(admin.locator('nav[data-panel="admin"]').getByRole('link', {
        name: lang === 'en' ? en : pl, exact: true,
      })).toHaveAttribute('href', path);
    }
    for (const [path, en, pl, bad] of pages) {
      expect((await visit(admin, path)).status(), path).toBe(200);
      await expect(admin.locator('html')).toHaveAttribute('lang', lang);
      await expect(admin.getByRole('heading', {level: 1})).toBeVisible();
      if (path !== '/admin/analytics') {
        await expect(admin.getByRole('heading', {level: 1})).toHaveText(lang === 'en' ? en : pl);
      }
      expect(await admin.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
      expect((await staff.request.get(path + bad)).status(), path).toBe(400);
      // GET filters and exports are read-only. Reject forged POSTs with no CSRF token.
      expect([403, 405]).toContain((await staff.request.post(path, {form: {days: '1'}})).status());
    }
    await visit(admin, '/admin/operations/statistics');
    await admin.locator('select[name="days"]').selectOption('1');
    await admin.locator('form button').click();
    await expect(admin).toHaveURL(/days=1$/);
    await expect(admin.locator('tbody tr')).toHaveCount(1);

    const exportResponse = await staff.request.get(csv);
    expect(exportResponse.status()).toBe(200);
    expect(exportResponse.headers()['content-type']).toContain('text/csv');
    expect(await exportResponse.text()).toMatch(/^policy,limit,window_seconds,saturated_key_windows_last_hour\r\n/);
    expect((await staff.request.get(csv + '?account=other')).status()).toBe(400);
    expect([403, 405]).toContain((await staff.request.post(csv, {form: {days: '1'}})).status());
  } finally {
    await staff.close();
  }
});

test('analytics drills into the selected qualified UTC bucket', async ({page}, info) => {
  const lang = info.project.metadata.locale;
  await panelActor(page.context(), lang, 'platform_admin');
  const authority = process.env.OTERYN_ADMIN_ANALYTICS_AUTHORITY;
  const privacy = process.env.OTERYN_ADMIN_ANALYTICS_PRIVACY;
  const family = 'world_activity_hourly';
  const db = pool();
  const connection = await db.connect();
  let fixture;
  try {
    await connection.query('BEGIN');
    const {rows: [{world, now}]} = await connection.query(
      'SELECT uuidv7() AS world, floor(extract(epoch FROM statement_timestamp()))::int AS now',
    );
    const start = Math.floor(now / 3600) * 3600 - 26 * 3600;
    const end = start + 3600;
    await connection.query(
      'INSERT INTO admin_analytics.fence(authority,epoch,privacy_floor) VALUES($1,1,$2) ON CONFLICT(authority) DO NOTHING',
      [authority, privacy],
    );
    const {rows: [fence]} = await connection.query(
      'SELECT active_generation AS generation,epoch FROM admin_analytics.fence WHERE authority=$1 FOR UPDATE',
      [authority],
    );
    let generation = fence.generation;
    if (!generation) {
      const {rows: [created]} = await connection.query(
        `INSERT INTO admin_analytics.generations(id,authority,epoch,baseline_digest,baseline_cut,accepted_at)
         VALUES(uuidv7(),$1,$2,$3,1,now()) RETURNING id`,
        [authority, fence.epoch, 'a'.repeat(64)],
      );
      generation = created.id;
      await connection.query('UPDATE admin_analytics.fence SET active_generation=$2 WHERE authority=$1', [authority, generation]);
    }
    await connection.query(
      `INSERT INTO admin_analytics.watermarks(authority,family,world,epoch,privacy,cut,observed,digest)
       VALUES($1,$2,$3,$4,$5,$6,$6,$7)`,
      [authority, family, world, fence.epoch, privacy, now, 'c'.repeat(64)],
    );
    for (const bucket of [start - 3600, start]) {
      await connection.query(
        `INSERT INTO admin_analytics.subjects(authority,family,world,bucket_start,bucket_end,epoch,revision,privacy,observed,commit_digest,erased,generation)
         VALUES($1,$2,$3,$4,$5,$6,1,$7,$8,$9,false,$10)`,
        [authority, family, world, bucket, bucket + 3600, fence.epoch, privacy, now, 'b'.repeat(64), generation],
      );
      await connection.query(
        `INSERT INTO admin_analytics.rows(generation,authority,family,world,bucket_start,bucket_end,key,state,values,epoch,revision,privacy,observed)
         VALUES($1,$2,$3,$4,$5,$6,'world','visible',$7,$8,1,$9,$10)`,
        [generation, authority, family, world, bucket, bucket + 3600, JSON.stringify(['5', '3600', '5', '5000']), fence.epoch, privacy, now],
      );
    }
    fixture = {world, start, end};
    await connection.query('COMMIT');
  } catch (error) {
    await connection.query('ROLLBACK');
    throw error;
  } finally {
    connection.release();
    await db.end();
  }
  const {world, start, end} = fixture;
  const path = '/admin/analytics/activity';
  const selected = `?world=${world}&family=${family}&start=${start}&end=${end}`;
  expect((await visit(page, path + selected.replace(`start=${start}`, `start=${start - 3600}`))).status()).toBe(200);
  const rows = page.locator('tr[data-state="visible"]');
  await expect(rows).toHaveCount(2);
  await expect(rows.last().getByRole('link')).toHaveAttribute('href', path + selected);
  await rows.last().getByRole('link').click();
  await expect(page).toHaveURL(new RegExp(`start=${start}&end=${end}$`));
  await expect(rows).toHaveCount(1);
  await expect(rows.first()).toContainText('5000');
});
