import {createHash, randomBytes} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {mkdtempSync, rmSync, writeFileSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {expect, test} from '../../fixtures.mjs';
import {csrfRejected, pool} from '../../helpers.mjs';
import {uuid7} from '../../staff-commerce-helpers.mjs';
import {query, unique} from '../../staff-rbac-helpers.mjs';

const origin = process.env.PORTAL_E2E_ORIGIN || 'https://localhost:18443';
const digest = value => createHash('sha256').update(value).digest();

async function actor(context, lang, staffRole) {
  const session = randomBytes(32).toString('hex');
  const csrf = randomBytes(32).toString('hex');
  const grant = randomBytes(32).toString('hex');
  const [{id, password_hash}] = await query(
    `INSERT INTO accounts(email,password_hash,status,email_verified_at,locale)
     VALUES ($1,'commerce-test-only','active',now(),$2) RETURNING id,password_hash`,
    [`${unique()}@example.test`, lang],
  );
  if (staffRole) {
    await query("INSERT INTO account_roles(account_id,role_id) SELECT $1,id FROM roles WHERE key=$2", [id, staffRole]);
    const [{generation}] = await query(
      'INSERT INTO account_mfa_totp(account_id,secret,confirmed) VALUES($1,$2,true) RETURNING generation',
      [id, randomBytes(32)],
    );
    await query(
      `INSERT INTO account_mfa_tokens(token_hash,account_id,generation,credential_hash,purpose,expires_at)
       VALUES($1,$2,$3,$4,'grant',clock_timestamp()+interval '15 minutes')`,
      [digest(grant), id, generation, digest(password_hash)],
    );
  }
  await query(
    `INSERT INTO web_sessions(id_hash,account_id,idle_expires_at,absolute_expires_at,user_agent_summary,csrf_token,data,authorization_generation)
     SELECT $1,id,now()+interval '15 minutes',now()+interval '1 hour','commerce test',$3,$4,native_security_generation
     FROM accounts WHERE id=$2`,
    [digest(session), id, csrf, JSON.stringify(staffRole ? {mfa_grant: grant} : {})],
  );
  await context.addCookies([{name: '__Host-oteryn_session', value: session, url: origin, secure: true, httpOnly: true}]);
  return {id, csrf};
}

async function open(page, path, status = 200) {
  expect((await page.goto(path, {waitUntil: 'domcontentloaded'})).status()).toBe(status);
}
async function width(page) {
  expect(await page.evaluate(() => document.documentElement.scrollWidth)).toBeLessThanOrEqual(page.viewportSize().width);
}
async function post(page, path, form, status) {
  const response = await page.request.post(path, {form, headers: {origin}, maxRedirects: 0});
  expect(response.status()).toBe(status);
}

test('store orders and delivery are scoped to the signed-in buyer', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const {id: buyer} = await actor(page.context(), lang);
  const sellerContext = await browser.newContext({baseURL: origin, ignoreHTTPSErrors: true, locale: lang});
  const order = uuid7();
  const request = uuid7();
  const operation = uuid7();
  const line = uuid7();
  const revision = digest(order).toString('hex');
  const offer = `test.order.${order}`;
  const db = pool();
  const conn = await db.connect();
  try {
    await conn.query('BEGIN');
    await conn.query('INSERT INTO wallet_accounts(account_id) VALUES($1)', [buyer]);
    await conn.query("INSERT INTO wallet_operations(id,kind,account_id,amount) VALUES($1,'store_purchase',$2,-17)", [operation, buyer]);
    await conn.query("INSERT INTO store_purchase_requests(purchase_request_id,account_id,request_bytes,request_digest,result,answer_bytes) VALUES($1,$2,'{}',$3,'ACCEPTED','{}')", [request, buyer, revision]);
    await conn.query("INSERT INTO store_price_list_imports(catalog_revision,canonical_bytes,imported_by) VALUES($1,'{}',$2)", [revision, buyer]);
    await conn.query("INSERT INTO store_offers(catalog_revision,offer_key,currency,unit_price,max_quantity,purchasable,canonical_bytes) VALUES($1,$2,'COINS',17,1,true,'{}')", [revision, offer]);
    await conn.query("INSERT INTO store_orders(order_id,purchase_request_id,account_id,character_id,world_id,wallet_operation_id,wallet_idempotency_key,currency,total_price) VALUES($1,$2,$3,$4,$5,$6,$7,'COINS',17)", [order, request, buyer, uuid7(), uuid7(), operation, order]);
    await conn.query('INSERT INTO store_order_lines(line_id,order_id,catalog_revision,offer_key,quantity,unit_price) VALUES($1,$2,$3,$4,1,17)', [line, order, revision, offer]);
    await conn.query("INSERT INTO store_fulfilment_units(entitlement_id,line_id,unit_index,account_id,product_id,product_version,unit_kind,grant_key,grant_revision,quantity) VALUES($1,$2,0,$3,'premium.test','1','PREMIUM_TIME','test','1',1)", [uuid7(), line, buyer]);
    await conn.query('COMMIT');
    const base = `/${lang}/account/store-orders`;
    await open(page, '/account');
    await expect(page.locator(`a[href="${base}"]`).first()).toBeVisible();
    await open(page, base);
    await expect(page.getByRole('link', {name: offer})).toBeVisible();
    await width(page);
    await page.getByRole('link', {name: offer}).click();
    await expect(page.locator('main')).toContainText('premium.test');
    await expect(page.locator('main')).toContainText(lang === 'pl' ? 'Oczekuje na dostawę' : 'Awaiting delivery');
    await expect(page.locator('main')).toContainText(/\d{4}-\d{2}-\d{2} .*UTC/);
    await width(page);
    await open(page, `${base}/bad-id`, 422);
    await open(page, `${base}?before=bad-id`, 422);

    await actor(sellerContext, lang);
    const other = await sellerContext.newPage();
    await open(other, base);
    await expect(other.locator('main')).not.toContainText(offer);
    await open(other, `${base}/${order}`, 404);
    await expect(other.locator('main')).not.toContainText(offer);
  } finally {
    await conn.query('ROLLBACK');
    conn.release();
    await db.end();
    await sellerContext.close();
  }
});

