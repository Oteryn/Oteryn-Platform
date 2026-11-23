import crypto from 'node:crypto';
import { test, expect } from '../fixtures.mjs';
import { csrfRejected, submit, waitForLoginBudget } from '../helpers.mjs';
import { query, seededAccount, seededLogin, seededStaff } from '../staff-rbac-helpers.mjs';

const png = {
  name: 'screenshot.png',
  mimeType: 'image/png',
  buffer: Buffer.from('iVBORw0KGgoAAAANSUhEUgAAABQAAAAUCAYAAACNiR0NAAAAHklEQVR4nGNQzrP5T03MMGrgqIGjBo4aOGrgSDUQANpkzt/P0xIDAAAAAElFTkSuQmCC', 'base64'),
};

const formFields = form => form.evaluate(element => Object.fromEntries(new FormData(element)));
const pathOf = page => new URL(page.url()).pathname;
const post = (page, path, data) => page.request.post(path, {
  ...data, headers: { origin: new URL(page.url()).origin }, maxRedirects: 0,
});

test('ticket attachment and staff reply through the player and support panels', async ({ page, browser }, info) => {
  const lang = info.project.metadata.locale;
  const owner = await seededAccount(lang);
  await seededLogin(page, lang, owner);
  await page.goto(`/${lang}/support/tickets/create`);
  const subject = `Display question ${crypto.randomUUID()}`;
  await submit(page, 'main form', { subject, body: 'The screenshot shows the display issue.' });
  await expect(page).toHaveURL(/support\/tickets\/[a-f0-9-]+$/);
  const ticket = pathOf(page);
  const id = ticket.split('/').at(-1);
  const screenshots = () => query(
    'SELECT id, uploader_id FROM ticket_screenshots WHERE ticket_id=$1 ORDER BY id', [id],
  );
  const uploadForm = page.locator('#screenshots form');
  const { _csrf, request_key } = await formFields(uploadForm);
  const upload = (actor, fields) => post(actor, `${ticket}/attachments`, { multipart: fields });
  expect((await upload(page, { request_key, image: png })).status()).toBe(403);
  expect((await upload(page, {
    _csrf, request_key, image: { ...png, buffer: Buffer.from('invalid image') },
  })).status()).toBe(422);
  expect(await screenshots()).toEqual([]);

  await uploadForm.locator('input[type=file]').setInputFiles(png);
  await uploadForm.getByRole('button', { name: lang === 'pl' ? 'Dodaj zrzut ekranu' : 'Attach screenshot' }).click();
  const image = page.locator('#screenshots img');
  await expect(image).toBeVisible();
  const src = await image.getAttribute('src');
  expect((await page.request.get(src)).headers()['cache-control']).toContain('no-store');
  expect((await upload(page, { _csrf, request_key, image: png })).status()).toBe(302);
  expect(await screenshots()).toEqual([{ id: src.split('/').at(-1), uploader_id: owner.id }]);

  const contextOptions = {
    baseURL: new URL(page.url()).origin, ignoreHTTPSErrors: true,
    locale: lang, viewport: info.project.use.viewport,
  };
  const support = await browser.newContext(contextOptions);
  const other = await browser.newContext(contextOptions);
  try {
    const staff = await support.newPage();
    const staffId = await seededStaff(staff, lang, 'support');
    const stranger = await other.newPage();
    await seededLogin(stranger, lang, await seededAccount(lang));
    expect((await stranger.request.get(src)).status()).toBe(404);
    const staffPath = `/admin/support/tickets/${id}`;
    expect((await stranger.request.get(staffPath)).status()).toBe(403);
    expect((await staff.goto('/admin/support/tickets')).status()).toBe(200);
    await staff.getByRole('link', { name: subject, exact: true }).click();
    await expect(staff).toHaveURL(url => url.pathname === staffPath);

    const reply = staff.locator('form:has(input[value=reply])');
    const body = 'Support reviewed the screenshot.';
    const fields = { ...await formFields(reply), body };
    const effects = () => query(`
      SELECT lock_version::int AS lock_version,
        (SELECT count(*)::int FROM ticket_messages WHERE ticket_id=tickets.id AND author_kind='staff') AS replies,
        (SELECT count(*)::int FROM audit_log WHERE target=$1::uuid::text AND action='support.ticket_replied') AS audit
      FROM tickets WHERE id=$1::uuid
    `, [id]);
    const [initial] = await effects();
    expect(initial).toMatchObject({ replies: 0, audit: 0 });
    await stranger.goto(`/${lang}/support/tickets/create`);
    const strangerCsrf = await stranger.locator('main input[name=_csrf]').inputValue();
    expect((await post(stranger, staffPath, { form: { ...fields, _csrf: strangerCsrf } })).status()).toBe(403);
    await csrfRejected(staff, staffPath, fields);
    expect((await post(staff, staffPath, { form: { ...fields, body: '' } })).status()).toBe(422);
    expect(await effects()).toEqual([initial]);

    await reply.locator('[name=body]').fill(body);
    await reply.getByRole('button').click();
    await expect(staff.locator('main')).toContainText(body);
    expect((await post(staff, staffPath, { form: fields })).status()).toBe(409);
    expect(await effects()).toEqual([{ lock_version: initial.lock_version + 1, replies: 1, audit: 1 }]);
    expect(await query(
      'SELECT author_account_id, body FROM ticket_messages WHERE ticket_id=$1 AND author_kind=$2',
      [id, 'staff'],
    )).toEqual([{ author_account_id: staffId, body }]);
    await page.reload();
    await expect(page.locator('main')).toContainText(body);
    await expect(image).toHaveCount(1);
    expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(info.project.use.viewport.width);
  } finally {
    await Promise.all([support.close(), other.close()]);
  }
});

