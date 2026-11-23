import { run as withPortal } from '../api/harness.mjs';
import { expect, test as browserTest } from '../fixtures.mjs';
import { csrfRejected, login, submit } from '../helpers.mjs';
import { query } from '../staff-rbac-helpers.mjs';

const authority = 'oteryn-game-e2e';
const test = browserTest.extend({
  portal: async ({}, use, info) => {
    const port = 19540 + info.parallelIndex * 10;
    await withPortal({
      harnessEnv: {
        PORTAL_E2E_HTTP_PORT: String(port),
        PORTAL_E2E_INTERNAL_PORT: String(port + 1),
        PORTAL_E2E_ADMISSION_PORT: String(port + 2),
        PORTAL_E2E_GATEWAY_PORT: String(port + 3),
        PORTAL_API_NATIVE_PORT: String(port + 4),
        PORTAL_API_SMTP_PORT: String(port + 5),
        PORTAL_E2E_KEEP_DATABASE: '0',
      },
      portalOptions: {
        OTERYN_ACCOUNT_OVERVIEW_ENABLED: 'true',
        OTERYN_ACCOUNT_CREDENTIALS: 'true',
        OTERYN_LCFA_ENABLED: 'true',
        OTERYN_LCFA_SOURCE_AUTHORITY: authority,
        OTERYN_CHARACTER_SERVICES_ENABLED: 'true',
        OTERYN_CHARACTER_SERVICES_SCHEDULE_ENABLED: 'true',
        OTERYN_CHARACTER_SERVICES_CANCEL_ENABLED: 'true',
        OTERYN_CHARACTER_SERVICES_EXCHANGE_ENABLED: 'true',
        OTERYN_CHARACTER_SERVICES_PROJECTION_ENABLED: 'true',
        OTERYN_CHARACTER_SERVICES_COMMAND_AUTHORITY: authority,
        OTERYN_CHARACTER_SERVICES_PROJECTION_AUTHORITY: authority,
        OTERYN_CHARACTER_SERVICES_CLOCK_UNCERTAINTY_SECONDS: '0',
      },
    }, use);
  },
  baseURL: async ({ portal }, use) => use(portal.origin),
});

test.use({ trace: 'off', screenshot: 'off' });

test('the owner queues deletion and can follow its pending status from the account', async ({ page, portal }, info) => {
  test.setTimeout(180_000);
  const lang = info.project.metadata.locale;
  const owner = portal.account.accountId;
  const sql = (statement, params = []) => query(statement, params, portal.databaseURL);
  await login(page, lang, portal.account, portal.databaseURL);
  expect((await page.goto(`/${lang}/account/sudo`)).status()).toBe(200);
  await submit(page, 'main form', { current_password: portal.account.password });

  const [{ character, world }] = await sql('SELECT uuidv7() AS character, uuidv7() AS world');
  const path = `/${lang}/account/characters/${character}/services`;
  const operator = `/${lang}/admin/character-services`;
  const tick = async () => {
    const [{ stamp }] = await sql('SELECT floor(extract(epoch FROM clock_timestamp()))::text AS stamp');
    await sql(`
      UPDATE account_character_feed SET highest_epoch='1',watermark_epoch='1',
        source_authority=$1,complete_through=$2,observed_at=$2 WHERE id=1
    `, [authority, stamp]);
    await sql(`
      UPDATE character_service_feed SET epoch=1,complete_through=$1,observed_at=$1 WHERE id=1
    `, [stamp]);
    return stamp;
  };
  const stamp = await tick();
  await sql(`
    INSERT INTO account_character_snapshots(account_id,source_authority,projection_epoch,
      projection_revision,source_observed_at,content_digest)
    VALUES ($1,$2,'1','1',$3,$4)
  `, [owner, authority, stamp, Buffer.alloc(32)]);
  await sql(`
    INSERT INTO account_characters(account_id,character_id,world_id,name,availability)
    VALUES ($1,$2,$3,'Aldric Vale','AVAILABLE')
  `, [owner, character, world]);

  expect((await page.goto('/account')).status()).toBe(200);
  const services = page.locator(`main a[href="${path}"]`);
  await expect(services).toHaveText(lang === 'pl' ? 'Usługi postaci' : 'Character services');
  const missing = await page.request.get(path);
  expect(missing.status()).toBe(503);
  expect(await missing.text()).toContain(lang === 'pl' ? 'Niedostępne' : 'Unavailable');
  expect((await page.request.get(operator)).status()).toBe(403);

  // Supply the existing read model with a local fixture. The request itself goes through the browser form.
  await sql(`
    INSERT INTO character_service_states(account_id,character_id,epoch,revision,character_revision,snapshot_json)
    VALUES ($1,$2,1,1,'1',$3)
  `, [owner, character, JSON.stringify({
    contract_version: 2, operation: 'PublishCharacterServiceStateV2', source_authority: authority,
    account_id: owner, character_id: character, projection_epoch: '1', projection_revision: '1',
    source_observed_at: stamp, character_revision: '1', world_id: world, name: 'Aldric Vale',
    sex: null, lifecycle: 'ACTIVE', deletion_operation_id: null,
    deletion_scheduled_at: null, deletion_effective_at: null,
  })]);
  await tick();
  await services.click();
  await expect(page.locator('main')).toContainText('Aldric Vale');
  const selector = 'form:has(input[name=action][value=schedule])';
  const form = page.locator(selector);
  const fields = await form.evaluate(element => Object.fromEntries(new FormData(element)));
  await csrfRejected(page, path, { ...fields, confirm: 'yes' });
  const invalid = await page.request.post(path, {
    headers: { origin: portal.origin },
    form: { ...fields, confirm: 'yes', action: 'world-transfer' },
  });
  expect(invalid.status()).toBe(400);
  const accepted = page.waitForResponse(response => response.request().method() === 'POST'
    && new URL(response.url()).pathname === path && response.status() === 202);
  await submit(page, selector, { confirm: true });
  await accepted;

  const repeat = requestId => page.request.post(path, {
    headers: { origin: portal.origin },
    form: { ...fields, confirm: 'yes', request_id: requestId },
  });
  expect((await repeat(fields.request_id)).status()).toBe(202);
  const [{ request_id: newRequestId }] = await sql('SELECT uuidv7() AS request_id');
  expect((await repeat(newRequestId)).status()).toBe(409);

  const orders = await sql(`
    SELECT c.id,c.state,c.receipt_json,o.financial_state FROM character_commands c
    JOIN character_service_orders o ON o.operation_id=c.id WHERE o.character_id=$1
  `, [character]);
  expect(orders).toHaveLength(1);
  const [order] = orders;
  expect(order).toMatchObject({ state: 'PENDING', receipt_json: null, financial_state: 'FREE' });
  await page.locator(`main a[href="${path}#history"]`).click();
  await expect(page.locator('#history')).toHaveText(lang === 'pl' ? 'Ostatnie zgłoszenia' : 'Recent requests');
  await expect(page.locator('main')).toContainText(order.id);
  await expect(page.locator(selector)).toHaveCount(0);
  expect(await page.evaluate(() => document.documentElement.scrollWidth))
    .toBeLessThanOrEqual(page.viewportSize().width);
});