function catalogCommand(args, account) {
  const result = spawnSync(process.env.PORTAL_E2E_BINARY, ['catalog', ...args, '--actor', account], {
    encoding: 'utf8', stdio: ['ignore', 'pipe', 'ignore'],
  });
  expect(result.status, 'catalog CLI succeeded').toBe(0);
  return result.stdout.trim();
}

function importGameCatalog(account, offer, price) {
  const directory = mkdtempSync(join(tmpdir(), 'portal-commerce-catalog-'));
  try {
    const path = join(directory, 'catalog.json');
    writeFileSync(path, JSON.stringify({
      schema: 'OTERYN_STORE_RUNTIME_CATALOGUE/v1',
      source: {fixture: offer},
      offers: [{key: offer, category: 'items', title: 'Exercise sword', price,
        currency: 'TIBIA_COIN', item: {key: 'oteryn:item.tibia.i28552', revision: 'definition-r1', charges: 1},
        source_line: 1}],
    }));
    return catalogCommand(['import', path], account);
  } finally {
    rmSync(directory, {recursive: true});
  }
}

test('staff stage proposals and activate verified Game revisions with replay fencing', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const staffContext = await browser.newContext({baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport});
  const offers = `/admin/store-catalog?lang=${lang}`;
  const activation = `/admin/store-activation?lang=${lang}`;
  const [{catalog_revision: previous}] = await query('SELECT catalog_revision FROM store_active_catalog WHERE id=1');
  try {
    const {id} = await actor(staffContext, lang, 'platform_admin');
    const admin = await staffContext.newPage();
    const {csrf: refusedCsrf} = await actor(page.context(), lang, 'security_admin');
    const offer = `commerce.${unique()}`;
    const current = importGameCatalog(id, offer, 21);
    const verified = importGameCatalog(id, offer, 41);
    catalogCommand(['activate', current], id);
    await open(page, offers, 403);
    await open(page, activation, 403);
    await open(admin, '/admin');
    await expect(admin.locator(`nav[data-panel=admin] a[href="${offers}"]`)).toBeVisible();
    await expect(admin.locator(`nav[data-panel=admin] a[href="${activation}"]`)).toBeVisible();
    await open(admin, `${offers}&unexpected=true`, 400);
    await open(admin, `${activation}&unexpected=true`, 400);
    await open(admin, offers);
    await expect(admin.locator('main')).toContainText(offer);
    await width(admin);
    const stage = admin.locator('main form');
    const csrf = await stage.locator('[name=_csrf]').inputValue();
    const fields = {_csrf: csrf, current, offer, price: '31', reason: 'Update verified test price'};
    await csrfRejected(admin, offers, fields);
    await post(admin, offers, {...fields, price: '-1'}, 422);
    await post(admin, offers, {...fields, reason: 'short'}, 422);
    await post(page, offers, {...fields, _csrf: refusedCsrf}, 403);
    await post(admin, offers, fields, 303);
    await post(admin, offers, fields, 303);
    expect(await query('SELECT unit_price::int AS price FROM store_price_proposals WHERE base_revision=$1 AND offer_key=$2', [current, offer])).toEqual([{price: 31}]);
    expect(await query('SELECT 1 FROM store_offers WHERE offer_key=$1 AND unit_price=31', [offer])).toEqual([]);
    expect((await query('SELECT catalog_revision FROM store_active_catalog WHERE id=1'))[0].catalog_revision).toBe(current);
    await open(admin, offers);
    await expect(admin.locator('main')).toContainText(`${offer} · 31 COINS`);

    async function activationForm(revision) {
      await open(admin, activation);
      const card = admin.locator('article').filter({hasText: revision});
      await expect(card).toContainText(revision === verified ? '21 → 41' : '41 → 21');
      await width(admin);
      const fields = await card.locator('form').evaluate(form => Object.fromEntries(new FormData(form)));
      return {...fields, reason: 'Approve verified Game catalog'};
    }
    const activate = await activationForm(verified);
    await csrfRejected(admin, activation, activate);
    await post(admin, activation, {...activate, revision: 'bad'}, 422);
    await post(page, activation, {...activate, _csrf: refusedCsrf}, 403);
    await post(admin, activation, activate, 303);
    const returnToA = await activationForm(current);
    await post(admin, activation, returnToA, 303);
    await post(admin, activation, activate, 409);
    expect((await query('SELECT catalog_revision FROM store_active_catalog WHERE id=1'))[0].catalog_revision).toBe(current);
    const audit = await query("SELECT action,before,after FROM audit_log WHERE actor=$1 AND staff_reason IS NOT NULL AND action IN ('store.catalog.price.staged','store.catalog.activated') ORDER BY created_at", [id]);
    expect(audit.map(row => row.action)).toEqual(['store.catalog.price.staged', 'store.catalog.activated', 'store.catalog.activated']);
    expect(audit[0].before.unit_price).toBe(21);
    expect(audit[0].after.unit_price).toBe(31);
  } finally {
    await query('UPDATE store_active_catalog SET catalog_revision=$1 WHERE id=1', [previous]);
    await staffContext.close();
  }
});