test('OAuth consent, code exchange and scope-preserving rotation', async ({ page }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seededAccount(lang);
  await seededLogin(page, lang, account);
  const verifier = crypto.randomBytes(32).toString('base64url');
  const redirect = 'https://localhost:18443/__oauth/callback';
  const scope = 'game:ticket';
  const client_id = 'e2e-local';
  const state = crypto.randomUUID();
  const fields = {
    response_type: 'code', client_id, redirect_uri: redirect, scope, state,
    code_challenge_method: 'S256',
    code_challenge: crypto.createHash('sha256').update(verifier).digest('base64url'),
  };
  const ledger = async () => (await query(`
    SELECT
      (SELECT count(*)::int FROM oauth_authorization_codes WHERE account_id=$1) AS codes,
      (SELECT count(*)::int FROM oauth_token_families WHERE account_id=$1) AS families,
      (SELECT count(*)::int FROM oauth_tokens t JOIN oauth_token_families f ON f.id=t.family_id
        WHERE f.account_id=$1) AS tokens
  `, [account.id]))[0];

  // Consent and token requests share the browser login source budget.
  await waitForLoginBudget(8);
  expect((await page.goto(`/oauth/authorize?${new URLSearchParams(fields)}`)).status()).toBe(200);
  await page.route('**/__oauth/callback?**', route => route.fulfill({ status: 200, body: 'Local OAuth callback' }));
  await csrfRejected(page, '/oauth/authorize', { ...fields, decision: 'approve' });
  expect(await ledger()).toEqual({ codes: 0, families: 0, tokens: 0 });
  await page.locator('main form [name=decision][value=approve]').click();
  await expect(page).toHaveURL(/\/__oauth\/callback\?/);
  const result = new URL(page.url()).searchParams;
  expect(result.get('state')).toBe(state);
  const code = result.get('code');
  expect(Boolean(code)).toBe(true);

  const token = form => page.request.post('/oauth/token', { form, maxRedirects: 0 });
  const exchange = { grant_type: 'authorization_code', client_id, redirect_uri: redirect, code };
  const malformed = await token(exchange);
  expect(malformed.status()).toBe(400);
  expect((await malformed.json()).error).toBe('invalid_request');
  expect(await ledger()).toEqual({ codes: 1, families: 0, tokens: 0 });

  const responses = await Promise.all([
    token({ ...exchange, code_verifier: verifier }), token({ ...exchange, code_verifier: verifier }),
  ]);
  expect(responses.map(response => response.status()).sort()).toEqual([200, 400]);
  const issued = await responses.find(response => response.ok()).json();
  expect((await responses.find(response => !response.ok()).json()).error).toBe('invalid_grant');
  expect(await ledger()).toEqual({ codes: 1, families: 1, tokens: 1 });

  const refresh = { client_id, grant_type: 'refresh_token', refresh_token: issued.refresh_token };
  const refused = await token({ ...refresh, scope: 'game:account-services' });
  expect(refused.status()).toBe(400);
  expect((await refused.json()).error).toBe('invalid_grant');
  expect(await ledger()).toEqual({ codes: 1, families: 1, tokens: 1 });
  expect((await token({ ...refresh, scope })).status()).toBe(200);
  expect(await ledger()).toEqual({ codes: 1, families: 1, tokens: 2 });
  expect(await query('SELECT scope FROM oauth_token_families WHERE account_id=$1', [account.id])).toEqual([{ scope }]);
});

test('sandbox checkout and provider event record one unpaid outcome', async ({ page, request }, info) => {
  const lang = info.project.metadata.locale;
  const account = await seededAccount(lang);
  await seededLogin(page, lang, account);
  const path = `/${lang}/shop/coins`;
  expect((await page.goto(path)).status()).toBe(200);
  const checkout = page.locator(`main form[action="${path}"]`).first();
  const fields = await formFields(checkout);
  const intent = () => query(`
    SELECT p.id, p.state, p.amount, p.currency, p.provider_reference, p.updated_at
    FROM store_web_topups t JOIN payment_intents p ON p.id=t.payment_intent_id
    WHERE t.operation_id=$1 AND p.account_id=$2
  `, [fields.operation, account.id]);
  await csrfRejected(page, path, fields);
  expect((await post(page, path, { form: { ...fields, operation: 'invalid' } })).status()).toBe(422);
  expect(await query('SELECT id FROM payment_intents WHERE account_id=$1', [account.id])).toEqual([]);

  await checkout.getByRole('button').click();
  await expect(page).toHaveURL(/\/shop\/test-checkout\/[a-f0-9-]+$/);
  const intents = await intent();
  expect(intents).toHaveLength(1);
  const [created] = intents;
  expect(pathOf(page)).toBe(`/${lang}/shop/test-checkout/${created.id}`);
  const replay = await post(page, path, { form: fields });
  expect(replay.status()).toBe(303);
  expect(replay.headers().location).toBe(pathOf(page));
  expect(await intent()).toEqual([created]);
  expect(await query('SELECT id FROM payment_intents WHERE account_id=$1', [account.id])).toEqual([{ id: created.id }]);

  const event = {
    event_id: crypto.randomUUID(), intent_id: created.id, provider_reference: created.provider_reference,
    amount: { minor_units: Number(created.amount), currency: created.currency }, state: 'failed',
  };
  const raw = JSON.stringify(event);
  const sign = body => crypto.createHmac('sha256', process.env.OTERYN_SHOP_TEST_PAYMENT_KEY).update(body).digest('base64');
  const deliver = (body, signature) => request.post('/__test/payments/webhook', {
    data: body,
    headers: { 'content-type': 'application/json', ...(signature ? { 'x-test-payment-signature': signature } : {}) },
  });
  const paymentEffects = async () => (await query(`
    SELECT
      (SELECT count(*)::int FROM payment_events WHERE intent_id=$1::uuid) AS events,
      (SELECT count(*)::int FROM invoices WHERE intent_id=$1::uuid) AS invoices,
      (SELECT count(*)::int FROM outbox WHERE idempotency_key=$1::uuid::text AND kind='wallet.top_up') AS jobs,
      (SELECT count(*)::int FROM wallet_entries WHERE operation_id=$1::uuid) AS entries
  `, [created.id]))[0];
  expect((await deliver(raw)).status()).toBe(400);
  const malformed = JSON.stringify({ ...event, amount: null });
  expect((await deliver(malformed, sign(malformed))).status()).toBe(400);
  const mismatched = JSON.stringify({
    ...event, amount: { ...event.amount, minor_units: event.amount.minor_units + 1 },
  });
  expect((await deliver(mismatched, sign(mismatched))).status()).toBe(400);
  expect(await intent()).toEqual([created]);
  expect(await paymentEffects()).toEqual({ events: 0, invoices: 0, jobs: 0, entries: 0 });

  const signature = sign(raw);
  const deliveries = await Promise.all([deliver(raw, signature), deliver(raw, signature)]);
  expect(deliveries.map(response => response.status())).toEqual([204, 204]);
  const [failed] = await intent();
  expect(failed.state).toBe('failed');
  expect(await paymentEffects()).toEqual({ events: 1, invoices: 0, jobs: 0, entries: 0 });
  expect(await query('SELECT provider_event_id, state FROM payment_events WHERE intent_id=$1', [created.id]))
    .toEqual([{ provider_event_id: event.event_id, state: 'failed' }]);
  await page.reload();
  await expect(page.locator('main')).toContainText(lang === 'pl'
    ? 'Płatność odrzucona lub anulowana. Nie przyznano coinów.'
    : 'Payment rejected or cancelled. No coins were credited.');
  await expect(page.locator('main form[action*="/shop/test-checkout/"]')).toHaveCount(0);
});