test('payment review supports scoped search, reconciliation, and four-eyes refund', async ({page, browser}, info) => {
  const lang = info.project.metadata.locale;
  const {id: account, csrf: refusedCsrf} = await actor(page.context(), lang, 'security_admin');
  const opts = {baseURL: origin, ignoreHTTPSErrors: true, locale: lang, viewport: info.project.use.viewport};
  const staff = await browser.newContext(opts);
  const reviewer = await browser.newContext(opts);
  try {
    const {csrf} = await actor(staff, lang, 'platform_admin');
    const {id: reviewerId, csrf: reviewerCsrf} = await actor(reviewer, lang, 'platform_admin');
    const admin = await staff.newPage();
    const second = await reviewer.newPage();
    const intent = uuid7();
    await query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,250,'PLN','succeeded')", [intent, account, `test-${intent}`]);
    const base = `/admin/payment-review?lang=${lang}`;
    await open(page, base, 403);
    await open(admin, '/admin');
    await expect(admin.locator(`nav[data-panel=admin] a[href="${base}"]`)).toBeVisible();
    await open(admin, `${base}&account=${account}`);
    await expect(admin.getByRole('link', {name: intent})).toBeVisible();
    await width(admin);
    await admin.getByRole('link', {name: intent}).click();
    await expect(admin.locator('main')).toContainText('2.50 PLN');
    await expect(admin.locator('main')).toContainText(account);
    await width(admin);
    await open(admin, `${base}&id=invalid`, 422);
    await open(admin, `${base}&id=${uuid7()}`, 404);
    await open(admin, `${base}&queue=true`);
    const reversed = uuid7();
    const quote = uuid7();
    const order = uuid7();
    const packageId = uuid7();
    const invoice = uuid7();
    const state = lang === 'pl' ? 'chargeback' : 'refunded';
    const kind = state === 'chargeback' ? state : 'refund';
    await query("INSERT INTO payment_intents(id,account_id,provider,provider_reference,amount,currency,state) VALUES($1,$2,'test',$3,250,'EUR',$4)",
      [reversed, account, `test-${reversed}`, state]);
    await query("INSERT INTO invoices(id,intent_id,account_id,amount,currency) VALUES($1,$2,$3,250,'EUR')", [invoice, reversed, account]);
    await query("INSERT INTO coin_purchase_quotes(id,payer_account_id,package_id,coins,minor_units,currency,exponent,environment,expires_at) VALUES($1,$2,$3,25,250,'EUR',3,'test',now())",
      [quote, account, packageId]);
    await query(`INSERT INTO coin_purchase_orders(id,payer_account_id,request_id,quote_id,package_id,coins,minor_units,currency,exponent,environment,payment_intent_id,state,receipt_id,review_required,code)
      VALUES($1,$2,$3,$4,$5,25,250,'EUR',3,'test',$6,$7,$8,true,'review')`,
      [order, account, uuid7(), quote, packageId, reversed, state, invoice]);
    await query("INSERT INTO coin_purchase_reversals(id,order_id,payer_account_id,kind,provider,provider_event_id,coins,debited,outcome) VALUES($1,$2,$3,$4,'test',$5,25,0,'held')",
      [uuid7(), order, account, kind, reversed]);
    await query("INSERT INTO coin_purchase_holds(account_id,order_id,reason) VALUES($1,$2,'reversal_insufficient_balance')", [account, order]);
    await open(admin, `${base}&queue=true`);
    await admin.getByRole('link', {name: reversed}).click();
    await expect(admin.locator('main')).toContainText('0.250 EUR');
    await expect(admin.locator('main')).toContainText('25 COINS');
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Blokada zakupów' : 'Purchase hold');
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Obciążenie zwrotne' : 'Refund reversal');
    if (state === 'refunded') await expect(admin.locator('main')).toContainText('Refund settlement verified');
    await width(admin);

    const reconcile = '/admin/payment-review';
    const reviewForm = admin.locator(`form[action^="${reconcile}?"]`);
    const reviewFields = {
      ...await reviewForm.evaluate(form => Object.fromEntries(new FormData(form))),
      reason: 'Verify recorded reversal and outstanding balance',
    };
    await csrfRejected(admin, reconcile, reviewFields);
    await post(page, reconcile, {...reviewFields, _csrf: refusedCsrf}, 403);
    await post(admin, reconcile, {...reviewFields, id: 'bad'}, 422);
    await post(admin, reconcile, {...reviewFields, reason: ''}, 422);
    await post(admin, reconcile, reviewFields, 303);
    await post(admin, reconcile, reviewFields, 303);
    await expect.poll(() => query('SELECT state FROM payment_intents WHERE id=$1', [reversed])).toEqual([{state}]);
    expect(await query("SELECT staff_reason FROM audit_log WHERE target=$1 AND action='commerce.payment.reviewed'", [reversed])).toEqual([{staff_reason: reviewFields.reason}]);
    await open(admin, `${base}&id=${reversed}`);
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Ostatnia zapisana weryfikacja' : 'Last review recorded');
    await open(admin, `${base}&queue=true`);
    await expect(admin.getByRole('link', {name: reversed})).toHaveCount(0);

    const common = {target: intent, reason: 'Verify payment and provider records'};
    const refund = '/admin/commerce/refund';
    const request = uuid7();
    await csrfRejected(admin, refund, {...common, request_id: request});
    await post(admin, refund, {...common, reason: '', _csrf: csrf, request_id: request}, 422);
    await post(admin, refund, {...common, _csrf: csrf, request_id: request}, 303);
    const approval = `/admin/commerce/approvals/${request}`;
    await open(second, '/admin/commerce');
    await csrfRejected(second, approval);
    const approvalReason = 'Independent reviewer approved the full refund';
    await post(admin, approval, {_csrf: csrf, reason: approvalReason}, 403);
    await post(second, approval, {_csrf: reviewerCsrf}, 422);
    await post(second, approval, {_csrf: reviewerCsrf, reason: 'short'}, 422);
    await post(second, '/admin/commerce/approvals/bad', {_csrf: reviewerCsrf}, 422);
    const approvalForm = second.locator(`form[action="${approval}"]`);
    await expect(approvalForm.locator('textarea[name=reason]')).toHaveAttribute('required', '');
    await post(second, approval, {_csrf: reviewerCsrf, reason: approvalReason}, 303);
    await post(second, approval, {_csrf: reviewerCsrf, reason: approvalReason}, 303);
    expect(await query("SELECT staff_reason FROM audit_log WHERE actor=$1 AND action='commerce.approved' AND target=$2", [reviewerId, intent])).toEqual([{staff_reason: approvalReason}]);
    await expect.poll(() => query('SELECT state FROM commerce_refunds WHERE operation_id=$1', [request]), {timeout: 30_000}).toEqual([{state: 'done'}]);
    expect((await query('SELECT intent_id FROM commerce_refunds WHERE operation_id=$1', [request]))[0].intent_id).toBe(intent);
    await open(admin, `${base}&id=${intent}`);
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Próby przekazania zwrotu operatorowi' : 'Provider delivery attempts');
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Wysłano do operatora' : 'Dispatched to provider');
    expect(await query('SELECT state FROM payment_intents WHERE id=$1', [intent])).toEqual([{state: 'succeeded'}]);
    // Project a definitive refusal from the same records written by the worker's refusal path.
    await query("UPDATE commerce_refunds SET state='pending',provider_reference=NULL,attempt_id=NULL,claimed_at=NULL WHERE operation_id=$1", [request]);
    await query("UPDATE commerce_operations SET result=result || jsonb_build_object('refund_outcome','refused','queued',false) WHERE id=$1", [request]);
    await open(admin, `${base}&id=${intent}`);
    await expect(admin.locator('main')).toContainText(lang === 'pl' ? 'Operator odmówił zwrotu' : 'Refund refused by provider');

  } finally {
    await staff.close();
    await reviewer.close();
  }
});
